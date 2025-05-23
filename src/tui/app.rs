//! # TUI Application State Module
//!
//! This module provides the core application state management for the terminal user interface.
//! It handles user input, event processing, and maintains the conversation state.
//!
//! ## Key Components
//!
//! - `App`: Main application state structure that tracks all UI state
//!
//! ## Features
//!
//! - Text input handling with cursor management
//! - Multi-line text editing with proper line wrapping
//! - Mouse and keyboard event processing
//! - Scrolling for both chat history and input field
//! - Message rendering with markdown support
//! - Loading indicator animations
//! - Debug logging capabilities
//!
//! The application state is the central component that coordinates all user interactions,
//! manages the display of messages, and handles input editing with support for complex
//! text operations like cursor positioning and scrolling.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
use ratatui::text::Text;
use tokio::sync::mpsc;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::tui::error::{Error, Result};
use crate::tui::event::{AppEvent, Event, EventHandler};
use crate::tui::markdown::markdown_to_ratatui_text;
use crate::tui::scroll::ScrollState;

/// Application state
pub struct App {
    /// Current input text
    pub input: String,
    /// Cursor position in the input field
    pub cursor_position: usize,
    /// Flag to indicate if the application should quit
    pub should_quit: bool,
    /// Rendered messages for display
    pub rendered_messages: Vec<(String, Text<'static>)>, // (role, rendered_text)
    /// Flag to indicate if we're waiting for LLM response
    pub is_loading: bool,
    /// Counter for spinner animation frames
    pub spinner_frame: usize,
    /// Enhanced scroll state for chat history
    pub chat_scroll: ScrollState,
    /// Enhanced scroll state for input field
    pub input_scroll: ScrollState,
    /// Event handler
    event_handler: EventHandler,
}

impl App {
    /// Create a new application state
    pub fn new() -> Self {
        // Only create debug log file if HAL_TUI_DEBUG environment variable is set
        if std::env::var("HAL_TUI_DEBUG").is_ok() {
            let _ = std::fs::write("hal-debug.log", "Debug log started\n");
        }

        Self {
            input: String::new(),
            cursor_position: 0,
            should_quit: false,
            rendered_messages: Vec::new(),
            is_loading: false,
            spinner_frame: 0,
            chat_scroll: ScrollState::new(),
            input_scroll: ScrollState::new(),
            event_handler: EventHandler::new(),
        }
    }

    /// Get the next event
    pub async fn next_event(&mut self) -> Option<Event> {
        if let Some(event) = self.event_handler.next().await {
            match &event {
                Event::Terminal(term_event) => {
                    if let Err(e) = self.handle_terminal_event(term_event) {
                        eprintln!("Error handling terminal event: {}", e);
                    }
                }
                Event::Tick => {
                    self.tick_spinner();
                }
                Event::App(app_event) => {
                    if let Err(e) = self.handle_app_event(app_event) {
                        eprintln!("Error handling app event: {}", e);
                    }
                }
            }
            Some(event)
        } else {
            None
        }
    }

    /// Get the event sender
    pub fn event_sender(&self) -> mpsc::UnboundedSender<Event> {
        self.event_handler.sender()
    }

