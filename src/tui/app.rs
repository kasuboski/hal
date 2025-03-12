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
    /// Scroll position in the chat history (in lines)
    pub line_scroll: usize,
    /// Flag to indicate if the application should quit
    pub should_quit: bool,
    /// Rendered messages for display
    pub rendered_messages: Vec<(String, Text<'static>)>, // (role, rendered_text)
    /// Flag to indicate if we're waiting for LLM response
    pub is_loading: bool,
    /// Counter for spinner animation frames
    pub spinner_frame: usize,
}

impl App {
    /// Create a new application state
    pub fn new() -> Self {
        Self {
            message_history: Vec::new(),
            input: String::new(),
            cursor_position: 0,
            line_scroll: 0,
            should_quit: false,
            rendered_messages: Vec::new(),
            is_loading: false,
            spinner_frame: 0,
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

        // Auto-scroll to the bottom when a new message is added
        self.scroll_to_bottom();
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
            self.line_scroll = 0;
            return;
        }
        
        // Calculate total height
        let total_height = self.calculate_total_height();
        
        // For content that fits in viewport, show everything from the start
        if total_height <= viewport_height {
            self.line_scroll = 0;
            return;
        }
        
        // Otherwise, scroll to show the latest content while keeping as much context visible as possible
        self.line_scroll = total_height.saturating_sub(viewport_height);
    }
    
    /// Scroll to the bottom of the chat history
    /// Uses a reasonable default viewport height if actual height is not available
    pub fn scroll_to_bottom(&mut self) {
        // Use a reasonable minimum viewport height as default
        self.scroll_to_show_latest(20);
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
        // Scroll by 3 lines at a time for smoother scrolling
        if self.line_scroll >= 3 {
            self.line_scroll -= 3;
        } else {
            self.line_scroll = 0;
        }
    }
    
    /// Scroll chat history down
    pub fn scroll_down(&mut self) {
        let total_height = self.calculate_total_height();
        // Only scroll if there's more content below
        if self.line_scroll + 3 < total_height {
            self.line_scroll += 3;
        }
    }
    
    /// Scroll by a specific number of lines (positive = down, negative = up)
    pub fn scroll_by(&mut self, lines: i32) {
        let total_height = self.calculate_total_height();
        
        if lines < 0 {
            // Scrolling up
            let up_amount = lines.abs() as usize;
            if self.line_scroll >= up_amount {
                self.line_scroll -= up_amount;
            } else {
                self.line_scroll = 0;
            }
        } else {
            // Scrolling down - ensure we don't scroll past the content
            let down_amount = lines as usize;
            let max_scroll = total_height.saturating_sub(1); // Keep at least one line visible
            self.line_scroll = (self.line_scroll + down_amount).min(max_scroll);
        }
    }
    
    /// Ensure scroll position is valid for current viewport
    pub fn clamp_scroll(&mut self, viewport_height: usize) {
        let total_height = self.calculate_total_height();
        
        // If content fits in viewport, reset scroll to top
        if total_height <= viewport_height {
            self.line_scroll = 0;
            return;
        }
        
        // Otherwise ensure scroll position shows as much content as possible
        let max_scroll = total_height.saturating_sub(viewport_height);
        self.line_scroll = self.line_scroll.min(max_scroll);
    }
    
    /// Update spinner frame
    pub fn tick_spinner(&mut self) {
        if self.is_loading {
            self.spinner_frame = (self.spinner_frame + 1) % 8;
        }
    }
} 