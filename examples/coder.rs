use anyhow::Result;
use hal::tools;
use rig::{
    cli_chatbot::cli_chatbot,
    completion::ToolDefinition,
    tool::{Tool, ToolEmbedding, ToolSet},
    vector_store::in_memory_store::InMemoryVectorStore,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

// Main function that sets up the CLI chatbot with the tools
#[tokio::main]
async fn main() -> Result<()> {
    let client = hal::model::Client::new_gemini_free_from_env();

    // Create toolset with all the defined tools
    let toolset = ToolSet::from_tools(tools::get_all_tools());

    let completion = client.completion().clone();
    let agent = AgentBuilder::new(completion).tool_set(toolset).build();

    // Start the CLI chatbot
    cli_chatbot(agent).await?;

    Ok(())
}
