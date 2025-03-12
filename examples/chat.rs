use hal::Client;
use hal::prelude::{Result, Content, Part};
use std::io::{self, Write};

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
    
    println!("Chat session started with gemini.");
    println!("Type your messages and press Enter to send.");
    println!("Type 'exit' to end the conversation.");
    println!("-----------------------------------------");
    
    // Main chat loop
    loop {
        // Display prompt and get user input
        print!("You: ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");
        
        let input = input.trim();
        
        // Check if user wants to exit
        if input.to_lowercase() == "exit" {
            println!("Ending chat session.");
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
                println!("AI: {}", response_text);
                
                // Add AI response to history
                message_history.push(Content::new().with_role("model").with_text(response_text));
            },
            Err(e) => {
                eprintln!("Error: {}", e);
                println!("AI: Sorry, I encountered an error processing your request.");
                
                // Add error response to history
                message_history.push(Content::new().with_role("model").with_text("Sorry, I encountered an error processing your request."));
            }
        }
        
        println!();
    }
    
    // Display message history at the end
    println!("-----------------------------------------");
    println!("Chat History:");
    for message in &message_history {
        let role = message.role.as_deref().unwrap_or("unknown");
        let text = message.parts.first().map_or("", |part| {
            if let Part::Text(text) = part {
                text
            } else {
                "[non-text content]"
            }
        });
        println!("{}: {}", role, text);
    }
    
    Ok(())
}