# Product Requirements Document: Enhanced Terminal Scrolling for hal TUI

## 1. Executive Summary

This PRD outlines requirements for enhancing the terminal scrolling experience in the hal TUI implementation. The focus is on creating a responsive, efficient, and intuitive scrolling experience that aligns with terminal UI conventions while drawing inspiration from clean interfaces like Claude Code. This implementation will replace existing scrolling behavior with a more refined approach that maintains the model-view architecture.

## 2. Background

The current TUI implementation uses ratatui for rendering and handles scrolling in both the chat history area and the multi-line input field. The scrolling behavior can be improved to provide better feedback, more consistent navigation, and an overall enhanced user experience within the constraints of terminal interfaces.

## 3. Goals and Objectives

- Create a responsive scrolling experience aligned with terminal UI conventions
- Improve visual feedback during scrolling operations
- Enhance navigation within chat history and multi-line input
- Optimize rendering for improved performance with large content
- Create a clean, Claude Code-inspired interface
- Maintain separation between model (state) and view (rendering)

## 4. User Experience Requirements

### 4.1 Chat History Scrolling

#### 4.1.1 Responsive Scrolling
- Implement immediate, discrete scrolling with appropriate step sizes
- Add support for both line-by-line and page-by-page navigation
- Ensure scrolling response feels immediate and predictable

#### 4.1.2 Visual Indicators
- Enhance scrollbar visibility with clearer thumb representation
- Add optional position indicator showing current location (e.g., "50/100")
- Use highlighting to indicate when scrolling reaches boundaries

#### 4.1.3 Smart Navigation
- Implement automatic scrolling to bottom when new messages arrive
- Add "sticky bottom" behavior that keeps view at bottom unless manually scrolled
- Add keyboard shortcuts for quick navigation (top, bottom, prev/next message)

### 4.2 Input Field Scrolling

#### 4.2.1 Cursor Visibility
- Ensure cursor always remains visible when typing at edges
- Implement immediate scrolling when cursor would move out of view
- Improve cursor positioning with multi-line text

#### 4.2.2 Multi-line Input Enhancements
- Add clear indicators for wrapped lines
- Implement efficient vertical scrolling within input field
- Improve text selection with mouse

#### 4.2.3 Command-line Style
- Implement a clean, prompt-style interface inspired by Claude Code
- Support keyboard shortcuts for navigation
- Add visual distinction between input area and chat history

### 4.3 Visual Feedback

#### 4.3.1 Scrollbar Enhancements
- Create a more visible scrollbar with clearer indicators
- Highlight scrollbar thumb when actively scrolling
- Show clear indicators when reaching top or bottom boundaries

#### 4.3.2 Message Navigation
- Add visual indicators for unread/new messages
- Implement clear boundaries between messages
- Support quick navigation between messages

## 5. Technical Requirements

### 5.1 Scrolling Implementation

- Create an improved scrolling system with discrete position updates
- Implement different scroll step sizes based on input method
- Maintain current vertical position when content changes

```rust
/// Enhanced scroll management
struct ScrollState {
    /// Current scroll position
    position: usize,
    /// Maximum scroll position
    max_position: usize,
    /// Whether view is "stuck" to bottom
    stick_to_bottom: bool,
    /// Store last scroll direction for multi-step scrolling
    last_direction: Option<ScrollDirection>,
}

impl ScrollState {
    fn new() -> Self {
        Self {
            position: 0,
            max_position: 0,
            stick_to_bottom: true,
            last_direction: None,
        }
    }

    /// Immediately scroll to a specific position
    fn scroll_to(&mut self, position: usize) {
        let clamped_position = position.min(self.max_position);
        self.position = clamped_position;
        self.stick_to_bottom = clamped_position >= self.max_position;
    }

    /// Scroll by a specific number of lines
    fn scroll_by(&mut self, delta: i32) {
        if delta > 0 {
            let new_pos = self.position.saturating_add(delta as usize);
            self.scroll_to(new_pos);
        } else {
            let new_pos = self.position.saturating_sub(delta.unsigned_abs() as usize);
            self.scroll_to(new_pos);
        }
    }

    /// Scroll by page (visible height)
    fn scroll_page(&mut self, delta: i32, page_height: usize) {
        self.scroll_by(delta * page_height as i32);
    }

    /// Update max position and maintain relative position if needed
    fn update_content_size(&mut self, content_height: usize, viewport_height: usize) {
        let old_max = self.max_position;
        self.max_position = content_height.saturating_sub(viewport_height);

        // If stuck to bottom, maintain that position
        if self.stick_to_bottom {
            self.position = self.max_position;
        } else if content_height > viewport_height {
            // Clamp current position to valid range
            self.position = self.position.min(self.max_position);
        }
    }
}
```

