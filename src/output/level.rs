//! Output verbosity levels.

use std::fmt;

/// Verbosity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Level {
    /// Minimal output.
    Silent = 0,

    /// Normal output.
    #[default]
    Normal = 1,

    /// Verbose output.
    Verbose = 2,

    /// Debug output.
    Debug = 3,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Silent => write!(f, "silent"),
            Self::Normal => write!(f, "normal"),
            Self::Verbose => write!(f, "verbose"),
            Self::Debug => write!(f, "debug"),
        }
    }
}
