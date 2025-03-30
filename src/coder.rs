//! The Coder Module orchestrates interactions between a planning ("Pro") agent
//! and an executing ("Junior") agent with tools to accomplish coding tasks.

// Declare submodules according to Rust 2018+ conventions
pub mod config;
pub mod error;
pub mod events;
mod session; // Private implementation module

// Re-export public types for easier access
pub use config::CoderConfig;
pub use error::CoderError;
pub use events::CoderEvent;

use futures::stream::Stream;
use rig::{completion::CompletionModel, message::Message};

/// Runs a Pro/Junior coder session based on the user request and prior history.
///
/// This function orchestrates the interaction between a planning agent (Pro)
/// and an executing agent (Junior) equipped with tools. It manages the flow
/// of planning, execution, tool usage, and analysis.
///
/// Progress, results, and errors are reported asynchronously via the returned
/// event stream.
///
/// # Arguments
///
/// * `config` - A `CoderConfig` instance containing the agents, tool definitions, and limits.
/// * `user_request` - The specific task or instruction provided by the user for this turn.
/// * `initial_history` - A `Vec<Message>` representing the conversation history *before*
///   the current `user_request`. This provides context to the agents.
///
/// # Returns
///
/// * An implementation of `Stream<Item = CoderEvent>` that yields events as the
///   session progresses. The caller should iterate over this stream to receive updates.
///   The stream is `Send` bounds to allow processing in different async tasks if needed.
///
/// # Example Usage (Conceptual)
///
/// ```no_run
/// # use hal::coder::{CoderConfig, CoderEvent, run_coder_session};
/// # use rig::agent::Agent;
/// # use rig::completion::NoopModel;
/// # use rig::message::Message;
/// # use futures::StreamExt;
/// # use std::sync::Arc;
/// #
/// # #[tokio::main]
/// # async fn main() {
/// # let pro_agent: Agent<NoopModel> = Agent::noop(); // Placeholder
/// # let junior_agent: Agent<NoopModel> = Agent::noop(); // Placeholder
/// # let tool_defs = vec![];
/// # let config = CoderConfig::new(pro_agent, junior_agent, tool_defs, 10);
/// let user_request = "Refactor the login function.".to_string();
/// let history: Vec<Message> = vec![]; // Start with empty history
///
/// let session_stream = run_coder_session(config, user_request, history);
/// futures::pin_mut!(session_stream);
///
/// while let Some(event) = session_stream.next().await {
///     match event {
///         CoderEvent::ProPlanReceived { plan } => println!("Plan: {}", plan),
///         CoderEvent::JuniorThinking { text } => println!("Junior: {}", text),
///         // ... handle other events ...
///         CoderEvent::SessionEnded { final_analysis, history: updated_history } => {
///             println!("Analysis: {}", final_analysis);
///             // Save updated_history for the next turn
///             break;
///         },
///         CoderEvent::SessionFailed { error } => {
///             eprintln!("Error: {}", error);
///             break;
///         }
///         _ => {} // Handle or ignore other events
///     }
/// }
/// # }
/// ```
pub fn run_coder_session<C>(
    config: &CoderConfig<C>,
    user_request: String,
    initial_history: Vec<Message>,
) -> impl Stream<Item = CoderEvent> + Send + use<'_, C>
where
    C: CompletionModel + Clone + Send + Sync + 'static,
{
    // Delegate to the private implementation module
    session::run(config, user_request, initial_history)
}