### 5.2 Enhanced Rendering

- Implement windowed rendering that only processes visible content
- Add clear visual distinction between messages
- Optimize text measurement for improved performance

```rust
/// Render visible messages with windowing
fn render_messages(f: &mut Frame, app: &App, area: Rect) {
    let messages_block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            "Chat History",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));

    f.render_widget(messages_block.clone(), area);
    let inner_area = messages_block.inner(area);

    // Get visible content range based on scroll position
    let scroll_offset = app.scroll_state.position;
    let viewport_height = inner_area.height as usize;

    // Determine visible message range
    let mut current_height = 0;
    let mut visible_lines = Vec::new();
    let mut visible_message_count = 0;

    // Build visible content by processing only messages in view
    for (i, (role, content)) in app.rendered_messages.iter().enumerate() {
        let message_height = content.height() + 2; // +2 for role and separator

        // Check if this message is visible in the viewport
        let message_start = current_height;
        let message_end = current_height + message_height;

        if message_end > scroll_offset &&
           message_start < scroll_offset + viewport_height {
            // Process only visible messages
            visible_message_count += 1;

            // Add role indicator
            let role_style = match role.as_str() {
                "user" => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                "model" => Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
                _ => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            };

            let role_text = match role.as_str() {
                "user" => "You",
                "model" => "AI",
                _ => role,
            };

            visible_lines.push(Line::from(vec![
                Span::styled(format!("{}: ", role_text), role_style)
            ]));

            // Calculate which content lines are visible
            let content_offset = if message_start < scroll_offset {
                scroll_offset - message_start
            } else {
                0
            };

            // Add visible content lines
            let visible_content_lines = content.lines.iter()
                .skip(content_offset)
                .take(viewport_height.saturating_sub(visible_lines.len()))
                .cloned();

            visible_lines.extend(visible_content_lines);

            // Add separator if not the last message
            if i < app.rendered_messages.len() - 1 {
                visible_lines.push(Line::from(vec![Span::styled(
                    "────────────────────────────────────────────────────────────────────────────────",
                    Style::default().fg(Color::DarkGray),
                )]));
            }
        }

        current_height += message_height;

        // Stop processing once we've filled the viewport
        if visible_lines.len() >= viewport_height {
            break;
        }
    }

    // Show spinner if loading
    if app.is_loading {
        visible_lines.push(Line::from(vec![Span::styled(
            format!("{} Thinking...", SPINNER_FRAMES[app.spinner_frame]),
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
        )]));
    }

    // Create paragraph with visible content
    let messages = Paragraph::new(visible_lines)
        .wrap(Wrap { trim: true });

    f.render_widget(messages, inner_area);

    // Render enhanced scrollbar
    render_enhanced_scrollbar(
        f,
        inner_area,
        app.scroll_state.position,
        app.scroll_state.max_position,
        viewport_height,
    );

    // Optional position indicator (e.g., "50/100")
    if app.scroll_state.max_position > 0 {
        let position_text = format!(
            "{}/{}",
            app.scroll_state.position.saturating_add(1),
            app.scroll_state.max_position.saturating_add(1)
        );

        let position_widget = Paragraph::new(Span::styled(
            position_text,
            Style::default().fg(Color::DarkGray)
        ));

        let position_area = Rect::new(
            inner_area.right() - 10,
            inner_area.top,
            10,
            1
        );

        f.render_widget(position_widget, position_area);
    }
}
```

