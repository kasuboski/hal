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

    // Main loop
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();
    
    loop {
        terminal.draw(|f| draw(f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.should_quit = true;
                    }
                    KeyCode::Enter => {
                        let input = app.input.trim().to_string();
                        if !input.is_empty() {
                            // Add user message to history
                            app.add_message("user", &input);
                            
                            // Reset input field
                            app.reset_input();
                            
                            // Redraw UI to show user message
                            terminal.draw(|f| draw(f, &app))?;
                            
                            // Send message to LLM and get response
                            match chat.send_message(&input, Some(app.message_history.clone().into_iter()
                                .filter(|content| {
                                    // Only include user and model messages, filter out ui messages
                                    content.role.as_deref().unwrap_or("") != "ui"
                                })
                                .collect())).await {
                                Ok(response) => {
                                    let response_text = response.text();
                                    app.add_message("model", &response_text);
                                },
                                Err(e) => {
                                    app.add_message("model", &format!("Error: {}", e));
                                }
                            }
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
        
        if last_tick.elapsed() >= tick_rate {
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