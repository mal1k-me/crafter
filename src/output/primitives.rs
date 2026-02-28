//! Reusable formatting primitives.

use std::io;
use termcolor::{Color, ColorSpec, WriteColor};

/// Severity level for bracketed output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Ok,
    Info,
    Warn,
    Error,
}

impl Level {
    /// Bracket label.
    #[must_use] 
    pub const fn bracket(&self) -> &'static str {
        match self {
            Self::Ok => "[OK]",
            Self::Info => "[INFO]",
            Self::Warn => "[WARN]",
            Self::Error => "[ERR]",
        }
    }

    /// Color style.
    #[must_use] 
    pub fn color_spec(&self) -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_bold(true);

        match self {
            Self::Ok => spec.set_fg(Some(Color::Green)),
            Self::Info => spec.set_fg(Some(Color::Cyan)),
            Self::Warn => spec.set_fg(Some(Color::Yellow)),
            Self::Error => spec.set_fg(Some(Color::Red)),
        };

        spec
    }
}

/// Apply a color style, write text, then reset.
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn write_colored(w: &mut dyn WriteColor, spec: &ColorSpec, text: &str) -> io::Result<()> {
    w.set_color(spec)?;
    write!(w, "{text}")?;
    w.reset()
}

/// Write a dimmed suggestion block.
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn write_suggestion(w: &mut dyn WriteColor, msg: &str) -> io::Result<()> {
    let mut dimmed = ColorSpec::new();
    dimmed.set_dimmed(true);
    w.set_color(&dimmed)?;
    writeln!(w, "Suggestion:")?;
    writeln!(w, "  {msg}")?;
    w.reset()
}

/// Write a standard empty-state block with optional suggestion.
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn write_empty_state(
    w: &mut dyn WriteColor,
    message: &str,
    suggestion: Option<&str>,
) -> io::Result<()> {
    writeln!(w, "  {message}")?;
    if let Some(suggestion) = suggestion {
        writeln!(w)?;
        write_suggestion(w, suggestion)?;
    }
    Ok(())
}

/// Write a standard total/count line.
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn write_total_line(w: &mut dyn WriteColor, label: &str, value: &str) -> io::Result<()> {
    writeln!(w, "{label}: {value}")
}

/// Write plain summary lines.
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn write_summary_lines(
    w: &mut dyn WriteColor,
    title: &str,
    items: &[(String, String)],
) -> io::Result<()> {
    writeln!(w, "{title}")?;
    for (key, value) in items {
        writeln!(w, "  {key}: {value}")?;
    }
    Ok(())
}

/// Write plain tab-separated table lines.
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn write_table_lines(
    w: &mut dyn WriteColor,
    headers: &[String],
    rows: &[Vec<String>],
) -> io::Result<()> {
    if !headers.is_empty() {
        writeln!(w, "{}", headers.join("\t"))?;
    }
    for row in rows {
        writeln!(w, "{}", row.join("\t"))?;
    }
    Ok(())
}

/// Write a bracketed message line.
/// # Errors
/// Returns an error if the underlying operation fails.
fn write_bracket_prefix(w: &mut dyn WriteColor, level: Level) -> io::Result<()> {
    w.set_color(&level.color_spec())?;
    write!(w, "{:<7}", level.bracket())?;
    w.reset()?;
    Ok(())
}

/// Write a bracketed message line.
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn write_bracketed_message(
    w: &mut dyn WriteColor,
    level: Level,
    message: &str,
) -> io::Result<()> {
    write_bracket_prefix(w, level)?;
    writeln!(w, " {message}")
}

/// Left-aligned bracketed status line.
#[derive(Debug)]
pub struct BracketedLine<'a> {
    pub level: Level,
    pub label: &'a str,
    pub message: &'a str,
    pub suggestion: Option<&'a str>,
}

impl<'a> BracketedLine<'a> {
    /// Create `[OK]` line.
    #[must_use] 
    pub const fn ok(label: &'a str, message: &'a str) -> Self {
        Self {
            level: Level::Ok,
            label,
            message,
            suggestion: None,
        }
    }

    /// Create `[WARN]` line.
    #[must_use] 
    pub const fn warn(label: &'a str, message: &'a str) -> Self {
        Self {
            level: Level::Warn,
            label,
            message,
            suggestion: None,
        }
    }

    /// Create `[ERR]` line.
    #[must_use] 
    pub const fn error(label: &'a str, message: &'a str) -> Self {
        Self {
            level: Level::Error,
            label,
            message,
            suggestion: None,
        }
    }

    /// Create `[INFO]` line.
    #[must_use] 
    pub const fn info(label: &'a str, message: &'a str) -> Self {
        Self {
            level: Level::Info,
            label,
            message,
            suggestion: None,
        }
    }

    /// Attach optional suggestion text.
    #[must_use] 
    pub const fn with_suggestion(mut self, suggestion: &'a str) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    /// Write the line to the writer.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn write(&self, w: &mut dyn WriteColor) -> io::Result<()> {
        // Write left-aligned bracket (7 chars total)
        write_bracket_prefix(w, self.level)?;

        // Write label (bold)
        write!(w, " ")?;
        let mut bold = ColorSpec::new();
        bold.set_bold(true);
        w.set_color(&bold)?;
        write!(w, "{}", self.label)?;
        w.reset()?;

        // Write message
        writeln!(w, " - {}", self.message)?;

        // Write optional suggestion (dimmed, indented)
        if let Some(suggestion) = self.suggestion {
            let mut dimmed = ColorSpec::new();
            dimmed.set_dimmed(true);
            w.set_color(&dimmed)?;
            writeln!(w, "  {suggestion}")?;
            w.reset()?;
        }

        Ok(())
    }
}

