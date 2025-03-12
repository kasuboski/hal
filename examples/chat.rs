use hal::Client;
use hal::prelude::{Result, Content, Part};
use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use std::sync::Mutex;
use hal::format_markdown;

// Create a global colorized stdout writer
lazy_static::lazy_static! {
    static ref STDOUT: Mutex<StandardStream> = Mutex::new(StandardStream::stdout(ColorChoice::Auto));
}

// Helper function to print colored text
fn print_colored(text: &str, color: Color, bold: bool) {
    let mut stdout = STDOUT.lock().unwrap();
    stdout.set_color(ColorSpec::new().set_fg(Some(color)).set_bold(bold)).unwrap();
    write!(stdout, "{}", text).unwrap();
    stdout.reset().unwrap();
}

// Helper function to print a styled header
fn print_header(text: &str) {
    println!();
    print_colored("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", Color::Cyan, true);
    println!();
    print_colored("  ", Color::White, false);
    print_colored(text, Color::Cyan, true);
    println!();
    print_colored("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", Color::Cyan, true);
    println!();
}

// Helper function to print a separator
fn print_separator() {
    print_colored("────────────────────────────────────────────────────────────────────────────────", Color::Blue, false);
    println!();
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the client with API key from environment variable
    let api_key = std::env::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY environment variable must be set");
    
    let client = Client::with_api_key(api_key);
    
    // Create a chat session
    let chat = client.chats().create("gemini-2.0-flash").await?;
    
    // Store message history as Vec<Content>
    let mut message_history: Vec<Content> = Vec::new();
    
    // Print welcome message with styling
    print_header("Chat session started with Gemini");
    
    print_colored("• ", Color::Green, true);
    print_colored("Type your messages and press Enter to send.\n", Color::White, false);
    
    print_colored("• ", Color::Yellow, true);
    print_colored("Type 'exit' to end the conversation.\n", Color::White, false);
    
    print_separator();
    
    // Main chat loop
    loop {
        // Display colorized prompt and get user input
        print_colored("You", Color::Green, true);
        print_colored(": ", Color::White, true);
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");
        
        let input = input.trim();
        
        // Check if user wants to exit
        if input.to_lowercase() == "exit" {
            print_colored("\nEnding chat session.\n", Color::Yellow, true);
            break;
        }
        
        // Create user message content and add to history
        let user_message = Content::new().with_role("user").with_text(input);
        
        // Send message to LLM and get response
        // Add the new user message to history and send
        message_history.push(user_message);
        match chat.send_message(input, Some(message_history.clone())).await {
            Ok(response) => {
                let response_text = response.text();
                print_colored("AI", Color::Blue, true);
                print_colored(": ", Color::White, true);
                format_markdown(&response_text)?;
                println!();
                
                // Add AI response to history
                message_history.push(Content::new().with_role("model").with_text(response_text));
            },
            Err(e) => {
                // Print error message in red
                let mut stderr = StandardStream::stderr(ColorChoice::Auto);
                stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true)).unwrap();
                writeln!(stderr, "Error: {}", e).unwrap();
                stderr.reset().unwrap();
                
                print_colored("AI", Color::Blue, true);
                print_colored(": ", Color::White, true);
                print_colored("Sorry, I encountered an error processing your request.", Color::Red, false);
                println!();
                
                // Add error response to history
                message_history.push(Content::new().with_role("model").with_text("Sorry, I encountered an error processing your request."));
            }
        }
        
        println!();
    }
    
    // Display message history at the end with styling
    print_header("Chat History");
    
    for (i, message) in message_history.iter().enumerate() {
        let role = message.role.as_deref().unwrap_or("unknown");
        let text = message.parts.first().map_or("", |part| {
            if let Part::Text(text) = part {
                text
            } else {
                "[non-text content]"
            }
        });
        
        // Color-code based on role
        match role {
            "user" => {
                print_colored("You", Color::Green, true);
                print_colored(": ", Color::White, true);
                print_colored(text, Color::White, false);
                println!();
            },
            "model" => {
                print_colored("AI", Color::Blue, true);
                print_colored(": ", Color::White, true);
                print_colored(text, Color::White, false);
                println!();
            },
            _ => {
                print_colored(role, Color::Yellow, true);
                print_colored(": ", Color::White, true);
                print_colored(text, Color::White, false);
                println!();
            }
        }
        
        // Add a subtle separator between messages, but not after the last message
        if i < message_history.len() - 1 {
            print_colored("────────────────────────────────────────────────────────────────────────────────", Color::Rgb(100, 100, 100), false);
            println!();
        }
    }
    
    Ok(())
}