//! Agent executor for managing tool-using agent interactions.
//!
//! This module provides a reusable pattern for tools-using agents where events
//! are sent through a channel back to the caller as they occur. The executor handles
//! the complexity of maintaining conversation history, processing tool calls,
//! and managing the interaction loop.

use crate::coder::error::CoderError;
use rig::completion::CompletionResponse;
use rig::one_or_many::OneOrMany; // Corrected import path
use rig::{
    agent::Agent,
    completion::{Completion as _, CompletionModel, ToolDefinition},
    message::{AssistantContent, Message, ToolCall, ToolResult, ToolResultContent, UserContent},
};
use serde_json::json;
use std::{collections::VecDeque, sync::Arc}; // Removed unused std imports
use tokio::sync::mpsc::Sender; // Removed unused mpsc imports
use tracing::{debug, error, info, instrument, warn};

/// Events that can be emitted by the AgentExecutor during execution.
#[derive(Debug, Clone)]
pub enum ExecutorEvent {
    /// The agent has produced some explanatory text (thought process).
    Thinking { text: String },

    /// The agent is attempting to call a tool.
    ToolCallAttempted { call: ToolCall },

    /// A tool call initiated by the agent has completed.
    ToolCallCompleted {
        /// The ID matching the corresponding `ToolCallAttempted` event's call.
        id: String,
        /// The result returned by the tool execution (could be success or error message).
        result: String,
        /// The name of the tool that was called.
        tool_name: String,
    },

    /// An error occurred during execution that doesn't stop the process.
    ExecutionError { error: String },

    /// The agent called the 'finish' tool, indicating task completion.
    Finished { summary: String },
}

/// Contains the final state after executor execution.
pub struct ExecutionOutcome {
    /// The full conversation history
    pub history: Vec<Message>,
}

/// An executor for agent interactions that yields events as they occur
///
/// The `AgentExecutor` encapsulates the pattern of:
/// 1. Sending messages to an agent
/// 2. Processing its responses (text and tool calls)
/// 3. Executing tool calls and feeding results back
/// 4. Maintaining conversation state
/// 5. Sending events through a channel to the caller
///
/// # Examples
///
/// ```no_run
/// # use hal::coder::executor::{AgentExecutor, ExecutorEvent};
/// # use rig::agent::Agent;
/// # use rig::completion::NoopModel;
/// # use rig::completion::ToolDefinition;
/// # use tokio::sync::mpsc;
/// # use std::sync::Arc;
/// #
/// # async fn example() {
/// # let agent: Agent<NoopModel> = Agent::noop();
/// # let tool_defs = Arc::new(vec![]);
/// // Create a channel for receiving events
/// let (tx, mut rx) = mpsc::channel(32);
///
/// // Create and run the executor
/// let executor_handle = tokio::spawn(async move {
///     let mut executor = AgentExecutor::new(
///         agent,
///         tool_defs.clone(),
///         10 // max iterations
///     );
///
///     executor.execute("Please do this task...".to_string(), tx).await
/// });
///
/// // Process events as they arrive
/// while let Some(event) = rx.recv().await {
///     match event {
///         Ok(ev) => match ev {
///             ExecutorEvent::Thinking { text } => println!("Agent thought: {}", text),
///             ExecutorEvent::ToolCallAttempted { call } => println!("Tool call: {}", call.function.name),
///             // Handle other events...
///             _ => {}
///         },
///         Err(e) => {
///             // Handle errors
///             eprintln!("Error: {}", e);
///             break;
///         }
///     }
/// }
///
/// // Get the result including history
/// let result = executor_handle.await.unwrap();
/// # }
/// ```
pub struct AgentExecutor<C>
where
    C: CompletionModel + Clone + Send + Sync + 'static,
{
    /// The agent that will perform the execution
    agent: Arc<Agent<C>>,
    /// Definitions of tools available to the agent
    tool_defs: Arc<Vec<ToolDefinition>>,
    /// Maximum number of interaction loops before stopping
    max_iterations: usize,
    /// Conversation history maintained during execution
    history: Vec<Message>,
    /// Queue of pending responses from the agent
    responses: VecDeque<AssistantContent>,
}

