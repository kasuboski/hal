use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};

/// Render an enhanced scrollbar with better visibility
pub fn render_enhanced_scrollbar(
    buffer: &mut Buffer,
    area: Rect,
    position: usize,
    max_position: usize,
    viewport_height: usize,
) {
    // Only show scrollbar if needed
    if max_position == 0 {
        return;
    }

    // Calculate thumb attributes
    let _total_height = area.height as usize;
    let content_ratio = viewport_height as f64 / (max_position + viewport_height) as f64;
    let thumb_height = (area.height as f64 * content_ratio).max(1.0) as u16;

    let scroll_progress = if max_position > 0 {
        position as f64 / max_position as f64
    } else {
        0.0
    };

    let thumb_top = area.top() + (scroll_progress * (area.height - thumb_height) as f64) as u16;

    // Select style based on scroll position
    let style = if position == 0 {
        // At top
        Style::default().fg(Color::Yellow)
    } else if position >= max_position {
        // At bottom
        Style::default().fg(Color::Yellow)
    } else {
        // Normal position
        Style::default().fg(Color::Gray)
    };

    // Draw scrollbar track and thumb
    for y in area.top()..area.bottom() {
        let cell_style = if y >= thumb_top && y < thumb_top + thumb_height {
            style
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Use different characters for thumb and track
        let symbol = if y >= thumb_top && y < thumb_top + thumb_height {
            "█"
        } else {
            "│"
        };

        // Add top/bottom indicators
        let symbol = if y == area.top() && position > 0 {
            "▲"
        } else if y == area.bottom() - 1 && position < max_position {
            "▼"
        } else {
            symbol
        };

        // Use direct indexing instead of get
        let x = area.right() - 1;
        buffer[(x, y)].set_symbol(symbol);
        buffer[(x, y)].set_style(cell_style);
    }
}
