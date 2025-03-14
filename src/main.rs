mod tui;

use clap::{Parser, Subcommand, Args};

#[derive(Parser)]
#[command(author, version, about = "A Rust client for Google's Gemini AI API", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start an interactive chat session with Gemini
    Chat(ChatArgs),
    // Add more commands here as needed
}

#[derive(Args)]
struct ChatArgs {
    /// Gemini model to use (default: gemini-2.0-flash)
    #[arg(short, long, default_value = "gemini-2.0-flash")]
    model: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Execute the appropriate command
    match cli.command {
        Some(Commands::Chat(args)) => {
            // Get API key from environment variable
            let api_key = std::env::var("GEMINI_API_KEY")
                .expect("GEMINI_API_KEY environment variable must be set");
            
            // Print the selected model
            println!("Starting chat with model: {}", args.model);
            
            // Run the TUI application
            tui::run(api_key, args.model).await?;
        }
        None => {
            // If no command is provided, show help
            let _ = Cli::parse_from(&["--help"]);
        }
    }
    
    Ok(())
}
