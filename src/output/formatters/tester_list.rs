//! Formatter for installed tester tables.

use crate::output::formatter::Formatter;
use crate::output::primitives::{write_empty_state, write_total_line, Section};
use crate::output::utils::{format_path, format_size};
use comfy_table::{presets, Cell, CellAlignment, ContentArrangement, Table};
use std::io;
use std::path::PathBuf;
use termcolor::WriteColor;

/// Single installed tester entry.
#[derive(Debug, Clone)]
pub struct TesterEntry {
    pub challenge: String,
    pub version: String,
    pub size_bytes: u64,
    pub has_wrapper: bool,
    pub path: PathBuf,
    pub raw_sizes: bool,
    pub full_paths: bool,
}

/// Formats installed testers as a table.
pub struct TesterListFormatter {
    entries: Vec<TesterEntry>,
    raw_sizes: bool,
    full_paths: bool,
}

impl TesterListFormatter {
    #[must_use] 
    pub const fn new(entries: Vec<TesterEntry>) -> Self {
        Self {
            entries,
            raw_sizes: false,
            full_paths: false,
        }
    }

    #[must_use] 
    pub const fn with_raw_sizes(mut self, v: bool) -> Self {
        self.raw_sizes = v;
        self
    }

    #[must_use] 
    pub const fn with_full_paths(mut self, v: bool) -> Self {
        self.full_paths = v;
        self
    }
}

impl Formatter for TesterListFormatter {
    fn format(&self, w: &mut dyn WriteColor) -> io::Result<()> {
        if self.entries.is_empty() {
            Section::new("TESTERS").write(w)?;
            write_empty_state(
                w,
                "No testers installed",
                Some("Testers are downloaded automatically when you run tests.\n  crafter test"),
            )?;
            return Ok(());
        }

        // Build table
        let mut table = Table::new();
        table
            .load_preset(presets::NOTHING)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("CHALLENGE"),
                Cell::new("VERSION"),
                Cell::new("SIZE"),
                Cell::new("WRAPPER"),
                Cell::new("PATH"),
            ]);

        for i in 0..5 {
            if let Some(col) = table.column_mut(i) {
                col.set_cell_alignment(CellAlignment::Left);
            }
        }

        let total_size: u64 = self.entries.iter().map(|e| e.size_bytes).sum();

        for entry in &self.entries {
            let path_str = format_path(&entry.path, self.full_paths);

            table.add_row(vec![
                Cell::new(&entry.challenge),
                Cell::new(&entry.version),
                Cell::new(format_size(entry.size_bytes, self.raw_sizes)),
                Cell::new(if entry.has_wrapper { "yes" } else { "no" }),
                Cell::new(path_str),
            ]);
        }

        // Write table to a String then to the writer (comfy_table doesn't implement WriteColor)
        let table_str = format!("{table}\n");
        write!(w, "{table_str}")?;
        writeln!(w)?;
        write_total_line(
            w,
            "Total",
            &format!("{} testers, {}", self.entries.len(), format_size(total_size, self.raw_sizes)),
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use termcolor::Buffer;

    #[test]
    fn test_tester_list_empty_snapshot_like() {
        let mut buffer = Buffer::no_color();
        TesterListFormatter::new(vec![])
            .format(&mut buffer)
            .unwrap();

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("TESTERS"));
        assert!(output.contains("No testers installed"));
        assert!(output.contains("Suggestion:"));
        assert!(output.contains("crafter test"));
    }

    #[test]
    fn test_tester_list_populated_contract() {
        let entry = TesterEntry {
            challenge: "redis".to_string(),
            version: "v1.2.3".to_string(),
            size_bytes: 1_048_576,
            has_wrapper: true,
            path: PathBuf::from("/tmp/testers/redis"),
            raw_sizes: false,
            full_paths: true,
        };

        let mut buffer = Buffer::no_color();
        TesterListFormatter::new(vec![entry])
            .with_full_paths(true)
            .format(&mut buffer)
            .unwrap();

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("CHALLENGE"));
        assert!(output.contains("redis"));
        assert!(output.contains("v1.2.3"));
        assert!(output.contains("yes"));
        assert!(output.contains("Total: 1 testers"));
    }
}
