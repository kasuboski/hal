use pulldown_cmark::{Parser, Event, Tag, TagEnd, HeadingLevel, CodeBlockKind, Options};
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use crate::error::{Error, Result};

/// Formats markdown text for terminal output with colors and styling
pub fn format_markdown(markdown: &str) -> Result<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    let parser = Parser::new_ext(markdown, Options::all());
    let mut format_state = FormatState::new();
    
    for event in parser {
        format_state.handle_event(&mut stdout, event)?;
    }
    
    Ok(())
}

/// Tracks the current formatting state
struct FormatState {
    in_code_block: bool,
    list_level: usize,
    format_stack: Vec<ColorSpec>,
    current_list_type: Option<bool>,
}

impl FormatState {
    fn new() -> Self {
        Self {
            in_code_block: false,
            list_level: 0,
            format_stack: Vec::new(),
            current_list_type: None,
        }
    }
    
    fn handle_event(&mut self, stdout: &mut StandardStream, event: Event) -> Result<()> {
        match event {
            Event::Start(tag) => self.handle_start(stdout, tag),
            Event::End(tag_end) => self.handle_end(stdout, tag_end),
            Event::Text(text) => self.write_text(stdout, &text),
            Event::Code(code) => self.write_inline_code(stdout, &code),
            Event::SoftBreak | Event::HardBreak => writeln!(stdout).map_err(Error::Markdown),
            _ => Ok(()),
        }
    }
    
    fn handle_start(&mut self, stdout: &mut StandardStream, tag: Tag) -> Result<()> {
        match tag {
            Tag::Heading { level, .. } => {
                let level_color = match level {
                    HeadingLevel::H1 => Color::Rgb(255, 99, 71),  // Tomato red for h1
                    HeadingLevel::H2 => Color::Rgb(70, 130, 180), // Steel blue for h2
                    _ => Color::Cyan,                            // Cyan for other levels
                };
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(level_color)).set_bold(true);
                self.format_stack.push(spec.clone());
                stdout.set_color(&spec).map_err(Error::Markdown)?;
                if level == HeadingLevel::H1 {
                    writeln!(stdout).map_err(Error::Markdown)?
                }
            },
            Tag::Paragraph => writeln!(stdout).map_err(Error::Markdown)?,
            Tag::Strong => {
                let mut spec = ColorSpec::new();
                spec.set_bold(true);
                self.format_stack.push(spec.clone());
                stdout.set_color(&spec).map_err(Error::Markdown)?
            },
            Tag::Emphasis => {
                let mut spec = ColorSpec::new();
                spec.set_italic(true);
                self.format_stack.push(spec.clone());
                stdout.set_color(&spec).map_err(Error::Markdown)?
            },
            Tag::BlockQuote(_) => {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::Yellow));
                self.format_stack.push(spec.clone());
                stdout.set_color(&spec).map_err(Error::Markdown)?;
                write!(stdout, "  │ ").map_err(Error::Markdown)?
            },
            Tag::CodeBlock(kind) => {
                self.in_code_block = true;
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::Green));
                self.format_stack.push(spec.clone());
                stdout.set_color(&spec).map_err(Error::Markdown)?;
                match kind {
                    CodeBlockKind::Fenced(lang) => {
                        writeln!(stdout).map_err(Error::Markdown)?;
                        let lang = lang.to_string();
                        if !lang.is_empty() {
                            let mut lang_spec = ColorSpec::new();
                            lang_spec.set_fg(Some(Color::Blue)).set_italic(true);
                            stdout.set_color(&lang_spec).map_err(Error::Markdown)?;
                            write!(stdout, "[{}]\n", lang).map_err(Error::Markdown)?;
                            stdout.set_color(&spec).map_err(Error::Markdown)?
                        }
                    },
                    CodeBlockKind::Indented => writeln!(stdout).map_err(Error::Markdown)?,
                }
            },
            Tag::List(start) => {
                self.list_level += 1;
                self.current_list_type = Some(start.is_some());
                if let Some(num) = start {
                    write!(stdout, "{}{:2}. ", "  ".repeat(self.list_level - 1), num).map_err(Error::Markdown)?
                }
            },
            Tag::Item => {
                if self.list_level > 0 {
                    match self.current_list_type {
                        Some(true) => (), // Ordered list items are handled in List(start)
                        Some(false) => write!(stdout, "{}• ", "  ".repeat(self.list_level - 1)).map_err(Error::Markdown)?,
                        None => write!(stdout, "{}• ", "  ".repeat(self.list_level - 1)).map_err(Error::Markdown)?,
                    }
                }
            },
            Tag::Link { dest_url, .. } => {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::Blue)).set_underline(true);
                self.format_stack.push(spec.clone());
                stdout.set_color(&spec).map_err(Error::Markdown)?;
                write!(stdout, "{}", dest_url).map_err(Error::Markdown)?
            },
            _ => {},
        }
        Ok(())
    }
    
    fn handle_end(&mut self, stdout: &mut StandardStream, tag_end: TagEnd) -> Result<()> {
        match tag_end {
            TagEnd::Heading(_) => {
                self.format_stack.pop();
                writeln!(stdout).map_err(Error::Markdown)?
            },
            TagEnd::Paragraph => writeln!(stdout).map_err(Error::Markdown)?,
            TagEnd::Strong | TagEnd::Emphasis | TagEnd::Link => {
                self.format_stack.pop();
                if let Some(spec) = self.format_stack.last() {
                    stdout.set_color(spec).map_err(Error::Markdown)?
                } else {
                    stdout.reset().map_err(Error::Markdown)?
                }
                if matches!(tag_end, TagEnd::Link) {
                    write!(stdout, " ").map_err(Error::Markdown)?
                }
            },
            TagEnd::BlockQuote(_) => {
                self.format_stack.pop();
                if let Some(spec) = self.format_stack.last() {
                    stdout.set_color(spec).map_err(Error::Markdown)?
                } else {
                    stdout.reset().map_err(Error::Markdown)?
                }
                writeln!(stdout).map_err(Error::Markdown)?
            },
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                self.format_stack.pop();
                if let Some(spec) = self.format_stack.last() {
                    stdout.set_color(spec).map_err(Error::Markdown)?
                } else {
                    stdout.reset().map_err(Error::Markdown)?
                }
                writeln!(stdout).map_err(Error::Markdown)?
            },
            TagEnd::List(_) => {
                self.list_level -= 1;
                if self.list_level == 0 {
                    self.current_list_type = None;
                    writeln!(stdout).map_err(Error::Markdown)?
                }
            },
            TagEnd::Item => writeln!(stdout).map_err(Error::Markdown)?,
            _ => {},
        }
        Ok(())
    }
    
    fn write_text(&self, stdout: &mut StandardStream, text: &str) -> Result<()> {
        write!(stdout, "{}", text).map_err(Error::Markdown)
    }
    
    fn write_inline_code(&self, stdout: &mut StandardStream, code: &str) -> Result<()> {
        let current_spec = if let Some(spec) = self.format_stack.last() {
            spec.clone()
        } else {
            ColorSpec::new()
        };
        
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green))).map_err(Error::Markdown)?;
        write!(stdout, "`{}`", code).map_err(Error::Markdown)?;
        
        // Restore previous color spec
        stdout.set_color(&current_spec).map_err(Error::Markdown)?;
        Ok(())
    }
}