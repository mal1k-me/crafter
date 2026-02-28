//! Output writer abstraction.

use serde::Serialize;
use std::io;

/// Trait for output writer implementations.
pub trait OutputWriter {
    /// Write a step message.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    fn step(&mut self, msg: &str) -> io::Result<()>;

    /// Write a detail message.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    fn detail(&mut self, msg: &str) -> io::Result<()>;

    /// Write an error message.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    fn error(&mut self, msg: &str) -> io::Result<()>;

    /// Write a warning message.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    fn warn(&mut self, msg: &str) -> io::Result<()>;

    /// Write a success message.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    fn success(&mut self, msg: &str) -> io::Result<()>;

    /// Write a list of items.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    fn list<T: AsRef<str>>(&mut self, items: &[T]) -> io::Result<()>;

    /// Write key-value pairs.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    fn keyval(&mut self, pairs: &[(&str, &str)]) -> io::Result<()>;

    /// Write JSON object.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    fn json<T: Serialize>(&mut self, obj: &T) -> io::Result<()>;
}
