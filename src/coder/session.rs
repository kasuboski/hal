//! Private implementation details for running a coder session.

use super::config::CoderConfig;
use super::error::CoderError; // Import custom error
use super::events::CoderEvent;
use async_stream::stream;
use futures::stream::Stream;
use rig::{
    agent::Agent,
    completion::{Completion as _, CompletionError, CompletionModel, ToolDefinition}, // Removed Completion as _
    message::{AssistantContent, Message, ToolCall, ToolResult, ToolResultContent, UserContent},
    tool::{ToolSet, ToolSetError}, // Added import
    OneOrMany,
};
use std::{collections::VecDeque, sync::Arc}; // Added import
use tracing::{debug, error, info, instrument, warn};

/// Runs the core Pro/Junior agent interaction loop.
#[instrument(name = "coder_session", skip_all, fields(user_request_len = user_request.len(), initial_history_len = initial_history.len()))]
pub(super) fn run<C>(
    config: &CoderConfig<C>,
    user_request: String,
    initial_history: Vec<Message>,
) -> impl Stream<Item = CoderEvent> + Send + use<'_, C>
where
    C: CompletionModel + Clone + Send + Sync + 'static,
{
    stream! {
        let mut pro_history = initial_history;
        let pro_agent = &config.pro_agent;
        let junior_agent = &config.junior_agent;

        // --- 1. Get Plan from Pro Agent ---
        info!("Requesting plan from Pro agent");
        let plan_prompt_msg = Message::user(&user_request);
        let mut current_pro_turn_history = pro_history.clone();
        current_pro_turn_history.push(plan_prompt_msg);

        let plan_result = run_pro_completion(pro_agent, current_pro_turn_history.as_slice()).await;

        let plan = match plan_result {
            Ok(p) => {
                if p.is_empty() {
                    let error_msg = CoderError::EmptyPlan.to_string();
                    error!(error=%error_msg);
                    yield CoderEvent::SessionFailed { error: error_msg };
                    return;
                }
                pro_history = current_pro_turn_history;
                pro_history.push(Message::assistant(&p));
                yield CoderEvent::ProPlanReceived { plan: p.clone() };
                p
            },
            // Use CoderError::CompletionError implicitly via #[from]
            Err(e) => {
                let coder_error = CoderError::from(e);
                let error_msg = format!("Failed to get plan from Pro agent: {}", coder_error);
                error!(error=%coder_error, "Pro agent planning failed");
                yield CoderEvent::SessionFailed { error: error_msg };
                return;
            }
        };

        // --- 2. Execute Plan with Junior Agent ---
        info!("Starting Junior agent execution");
        let junior_result = run_junior_execution(
            junior_agent,
            &config.tool_defs,
            &plan,
            config.max_junior_iterations,
        )
        .await;

        let junior_log = match junior_result {
            Ok((final_log, events)) => {
                for event in events {
                    yield event;
                }
                final_log
            }
            // run_junior_execution now returns CoderError
            Err(e) => {
                let error_msg = format!("Junior agent execution failed critically: {}", e);
                error!(error=%e, "Junior agent execution failed");
                yield CoderEvent::JuniorExecutionError{ error: error_msg.clone() };
                yield CoderEvent::SessionFailed { error: error_msg };
                return;
            }
        };

        // --- 3. Get Analysis from Pro Agent ---
        info!("Requesting analysis from Pro agent");
        let junior_info = junior_log
            .iter()
            .map(message_to_string_for_analysis)
            .collect::<Vec<String>>()
            .join("\n\n");

        let analysis_prompt = format!(
            "Analyze the implementation of the plan by the junior developer based on the following log:\n\n<junior_developer_log>\n{}\n</junior_developer_log>\n\nProvide your analysis. The junior developer should have called the 'finish' tool if the task was complete.",
            junior_info
        );

        pro_history.push(Message::user(&analysis_prompt));

        let analysis_result = run_pro_completion(pro_agent, &pro_history).await;

        let analysis = match analysis_result {
             Ok(a) => {
                 if a.is_empty() {
                     warn!("Pro agent returned empty analysis. Assuming simple completion.");
                     "Analysis complete.".to_string()
                 } else {
                    pro_history.push(Message::assistant(&a));
                    yield CoderEvent::AnalysisReceived { analysis: a.clone() };
                    a
                 }
             },
             Err(e) => {
                 let coder_error = CoderError::from(e);
                 let error_msg = format!("Failed to get analysis from Pro agent: {}", coder_error);
                 error!(error=%coder_error, "Pro agent analysis failed");
                 yield CoderEvent::SessionFailed { error: error_msg };
                 return;
             }
        };

        // --- 4. Session Complete ---
        info!("Coder session completed successfully");
        yield CoderEvent::SessionEnded {
            final_analysis: analysis,
            history: pro_history,
        };
    }
}

