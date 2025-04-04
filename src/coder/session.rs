//! Private implementation details for running a coder session.

use super::config::CoderConfig;
use super::error::CoderError;
use super::events::CoderEvent;
use super::executor::{AgentExecutor, ExecutorEvent};
use async_stream::stream;
use futures::stream::Stream;
use rig::{
    completion::CompletionModel,
    message::{AssistantContent, Message, ToolResultContent, UserContent},
};
use tracing::{error, info, instrument, warn};

/// Runs the core Pro/Junior agent interaction loop.
#[instrument(name = "coder_session", skip_all, fields(user_request_len = user_request.len(), initial_history_len = initial_history.len()))]
pub(super) fn run<C>(
    config: &CoderConfig<C>,
    user_request: String,
    initial_history: Vec<Message>,
) -> impl Stream<Item = CoderEvent> + Send + '_
where
    C: CompletionModel + Clone + Send + Sync + 'static,
{
    stream! {
        info!("Starting agent execution");
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<ExecutorEvent, CoderError>>(32);

        // Create and configure the executor
        let tool_defs = config.tool_defs.clone();
        let max_iterations = config.max_junior_iterations;
        let executor = AgentExecutor::new(
            config.junior_agent.clone(),
            tool_defs, // Arc clone is cheap
            max_iterations,
        );

        let mut executor = executor.with_history(initial_history.clone());

        // Format the initial prompt
        let initial_prompt = format!("<user_task>{}</user_task>\nDo NOTHING else.", user_request);

        // Spawn the executor task
        let executor_handle = tokio::spawn(async move {
            executor.execute(initial_prompt, tx).await
        });

        // Forward all junior events
        let mut junior_executed = false;
        let mut had_error = false;
        let mut junior_finished = false;

        while let Some(event_result) = rx.recv().await {
            junior_executed = true;
            match event_result {
                Ok(executor_event) => {
                    // Convert ExecutorEvent to CoderEvent and yield
                    let coder_event = match executor_event {
                        ExecutorEvent::Thinking { text } => CoderEvent::JuniorThinking { text },
                        ExecutorEvent::ToolCallAttempted { call } => CoderEvent::JuniorToolCallAttempted { call },
                        ExecutorEvent::ToolCallCompleted { id, result, tool_name } => CoderEvent::JuniorToolCallCompleted { id, result, tool_name },
                        ExecutorEvent::ExecutionError { error } => {
                            had_error = true;
                            CoderEvent::JuniorExecutionError { error }
                        },
                        ExecutorEvent::Finished { summary } => {
                            info!(summary = %summary, "Junior agent finished execution via 'finish' tool");
                            junior_finished = true;
                            // Optionally yield a specific event, or just let the loop end
                            // For now, just break the loop after processing this
                            continue; // Or yield an event if defined
                        }
                    };
                    yield coder_event;
                }
                Err(coder_error) => {
                    // This error likely came from prompt_to_continue failing a completion request
                    // or a terminal error like MaxIterationsReached from the executor loop itself.
                    let error_msg = format!("Junior agent execution failed: {}", coder_error);
                    error!(error=%coder_error, "Junior execution failed");
                    had_error = true;
                    yield CoderEvent::JuniorExecutionError { error: error_msg.clone() };

                    // Check if this specific error should be fatal for the whole session
                    match coder_error {
                        CoderError::CompletionError(_) | // Failure during agent call (e.g., reaction, continue)
                        CoderError::MaxIterationsReached(_) |
                        CoderError::AgentNoInitialResponse |
                        CoderError::AgentStoppedResponding(_) => {
                            yield CoderEvent::SessionFailed { error: error_msg };
                            // Close the receiver to ensure the spawned task doesn't hang
                            rx.close();
                            // Wait for the task to finish to avoid detached task warnings
                            let _ = executor_handle.await;
                            return;
                        }
                        _ => {} // Other errors (like ToolError) might have been reported as ExecutionError event
                    }
                }
            }
        }

        // --- Get Executor Result ---
        let execution_outcome = match executor_handle.await {
            Ok(res) => res,
            Err(join_error) => {
                let error_msg = format!("Junior executor task failed: {:?}", join_error);
                error!(error=%join_error, "Executor task join error");
                yield CoderEvent::SessionFailed { error: error_msg };
                return;
            }
        };

        // Get the junior history for analysis
        let junior_log = execution_outcome.history;

        // Removed the block checking execution_outcome.error - errors are handled via channel

        // If junior didn't execute at all (e.g., initial response failed hard) and no error was reported
        if !junior_executed && !junior_finished && !had_error {
            warn!("Junior agent execution yielded no events and finished without error/finish call.");
            // This might indicate an issue like the initial response failing before the loop
            // The error should be in executor_result.error if it happened
            if junior_log.is_empty() {
                yield CoderEvent::SessionFailed { error: "Junior agent execution failed silently.".to_string() };
                return;
            }
        }

        // --- 3. Get Analysis from Pro Agent using tools ---
        info!("Requesting analysis from Pro agent with tools");

        // Create the junior developer log
        let junior_info = junior_log
            .iter()
            .map(message_to_string_for_analysis)
            .collect::<Vec<String>>()
            .join("\n\n");

        // Format the analysis prompt
        let analysis_prompt = format!(
            "Analyze the implementation of the plan by the junior developer based on the following log:\n\n<junior_developer_log>\n{}\n</junior_developer_log>\n\n\
            You have access to various tools that can help you analyze the implementation.\n\n\
            Your goal is to:\n\
            1. Use available tools to gather any additional information you need\n\
            2. Analyze how well the junior developer followed the plan\n\
            3. Identify any issues or improvements in the implementation\n\
            4. When you have completed your analysis, call the \"finish\" tool with your complete analysis in the summary parameter\n\n\
            IMPORTANT: Your analysis must be complete and detailed in the summary of the finish tool call.",
            junior_info
        );

        // Create a channel for Pro agent executor events
        let (pro_tx, mut pro_rx) = tokio::sync::mpsc::channel::<Result<ExecutorEvent, CoderError>>(32);

        let mut pro_history = initial_history.clone();
        // Create the Pro agent executor with appropriate tools and limits
        let mut pro_executor = AgentExecutor::new(
            config.pro_agent.clone(),
            config.pro_tool_defs.clone(),
            config.max_pro_iterations,
        );

        // Set history from previous Pro agent execution
        pro_executor = pro_executor.with_history(pro_history.clone());

        // Spawn the Pro agent executor task
        let pro_executor_handle = tokio::spawn(async move {
            pro_executor.execute(analysis_prompt, pro_tx).await
        });

        // Process Pro agent events and extract the analysis
        let mut analysis: Option<String> = None;

        while let Some(event_result) = pro_rx.recv().await {
            match event_result {
                Ok(ExecutorEvent::Thinking { text }) => {
                    // Convert to ProThinking event
                    yield CoderEvent::ProThinking { text };
                }
                Ok(ExecutorEvent::ToolCallAttempted { call }) => {
                    // Convert to ProToolCall event
                    yield CoderEvent::ProToolCall {
                        tool: call.function.name.clone(),
                        args: call.function.arguments.to_string(),
                    };
                }
                Ok(ExecutorEvent::ToolCallCompleted { id: _, result, tool_name }) => {
                    // Convert to ProToolResult event
                    yield CoderEvent::ProToolResult {
                        tool: tool_name,
                        result: result.clone(),
                    };
                }
                Ok(ExecutorEvent::Finished { summary }) => {
                    // Extract the analysis from the finish tool's summary
                    analysis = Some(summary.clone());
                    yield CoderEvent::AnalysisReceived { analysis: summary.clone() };
                }
                Ok(ExecutorEvent::ExecutionError { error }) => {
                    // Handle non-fatal errors
                    yield CoderEvent::Warning {
                        message: format!("Pro agent tool error during analysis: {}", error)
                    };
                }
                Err(e) => {
                    // Handle fatal errors
                    let error_msg = format!("Pro agent analysis failed: {}", e);
                    error!(error=%e, "Pro agent analysis failed");
                    yield CoderEvent::SessionFailed { error: error_msg };
                    return;
                }
            }
        }

        // Wait for the Pro executor to complete
        let pro_outcome = match pro_executor_handle.await {
            Ok(outcome) => outcome,
            Err(e) => {
                let error_msg = format!("Failed to join Pro agent executor during analysis: {}", e);
                error!(error=%e, "Pro executor join error during analysis");
                yield CoderEvent::SessionFailed { error: error_msg };
                return;
            }
        };

        // Update pro_history with the history from the executor
        pro_history = pro_outcome.history;

        // Ensure we have an analysis
        let analysis = match analysis {
            Some(a) => {
                if a.is_empty() {
                    warn!("Pro agent returned empty analysis. Assuming simple completion.");
                    "Analysis complete.".to_string()
                } else {
                    a
                }
            },
            None => {
                let error_msg = CoderError::AgentError("Pro agent did not generate an analysis".to_string()).to_string();
                error!(error=%error_msg);
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
