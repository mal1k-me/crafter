//! Core formatter abstraction.

use std::io;
use termcolor::WriteColor;

/// Trait implemented by all output formatters.
pub trait Formatter {
    /// Write formatted output.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    fn format(&self, w: &mut dyn WriteColor) -> io::Result<()>;
}