    /// Handle terminal events
    fn handle_terminal_event(&mut self, event: &crossterm::event::Event) -> Result<()> {
        match event {
            crossterm::event::Event::Key(key) => self.handle_key_event(*key)?,
            crossterm::event::Event::Mouse(mouse) => {
                // Get terminal size
                if let Ok((width, height)) = crossterm::terminal::size() {
                    // Input area is the bottom 5 lines
                    let is_input_area = mouse.row >= height.saturating_sub(5);

                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            if is_input_area {
                                self.debug_log(&format!(
                                    "Input scroll up at row {} (terminal height: {})",
                                    mouse.row, height
                                ));

                                // Calculate total lines and visible height
                                let mut total_lines = 0;
                                let lines: Vec<&str> = self.input.split('\n').collect();
                                for line in lines.iter() {
                                    let line_width = width.saturating_sub(2); // -2 for borders
                                    total_lines +=
                                        (line.width() as u16).saturating_sub(1) / line_width + 1;
                                }

                                let input_height = 3; // 5 - 2 for borders

                                // Update content size and then scroll up
                                self.input_scroll.update_content_size(
                                    total_lines as usize,
                                    input_height as usize,
                                );
                                self.input_scroll.scroll_by(-1);

                                self.debug_log(&format!(
                                    "Manual scroll up, new position: {}",
                                    self.input_scroll.position
                                ));

                                // Update cursor position to match the first visible line
                                self.update_cursor_for_scroll(width);
                            } else {
                                self.debug_log(&format!(
                                    "Chat scroll up at row {} (terminal height: {})",
                                    mouse.row, height
                                ));
                                self.chat_scroll.scroll_by(-1);
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if is_input_area {
                                self.debug_log(&format!(
                                    "Input scroll down at row {} (terminal height: {})",
                                    mouse.row, height
                                ));

                                // Calculate total lines and visible height
                                let mut total_lines = 0;
                                let lines: Vec<&str> = self.input.split('\n').collect();
                                for line in lines.iter() {
                                    let line_width = width.saturating_sub(2); // -2 for borders
                                    total_lines +=
                                        (line.width() as u16).saturating_sub(1) / line_width + 1;
                                }

                                let input_height = 3; // 5 - 2 for borders

                                // Update content size and then scroll down
                                self.input_scroll.update_content_size(
                                    total_lines as usize,
                                    input_height as usize,
                                );
                                self.input_scroll.scroll_by(1);

                                self.debug_log(&format!(
                                    "Manual scroll down, new position: {}",
                                    self.input_scroll.position
                                ));

                                // Update cursor position to match the first visible line
                                self.update_cursor_for_scroll(width);
                            } else {
                                self.debug_log(&format!(
                                    "Chat scroll down at row {} (terminal height: {})",
                                    mouse.row, height
                                ));
                                self.chat_scroll.scroll_by(1);
                            }
                        }
                        MouseEventKind::Down(_) => {
                            if is_input_area {
                                // Handle mouse click in input area
                                self.debug_log(&format!(
                                    "Mouse click in input area at row {}, column {}",
                                    mouse.row, mouse.column
                                ));

                                // Calculate the position in the input text based on click coordinates
                                let input_area_height = 5; // Total height of input area

                                // Calculate relative position within input area
                                let relative_row = mouse.row - (height - input_area_height);
                                let relative_column = mouse.column.saturating_sub(1); // Subtract left border

                                if relative_row > 0 && relative_row < input_area_height - 1 {
                                    // Click is within the inner area (not on borders)
                                    let click_row =
                                        relative_row - 1 + self.input_scroll.position as u16;

                                    // Find the character position based on click coordinates
                                    self.set_cursor_position_from_click(
                                        click_row,
                                        relative_column,
                                        width.saturating_sub(2),
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle application events
    fn handle_app_event(&mut self, event: &AppEvent) -> Result<()> {
        match event {
            AppEvent::Submit(input) => {
                self.add_message("user", input);
                self.is_loading = true;
                self.reset_input();
            }
            AppEvent::LLMResponse(response) => {
                self.is_loading = false;
                self.add_message("model", response);
            }
            AppEvent::LLMError(error) => {
                self.is_loading = false;
                self.add_message("model", &format!("Error: {}", error));
            }
            AppEvent::Quit => {
                self.should_quit = true;
            }
        }
        Ok(())
    }

    /// Handle key events
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.event_handler
                    .sender()
                    .send(Event::App(AppEvent::Quit))
                    .map_err(|e| Error::Event(e.to_string()))?;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.event_handler
                    .sender()
                    .send(Event::App(AppEvent::Quit))
                    .map_err(|e| Error::Event(e.to_string()))?;
            }
            KeyCode::Enter => {
                // Log the key event details
                self.debug_log(&format!("Enter pressed - modifiers: {:?}", key.modifiers));

                if key.modifiers.contains(KeyModifiers::ALT) {
                    self.debug_log("Alt+Enter detected!");
                    // Insert newline at cursor position
                    self.input.insert(self.cursor_position, '\n');
                    self.cursor_position += 1;
                    self.debug_log(&format!(
                        "Input after newline: {:?}, cursor: {}",
                        self.input, self.cursor_position
                    ));

                    // Update scroll position after inserting newline
                    if let Ok((width, _)) = crossterm::terminal::size() {
                        self.update_input_scroll_position(width);
                    }
                } else {
                    self.debug_log("Regular Enter detected");
                    let input = self.input.trim().to_string();
                    if !input.is_empty() {
                        self.event_handler
                            .sender()
                            .send(Event::App(AppEvent::Submit(input)))
                            .map_err(|e| Error::Event(e.to_string()))?;
                    }
                }
            }
            KeyCode::Char(c) => {
                self.insert_char(c);
            }
            KeyCode::Backspace => {
                self.backspace();
            }
            KeyCode::Delete => {
                self.delete_char();
            }
            KeyCode::Left => {
                self.move_cursor_left();
            }
            KeyCode::Right => {
                self.move_cursor_right();
            }
            KeyCode::Up => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.scroll_up();
                }
            }
            KeyCode::Down => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.scroll_down();
                }
            }
            KeyCode::PageUp => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // Get a rough estimate of page size (half the terminal height)
                    if let Ok((_, height)) = crossterm::terminal::size() {
                        let page_size = (height / 2) as i32;
                        self.chat_scroll.scroll_by(-page_size);
                    } else {
                        // Fallback if terminal size can't be determined
                        self.chat_scroll.scroll_by(-10);
                    }
                }
            }
            KeyCode::PageDown => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // Get a rough estimate of page size (half the terminal height)
                    if let Ok((_, height)) = crossterm::terminal::size() {
                        let page_size = (height / 2) as i32;
                        self.chat_scroll.scroll_by(page_size);
                    } else {
                        // Fallback if terminal size can't be determined
                        self.chat_scroll.scroll_by(10);
                    }
                }
            }
            KeyCode::Home => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.scroll_to_top();
                }
            }
            KeyCode::End => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.scroll_to_bottom();
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Add a message to the chat history
    pub fn add_message(&mut self, role: &str, text: &str) {
        // Add to rendered messages for display
        let rendered_text = markdown_to_ratatui_text(text);
        self.rendered_messages
            .push((role.to_string(), rendered_text));

        // Calculate total height
        let _total_height = self.calculate_total_height();

        // Note: We'll update the scroll position in the UI rendering code
        // based on the viewport size and content height
    }

    /// Calculate total height of all messages
    pub fn calculate_total_height(&self) -> usize {
        self.rendered_messages
            .iter()
            .map(|(_, text)| text.height() + 2) // +2 for role line and separator
            .sum()
    }

    /// Reset the input field
    pub fn reset_input(&mut self) {
        self.input.clear();
        self.cursor_position = 0;
    }

    /// Move cursor left in the input field
    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            // Update scroll position after moving cursor
            if let Ok((width, _)) = crossterm::terminal::size() {
                self.update_input_scroll_position(width);
            }
        }
    }

    /// Move cursor right in the input field
    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.input.len() {
            self.cursor_position += 1;
            // Update scroll position after moving cursor
            if let Ok((width, _)) = crossterm::terminal::size() {
                self.update_input_scroll_position(width);
            }
        }
    }

    /// Insert character at cursor position
    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_position, c);
        self.cursor_position += 1;
        // Update scroll position after typing
        if let Ok((width, _)) = crossterm::terminal::size() {
            self.update_input_scroll_position(width);
        }
    }

    /// Delete character at cursor position
    pub fn delete_char(&mut self) {
        if self.cursor_position < self.input.len() {
            self.input.remove(self.cursor_position);
            // Update scroll position after typing
            if let Ok((width, _)) = crossterm::terminal::size() {
                self.update_input_scroll_position(width);
            }
        }
    }

    /// Delete character before cursor position (backspace)
    pub fn backspace(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.input.remove(self.cursor_position);
            // Update scroll position after typing
            if let Ok((width, _)) = crossterm::terminal::size() {
                self.update_input_scroll_position(width);
            }
        }
    }

    /// Scroll chat history up
    pub fn scroll_up(&mut self) {
        self.chat_scroll.scroll_by(-1);
    }

    /// Scroll chat history down
    pub fn scroll_down(&mut self) {
        self.chat_scroll.scroll_by(1);
    }

    /// Scroll to the top of chat history
    pub fn scroll_to_top(&mut self) {
        self.chat_scroll.scroll_to_top();
    }

    /// Scroll to the bottom of chat history
    pub fn scroll_to_bottom(&mut self) {
        self.chat_scroll.scroll_to_bottom();
    }

    // Remove the scroll_by method since we're now using chat_scroll.scroll_by directly

    /// Update spinner frame
    pub fn tick_spinner(&mut self) {
        if self.is_loading {
            self.spinner_frame = (self.spinner_frame + 1) % 8;
        }
    }

    /// Update input scroll position with proper clamping
    fn update_input_scroll_position(&mut self, terminal_width: u16) {
        // Get the visible height of the input area (5 lines minus borders)
        let input_height = 3; // 5 - 2 for borders

        // Calculate total lines in input
        let mut total_lines = 0;
        let lines: Vec<&str> = self.input.split('\n').collect();
        for line in lines.iter() {
            // Use the actual terminal width for calculations
            let width = terminal_width.saturating_sub(2); // -2 for borders
            total_lines += (line.width() as u16).saturating_sub(1) / width + 1;
        }

        // Calculate cursor's line position
        let before_cursor = self.input[..self.cursor_position].to_string();
        let cursor_lines: Vec<&str> = before_cursor.split('\n').collect();
        let current_line = cursor_lines.last().unwrap_or(&"");
        let line_count = cursor_lines.len().saturating_sub(1);

        // Use the actual terminal width for cursor position calculation
        let width = terminal_width.saturating_sub(2); // -2 for borders
        let cursor_y = (current_line.width() as u16 / width + line_count as u16) as usize;

        // Update the scroll state with the new content size
        self.input_scroll
            .update_content_size(total_lines as usize, input_height as usize);

        // Debug log the cursor position and scroll calculations
        self.debug_log(&format!(
            "Cursor position: {}, Line: {}, Total lines: {}, Input height: {}, Current scroll: {}",
            self.cursor_position, cursor_y, total_lines, input_height, self.input_scroll.position
        ));

        // Only adjust scroll if cursor is completely outside visible area
        // This allows manual scrolling to work while still keeping cursor in view when typing
        if cursor_y >= self.input_scroll.position + input_height as usize {
            // Cursor is below visible area
            let new_scroll = cursor_y - input_height as usize + 1;
            self.input_scroll.scroll_to(new_scroll);
            self.debug_log(&format!(
                "Scrolling to keep cursor visible (below): {}",
                new_scroll
            ));
        } else if cursor_y < self.input_scroll.position {
            // Cursor is above visible area
            self.input_scroll.scroll_to(cursor_y);
            self.debug_log(&format!(
                "Scrolling to keep cursor visible (above): {}",
                cursor_y
            ));
        }
    }

    /// Set cursor position based on mouse click coordinates
    fn set_cursor_position_from_click(
        &mut self,
        click_row: u16,
        click_column: u16,
        line_width: u16,
    ) {
        self.debug_log(&format!(
            "Setting cursor from click at row {}, column {}",
            click_row, click_column
        ));

        // Split input into lines
        let lines: Vec<&str> = self.input.split('\n').collect();

        // Find which logical line the click is on
        let mut current_row = 0;
        let mut char_index = 0;

        for (i, line) in lines.iter().enumerate() {
            let line_height = (line.width() as u16).saturating_sub(1) / line_width + 1;

            if current_row <= click_row && click_row < current_row + line_height {
                // Click is on this line
                let row_in_line = click_row - current_row;
                let column_in_line = if row_in_line == 0 {
                    click_column
                } else {
                    click_column + (row_in_line * line_width)
                };

                // Find the character at this position
                let mut char_pos = 0;
                for c in line.chars() {
                    if char_pos >= column_in_line {
                        break;
                    }
                    char_pos += c.width().unwrap_or(1) as u16;
                    char_index += 1;
                }

                // Add newlines and previous lines
                if i > 0 {
                    char_index += i; // Add one for each newline
                    for prev_line in &lines[0..i] {
                        char_index += prev_line.len();
                    }
                }

                break;
            }

            current_row += line_height;
            char_index += line.len() + 1; // +1 for newline
        }

        // Clamp to valid range
        self.cursor_position = char_index.min(self.input.len());
        self.debug_log(&format!("Set cursor position to {}", self.cursor_position));

        // Update scroll position to ensure cursor is visible
        if let Ok((width, _)) = crossterm::terminal::size() {
            // Set scroll position to match the clicked row
            self.input_scroll
                .scroll_to(click_row.saturating_sub(1) as usize);
            self.debug_log(&format!(
                "Updated scroll position to {}",
                self.input_scroll.position
            ));

            // Make sure the cursor is visible with the new scroll position
            self.update_input_scroll_position(width);
        }
    }

    /// Update cursor position to match the first visible line after scrolling
    fn update_cursor_for_scroll(&mut self, terminal_width: u16) {
        // If input is empty, nothing to do
        if self.input.is_empty() {
            return;
        }

        let width = terminal_width.saturating_sub(2); // -2 for borders
        let scroll_pos = self.input_scroll.position;

        // Calculate character position for each line
        let mut line_starts: Vec<usize> = Vec::new();
        line_starts.push(0); // First line starts at position 0

        // Find all line breaks in the input
        let mut pos = 0;
        while let Some(newline_pos) = self.input[pos..].find('\n') {
            pos += newline_pos + 1; // +1 to move past the newline
            line_starts.push(pos);
        }

        // Add a sentinel value for the end of the text
        line_starts.push(self.input.len());

        // Calculate the visual line (accounting for wrapping) for each logical line
        let mut visual_line = 0_usize;
        let mut target_char_pos = 0;

        for i in 0..line_starts.len() - 1 {
            let line_start = line_starts[i];
            let line_end = line_starts[i + 1];
            let line = if line_end > 0 && self.input.as_bytes()[line_end - 1] == b'\n' {
                &self.input[line_start..line_end - 1]
            } else {
                &self.input[line_start..line_end]
            };

            // Calculate how many visual lines this logical line takes up
            let line_width = line.width();
            let wrapped_lines = ((line_width as u16).saturating_sub(1) / width + 1) as usize;

            // If this line is visible (or partially visible)
            if visual_line + wrapped_lines > scroll_pos {
                // Calculate how many characters to skip to get to the visible part
                let visible_wrapped_line = scroll_pos.saturating_sub(visual_line);
                if visible_wrapped_line > 0 {
                    // We're in the middle of a wrapped line
                    // Calculate approximately how many characters to skip
                    let chars_to_skip = visible_wrapped_line * (width as usize);

                    // Count characters up to the visible part
                    let mut char_count = 0;
                    let mut char_width = 0_usize;
                    for c in line.chars() {
                        char_width += c.width().unwrap_or(1);
                        char_count += 1;
                        if char_width >= chars_to_skip {
                            break;
                        }
                    }

                    target_char_pos = line_start + char_count;
                } else {
                    // We're at the beginning of a line
                    target_char_pos = line_start;
                }

                self.debug_log(&format!(
                    "Found visible line at position {}, visual line {}, scroll pos {}",
                    target_char_pos, visual_line, scroll_pos
                ));
                break;
            }

            visual_line += wrapped_lines;
        }

        // Update cursor position
        self.cursor_position = target_char_pos.min(self.input.len());
        self.debug_log(&format!(
            "Updated cursor position to {} after scrolling",
            self.cursor_position
        ));
    }

    // Helper function to write debug logs
    fn debug_log(&self, message: &str) {
        // Only write debug logs if HAL_DEBUG environment variable is set
        if std::env::var("HAL_TUI_DEBUG").is_ok() {
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("hal-debug.log")
            {
                use std::io::Write;
                let _ = writeln!(file, "{}", message);
            }
        }
    }
}
