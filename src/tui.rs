pub mod app;
pub mod event;
pub mod ui;
pub mod markdown;
pub mod error;

use std::io;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use hal::prelude::Result;
use tokio::sync::mpsc;

use crate::tui::app::App;
use crate::tui::event::{Event, AppEvent};
use crate::tui::ui::draw;

/// Run the TUI application
pub async fn run(api_key: String) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize the client
    let client = hal::Client::with_api_key(api_key);
    
    // Create a chat session
    let chat = client.chats().create("gemini-2.0-flash").await?;
    
    // Create app state
    let mut app = App::new();
    
    // Add welcome message
    app.add_message(
        "ui", 
        "# Welcome to HAL Chat\n\n* Type your messages and press Enter to send.\n* Press Esc or Ctrl+C to exit.\n* Use arrow keys to navigate history."
    );

    // Create channels for LLM communication
    let (llm_tx, mut llm_rx) = mpsc::unbounded_channel();
    let event_sender = app.event_sender();
    
    // Set up LLM response handler
    let chat_clone = chat.clone();
    tokio::spawn(async move {
        while let Some(input) = llm_rx.recv().await {
            let result = chat_clone.send_message(&input, None).await;
            match result {
                Ok(response) => {
                    let _ = event_sender.send(Event::App(AppEvent::LLMResponse(response.text())));
                }
                Err(e) => {
                    let _ = event_sender.send(Event::App(AppEvent::LLMError(e.to_string())));
                }
            }
        }
    });

    // Run the application
    terminal.clear()?;
    
    // Main event loop
    while !app.should_quit {
        // Draw the current state
        terminal.draw(|f| draw(f, &app))?;
        
        // Process the next event
        if let Some(event) = app.next_event().await {
            match event {
                Event::App(AppEvent::Submit(input)) => {
                    let _ = llm_tx.send(input);
                }
                Event::App(AppEvent::Quit) => {
                    app.should_quit = true;
                }
                _ => {} // Other events are handled by the App
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
} 