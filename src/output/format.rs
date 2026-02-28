//! Output format types.

use std::fmt;
use std::str::FromStr;

/// Output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Default)]
pub enum Format {
    /// Human-readable output.
    #[default]
    Human,

    /// Plain-text output.
    Simple,

    /// JSON output.
    Json,
}

impl FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "human" => Ok(Self::Human),
            "simple" => Ok(Self::Simple),
            "json" => Ok(Self::Json),
            _ => Err(format!(
                "Unknown format: {s}. Valid values: human, simple, json"
            )),
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Human => write!(f, "human"),
            Self::Simple => write!(f, "simple"),
            Self::Json => write!(f, "json"),
        }
    }
}