/// Runs the Junior agent's execution loop, handling thoughts, tool calls, and reactions.
/// Returns the final junior message log and a Vec of events generated during execution.
#[instrument(name = "junior_execution", skip_all, fields(plan_len = plan.len()))]
async fn run_junior_execution<C>(
    junior_agent: &Agent<C>,
    tool_defs: &Arc<Vec<ToolDefinition>>,
    plan: &str,
    max_iterations: usize,
) -> Result<(Vec<Message>, Vec<CoderEvent>), CoderError>
where
    C: CompletionModel + Clone + Send + Sync + 'static,
{
    let mut junior_log = vec![];
    let mut events = Vec::new();
    let mut responses = VecDeque::new();

    let initial_prompt = format!("<user_task>{}</user_task>", plan);
    junior_log.push(Message::user(&initial_prompt));
    debug!(prompt = initial_prompt, "Sending initial task to Junior");

    // Initial call to Junior
    let initial_response = junior_agent
        .completion(initial_prompt.clone(), junior_log.clone())
        .await
        .map_err(CoderError::CompletionError)?
        .tools(tool_defs.as_ref().clone().into())
        .send()
        .await
        .map_err(CoderError::CompletionError)?;

    initial_response
        .choice
        .iter()
        .for_each(|c| responses.push_back(c.clone())); // Clone content for the queue

    if responses.is_empty() {
        warn!("Junior agent returned no initial response to the plan.");
        events.push(CoderEvent::JuniorExecutionError {
            error: CoderError::JuniorNoInitialResponse.to_string(), // Use specific error
        });
        return Ok((junior_log, events));
    }

    let mut iteration_count = 0;
    'junior_loop: loop {
        if iteration_count >= max_iterations {
            warn!(max_iterations, "Junior execution reached max iterations");
            let err = CoderError::MaxIterationsReached(max_iterations);
            events.push(CoderEvent::JuniorExecutionError {
                error: err.to_string(),
            });
            // Decide whether to return Ok or Err. Let's return Err here.
            return Err(err);
        }
        iteration_count += 1;
        debug!(
            iteration = iteration_count,
            "Starting Junior loop iteration"
        );

        let mut reacted_in_iteration = false;
        while let Some(content) = responses.pop_front() {
            reacted_in_iteration = true;
            let assistant_message = Message::Assistant {
                content: OneOrMany::one(content.clone()),
            };
            junior_log.push(assistant_message);

            match content {
                AssistantContent::Text(text) => {
                    info!(junior_thought = %text.text, "Junior thought");
                    events.push(CoderEvent::JuniorThinking {
                        text: text.text.clone(),
                    });
                }
                AssistantContent::ToolCall(tool_call) => {
                    let id = tool_call.id.clone();
                    let name = tool_call.function.name.clone();
                    debug!(tool_name=%name, tool_id=%id, tool_args=?tool_call.function.arguments, "Junior tool call initiated");

                    events.push(CoderEvent::JuniorToolCallAttempted {
                        call: tool_call.clone(),
                    });

                    // Execute Tool Call
                    let tool_result_str = match execute_tool_call(&junior_agent.tools, &tool_call)
                        .await
                    {
                        Ok(result) => result,
                        // Convert ToolSetError to string for the event log, but keep original error type
                        Err(e) => {
                            let coder_error = CoderError::from(e); // Use #[from]
                            let error_msg = format!("Tool call '{}' failed: {}", name, coder_error);
                            error!(error=%coder_error, tool_name=%name, "Tool call execution failed");
                            events.push(CoderEvent::JuniorExecutionError {
                                error: error_msg.clone(),
                            });
                            error_msg // Return error message as the tool result string
                        }
                    };

                    let tool_result_str_final = if tool_result_str.is_empty() {
                        warn!(tool_name=%name, "Tool returned empty result");
                        "Tool returned no result".to_string()
                    } else {
                        tool_result_str
                    };

                    debug!(tool_name=%name, tool_id=%id, result_len=tool_result_str_final.len(), "Junior tool call completed");
                    events.push(CoderEvent::JuniorToolCallCompleted {
                        id: id.clone(),
                        result: tool_result_str_final.clone(),
                        tool_name: name.clone(),
                    });

                    let tool_message = Message::User {
                        content: OneOrMany::one(UserContent::ToolResult(ToolResult {
                            id,
                            content: OneOrMany::one(ToolResultContent::text(tool_result_str_final)),
                        })),
                    };
                    junior_log.push(tool_message);

                    if name == "finish" {
                        info!("Junior agent called finish tool. Exiting loop.");
                        break 'junior_loop;
                    }

                    // React to the tool call result
                    debug!("Requesting Junior reaction to tool result");
                    match junior_agent
                        .completion("", junior_log.clone())
                        .await
                        .map_err(CoderError::CompletionError)? // Convert rig error
                        .tools(tool_defs.as_ref().clone().into())
                        .send()
                        .await // Result<CompletionResponse, CompletionError>
                    {
                        Ok(out) => {
                            if out.choice.is_empty() {
                                warn!("Junior agent returned no reaction to tool result.");
                            }
                            out.choice.iter().for_each(|c| responses.push_back(c.clone()));
                        }
                        Err(e) => {
                            let coder_error = CoderError::from(e); // Convert rig error
                            let error_msg =
                                format!("Junior agent failed to react to tool result: {}", coder_error);
                            error!(error=%coder_error, "Junior agent reaction failed");
                            events.push(CoderEvent::JuniorExecutionError { error: error_msg });
                            warn!("Error reacting to tool response, continuing loop if possible.");
                        }
                    }
                }
            }
        } // End while let Some(content)

        if responses.is_empty() && !reacted_in_iteration {
            warn!("Junior response queue empty and no reaction in iteration. Breaking loop.");
            let err = CoderError::JuniorStoppedResponding;
            events.push(CoderEvent::JuniorExecutionError {
                error: err.to_string(),
            });
            // Return Err to signal failure
            return Err(err);
        } else if responses.is_empty() {
            debug!("Junior response queue empty, prompting to continue/finish.");
            let continue_prompt = "Have you solved the task? If not, continue solving the task. If you have solved the task, call the 'finish' tool to end your turn.";
            junior_log.push(Message::user(continue_prompt));
            match junior_agent
                .completion(continue_prompt, junior_log.clone())
                .await
                .map_err(CoderError::CompletionError)? // Convert rig error
                .tools(tool_defs.as_ref().clone().into())
                .send()
                .await // Result<CompletionResponse, CompletionError>
            {
                Ok(out) => {
                    if out.choice.is_empty() {
                        warn!("Junior agent returned no response to continue prompt. Ending loop.");
                        let err = CoderError::JuniorStoppedResponding;
                        events.push(CoderEvent::JuniorExecutionError {
                            error: err.to_string(),
                        });
                        // Return Err to signal failure
                        return Err(err);
                    }
                    // Use iter() and clone()
                    out.choice.iter().for_each(|c| responses.push_back(c.clone()));
                }
                Err(e) => {
                    let coder_error = CoderError::from(e); // Convert rig error
                    let error_msg = format!("Junior agent failed on continue prompt: {}", coder_error);
                    error!(error=%coder_error, "Junior continue prompt failed");
                    events.push(CoderEvent::JuniorExecutionError { error: error_msg });
                    warn!("Error on continue prompt. Ending loop.");
                    // Return Err to signal failure
                    return Err(coder_error);
                }
            }
        }
    } // End 'junior_loop

    Ok((junior_log, events))
}