/// Key-value output list with optional indentation.
#[derive(Debug, Default)]
pub struct KeyValueList {
    pairs: Vec<(String, String)>,
    indent: usize,
}

impl KeyValueList {
    /// Create an empty list.
    #[must_use] 
    pub const fn new() -> Self {
        Self {
            pairs: Vec::new(),
            indent: 0,
        }
    }

    /// Set indentation width.
    #[must_use] 
    pub const fn with_indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    /// Add a key-value pair.
    #[must_use]
    pub fn add(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.pairs.push((key.into(), value.into()));
        self
    }

    /// Add a key-value pair (mutable API).
    pub fn add_mut(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.pairs.push((key.into(), value.into()));
    }

    /// Write all pairs.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn write(&self, w: &mut dyn WriteColor) -> io::Result<()> {
        for (key, value) in &self.pairs {
            write!(w, "{:indent$}{}: {}", "", key, value, indent = self.indent)?;
            writeln!(w)?;
        }
        Ok(())
    }
}

/// Summary section with optional colored values.
#[derive(Debug)]
pub struct SummaryBlock {
    title: String,
    items: Vec<(String, String, Option<Color>)>,
}

impl SummaryBlock {
    /// Create a new summary block.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
        }
    }

    /// Add an item.
    #[must_use]
    pub fn add(
        mut self,
        label: impl Into<String>,
        value: impl Into<String>,
        color: Option<Color>,
    ) -> Self {
        self.items.push((label.into(), value.into(), color));
        self
    }

    /// Add an item (mutable API).
    pub fn add_mut(
        &mut self,
        label: impl Into<String>,
        value: impl Into<String>,
        color: Option<Color>,
    ) {
        self.items.push((label.into(), value.into(), color));
    }

    /// Write the summary.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn write(&self, w: &mut dyn WriteColor) -> io::Result<()> {
        // Title (bold)
        writeln!(w)?;
        let mut bold = ColorSpec::new();
        bold.set_bold(true);
        w.set_color(&bold)?;
        writeln!(w, "{}", self.title)?;
        w.reset()?;
        writeln!(w)?;

        // Items (indented, optionally colored)
        for (label, value, color) in &self.items {
            write!(w, "  {label}: ")?;

            if let Some(c) = color {
                let mut colored = ColorSpec::new();
                colored.set_fg(Some(*c));
                w.set_color(&colored)?;
                writeln!(w, "{value}")?;
                w.reset()?;
            } else {
                writeln!(w, "{value}")?;
            }
        }
        writeln!(w)?;

        Ok(())
    }
}

/// Bold section header.
#[derive(Debug)]
pub struct Section {
    title: String,
}

impl Section {
    /// Create a new section header.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
        }
    }

    /// Write the section header.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn write(&self, w: &mut dyn WriteColor) -> io::Result<()> {
        writeln!(w)?;
        let mut bold = ColorSpec::new();
        bold.set_bold(true);
        w.set_color(&bold)?;
        writeln!(w, "{}", self.title)?;
        w.reset()?;
        writeln!(w)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use termcolor::Buffer;

    #[test]
    fn q_a_bracketed_line_variants_render_expected_tokens() {
        let mut buffer = Buffer::no_color();
        BracketedLine::ok("Test", "Success message")
            .write(&mut buffer)
            .unwrap();
        BracketedLine::warn("Warning", "Something happened")
            .with_suggestion("Try running: cargo fix")
            .write(&mut buffer)
            .unwrap();

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("[OK]"));
        assert!(output.contains("[WARN]"));
        assert!(output.contains("Success message"));
        assert!(output.contains("Try running: cargo fix"));
    }

    #[test]
    fn q_a_key_value_and_summary_blocks_render() {
        let mut buffer = Buffer::no_color();
        KeyValueList::new()
            .add("key1", "value1")
            .add("key2", "value2")
            .write(&mut buffer)
            .unwrap();
        SummaryBlock::new("Summary")
            .add("Passed", "10", Some(Color::Green))
            .add("Failed", "0", None)
            .write(&mut buffer)
            .unwrap();

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("Summary"));
        assert!(output.contains("Passed: 10"));
        assert!(output.contains("Failed: 0"));
    }

    #[test]
    fn q_a_section_and_colored_writes_work() {
        let mut buffer = Buffer::no_color();
        Section::new("Test Section").write(&mut buffer).unwrap();
        let mut spec = ColorSpec::new();
        spec.set_bold(true);
        write_colored(&mut buffer, &spec, "hello").unwrap();

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("Test Section"));
        assert!(output.contains("hello"));
    }

    #[test]
    fn q_a_write_suggestion_prefix_is_present() {
        let mut buffer = Buffer::no_color();
        write_suggestion(&mut buffer, "run crafter test").unwrap();

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("Suggestion:"));
        assert!(output.contains("  run crafter test"));
    }
}
