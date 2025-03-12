use hal::prelude::Content;
use ratatui::text::Text;
use crate::tui::markdown::markdown_to_ratatui_text;

/// Application state for the TUI
pub struct App {
    /// Message history for the chat
    pub message_history: Vec<Content>,
    /// Current input text
    pub input: String,
    /// Cursor position in the input field
    pub cursor_position: usize,
    /// Scroll position in the chat history
    pub chat_scroll: usize,
    /// Flag to indicate if the application should quit
    pub should_quit: bool,
    /// Rendered messages for display
    pub rendered_messages: Vec<(String, Text<'static>)>, // (role, rendered_text)
}

impl App {
    /// Create a new application state
    pub fn new() -> Self {
        Self {
            message_history: Vec::new(),
            input: String::new(),
            cursor_position: 0,
            chat_scroll: 0,
            should_quit: false,
            rendered_messages: Vec::new(),
        }
    }
    
    /// Add a message to the chat history
    pub fn add_message(&mut self, role: &str, text: &str) {
        let content = match role {
            "user" => Content::new().with_role("user").with_text(text),
            "model" => Content::new().with_role("model").with_text(text),
            _ => Content::new().with_role(role).with_text(text),
        };
        
        self.message_history.push(content);
        
        // Also add to rendered messages for display
        let rendered_text = markdown_to_ratatui_text(text);
        self.rendered_messages.push((role.to_string(), rendered_text));
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
        }
    }
    
    /// Move cursor right in the input field
    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.input.len() {
            self.cursor_position += 1;
        }
    }
    
    /// Insert character at cursor position
    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }
    
    /// Delete character at cursor position
    pub fn delete_char(&mut self) {
        if self.cursor_position < self.input.len() {
            self.input.remove(self.cursor_position);
        }
    }
    
    /// Delete character before cursor position (backspace)
    pub fn backspace(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.input.remove(self.cursor_position);
        }
    }
    
    /// Scroll chat history up
    pub fn scroll_up(&mut self) {
        if self.chat_scroll > 0 {
            self.chat_scroll -= 1;
        }
    }
    
    /// Scroll chat history down
    pub fn scroll_down(&mut self) {
        self.chat_scroll += 1;
    }
} 