//! Output system (human, simple, and JSON).

use serde::Serialize;
use serde_json::Value as JsonValue;
use std::io::{self, Write};
use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};

pub mod compat;
pub mod config_loader;
mod format;
pub mod formatter;
pub mod formatters;
mod level;
pub mod primitives;
pub mod utils;
mod writer;

pub use config_loader::{CliFlag, CliOutputArgs, CliVerbosity, ConfigLoader};
pub use format::Format;
pub use level::Level;
pub use writer::OutputWriter;

#[derive(Debug)]
enum OutputEvent {
    Step(String),
    Detail(String),
    Debug(String),
    Error(String),
    Warn(String),
    Info(String),
    Suggestion(String),
    Success(String),
    List(Vec<String>),
    KeyValue(Vec<(String, String)>),
    Summary {
        title: String,
        items: Vec<(String, String)>,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Json(JsonValue),
}

/// Output policy.
#[derive(Debug, Clone)]
pub struct OutputPolicy {
    /// Verbosity level.
    pub level: Level,

    /// Output format.
    pub format: Format,

    /// Color choice.
    pub color: ColorChoice,

    /// Whether stdout is a TTY.
    pub is_tty: bool,

    /// Show raw byte sizes.
    pub raw_sizes: bool,

    /// Show full paths.
    pub full_paths: bool,
}

impl OutputPolicy {
    /// Create default policy.
    #[must_use] 
    pub fn new() -> Self {
        let is_tty = atty::is(atty::Stream::Stdout);

        Self {
            level: Level::Normal,
            format: if is_tty {
                Format::Human
            } else {
                Format::Simple
            },
            color: if is_tty {
                ColorChoice::Auto
            } else {
                ColorChoice::Never
            },
            is_tty,
            raw_sizes: false,
            full_paths: false,
        }
    }

    /// Set verbosity level.
    #[must_use] 
    pub const fn with_level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// Set output format.
    #[must_use] 
    pub const fn with_format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }

    /// Set color choice.
    #[must_use] 
    pub const fn with_color(mut self, color: ColorChoice) -> Self {
        self.color = color;
        self
    }

    /// Set quiet mode.
    #[must_use] 
    pub const fn quiet(mut self) -> Self {
        self.level = Level::Silent;
        self
    }

    /// Set verbose mode by verbosity count.
    #[must_use] 
    pub const fn verbose(mut self, count: u8) -> Self {
        self.level = match count {
            0 => Level::Normal,
            1 => Level::Verbose,
            _ => Level::Debug,
        };
        self
    }

    /// Enable raw size output.
    #[must_use] 
    pub const fn with_raw_sizes(mut self, raw: bool) -> Self {
        self.raw_sizes = raw;
        self
    }

    /// Enable full path output.
    #[must_use] 
    pub const fn with_full_paths(mut self, full: bool) -> Self {
        self.full_paths = full;
        self
    }

    /// Force color output.
    #[must_use] 
    pub const fn with_force_color(mut self, force: bool) -> Self {
        if force {
            self.color = ColorChoice::Always;
        }
        self
    }

    /// Resolve color choice from policy and output format.
    #[must_use]
    pub const fn effective_color_choice(&self) -> ColorChoice {
        match self.format {
            Format::Simple | Format::Json => ColorChoice::Never,
            Format::Human => self.color,
        }
    }

    /// Check whether output should be shown.
    #[must_use] 
    pub fn should_show(&self, min_level: Level) -> bool {
        self.level >= min_level
    }
}

impl Default for OutputPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// Backward-compatible alias for older call sites.
///
/// Prefer `OutputPolicy` in new code.
pub type OutputConfig = OutputPolicy;

/// Main output handler using an `OutputPolicy`.
pub struct Output {
    config: OutputPolicy,
    stdout: StandardStream,
    stderr: StandardStream,
}

impl Output {
    /// Create output handler.
    #[must_use] 
    pub fn new(config: OutputPolicy) -> Self {
        let effective_color = config.effective_color_choice();
        Self {
            stdout: StandardStream::stdout(effective_color),
            stderr: StandardStream::stderr(effective_color),
            config,
        }
    }

    fn emit_json_line<T: Serialize>(&mut self, value: &T) {
        if serde_json::to_writer(&mut self.stdout, value).is_ok() {
            let _ = writeln!(self.stdout);
        }
    }

