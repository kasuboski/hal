use anyhow::Result;
use hal::tools;
use rig::{agent::AgentBuilder, cli_chatbot::cli_chatbot, tool::ToolSet};

// Main function that sets up the CLI chatbot with the tools
#[tokio::main]
async fn main() -> Result<()> {
    let client = hal::model::Client::new_gemini_free_from_env();

    // Create toolset with all the defined tools
    let mut toolset = ToolSet::default();
    toolset.add_tools(tools::get_all_tools());

    let tool_explanation = toolset
        .documents()
        .await?
        .iter()
        .cloned()
        .map(|doc| doc.text)
        .collect::<Vec<String>>()
        .join("\n");

    let completion = client.completion().clone();
    let mut agent = AgentBuilder::new(completion)
        .preamble("You are an expert coder. You have access to various tools to implement your goals. Do as the user asks, maintaining good code quality.")
        .append_preamble(tool_explanation.as_str())
        .build();
    agent.tools = toolset;

    // Start the CLI chatbot
    cli_chatbot(agent).await?;

    Ok(())
}
