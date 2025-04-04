//! Events emitted by the Coder Module during a session.

use rig::message::{Message, ToolCall};

/// Represents the various states, outputs, and errors that occur during
/// a Pro/Junior coder session managed by `run_coder_session`.
///
/// Callers can listen to the event stream and react accordingly (e.g., update UI).
#[derive(Debug, Clone)] // Add Serialize/Deserialize if needed
pub enum CoderEvent {
    /// The Pro agent has generated an initial plan via basic completion (legacy mode).
    ProPlanReceived { plan: String },

    /// The Pro agent has generated an initial plan via the finish tool.
    ProPlanGenerated { plan: String },

    /// The Pro agent has produced some explanatory text (thought process).
    ProThinking { text: String },

    /// The Pro agent is attempting to call a tool.
    ProToolCall {
        /// The name of the tool being called.
        tool: String,
        /// The arguments for the tool call (serialized as a string).
        args: String,
    },

    /// A tool call initiated by the Pro agent has completed.
    ProToolResult {
        /// The name of the tool that was called.
        tool: String,
        /// The result returned by the tool execution.
        result: String,
    },

    /// A non-fatal warning occurred during execution.
    Warning { message: String },

    /// The Junior agent has produced some explanatory text (thought process).
    JuniorThinking { text: String },

    /// The Junior agent is attempting to call a tool.
    JuniorToolCallAttempted { call: ToolCall }, // Clone ToolCall might be expensive

    /// A tool call initiated by the Junior agent has completed.
    JuniorToolCallCompleted {
        /// The ID matching the corresponding `JuniorToolCallAttempted` event's call.
        id: String,
        /// The result returned by the tool execution (could be success or error message).
        result: String,
        /// The name of the tool that was called.
        tool_name: String,
    },

    /// An error occurred specifically during the Junior agent's execution phase.
    /// This might be recoverable if the session continues.
    JuniorExecutionError { error: String },

    /// The Pro agent has provided an analysis of the Junior agent's work.
    AnalysisReceived { analysis: String },

    /// The coder session has ended successfully.
    SessionEnded {
        /// The final analysis provided by the Pro agent.
        final_analysis: String,
        /// The complete message history of the session (Pro's perspective).
        history: Vec<Message>,
    },

    /// A fatal error occurred that terminated the session prematurely.
    SessionFailed { error: String },
}