### 5.3 Enhanced Scrollbar

- Create a more visible scrollbar with clearer indicators
- Add support for different scroll states (active, at boundary)
- Implement improved thumb rendering with block characters

```rust
fn render_enhanced_scrollbar(
    f: &mut Frame,
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
    let total_height = area.height as usize;
    let content_ratio = viewport_height as f64 / (max_position + viewport_height) as f64;
    let thumb_height = (area.height as f64 * content_ratio).max(1.0) as u16;

    let scroll_progress = if max_position > 0 {
        position as f64 / max_position as f64
    } else {
        0.0
    };

    let thumb_top = area.top + (scroll_progress * (area.height - thumb_height) as f64) as u16;

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
    for y in area.top..area.bottom {
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
        let symbol = if y == area.top && position > 0 {
            "▲"
        } else if y == area.bottom - 1 && position < max_position {
            "▼"
        } else {
            symbol
        };

        f.buffer_mut().get_mut(area.right() - 1, y)
            .map(|cell| {
                cell.set_symbol(symbol.to_owned());
                cell.set_style(cell_style);
            });
    }
}
```

### 5.4 Input Field Improvements

- Implement clear cursor visibility management
- Add support for efficient vertical scrolling
- Create prompt-style input area inspired by Claude Code

```rust
fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            "Input",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));

    f.render_widget(input_block.clone(), area);
    let inner_area = input_block.inner(area);

    // Calculate cursor position
    let before_cursor = app.input[..app.cursor_position].to_string();
    let lines: Vec<&str> = before_cursor.split('\n').collect();
    let cursor_line = lines.len() - 1;
    let line_before_cursor = lines.last().unwrap_or(&"");
    let cursor_column = line_before_cursor.width();

    // Calculate visible portion based on cursor position
    let line_width = inner_area.width as usize;
    let mut total_lines = 0;
    let mut cursor_y = 0;

    // Calculate cursor vertical position accounting for wrapping
    for (i, line) in app.input.split('\n').enumerate() {
        if i < cursor_line {
            // Add whole line height (including wrapping)
            let line_height = (line.width() + line_width - 1) / line_width;
            total_lines += line_height;
            cursor_y += line_height;
        } else if i == cursor_line {
            // Current line - calculate exact cursor position
            cursor_y += line_before_cursor.width() / line_width;
            break;
        }
    }

    // Calculate horizontal position with wrapping
    let cursor_x = cursor_column % line_width;

    // Ensure cursor is visible by updating scroll position
    if cursor_y < app.input_scroll_position {
        app.input_scroll_position = cursor_y;
    } else if cursor_y >= app.input_scroll_position + inner_area.height as usize {
        app.input_scroll_position = cursor_y - inner_area.height as usize + 1;
    }

    // Claude Code-inspired prompt at beginning of text
    let prompt = "> ";

    // Create a modified input text with prompt
    let display_lines: Vec<Line> = app.input.split('\n')
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
    let input_text = Paragraph::new(display_lines)
        .scroll((app.input_scroll_position as u16, 0));

    f.render_widget(input_text, inner_area);

    // Render scrollbar for input if needed
    let input_height = app.input.split('\n').count()
        + app.input.chars().filter(|&c| c != '\n').count() / line_width;

    if input_height > inner_area.height as usize {
        render_enhanced_scrollbar(
            f,
            Rect::new(inner_area.right() - 1, inner_area.top, 1, inner_area.height),
            app.input_scroll_position,
            input_height - inner_area.height as usize,
            inner_area.height as usize,
        );
    }

    // Set cursor position, adjusting for prompt on first line and scroll
    let adjusted_cursor_x = if cursor_line == 0 {
        // First line - account for prompt width
        cursor_x + prompt.width()
    } else {
        // Other lines - account for indentation
        cursor_x + 2
    };

    f.set_cursor(
        inner_area.left + adjusted_cursor_x as u16,
        inner_area.top + (cursor_y - app.input_scroll_position) as u16
    );
}
```