/// Helper to call the Pro agent for planning or analysis.
#[instrument(name = "pro_completion", skip_all, fields(history_len = history.len()))]
async fn run_pro_completion<C>(
    pro_agent: &Agent<C>,
    history: &[Message],
) -> Result<String, CompletionError>
// This returns the rig error directly
where
    C: CompletionModel + Clone,
{
    if history.is_empty() {
        return Ok("".to_string());
    }
    debug!("Sending request to Pro agent");
    let builder = pro_agent.completion("", history.to_vec()).await?;

    let response = builder.send().await?;
    debug!("Received response from Pro agent");

    // Check choice length *before* iterating if needed later
    let choice_len = response.choice.len();

    let text_response = response
        .choice
        .iter() // Use iter() to borrow
        .filter_map(|c| match c {
            AssistantContent::Text(text) => Some(text.text.clone()), // Clone text
            _ => {
                warn!(pro_tool_call=?c, "Pro agent attempted non-text response (e.g., tool call)");
                None
            }
        })
        .collect::<Vec<String>>()
        .join("\n");

    // Check collected text length vs original choice length
    if text_response.is_empty() && choice_len > 0 {
        warn!("Pro agent returned a non-text response when text was expected.");
    } else if text_response.is_empty() {
        debug!("Pro agent returned empty text response.");
    }

    Ok(text_response)
}

