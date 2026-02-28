//! Formatter for challenge language lists.

use crate::output::formatter::Formatter;
use crate::output::primitives::{write_empty_state, write_total_line, Section};
use std::io;
use termcolor::WriteColor;

/// Formats available languages for a challenge.
pub struct LanguageListFormatter {
    challenge: String,
    languages: Vec<String>,
}

impl LanguageListFormatter {
    pub fn new(challenge: impl Into<String>, languages: Vec<String>) -> Self {
        Self {
            challenge: challenge.into(),
            languages,
        }
    }
}

impl Formatter for LanguageListFormatter {
    fn format(&self, w: &mut dyn WriteColor) -> io::Result<()> {
        Section::new(format!("Available languages for {}:", self.challenge)).write(w)?;

        if self.languages.is_empty() {
            write_empty_state(w, "No languages available for this challenge", None)?;
            return Ok(());
        }

        for lang in &self.languages {
            writeln!(w, "  {lang}")?;
        }

        writeln!(w)?;
        write_total_line(w, "Total", &format!("{} language(s)", self.languages.len()))?;

        Ok(())
    }
}
