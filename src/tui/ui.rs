use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap, Clear},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::tui::app::App;

const SPINNER_FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

/// Draw the UI
pub fn draw(f: &mut Frame, app: &App) {
    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),     // Chat history
            Constraint::Length(3),   // Input field
        ])
        .split(f.size());
    
    // Render chat history
    render_messages(f, app, chunks[0]);
    
    // Render input field
    render_input(f, app, chunks[1]);
}

/// Render chat messages
fn render_messages(f: &mut Frame, app: &App, area: Rect) {
    let messages_block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            "Chat History",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));
    
    let inner_area = messages_block.inner(area);
    f.render_widget(messages_block, area);
    
    // Create a paragraph for each message
    let mut current_y: u16 = 0;
    let mut total_height: u16 = 0;
    
    // First pass: calculate total height and find which messages to render
    let mut start_message = 0;
    let mut accumulated_height: u16 = 0;
    for (i, (_, text)) in app.rendered_messages.iter().enumerate() {
        let message_height = text.height() as u16 + 2; // +2 for role line and separator
        
        // If we haven't scrolled past this message yet, it's our starting point
        if accumulated_height + message_height > app.line_scroll as u16 {
            start_message = i;
            // Remember how many lines we've scrolled into this message
            total_height = accumulated_height;
            break;
        }
        
        accumulated_height += message_height;
        
        // If we've scrolled past this message entirely, it becomes our start
        if accumulated_height <= app.line_scroll as u16 {
            start_message = i + 1;
            total_height = accumulated_height;
        }
    }
    
    // Reset current_y for second pass
    current_y = 0;
    
    // Second pass: render visible messages
    for (i, (role, text)) in app.rendered_messages.iter().enumerate().skip(start_message) {
        let message_height = text.height() as u16 + 2; // +2 for role line and separator
        
        // Skip if this message would start beyond our viewport
        if current_y >= inner_area.height {
            break;
        }
        
        let role_style = match role.as_str() {
            "user" => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            "model" => Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
            _ => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        };
        
        // Calculate the visible portion of this message
        let visible_height = message_height.min(inner_area.height.saturating_sub(current_y));
        if visible_height == 0 {
            break;
        }
        
        // Render role indicator
        let role_text = match role.as_str() {
            "user" => "You",
            "model" => "AI",
            _ => role,
        };
        
        let role_span = Span::styled(format!("{}: ", role_text), role_style);
        let role_line = Line::from(vec![role_span]);
        
        // Create a sub-area for this message
        let message_area = Rect {
            x: inner_area.x,
            y: inner_area.y + current_y,
            width: inner_area.width,
            height: visible_height,
        };
        
        // Render role line
        let role_area = Rect {
            height: 1,
            ..message_area
        };
        f.render_widget(Paragraph::new(role_line), role_area);
        
        // Render message content
        let content_area = Rect {
            y: message_area.y + 1,
            height: message_area.height.saturating_sub(1),
            ..message_area
        };
        
        f.render_widget(
            Paragraph::new(text.clone())
                .wrap(Wrap { trim: true }),
            content_area,
        );
        
        // Update current_y
        current_y += visible_height;
        
        // Add separator between messages
        if current_y < inner_area.height && i < app.rendered_messages.len() - 1 {
            let separator_area = Rect {
                y: inner_area.y + current_y,
                height: 1,
                ..inner_area
            };
            
            let separator = Line::from(vec![Span::styled(
                "────────────────────────────────────────────────────────────────────────────────",
                Style::default().fg(Color::DarkGray),
            )]);
            
            f.render_widget(Paragraph::new(separator), separator_area);
            current_y += 1;
        }
    }

    // Show spinner if loading
    if app.is_loading && current_y < inner_area.height {
        let spinner_area = Rect {
            y: inner_area.y + current_y,
            height: 1,
            ..inner_area
        };

        let spinner_line = Line::from(vec![
            Span::styled(
                format!("{} Thinking...", SPINNER_FRAMES[app.spinner_frame]),
                Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
            )
        ]);

        f.render_widget(Paragraph::new(spinner_line), spinner_area);
    }
}

/// Render input field
fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            "Input",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));
    
    let inner_area = input_block.inner(area);
    
    // Render input field
    let input = Paragraph::new(app.input.as_str())
        .style(Style::default())
        .block(input_block);
    
    f.render_widget(input, area);
    
    // Render cursor
    if app.cursor_position <= app.input.len() {
        // Make sure cursor is visible even when it's at the end of the input
        let cursor_x = app.input[..app.cursor_position].width() as u16;
        
        f.set_cursor(
            inner_area.x + cursor_x,
            inner_area.y,
        );
    }
}

/// Render a popup with the given title and text
#[allow(dead_code)]
pub fn render_popup(f: &mut Frame, title: &str, text: &str) {
    let size = f.size();
    
    // Calculate popup size
    let width = size.width.min(50);
    let height = size.height.min(10);
    let x = (size.width - width) / 2;
    let y = (size.height - height) / 2;
    
    let popup_area = Rect::new(x, y, width, height);
    
    // Render a clear widget to create a blank background
    f.render_widget(Clear, popup_area);
    
    // Render the popup block
    let popup_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black))
        .title(Span::styled(
            title,
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
    
    f.render_widget(popup_block.clone(), popup_area);
    
    // Render the text inside the popup
    let inner_area = popup_block.inner(popup_area);
    let text = Paragraph::new(text)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    
    f.render_widget(text, inner_area);
} 