use std::io::{self, Write as _};

use anyhow::Result;
use futures::future::join_all;
use hal::tools;
use rig::{
    agent::{Agent, AgentBuilder},
    completion::{Chat, Completion as _, CompletionModel, PromptError, ToolDefinition},
    message::{AssistantContent, Message},
    tool::{ToolError, ToolSet},
};
use tracing::instrument;

// Main function that sets up the CLI chatbot with the tools
#[tokio::main]
async fn main() -> Result<()> {
    let _otel = hal::telemetry::init_tracing_subscriber();
    let client = hal::model::Client::new_gemini_free_from_env();

    // Create toolset with all the defined tools
    let mut toolset = ToolSet::default();
    toolset.add_tools(tools::get_full_toolset());

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

#[instrument(skip(agent))]
pub async fn cli_chatbot<C>(agent: Agent<C>) -> Result<(), PromptError>
where
    C: CompletionModel,
{
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut chat_log = vec![];

    let tools = tools::get_all_tools();
    let tool_futures = tools.iter().map(|t| t.definition("".to_string()));

    let tool_defs = join_all(tool_futures).await;

    println!("Welcome to the chatbot! Type 'exit' to quit.");
    loop {
        print!("> ");
        // Flush stdout to ensure the prompt appears before input
        stdout.flush().unwrap();

        let mut input = String::new();
        match stdin.read_line(&mut input) {
            Ok(_) => {
                // Remove the newline character from the input
                let input = input.trim();
                // Check for a command to exit
                if input == "exit" {
                    break;
                }
                tracing::info!("Prompt:\n{}\n", input);

                let mut response = agent
                    .completion(input, chat_log.clone())
                    .await?
                    .tools(tool_defs.clone())
                    .send()
                    .await?;

                chat_log.push(Message::user(input));
                chat_log.push(Message::Assistant {
                    content: response.choice.clone(),
                });

                let text = assistant_content(response.choice.first());

                println!("========================== Response ============================");
                println!("{}", text);
                println!("================================================================\n\n");

                loop {
                    // keep prompting if we get tool calls
                    if let AssistantContent::ToolCall(tool_call) = response.choice.first() {
                        let name = tool_call.function.name.clone();
                        let args =
                            serde_json::to_string(&tool_call.function.arguments).map_err(|e| {
                                PromptError::ToolError(rig::tool::ToolSetError::JsonError(e))
                            })?;
                        println!(
                            "========================== Tool Call ============================"
                        );
                        println!("name: {}, args: {}", name, args);
                        println!(
                            "================================================================\n\n"
                        );
                        let tool_result = agent.tools.call(&name, args).await?;
                        println!(
                            "========================== Tool Response ============================"
                        );
                        println!("{tool_result}");
                        println!(
                            "================================================================\n\n"
                        );
                        chat_log.push(Message::assistant(tool_result));

                        let out = agent
                            .completion("", chat_log.clone())
                            .await?
                            .tools(tool_defs.clone())
                            .send()
                            .await?;
                        response = out;
                    } else {
                        break;
                    }
                }
            }
            Err(error) => println!("Error reading input: {}", error),
        }
    }

    Ok(())
}

fn assistant_content(content: AssistantContent) -> String {
    match content {
        AssistantContent::Text(text) => text.text,
        AssistantContent::ToolCall(tool_call) => tool_call.function.name,
    }
}
