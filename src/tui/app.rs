use hal::prelude::Content;
use ratatui::text::Text;
use ratatui::widgets::ScrollbarState;
use crate::tui::markdown::markdown_to_ratatui_text;

/// Application state for the TUI
pub struct App {
    /// Message history for the chat
    pub message_history: Vec<Content>,
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
    /// Scrollbar state for chat history
    pub scrollbar_state: ScrollbarState,
    /// Current scroll position
    pub scroll_position: usize,
}

impl App {
    /// Create a new application state
    pub fn new() -> Self {
        Self {
            message_history: Vec::new(),
            input: String::new(),
            cursor_position: 0,
            should_quit: false,
            rendered_messages: Vec::new(),
            is_loading: false,
            spinner_frame: 0,
            scrollbar_state: ScrollbarState::default(),
            scroll_position: 0,
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

        // Update scrollbar state with new content length
        let total_height = self.calculate_total_height();
        self.scrollbar_state = ScrollbarState::default()
            .content_length(total_height);
    }
    
    /// Calculate total height of all messages
    fn calculate_total_height(&self) -> usize {
        self.rendered_messages.iter()
            .map(|(_, text)| text.height() + 2) // +2 for role line and separator
            .sum()
    }
    
    /// Scroll to show the latest content, given the available viewport height
    pub fn scroll_to_show_latest(&mut self, viewport_height: usize) {
        if self.rendered_messages.is_empty() {
            self.scrollbar_state = ScrollbarState::default().content_length(0);
            return;
        }
        
        // Calculate total height
        let total_height = self.calculate_total_height();
        
        // For content that fits in viewport, show everything from the start
        if total_height <= viewport_height {
            self.scrollbar_state = ScrollbarState::default().content_length(total_height);
            return;
        }
        
        // Otherwise, scroll to show the latest content
        self.scrollbar_state = ScrollbarState::default()
            .content_length(total_height)
            .position(total_height.saturating_sub(viewport_height / 2));
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
        let total_height = self.calculate_total_height();
        self.scroll_position = self.scroll_position.saturating_sub(1);
        self.scrollbar_state = ScrollbarState::default()
            .content_length(total_height)
            .position(self.scroll_position);
    }
    
    /// Scroll chat history down
    pub fn scroll_down(&mut self) {
        let total_height = self.calculate_total_height();
        let max_pos = total_height.saturating_sub(1);
        self.scroll_position = self.scroll_position.saturating_add(1).min(max_pos);
        self.scrollbar_state = ScrollbarState::default()
            .content_length(total_height)
            .position(self.scroll_position);
    }
    
    /// Scroll by a specific number of lines (positive = down, negative = up)
    pub fn scroll_by(&mut self, delta: i32) {
        let total_height = self.calculate_total_height();
        
        self.scroll_position = if delta < 0 {
            self.scroll_position.saturating_sub(delta.unsigned_abs() as usize)
        } else {
            let max_pos = total_height.saturating_sub(1);
            self.scroll_position.saturating_add(delta as usize).min(max_pos)
        };
        
        self.scrollbar_state = ScrollbarState::default()
            .content_length(total_height)
            .position(self.scroll_position);
    }
    
    /// Ensure scroll position is valid for current viewport
    pub fn clamp_scroll(&mut self, viewport_height: u16) {
        let total_height = self.calculate_total_height();
        if total_height <= viewport_height as usize {
            self.scroll_position = 0;
            self.scrollbar_state = ScrollbarState::default()
                .content_length(total_height)
                .position(0);
        } else {
            let max_scroll = total_height.saturating_sub(viewport_height as usize);
            self.scroll_position = self.scroll_position.min(max_scroll);
            self.scrollbar_state = ScrollbarState::default()
                .content_length(total_height)
                .position(self.scroll_position);
        }
    }
    
    /// Update spinner frame
    pub fn tick_spinner(&mut self) {
        if self.is_loading {
            self.spinner_frame = (self.spinner_frame + 1) % 8;
        }
    }
} 