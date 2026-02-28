//! Formatter for challenge lists.

use crate::output::formatter::Formatter;
use crate::output::primitives::{write_empty_state, Section};
use std::io;
use termcolor::WriteColor;

/// Formats available or installed challenge names.
pub struct ChallengeListFormatter {
    names: Vec<String>,
    /// Whether list is limited to installed challenges.
    installed: bool,
}

impl ChallengeListFormatter {
    #[must_use] 
    pub const fn new(names: Vec<String>, installed: bool) -> Self {
        Self { names, installed }
    }
}

impl Formatter for ChallengeListFormatter {
    fn format(&self, w: &mut dyn WriteColor) -> io::Result<()> {
        let header = if self.installed {
            "Installed challenges:"
        } else {
            "Available challenges:"
        };

        Section::new(header).write(w)?;

        if self.names.is_empty() {
            if self.installed {
                write_empty_state(
                    w,
                    "No challenges installed yet",
                    Some("Run 'crafter challenge init <challenge> <language>' to get started"),
                )?;
            } else {
                write_empty_state(w, "No challenges available", None)?;
            }
            return Ok(());
        }

        for name in &self.names {
            writeln!(w, "  {name}")?;
        }

        Ok(())
    }
}
