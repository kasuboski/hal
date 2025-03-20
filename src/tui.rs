//! # Terminal User Interface Module
//! 
//! This module provides a terminal-based user interface for interacting with the
//! HAL framework, enabling chat-based RAG interactions without requiring a graphical
//! environment.
//! 
//! ## Key Components
//! 
//! - `app`: Application state management and event handling
//! - `error`: Error types specific to the TUI
//! - `event`: Event system for handling terminal and application events
//! - `logging`: Terminal-based logging utilities
//! - `markdown`: Markdown rendering for terminal display
//! - `ui`: UI rendering and layout components
//! 
//! ## Features
//! 
//! - Chat-based interface with markdown support
//! - Keyboard and mouse interaction
//! - Asynchronous LLM communication
//! - Multi-line text input with scrolling
//! - Loading indicators for ongoing operations
//! - Syntax highlighting for code blocks
//! - Responsive layout adapting to terminal size
//! 
//! The TUI module provides a complete terminal interface for RAG applications,
//! allowing users to interact with the system through a familiar chat interface
//! while leveraging the full capabilities of the HAL framework.

pub mod app;
pub mod error;
pub mod event;
pub mod logging;
pub mod markdown;
pub mod ui;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use hal::prelude::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use rig::{completion::Chat, message::Message, providers::gemini};
use std::io;
use tokio::sync::mpsc;

use crate::tui::app::App;
use crate::tui::event::{AppEvent, Event};
use crate::tui::ui::draw;

/// Run the TUI application
pub async fn run(api_key: String) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize the client and create agent
    let gemini = gemini::Client::new(&api_key);
    let client = hal::model::Client::new_gemini_free(gemini);
    let completion = client.completion().clone();
    let agent = completion
        .agent()
        .preamble("You are a helpful assistant.")
        .build();

    // Create app state
    let mut app = App::new();

    // Add welcome message
    app.add_message(
        "ui", 
        "# Welcome to HAL Chat\n\n* Type your messages and press Enter to send.\n* Press Alt+Enter (Option+Enter on macOS) to add a new line.\n* Use mouse wheel to scroll chat history and input field.\n* Press Esc or Ctrl+C to exit."
    );

    // Create channels for LLM communication
    let (llm_tx, mut llm_rx) = mpsc::unbounded_channel::<String>();
    let event_sender = app.event_sender();

    // Set up LLM response handler
    // let agent_clone = agent.clone();
    tokio::spawn(async move {
        let mut message_history = Vec::new();
        while let Some(input) = llm_rx.recv().await {
            match agent.chat(input.as_ref(), message_history.clone()).await {
                Ok(response) => {
                    message_history.push(Message::user(input));
                    message_history.push(Message::assistant(&response));
                    let _ = event_sender.send(Event::App(AppEvent::LLMResponse(response)));
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
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