### 5.5 Claude Code-Inspired UI Elements

- Implement clean border styling with command prompt design
- Add status line with helpful information
- Create consistent keyboard shortcut display

```rust
fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(Span::styled(
            " Welcome to HAL Chat ",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ));

    f.render_widget(status_block.clone(), area);
    let inner_area = status_block.inner(area);

    // Add working directory and status
    let status_text = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("cwd: ", Style::default().fg(Color::DarkGray)),
            Span::raw(std::env::current_dir().unwrap_or_default().to_string_lossy()),
        ]),
    ]);

    f.render_widget(status_text, inner_area);
}

fn render_command_help(f: &mut Frame, area: Rect) {
    let help_text = Line::from(vec![
        Span::styled("! ", Style::default().fg(Color::DarkGray)),
        Span::styled("for bash mode", Style::default().fg(Color::Gray)),
        Span::styled(" · ", Style::default().fg(Color::DarkGray)),
        Span::styled("/ ", Style::default().fg(Color::DarkGray)),
        Span::styled("for commands", Style::default().fg(Color::Gray)),
        Span::styled(" · ", Style::default().fg(Color::DarkGray)),
        Span::styled("esc ", Style::default().fg(Color::DarkGray)),
        Span::styled("to undo", Style::default().fg(Color::Gray)),
        Span::styled("             ", Style::default().fg(Color::DarkGray)),
        Span::styled("\\e ", Style::default().fg(Color::DarkGray)),
        Span::styled("for newline", Style::default().fg(Color::Gray)),
    ]);

    let help = Paragraph::new(help_text)
        .alignment(Alignment::Left);

    f.render_widget(help, area);
}
```

## 6. Implementation Plan

The implementation will be divided into phases to ensure clean, focused delivery:

### Phase 1: Model Enhancement (1 week)

1. Replace existing scroll tracking with improved ScrollState
2. Implement better cursor position tracking
3. Add support for different scroll behaviors (stick-to-bottom, etc.)

### Phase 2: View Enhancement (2 weeks)

1. Implement windowed rendering for chat history
2. Create enhanced scrollbar rendering
3. Add improved input area with Claude Code styling
4. Implement status bar and keyboard shortcut display

### Phase 3: Integration and Testing (1 week)

1. Connect enhanced model and view components
2. Test with large message volumes
3. Optimize performance for responsive scrolling
4. Verify compatibility across different terminal emulators

## 7. Acceptance Criteria

The implementation will be considered successful when:

1. **Responsive Scrolling**: Both chat history and input field scroll immediately and predictably
   - Keyboard and mouse scrolling provide consistent behavior
   - Cursor always remains visible during editing
   - Scrolling behavior handles edge cases gracefully

2. **Visual Clarity**: The UI provides clear visual feedback
   - Scrollbar clearly shows current position
   - Visual boundaries between messages are clear
   - Claude Code-inspired styling creates a coherent interface

3. **Performance**: The implementation maintains good performance
   - No noticeable lag even with 1000+ messages
   - Minimal terminal refreshes to prevent flicker
   - Efficient rendering for large content

4. **Navigation**: The UI provides intuitive navigation capabilities
   - Clear keyboard shortcuts for common actions
   - Automatic scrolling for new content when appropriate
   - Efficient movement between messages

5. **Robustness**: The implementation handles edge cases
   - Correctly manages scroll positions during window resizing
   - Properly handles content updates (new messages, edits)
   - Maintains correct boundaries at scrolling limits

## 8. Timeline and Resources

- **Total Duration**: 4 weeks
- **Resources Required**:
  - 1 developer familiar with Rust and ratatui
  - Testing across multiple terminal emulators

## 9. Conclusion

This implementation plan provides a clear path to enhancing the hal TUI with improved scrolling behavior that respects terminal interface conventions. By focusing on responsive interaction, clear visual feedback, and optimized rendering, we can create a significantly improved user experience without attempting to force GUI-style animations into a terminal environment.

The Claude Code-inspired styling will create a professional, cohesive look that enhances usability while maintaining the efficiency and directness expected from terminal applications.