/// Executes a tool call using the provided ToolSet. Returns the string result.
#[instrument(name = "execute_tool_call", skip(toolset, tool_call), fields(tool_name = %tool_call.function.name, tool_id = %tool_call.id))]
async fn execute_tool_call(
    toolset: &ToolSet,
    tool_call: &ToolCall,
) -> Result<String, ToolSetError> // Returns rig error directly
{
    let name = &tool_call.function.name;
    // Use map_err for serde error conversion if needed, but ToolSetError::JsonError handles it
    let args_json = serde_json::to_string(&tool_call.function.arguments)
        .map_err(rig::tool::ToolSetError::JsonError)?; // Convert serde error

    debug!(tool_args = %args_json, "Executing tool call");

    let result = toolset.call(name, args_json).await;

    match &result {
        Ok(res_str) => {
            debug!(result_len = res_str.len(), "Tool call successful");
        }
        Err(e) => {
            error!(error = %e, "Tool call failed during execution");
        }
    }

    result
}

/// Formats messages into a simple string representation suitable for the Pro agent's analysis prompt.
fn message_to_string_for_analysis(message: &Message) -> String {
    match message {
        Message::User { content } => {
            let parts: Vec<String> = content
                .iter()
                .map(|c| match c {
                    UserContent::Text(t) => format!("User Task/Input:\n{}", t.text),
                    UserContent::ToolResult(tr) => format!(
                        "Tool Result (ID: {}):\n---\n{}\n---",
                        tr.id,
                        tr.content
                            .iter()
                            .map(|tc| match tc {
                                ToolResultContent::Text(t) => t.text.clone(),
                                ToolResultContent::Image(_) => "Tool Result: <image>".to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    ),
                    _ => "User: [Unsupported Content Type]".to_string(),
                })
                .collect();
            parts.join("\n")
        }
        Message::Assistant { content } => {
            let parts: Vec<String> = content
                .iter()
                .map(|c| match c {
                    AssistantContent::Text(t) => format!("Assistant Thought/Response:\n{}", t.text),
                    AssistantContent::ToolCall(tc) => format!(
                        "Assistant Tool Call:\n  ID: {}\n  Name: {}\n  Args: {}",
                        tc.id,
                        tc.function.name,
                        serde_json::to_string(&tc.function.arguments)
                            .unwrap_or_else(|_| "{ serialization error }".to_string())
                    ),
                })
                .collect();
            parts.join("\n")
        }
    }
}