    fn render_event(&mut self, event: OutputEvent) {
        match event {
            OutputEvent::Step(msg) => {
                if self.config.should_show(Level::Normal)
                    && matches!(self.config.format, Format::Human | Format::Simple)
                {
                    let _ = writeln!(self.stderr, "{msg}");
                }
            }
            OutputEvent::Detail(msg) => {
                if self.config.should_show(Level::Verbose)
                    && matches!(self.config.format, Format::Human | Format::Simple)
                {
                    let _ = writeln!(self.stderr, "  {msg}");
                }
            }
            OutputEvent::Debug(msg) => {
                if !self.config.should_show(Level::Debug) {
                    return;
                }

                match self.config.format {
                    Format::Json => {}
                    Format::Human => {
                        self.stderr
                            .set_color(ColorSpec::new().set_dimmed(true))
                            .ok();
                        let _ = writeln!(self.stderr, "[DEBUG] {msg}");
                        self.stderr.reset().ok();
                    }
                    Format::Simple => {
                        let _ = writeln!(self.stderr, "Debug: {msg}");
                    }
                }
            }
            OutputEvent::Error(msg) => {
                if self.config.level == Level::Silent {
                    return;
                }

                match self.config.format {
                    Format::Json => self.emit_json_line(&serde_json::json!({
                        "type": "error",
                        "error": msg,
                        "message": msg,
                    })),
                    Format::Human => {
                        self.write_bracketed_to_stderr(primitives::Level::Error, &msg)
                    }
                    Format::Simple => {
                        let _ = writeln!(self.stderr, "Error: {msg}");
                    }
                }
            }
            OutputEvent::Warn(msg) => {
                if self.config.level == Level::Silent {
                    return;
                }

                match self.config.format {
                    Format::Json => self.emit_json_line(&serde_json::json!({
                        "type": "warning",
                        "warning": msg,
                        "message": msg,
                    })),
                    Format::Human => self.write_bracketed_to_stderr(primitives::Level::Warn, &msg),
                    Format::Simple => {
                        let _ = writeln!(self.stderr, "Warning: {msg}");
                    }
                }
            }
            OutputEvent::Info(msg) => {
                if !self.config.should_show(Level::Normal) {
                    return;
                }

                match self.config.format {
                    Format::Json => {}
                    Format::Human => self.write_bracketed_to_stderr(primitives::Level::Info, &msg),
                    Format::Simple => {
                        let _ = writeln!(self.stderr, "Info: {msg}");
                    }
                }
            }
            OutputEvent::Suggestion(msg) => {
                if self.config.level == Level::Silent {
                    return;
                }

                if matches!(self.config.format, Format::Human | Format::Simple) {
                    let _ = writeln!(self.stderr);
                    let _ = primitives::write_suggestion(&mut self.stderr, &msg);
                }
            }
            OutputEvent::Success(msg) => {
                if !self.config.should_show(Level::Normal) {
                    return;
                }

                match self.config.format {
                    Format::Json => {}
                    Format::Human => self.write_bracketed_to_stderr(primitives::Level::Ok, &msg),
                    Format::Simple => {
                        let _ = writeln!(self.stderr, "OK: {msg}");
                    }
                }
            }
            OutputEvent::List(items) => {
                if !self.config.should_show(Level::Normal) {
                    return;
                }

                match self.config.format {
                    Format::Json => self.emit_json_line(&items),
                    Format::Human | Format::Simple => {
                        for item in items {
                            let _ = writeln!(self.stdout, "{item}");
                        }
                    }
                }
            }
            OutputEvent::KeyValue(pairs) => match self.config.format {
                Format::Json => {
                    let map: std::collections::HashMap<_, _> = pairs.into_iter().collect();
                    self.emit_json_line(&map);
                }
                Format::Human | Format::Simple => {
                    for (key, value) in pairs {
                        let _ = writeln!(self.stdout, "{key}: {value}");
                    }
                }
            },
            OutputEvent::Summary { title, items } => match self.config.format {
                Format::Json => {
                    self.emit_json_line(&serde_json::json!({
                        "type": "summary",
                        "title": title,
                        "items": items,
                    }));
                }
                Format::Human | Format::Simple => {
                    let _ = primitives::write_summary_lines(&mut self.stdout, &title, &items);
                }
            },
            OutputEvent::Table { headers, rows } => match self.config.format {
                Format::Json => {
                    self.emit_json_line(&serde_json::json!({
                        "type": "table",
                        "headers": headers,
                        "rows": rows,
                    }));
                }
                Format::Human | Format::Simple => {
                    let _ = primitives::write_table_lines(&mut self.stdout, &headers, &rows);
                }
            },
            OutputEvent::Json(value) => {
                self.emit_json_line(&value);
            }
        }
    }

    /// Print step message.
    pub fn step(&mut self, msg: &str) {
        self.render_event(OutputEvent::Step(msg.to_string()));
    }

