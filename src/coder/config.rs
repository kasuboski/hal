//! Configuration for the Coder Module.

use rig::{
    agent::Agent,
    completion::{CompletionModel, ToolDefinition},
};
use std::sync::Arc; // Using Arc for Tool Definitions for cheaper cloning

/// Configuration parameters for initializing and running a coder session.
///
/// This struct holds the necessary components like agents, tool definitions,
/// and operational limits required by the `run_coder_session` function.
pub struct CoderConfig<C>
where
    C: CompletionModel + Clone + Send + Sync + 'static,
{
    /// The "Pro" or "Tech Lead" agent responsible for planning and analysis.
    pub pro_agent: Agent<C>,

    /// The "Junior" agent responsible for executing the plan and using tools.
    /// This agent should have its `ToolSet` pre-configured.
    pub junior_agent: Arc<Agent<C>>,

    /// Definitions of the tools available to the Junior agent.
    /// These are sent to the model to inform it about available functions.
    /// Using Arc to make cloning the config cheaper if tool defs are large/numerous.
    pub tool_defs: Arc<Vec<ToolDefinition>>,

    /// Maximum number of iterations the Junior agent loop should run.
    /// Helps prevent infinite loops or excessive execution time.
    pub max_junior_iterations: usize,
    // Optional: Add other limits or configuration as needed
    // pub max_tool_calls_per_step: usize,
    // pub analysis_prompt_template: String,
}

impl<C> CoderConfig<C>
where
    C: CompletionModel + Clone + Send + Sync + 'static,
{
    /// Creates a new CoderConfig.
    pub fn new(
        pro_agent: Agent<C>,
        junior_agent: Agent<C>,
        tool_defs: Vec<ToolDefinition>,
        max_junior_iterations: usize,
    ) -> Self {
        Self {
            pro_agent,
            junior_agent: Arc::new(junior_agent),
            tool_defs: Arc::new(tool_defs),
            max_junior_iterations,
        }
    }
}
