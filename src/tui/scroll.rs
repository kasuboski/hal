use std::cmp::min;

/// Direction of scrolling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    /// Scroll up (towards the start)
    Up,
    /// Scroll down (towards the end)
    Down,
}

/// Enhanced scroll management
#[derive(Debug, Clone)]
pub struct ScrollState {
    /// Current scroll position
    pub position: usize,
    /// Maximum scroll position
    pub max_position: usize,
    /// Whether view is "stuck" to bottom
    pub stick_to_bottom: bool,
    /// Store last scroll direction for multi-step scrolling
    pub last_direction: Option<ScrollDirection>,
}

impl ScrollState {
    /// Create a new scroll state
    pub fn new() -> Self {
        Self {
            position: 0,
            max_position: 0,
            stick_to_bottom: true,
            last_direction: None,
        }
    }

    /// Immediately scroll to a specific position
    pub fn scroll_to(&mut self, position: usize) {
        let clamped_position = position.min(self.max_position);
        self.position = clamped_position;
        self.stick_to_bottom = clamped_position >= self.max_position;
    }

    /// Scroll by a specific number of lines
    pub fn scroll_by(&mut self, delta: i32) {
        if delta > 0 {
            self.last_direction = Some(ScrollDirection::Down);
            let new_pos = self.position.saturating_add(delta as usize);
            self.scroll_to(new_pos);
        } else {
            self.last_direction = Some(ScrollDirection::Up);
            let new_pos = self.position.saturating_sub(delta.unsigned_abs() as usize);
            self.scroll_to(new_pos);
        }
    }

    /// Scroll by page (visible height)
    #[allow(dead_code)]
    pub fn scroll_page(&mut self, delta: i32, page_height: usize) {
        self.scroll_by(delta * page_height as i32);
    }

    /// Scroll to the bottom
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_to(self.max_position);
        self.stick_to_bottom = true;
    }

    /// Scroll to the top
    pub fn scroll_to_top(&mut self) {
        self.scroll_to(0);
        self.stick_to_bottom = false;
    }

    /// Update max position and maintain relative position if needed
    pub fn update_content_size(&mut self, content_height: usize, viewport_height: usize) {
        let _old_max = self.max_position;
        self.max_position = content_height.saturating_sub(viewport_height).max(0);

        // If stuck to bottom, maintain that position
        if self.stick_to_bottom {
            self.position = self.max_position;
        } else if content_height > viewport_height {
            // Clamp current position to valid range
            self.position = min(self.position, self.max_position);
        }
    }

    /// Check if we're at the top boundary
    #[allow(dead_code)]
    pub fn is_at_top(&self) -> bool {
        self.position == 0
    }

    /// Check if we're at the bottom boundary
    #[allow(dead_code)]
    pub fn is_at_bottom(&self) -> bool {
        self.position >= self.max_position
    }
}
