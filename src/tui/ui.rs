use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

use crate::tui::app::App;
use crate::tui::scrollbar::render_enhanced_scrollbar;

const SPINNER_FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

/// Draw the UI
pub fn draw(f: &mut Frame, app: &mut App) {
    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Chat history
            Constraint::Length(5), // Input field (increased height)
            Constraint::Length(1), // Status bar
        ])
        .split(f.area());

    // Render chat history
    render_messages(f, app, chunks[0]);

    // Render input field
    render_input(f, app, chunks[1]);

    // Render status bar
    render_command_help(f, chunks[2]);
}

/// Render chat messages
fn render_messages(f: &mut Frame, app: &mut App, area: Rect) {
    let messages_block = Block::default().borders(Borders::ALL).title(Span::styled(
        "Chat History",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));

    f.render_widget(messages_block.clone(), area);
    let inner_area = messages_block.inner(area);

    // Determine visible content based on scroll position
    let content_height = app.calculate_total_height();
    let viewport_height = inner_area.height as usize;

    // Update scroll state with current content
    app.chat_scroll
        .update_content_size(content_height, viewport_height);

    // Get visible content range based on scroll position
    let scroll_offset = app.chat_scroll.position;

    // Create a paragraph for each message
    let mut lines: Vec<Line> = Vec::new();
    let mut current_height = 0;
    // Keep track of what is visible
    let mut _visible_message_count = 0;

    for (i, (role, text)) in app.rendered_messages.iter().enumerate() {
        let message_height = text.height() + 2; // +2 for role line and separator

        // Check if this message is visible in the viewport
        let message_start = current_height;
        let message_end = current_height + message_height;

        if message_end > scroll_offset && message_start < scroll_offset + viewport_height {
            // Process only visible messages
            _visible_message_count += 1;

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

            // Skip lines that are above the visible area
            let lines_to_skip = scroll_offset.saturating_sub(message_start);

            // Only add the role line if it's visible
            if lines_to_skip == 0 {
                lines.push(Line::from(vec![role_span]));
            }

            // Add message content lines, but only those in view
            // Handle partial message visibility
            if lines_to_skip <= 1 {
                // Role line counts as 1
                let content_lines_to_skip = 0;
                let visible_lines = text
                    .lines
                    .iter()
                    .skip(content_lines_to_skip)
                    .take(viewport_height.saturating_sub(lines.len()))
                    .cloned();
                lines.extend(visible_lines);
            } else {
                // Skip some content lines
                let content_lines_to_skip = lines_to_skip - 1; // -1 for role line
                let visible_lines = text
                    .lines
                    .iter()
                    .skip(content_lines_to_skip)
                    .take(viewport_height.saturating_sub(lines.len()))
                    .cloned();
                lines.extend(visible_lines);
            }

            // Add separator between messages
            if i < app.rendered_messages.len() - 1 && lines.len() < viewport_height {
                lines.push(Line::from(vec![Span::styled(
                    "────────────────────────────────────────────────────────────────────────────────",
                    Style::default().fg(Color::DarkGray),
                )]));
            }
        }

        current_height += message_height;

        // Stop processing once we've filled the viewport
        if lines.len() >= viewport_height {
            break;
        }
    }

    // Show spinner if loading
    if app.is_loading && lines.len() < viewport_height {
        lines.push(Line::from(vec![Span::styled(
            format!("{} Thinking...", SPINNER_FRAMES[app.spinner_frame]),
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )]));
    }

    // Render messages without scrolling - we've already windowed the content
    let messages = Paragraph::new(lines).wrap(Wrap { trim: true });

    // Render messages
    f.render_widget(messages, inner_area);

    // Render enhanced scrollbar
    render_enhanced_scrollbar(
        f.buffer_mut(),
        inner_area,
        app.chat_scroll.position,
        app.chat_scroll.max_position,
        viewport_height,
    );

    // Optional position indicator
    if app.chat_scroll.max_position > 0 {
        let position_text = format!(
            "{}/{}",
            app.chat_scroll.position.saturating_add(1),
            app.chat_scroll.max_position.saturating_add(1)
        );

        let position_widget = Paragraph::new(Span::styled(
            position_text,
            Style::default().fg(Color::DarkGray),
        ));

        let position_area = Rect::new(inner_area.right() - 10, inner_area.top(), 10, 1);

        f.render_widget(position_widget, position_area);
    }
}