    /// Print detailed message.
    pub fn detail(&mut self, msg: &str) {
        self.render_event(OutputEvent::Detail(msg.to_string()));
    }

    /// Print debug message.
    pub fn debug(&mut self, msg: &str) {
        self.render_event(OutputEvent::Debug(msg.to_string()));
    }

    /// Write bracketed message to stderr.
    fn write_bracketed_to_stderr(&mut self, level: primitives::Level, msg: &str) {
        let _ = primitives::write_bracketed_message(&mut self.stderr, level, msg);
    }

    /// Print error.
    pub fn error(&mut self, msg: &str) {
        self.render_event(OutputEvent::Error(msg.to_string()));
    }

    /// Print warning.
    pub fn warn(&mut self, msg: &str) {
        self.render_event(OutputEvent::Warn(msg.to_string()));
    }

    /// Print info message.
    pub fn info(&mut self, msg: &str) {
        self.render_event(OutputEvent::Info(msg.to_string()));
    }

    /// Print suggestion message.
    pub fn suggestion(&mut self, msg: &str) {
        self.render_event(OutputEvent::Suggestion(msg.to_string()));
    }

    /// Print success message.
    pub fn success(&mut self, msg: &str) {
        self.render_event(OutputEvent::Success(msg.to_string()));
    }

    /// Print list items.
    pub fn list<T: AsRef<str>>(&mut self, items: &[T]) {
        let items = items.iter().map(|v| v.as_ref().to_string()).collect();
        self.render_event(OutputEvent::List(items));
    }

    /// Print key-value pairs.
    pub fn keyval(&mut self, pairs: &[(&str, &str)]) {
        let pairs = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        self.render_event(OutputEvent::KeyValue(pairs));
    }

    /// Print summary block data.
    pub fn summary(&mut self, title: &str, items: &[(&str, &str)]) {
        let items = items
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        self.render_event(OutputEvent::Summary {
            title: title.to_string(),
            items,
        });
    }

    /// Print table data.
    pub fn table(&mut self, headers: &[&str], rows: &[Vec<String>]) {
        self.render_event(OutputEvent::Table {
            headers: headers.iter().map(|h| (*h).to_string()).collect(),
            rows: rows.to_vec(),
        });
    }

    /// Print JSON object.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn json<T: Serialize>(&mut self, obj: &T) -> io::Result<()> {
        match self.config.format {
            Format::Json => {
                let value = serde_json::to_value(obj)?;
                self.render_event(OutputEvent::Json(value));
            }
            Format::Human => {
                // Pretty print for humans
                let json = serde_json::to_string_pretty(obj)?;
                writeln!(self.stdout, "{json}")?;
            }
            Format::Simple => {
                // Compact for scripts
                let json = serde_json::to_string(obj)?;
                writeln!(self.stdout, "{json}")?;
            }
        }
        Ok(())
    }
}

impl Default for Output {
    fn default() -> Self {
        Self::new(OutputPolicy::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = OutputPolicy::new();
        assert_eq!(config.level, Level::Normal);
    }

    #[test]
    fn test_config_builder() {
        let config = OutputPolicy::new().quiet().with_format(Format::Json);

        assert_eq!(config.level, Level::Silent);
        assert_eq!(config.format, Format::Json);
    }

    #[test]
    fn test_should_show() {
        let config = OutputPolicy::new().with_level(Level::Verbose);
        assert!(config.should_show(Level::Normal));
        assert!(config.should_show(Level::Verbose));
        assert!(!config.should_show(Level::Debug));
    }

    #[test]
    fn test_effective_color_choice_for_machine_formats() {
        let json_cfg = OutputPolicy::new()
            .with_format(Format::Json)
            .with_color(ColorChoice::Always);
        let simple_cfg = OutputPolicy::new()
            .with_format(Format::Simple)
            .with_color(ColorChoice::Always);

        assert_eq!(json_cfg.effective_color_choice(), ColorChoice::Never);
        assert_eq!(simple_cfg.effective_color_choice(), ColorChoice::Never);
    }

    #[test]
    fn test_effective_color_choice_for_human_format() {
        let auto_cfg = OutputPolicy::new()
            .with_format(Format::Human)
            .with_color(ColorChoice::Auto);
        let always_cfg = OutputPolicy::new()
            .with_format(Format::Human)
            .with_color(ColorChoice::Auto)
            .with_force_color(true);

        assert_eq!(auto_cfg.effective_color_choice(), ColorChoice::Auto);
        assert_eq!(always_cfg.effective_color_choice(), ColorChoice::Always);
    }
}
