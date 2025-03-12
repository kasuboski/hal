pub mod app;
pub mod ui;
pub mod markdown;

use std::io;
use std::time::{Duration, Instant};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use hal::Client;
use hal::prelude::Result;
use tokio::sync::mpsc;

use crate::tui::app::App;
use crate::tui::ui::draw;

/// Run the TUI application
pub async fn run(api_key: String) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize the client
    let client = Client::with_api_key(api_key);
    
    // Create a chat session
    let chat = client.chats().create("gemini-2.0-flash").await?;
    
    // Create app state
    let mut app = App::new();
    
    // Add welcome message
    app.add_message(
        "ui", 
        "# Welcome to HAL Chat\n\n* Type your messages and press Enter to send.\n* Press Esc or Ctrl+C to exit.\n* Use arrow keys to navigate history."
    );

    // Create a channel for LLM responses
    let (tx, mut rx) = mpsc::channel(32);

    // Main loop
    let tick_rate = Duration::from_millis(50);
    let mut last_tick = Instant::now();
    
    loop {
        // Draw the UI on every iteration
        terminal.draw(|f| draw(f, &app))?;

        // Handle events with timeout
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => {
                        app.should_quit = true;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.should_quit = true;
                    }
                    KeyCode::Enter => {
                        let input = app.input.trim().to_string();
                        if !input.is_empty() {
                            // Add user message to history
                            app.add_message("user", &input);
                            
                            // Reset input field and set loading state
                            app.reset_input();
                            app.is_loading = true;
                            
                            // Clone necessary values for the async task
                            let chat = chat.clone();
                            let tx = tx.clone();
                            let message_history = app.message_history.clone();
                            
                            // Spawn the LLM request in the background
                            tokio::spawn(async move {
                                let result = chat.send_message(&input, Some(message_history.into_iter()
                                    .filter(|content| {
                                        // Only include user and model messages, filter out ui messages
                                        content.role.as_deref().unwrap_or("") != "ui"
                                    })
                                    .collect())).await;
                                let _ = tx.send(result).await;
                            });
                        }
                    }
                    KeyCode::Char(c) => {
                        app.insert_char(c);
                    }
                    KeyCode::Backspace => {
                        app.backspace();
                    }
                    KeyCode::Delete => {
                        app.delete_char();
                    }
                    KeyCode::Left => {
                        app.move_cursor_left();
                    }
                    KeyCode::Right => {
                        app.move_cursor_right();
                    }
                    KeyCode::Up => {
                        app.scroll_up();
                    }
                    KeyCode::Down => {
                        app.scroll_down();
                    }
                    _ => {}
                }
            }
        }

        // Check for LLM responses
        if let Ok(response) = rx.try_recv() {
            match response {
                Ok(response) => {
                    let response_text = response.text();
                    app.is_loading = false;
                    app.add_message("model", &response_text);
                },
                Err(e) => {
                    app.is_loading = false;
                    app.add_message("model", &format!("Error: {}", e));
                }
            }
        }
        
        // Check if we should tick
        if last_tick.elapsed() >= tick_rate {
            app.tick_spinner();
            last_tick = Instant::now();
        }
        
        if app.should_quit {
            break;
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