impl<C> AgentExecutor<C>
where
    C: CompletionModel + Clone + Send + Sync + 'static,
{
    /// Creates a new executor with the given agent, tools, and iteration limit
    pub fn new(
        agent: Arc<Agent<C>>,
        tool_defs: Arc<Vec<ToolDefinition>>,
        max_iterations: usize,
    ) -> Self {
        Self {
            agent,
            tool_defs,
            max_iterations,
            history: Vec::new(),
            responses: VecDeque::new(),
        }
    }

    /// Sets existing history to start with
    pub fn with_history(mut self, history: Vec<Message>) -> Self {
        self.history = history;
        self
    }

    async fn request(
        &self,
        initial_prompt: String,
    ) -> Result<CompletionResponse<<C as CompletionModel>::Response>, CoderError>
    where
        C: CompletionModel + Clone + Send + Sync + 'static,
    {
        self.agent
            .completion(initial_prompt.clone(), self.history.clone())
            .await
            .map_err(CoderError::CompletionError)?
            .tools(self.tool_defs.as_ref().clone())
            .send()
            .await
            .map_err(CoderError::CompletionError)
    }

    /// Runs the agent until it completes or errors, sending events through the provided channel
    /// Returns the final conversation history and any error that terminated execution
    #[instrument(name = "agent_execution", skip_all, fields(
        prompt_len = initial_prompt.len(),
        max_iterations = self.max_iterations
    ))]
    pub async fn execute(
        &mut self,
        initial_prompt: String,
        event_sender: Sender<Result<ExecutorEvent, CoderError>>,
    ) -> ExecutionOutcome {
        self.history.push(Message::user(&initial_prompt));

        // Initial call to agent
        let initial_response = match self.request(initial_prompt).await {
            Ok(response) => response,
            Err(e) => {
                error!(error=%e, "Agent initial call failed");
                // Don't send the error via channel here, it's returned in ExecutionOutcome
                let _ = event_sender.send(Err(e)).await; // Send the error
                return ExecutionOutcome {
                    // Return history up to this point
                    history: self.history.clone(),
                };
            }
        };

        initial_response
            .choice
            .iter()
            .for_each(|c| self.responses.push_back(c.clone()));

        if self.responses.is_empty() {
            warn!("Agent returned no initial response");
            let err = CoderError::AgentNoInitialResponse;
            // Send the error via channel and return the outcome
            let _ = event_sender.send(Err(err)).await; // Send the error
            return ExecutionOutcome {
                // Return history up to this point
                history: self.history.clone(),
            };
        }

        let mut iteration_count = 0;
        'execution_loop: loop {
            if iteration_count >= self.max_iterations {
                warn!(
                    max_iterations = self.max_iterations,
                    "Execution reached max iterations"
                );
                let err = CoderError::MaxIterationsReached(self.max_iterations);
                // Send the error via channel and return the outcome
                let _ = event_sender.send(Err(err)).await; // Send the error
                return ExecutionOutcome {
                    // Return history up to this point
                    history: self.history.clone(),
                };
            }
            iteration_count += 1;
            debug!(iteration = iteration_count, "Starting loop iteration");

            let mut reacted_in_iteration = false;
            while let Some(content) = self.responses.pop_front() {
                reacted_in_iteration = true;
                let assistant_message = Message::Assistant {
                    content: OneOrMany::one(content.clone()),
                };
                self.history.push(assistant_message);

                match content {
                    AssistantContent::Text(text) => {
                        info!(thought = %text.text, "Agent thought");
                        let _ = event_sender
                            .send(Ok(ExecutorEvent::Thinking {
                                text: text.text.clone(),
                            }))
                            .await;
                    }
                    AssistantContent::ToolCall(tool_call) => {
                        let id = tool_call.id.clone();
                        let name = tool_call.function.name.clone();

                        debug!(
                            tool_name = %name,
                            tool_id = %id,
                            tool_args = ?tool_call.function.arguments,
                            "Tool call initiated"
                        );

                        let _ = event_sender
                            .send(Ok(ExecutorEvent::ToolCallAttempted {
                                call: tool_call.clone(),
                            }))
                            .await;

                        // Execute the tool call
                        match self.execute_tool_call(&tool_call).await {
                            Ok(result) => {
                                debug!(
                                    tool_name = %name,
                                    tool_id = %id,
                                    result_len = result.len(),
                                    "Tool call completed"
                                );

                                let _ = event_sender
                                    .send(Ok(ExecutorEvent::ToolCallCompleted {
                                        id: id.clone(),
                                        result: result.clone(),
                                        tool_name: name.clone(),
                                    }))
                                    .await;

                                // Special handling for the finish tool
                                if name == "finish" {
                                    let summary =
                                        match serde_json::from_str::<serde_json::Value>(&result) {
                                            Ok(finish_data) => finish_data
                                                .get("summary")
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string())
                                                .unwrap_or_else(|| "Task completed".to_string()),
                                            Err(_) => "Task completed".to_string(),
                                        };

                                    let _ = event_sender
                                        .send(Ok(ExecutorEvent::Finished {
                                            summary: summary.clone(),
                                        }))
                                        .await;

                                    let tool_message = Message::User {
                                        content: OneOrMany::one(UserContent::ToolResult(
                                            ToolResult {
                                                id,
                                                content: OneOrMany::one(ToolResultContent::text(
                                                    result,
                                                )),
                                            },
                                        )),
                                    };
                                    self.history.push(tool_message);

                                    info!(summary=%summary, "Agent called finish tool. Exiting loop.");
                                    break 'execution_loop;
                                }

                                let tool_message = Message::User {
                                    content: OneOrMany::one(UserContent::ToolResult(ToolResult {
                                        id,
                                        content: OneOrMany::one(ToolResultContent::text(result)),
                                    })),
                                };
                                self.history.push(tool_message);

                                // Request reaction
                                if let Err(e) = self.request_reaction().await {
                                    // Send the error via channel and return the outcome
                                    let _ = event_sender.send(Err(e)).await; // Send the error
                                    return ExecutionOutcome {
                                        // Return history up to this point
                                        history: self.history.clone(),
                                    };
                                }
                            }
                            Err(e) => {
                                let error_msg = format!("Tool call '{}' failed: {}", name, e);
                                error!(error = %e, tool_name = %name, "Tool call execution failed");

                                // Send error as event but continue execution
                                let _ = event_sender
                                    .send(Ok(ExecutorEvent::ExecutionError {
                                        error: error_msg.clone(),
                                    }))
                                    .await;

                                let error_result = json!({ "error": error_msg }).to_string();

                                let tool_message = Message::User {
                                    content: OneOrMany::one(UserContent::ToolResult(ToolResult {
                                        id,
                                        content: OneOrMany::one(ToolResultContent::text(
                                            error_result,
                                        )),
                                    })),
                                };
                                self.history.push(tool_message);

                                // Request reaction to the error
                                if let Err(e) = self.request_reaction().await {
                                    // Send the error via channel and return the outcome
                                    let _ = event_sender.send(Err(e)).await; // Send the error
                                    return ExecutionOutcome {
                                        // Return history up to this point
                                        history: self.history.clone(),
                                    };
                                }
                            }
                        }
                    }
                }
            } // End while let Some(content)

            if self.responses.is_empty() && !reacted_in_iteration {
                warn!("Response queue empty and no reaction in iteration. Breaking loop.");
                let err = CoderError::AgentStoppedResponding(self.history.clone());
                // Send the error via channel and return the outcome
                let _ = event_sender.send(Err(err)).await; // Send the error
                return ExecutionOutcome {
                    // Return history up to this point
                    history: self.history.clone(),
                };
            } else if self.responses.is_empty() {
            } else if self.responses.is_empty() {
                debug!("Response queue empty, prompting to continue/finish.");
                if let Err(e) = self.prompt_to_continue().await {
                    // Send the error via channel and return the outcome
                    let _ = event_sender.send(Err(e)).await; // Send the error
                    return ExecutionOutcome {
                        // Return history up to this point
                        history: self.history.clone(),
                    };
                }
            }
        } // End execution_loop

        // Successful completion (loop finished via 'finish' tool or other means)
        ExecutionOutcome {
            history: self.history.clone(),
        }
    }

    /// Executes a tool call and returns the result
    #[instrument(name = "execute_tool_call", skip(self, tool_call), fields(
        tool_name = %tool_call.function.name,
        tool_id = %tool_call.id
    ))]
    async fn execute_tool_call(&self, tool_call: &ToolCall) -> Result<String, CoderError> {
        let name = &tool_call.function.name;

        // Convert arguments to JSON string
        let args_json = serde_json::to_string(&tool_call.function.arguments)
            .map_err(|e| CoderError::ToolArgsSerializationError(e))?;

        debug!(tool_args = %args_json, "Executing tool call");

        // Execute the tool call
        let result = self
            .agent
            .tools
            .call(name, args_json)
            .await
            .map_err(CoderError::ToolError)?;

        // Handle empty result
        if result.is_empty() {
            warn!(tool_name = %name, "Tool returned empty result");
            Ok(json!({ "result": "Tool returned no result" }).to_string())
        } else {
            Ok(result)
        }
    }

    /// Requests a reaction from the agent to the current state
    #[instrument(name = "agent_reaction", skip(self), fields(
        history_len = self.history.len()
    ))]
    async fn request_reaction(&mut self) -> Result<(), CoderError> {
        debug!("Requesting agent reaction");

        match self
            .agent
            .completion("", self.history.clone())
            .await
            .map_err(CoderError::CompletionError)?
            .tools(self.tool_defs.as_ref().clone())
            .send()
            .await
        {
            Ok(out) => {
                if out.choice.is_empty() {
                    warn!("Agent returned no reaction to tool result.");
                }
                out.choice
                    .iter()
                    .for_each(|c| self.responses.push_back(c.clone()));
                Ok(())
            }
            Err(e) => {
                let coder_error = CoderError::CompletionError(e);
                // let error_msg = format!("Agent failed to react to tool result: {}", coder_error); // Removed unused variable
                error!(error = %coder_error, "Agent reaction failed");
                Err(coder_error)
            }
        }
    }

    /// Prompts the agent to continue or finish execution
    #[instrument(name = "prompt_to_continue", skip(self), fields(
        history_len = self.history.len()
    ))]
    async fn prompt_to_continue(&mut self) -> Result<(), CoderError> {
        let continue_prompt = "Have you completed the specific instruction given to you? Remember:
        1. If you were asked to read or gather information, and you've done that, call the 'finish' tool.
        2. If you were asked to implement something, and you've done that, call the 'finish' tool.
        3. If you're unsure or need clarification, call the 'finish' tool to get help.";

        self.history.push(Message::user(continue_prompt));

        match self
            .agent
            .completion(continue_prompt, self.history.clone())
            .await
            .map_err(CoderError::CompletionError)?
            .tools(self.tool_defs.as_ref().clone())
            .send()
            .await
        {
            Ok(out) => {
                if out.choice.is_empty() {
                    warn!("Agent returned no response to continue prompt. Ending loop.");
                    return Err(CoderError::AgentStoppedResponding(self.history.clone()));
                }
                out.choice
                    .iter()
                    .for_each(|c| self.responses.push_back(c.clone()));
                Ok(())
            }
            Err(e) => {
                let coder_error = CoderError::CompletionError(e);
                // let error_msg = format!("Agent failed on continue prompt: {}", coder_error); // Removed unused variable
                error!(error = %coder_error, "Continue prompt failed");
                Err(coder_error)
            }
        }
    }

    /// Returns the current conversation history
    pub fn history(&self) -> Vec<Message> {
        self.history.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::mock_model::MockCompletionModel; // Import the mock model
    use rig::agent::AgentBuilder; // Use AgentBuilder to create the agent
    use std::sync::Arc;

    // Basic test to ensure the executor can be created and initialized
    #[tokio::test]
    async fn test_agent_executor_creation() {
        // Create an instance of the mock model
        let mock_model = MockCompletionModel::new();
        let agent = AgentBuilder::new(mock_model).build(); // Create agent with the mock model
        let tool_defs = Arc::new(vec![]);
        let arc_agent = Arc::new(agent);

        let executor = AgentExecutor::new(arc_agent, tool_defs, 10);

        assert_eq!(executor.history().len(), 0);
        assert_eq!(executor.max_iterations, 10);
    }

    // More comprehensive tests would mock the agent and tool responses
    // to test the full execution flow, but that requires more complex setup
}
