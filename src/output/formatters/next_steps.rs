//! Formatter for post-init next steps.

use crate::output::formatter::Formatter;
use crate::output::utils;
use std::io;
use std::path::PathBuf;
use termcolor::WriteColor;

/// Formats next-step guidance shown after initialization.
pub struct NextStepsFormatter {
    target_dir: PathBuf,
    full_paths: bool,
}

impl NextStepsFormatter {
    #[must_use] 
    pub const fn new(target_dir: PathBuf) -> Self {
        Self {
            target_dir,
            full_paths: false,
        }
    }

    #[must_use]
    pub const fn with_full_paths(mut self, full_paths: bool) -> Self {
        self.full_paths = full_paths;
        self
    }
}

impl Formatter for NextStepsFormatter {
    fn format(&self, w: &mut dyn WriteColor) -> io::Result<()> {
        writeln!(w)?;
        writeln!(w, "Next steps:")?;
        writeln!(
            w,
            "    cd {}",
            utils::format_path(&self.target_dir, self.full_paths)
        )?;
        writeln!(w, "    # Make your changes")?;
        writeln!(w, "    crafter test")?;
        Ok(())
    }
}
