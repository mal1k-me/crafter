// Types module

pub mod config;
pub mod course;
pub mod error;
pub mod output;
pub mod test;
pub mod tester;

pub use config::{
    ChallengeEntry, ChallengesConfig, CodecraftersConfig, Config, OutputPreferences,
};
pub use course::{CourseDefinition, Extension, Stage};
pub use error::{CrafterError, Result};
pub use output::{
    DockerfileStatus, LanguageInfo, RepoStatus, StageInfo, StagesOutput, StatusOutput,
    TestAllStagesOutput, TestRunOutput, TestStageRunOutput, TesterInfo as TesterOutputInfo,
    TesterStatus, ValidationCheck, ValidationCheckOutput, ValidationReportOutput,
    ValidationResult,
};
pub use test::{TestOptions, TestOutput};
pub use tester::{BuildOptions, GitHubAsset, GitHubRelease, TesterInfo};
