//! Private implementation details for running a coder session.

use super::config::CoderConfig;
use super::error::CoderError;
use super::events::CoderEvent;
use super::executor::{AgentExecutor, ExecutorEvent}; // Removed unused ExecutionOutcome
use async_stream::stream;
use futures::stream::Stream;
use rig::completion::Completion as _;
// Removed StreamExt
use rig::{
    agent::Agent,
    // Removed CompletionError import as it's handled within CoderError now
    completion::{CompletionError, CompletionModel},
    message::{AssistantContent, Message, ToolResultContent, UserContent},
};
use tracing::{debug, error, info, instrument, warn};

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
        let mut pro_history = initial_history;
        let pro_agent = &config.pro_agent;
        // let junior_agent = &config.junior_agent; // No longer needed as a direct reference here

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
            }
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
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<ExecutorEvent, CoderError>>(32);

        // Create and configure the executor
        let tool_defs = config.tool_defs.clone();
        let max_iterations = config.max_junior_iterations;
        let mut executor = AgentExecutor::new(
            config.junior_agent.clone(),
            tool_defs, // Arc clone is cheap
            max_iterations,
        );

        // Format the initial prompt
        let initial_prompt = format!("<user_task>{}</user_task>\nDo NOTHING else.", plan);

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
                 let coder_error = CoderError::CompletionError(e);
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
