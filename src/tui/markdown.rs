use pulldown_cmark::{Event, Parser, Tag, TagEnd, Options, HeadingLevel, CodeBlockKind};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

/// Converts markdown text to ratatui Text for rendering in the terminal UI
pub fn markdown_to_ratatui_text(markdown: &str) -> Text<'static> {
    let parser = Parser::new_ext(markdown, Options::all());
    let mut lines: Vec<Line> = Vec::new();
    let mut current_line: Vec<Span> = Vec::new();
    let mut current_style = Style::default();
    let mut _in_code_block = false;
    let mut list_level = 0;
    
    for event in parser {
        match event {
            Event::Text(text) => {
                current_line.push(Span::styled(text.to_string(), current_style));
            },
            Event::Code(code) => {
                // Inline code styling
                current_line.push(Span::styled(
                    format!("`{}`", code), 
                    Style::default().fg(Color::Green)
                ));
            },
            Event::Start(tag) => {
                match tag {
                    Tag::Heading { level, .. } => {
                        let level_color = match level {
                            HeadingLevel::H1 => Color::Rgb(255, 99, 71),  // Tomato red for h1
                            HeadingLevel::H2 => Color::Rgb(70, 130, 180), // Steel blue for h2
                            _ => Color::Cyan,                            // Cyan for other levels
                        };
                        current_style = Style::default().fg(level_color).add_modifier(Modifier::BOLD);
                        
                        // Add a blank line before H1 headings
                        if level == HeadingLevel::H1 {
                            lines.push(Line::from(""));
                        }
                    },
                    Tag::Paragraph => {
                        // Start a new paragraph
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.drain(..).collect::<Vec<_>>()));
                        }
                    },
                    Tag::Strong => {
                        current_style = current_style.add_modifier(Modifier::BOLD);
                    },
                    Tag::Emphasis => {
                        current_style = current_style.add_modifier(Modifier::ITALIC);
                    },
                    Tag::BlockQuote(_) => {
                        current_style = Style::default().fg(Color::Yellow);
                        current_line.push(Span::raw("  │ "));
                    },
                    Tag::CodeBlock(kind) => {
                        _in_code_block = true;
                        current_style = Style::default().fg(Color::Green);
                        
                        // Add a blank line before code blocks
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.drain(..).collect::<Vec<_>>()));
                        }
                        lines.push(Line::from(""));
                        
                        // Add language indicator for fenced code blocks
                        if let CodeBlockKind::Fenced(lang) = kind {
                            if !lang.is_empty() {
                                lines.push(Line::from(vec![Span::styled(
                                    format!("[{}]", lang),
                                    Style::default().fg(Color::Blue).add_modifier(Modifier::ITALIC)
                                )]));
                            }
                        }
                    },
                    Tag::List(start) => {
                        list_level += 1;
                        if let Some(num) = start {
                            current_line.push(Span::raw(format!("{}{}", "  ".repeat(list_level - 1), num)));
                        }
                    },
                    Tag::Item => {
                        if list_level > 0 && current_line.is_empty() {
                            // Only add bullet if this is an unordered list item
                            current_line.push(Span::raw(format!("{}{} ", "  ".repeat(list_level - 1), "•")));
                        }
                    },
                    Tag::Link { dest_url, .. } => {
                        current_style = Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::UNDERLINED);
                        current_line.push(Span::styled(dest_url.to_string(), current_style));
                    },
                    _ => {},
                }
            },
            Event::End(tag_end) => {
                match tag_end {
                    TagEnd::Heading(_) => {
                        current_style = Style::default();
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.drain(..).collect::<Vec<_>>()));
                        }
                        lines.push(Line::from(""));
                    },
                    TagEnd::Paragraph => {
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.drain(..).collect::<Vec<_>>()));
                        }
                        lines.push(Line::from(""));
                    },
                    TagEnd::Strong => {
                        current_style = current_style.remove_modifier(Modifier::BOLD);
                    },
                    TagEnd::Emphasis => {
                        current_style = current_style.remove_modifier(Modifier::ITALIC);
                    },
                    TagEnd::BlockQuote(_) => {
                        current_style = Style::default();
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.drain(..).collect::<Vec<_>>()));
                        }
                        lines.push(Line::from(""));
                    },
                    TagEnd::CodeBlock => {
                        _in_code_block = false;
                        current_style = Style::default();
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.drain(..).collect::<Vec<_>>()));
                        }
                        lines.push(Line::from(""));
                    },
                    TagEnd::List(_) => {
                        list_level -= 1;
                        if list_level == 0 && !current_line.is_empty() {
                            lines.push(Line::from(current_line.drain(..).collect::<Vec<_>>()));
                            lines.push(Line::from(""));
                        }
                    },
                    TagEnd::Item => {
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.drain(..).collect::<Vec<_>>()));
                        }
                    },
                    TagEnd::Link => {
                        current_style = Style::default();
                        current_line.push(Span::raw(" "));
                    },
                    _ => {},
                }
            },
            Event::SoftBreak => {
                current_line.push(Span::raw(" "));
            },
            Event::HardBreak => {
                if !current_line.is_empty() {
                    lines.push(Line::from(current_line.drain(..).collect::<Vec<_>>()));
                }
            },
            _ => {},
        }
    }

    // Add any remaining content
    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }

    Text::from(lines)
} 