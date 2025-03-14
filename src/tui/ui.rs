use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
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
            Constraint::Min(1),    // Chat history
            Constraint::Length(5), // Input field (increased height)
        ])
        .split(f.area());

    // Render chat history
    render_messages(f, app, chunks[0]);

    // Render input field
    render_input(f, app, chunks[1]);
}

/// Render chat messages
fn render_messages(f: &mut Frame, app: &App, area: Rect) {
    let messages_block = Block::default().borders(Borders::ALL).title(Span::styled(
        "Chat History",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));

    f.render_widget(messages_block.clone(), area);

    // Create a paragraph for each message
    let mut lines: Vec<Line> = Vec::new();

    for (i, (role, text)) in app.rendered_messages.iter().enumerate() {
        let role_style = match role.as_str() {
            "user" => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            "model" => Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            _ => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        };

        // Render role indicator
        let role_text = match role.as_str() {
            "user" => "You",
            "model" => "AI",
            _ => role,
        };

        let role_span = Span::styled(format!("{}: ", role_text), role_style);
        lines.push(Line::from(vec![role_span]));

        // Add message content lines
        lines.extend(text.lines.clone());

        // Add separator between messages
        if i < app.rendered_messages.len() - 1 {
            lines.push(Line::from(vec![Span::styled(
                "────────────────────────────────────────────────────────────────────────────────",
                Style::default().fg(Color::DarkGray),
            )]));
        }
    }

    // Show spinner if loading
    if app.is_loading {
        lines.push(Line::from(vec![Span::styled(
            format!("{} Thinking...", SPINNER_FRAMES[app.spinner_frame]),
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )]));
    }

    // Get current scroll position
    let total_height = lines.len();

    // Render all messages in a single paragraph with scrolling
    let messages = Paragraph::new(lines.clone())
        .block(messages_block)
        .wrap(Wrap { trim: true })
        .scroll((app.scroll_position as u16, 0));

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));

    // Create local scrollbar state for rendering
    let mut scrollbar_state = ScrollbarState::default()
        .content_length(total_height)
        .position(app.scroll_position);

    // Render messages and scrollbar
    f.render_widget(messages, area);
    f.render_stateful_widget(
        scrollbar,
        area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );
}

/// Render input field
fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let input_block = Block::default().borders(Borders::ALL).title(Span::styled(
        "Input",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));

    let inner_area = input_block.inner(area);

    // Calculate total number of lines in input
    let all_lines: Vec<&str> = app.input.split('\n').collect();
    let mut total_lines = 0;
    for line in all_lines.iter() {
        total_lines += (line.width() as u16).saturating_sub(1) / inner_area.width + 1;
    }

    // Get scroll offset from app state
    let scroll_offset = app.input_scroll_position as u16;

    // Calculate cursor position more accurately
    let mut cursor_x = 0;
    let mut cursor_y = 0;

    if app.cursor_position <= app.input.len() {
        // Split input at cursor position
        let before_cursor = app.input[..app.cursor_position].to_string();

        // Count newlines before cursor to get line number
        let lines: Vec<&str> = before_cursor.split('\n').collect();
        let line_count = lines.len().saturating_sub(1);

        // Get the current line the cursor is on
        let current_line = lines.last().unwrap_or(&"");

        // Calculate cursor position on current line
        cursor_x = current_line.width() as u16 % inner_area.width;

        // Calculate vertical position including wrapped lines from previous lines
        let mut total_y = 0;
        for (i, line) in all_lines.iter().enumerate() {
            match i.cmp(&line_count) {
                std::cmp::Ordering::Less => {
                    // Add height of previous complete lines
                    total_y += (line.width() as u16).saturating_sub(1) / inner_area.width + 1;
                }
                std::cmp::Ordering::Equal => {
                    // Add height of current line up to cursor
                    total_y += current_line.width() as u16 / inner_area.width;
                    break;
                }
                std::cmp::Ordering::Greater => {
                    // We've processed all lines up to the cursor
                    break;
                }
            }
        }

        cursor_y = total_y;
    }

    // Adjust cursor_y based on scroll offset
    let visible_cursor_y = cursor_y.saturating_sub(scroll_offset);

    // Render input field with text wrapping and scrolling
    let input = Paragraph::new(app.input.as_str())
        .style(Style::default())
        .block(input_block)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));

    f.render_widget(input, area);

    // Add scrollbar if content exceeds visible area
    if total_lines > inner_area.height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        let mut scrollbar_state = ScrollbarState::default()
            .content_length(total_lines as usize)
            .position(scroll_offset as usize);

        f.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }

    // Always show cursor, clamping to visible area if needed
    let clamped_cursor_y = visible_cursor_y.min(inner_area.height.saturating_sub(1));

    // Set cursor position
    f.set_cursor_position((inner_area.x + cursor_x, inner_area.y + clamped_cursor_y));
}

/// Render a popup with the given title and text
#[allow(dead_code)]
pub fn render_popup(f: &mut Frame, title: &str, text: &str) {
    let size = f.area();

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
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));

    f.render_widget(popup_block.clone(), popup_area);

    // Render the text inside the popup
    let inner_area = popup_block.inner(popup_area);
    let text = Paragraph::new(text)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });

    f.render_widget(text, inner_area);
}
