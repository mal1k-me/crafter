// Crafter - Local CodeCrafters CLI
// Main library module

pub mod container;
pub mod constants;
pub mod core;
pub mod types;
pub mod utils;

// New modules (Unix refactor)
pub mod output;
pub mod error;

// Re-export commonly used types
pub use types::error::CrafterError as LegacyError;
pub use types::error::Result as LegacyResult;

// New error types (preferred)
pub use error::{CrafterError, Result};
pub use output::{Format, Level, Output, OutputPolicy, OutputConfig};