/// Render input field
fn render_input(f: &mut Frame, app: &mut App, area: Rect) {
    let input_block = Block::default().borders(Borders::ALL).title(Span::styled(
        "Input",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));

    f.render_widget(input_block.clone(), area);
    let inner_area = input_block.inner(area);

    // Calculate total number of lines in input
    let all_lines: Vec<&str> = app.input.split('\n').collect();
    let mut total_lines = 0;
    for line in all_lines.iter() {
        total_lines += (line.width() as u16).saturating_sub(1) / inner_area.width + 1;
    }

    // Update input scroll state
    app.input_scroll
        .update_content_size(total_lines as usize, inner_area.height as usize);

    // Get scroll offset from app state
    let scroll_offset = app.input_scroll.position as u16;

    // Calculate cursor position
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

    // Create input lines with Claude Code-inspired prompt
    let prompt = "> ";

    // Create a modified input text with prompt
    let display_lines: Vec<Line> = app
        .input
        .split('\n')
        .enumerate()
        .map(|(i, line)| {
            if i == 0 {
                // Add prompt to first line
                Line::from(vec![
                    Span::styled(prompt, Style::default().fg(Color::Green)),
                    Span::raw(line),
                ])
            } else {
                // Indent continuation lines to align with text after prompt
                Line::from(vec![
                    Span::raw("  "), // Same width as prompt
                    Span::raw(line),
                ])
            }
        })
        .collect();

    // Render input text with scrolling
    let input = Paragraph::new(display_lines)
        .style(Style::default())
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));

    f.render_widget(input, inner_area);

    // Add enhanced scrollbar if content exceeds visible area
    if total_lines > inner_area.height {
        render_enhanced_scrollbar(
            f.buffer_mut(),
            Rect::new(
                inner_area.right() - 1,
                inner_area.top(),
                1,
                inner_area.height,
            ),
            app.input_scroll.position,
            app.input_scroll.max_position,
            inner_area.height as usize,
        );
    }

    // Always show cursor, clamping to visible area if needed
    let clamped_cursor_y = visible_cursor_y.min(inner_area.height.saturating_sub(1));

    // Set cursor position, adjusting for prompt on first line
    let adjusted_cursor_x = if cursor_y == 0 {
        // First line - account for prompt width
        cursor_x + prompt.len() as u16
    } else {
        // Other lines - account for indentation
        cursor_x + 2
    };

    // Set cursor position
    f.set_cursor_position((
        inner_area.x + adjusted_cursor_x,
        inner_area.y + clamped_cursor_y,
    ));
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

/// Render command help bar
fn render_command_help(f: &mut Frame, area: Rect) {
    // Create a background block with subtle border
    let help_block = Block::default().style(Style::default().bg(Color::Black));

    let inner_area = help_block.inner(area);

    // Add status bar content with Claude Code-inspired styling
    let help_text = Line::from(vec![
        Span::styled("Alt+Enter ", Style::default().fg(Color::Yellow)),
        Span::styled("for newline", Style::default().fg(Color::Gray)),
        Span::styled(" · ", Style::default().fg(Color::DarkGray)),
        Span::styled("Ctrl+↑↓ ", Style::default().fg(Color::Yellow)),
        Span::styled("to scroll", Style::default().fg(Color::Gray)),
        Span::styled(" · ", Style::default().fg(Color::DarkGray)),
        Span::styled("Ctrl+Home/End ", Style::default().fg(Color::Yellow)),
        Span::styled("for top/bottom", Style::default().fg(Color::Gray)),
        Span::styled(" · ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc ", Style::default().fg(Color::Yellow)),
        Span::styled("to exit", Style::default().fg(Color::Gray)),
    ]);

    let help = Paragraph::new(help_text)
        .style(Style::default().bg(Color::Black))
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(help_block, area);
    f.render_widget(help, inner_area);

    // Add left-aligned information about current working directory
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(cwd_str) = cwd.to_str() {
            let cwd_text = Paragraph::new(Line::from(vec![
                Span::styled("cwd: ", Style::default().fg(Color::DarkGray)),
                Span::styled(cwd_str, Style::default().fg(Color::Gray)),
            ]))
            .style(Style::default().bg(Color::Black))
            .alignment(ratatui::layout::Alignment::Left);

            // Left-aligned area
            let left_area = Rect::new(inner_area.x + 1, inner_area.y, inner_area.width / 3, 1);

            f.render_widget(cwd_text, left_area);
        }
    }
}
