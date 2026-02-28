//! Domain-specific formatters
//!
//! Each formatter handles a specific use case (Status, Validation, etc.)
//! and composes primitives to create consistent output.

pub mod challenge_list;
pub mod config;
pub mod language_list;
pub mod next_steps;
pub mod stages;
pub mod status;
pub mod tester_list;
pub mod validation;

pub use challenge_list::ChallengeListFormatter;
pub use config::ConfigFormatter;
pub use language_list::LanguageListFormatter;
pub use next_steps::NextStepsFormatter;
pub use stages::{StageEntry, StagesFormatter};
pub use status::StatusFormatter;
pub use tester_list::{TesterEntry, TesterListFormatter};
pub use validation::ValidationFormatter;
