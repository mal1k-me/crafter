//! Structured output types for commands
//!
//! These types are used to provide JSON-serializable output
//! for various commands.

use serde::Serialize;

/// Stage information for JSON output
#[derive(Debug, Clone, Serialize)]
pub struct StageInfo {
    pub slug: String,
    pub name: String,
    pub difficulty: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension_slug: Option<String>,
}

/// Grouped stages output
#[derive(Debug, Clone, Serialize)]
pub struct StagesOutput {
    pub challenge: String,
    pub total: usize,
    pub stages: Vec<StageInfo>,
}

/// Language information
#[derive(Debug, Clone, Serialize)]
pub struct LanguageInfo {
    pub slug: String,
    pub name: String,
}

/// Validation result
#[derive(Debug, Clone, Serialize)]
pub struct ValidationResult {
    pub passed: bool,
    pub checks: Vec<ValidationCheck>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationCheck {
    pub name: String,
    pub passed: bool,
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Tester information
#[derive(Debug, Clone, Serialize)]
pub struct TesterInfo {
    pub challenge: String,
    pub version: Option<String>,
    pub last_updated: Option<String>,
}

/// Project status information
#[derive(Debug, Clone, Serialize)]
pub struct StatusOutput {
    pub directory: String,
    pub challenge: Option<String>,
    pub buildpack: String,
    pub debug: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenge_repo: Option<RepoStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tester: Option<TesterStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dockerfile: Option<DockerfileStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker: Option<DockerStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DockerStatus {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoStatus {
    pub downloaded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_mb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TesterStatus {
    pub downloaded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_mb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DockerfileStatus {
    pub found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Validation report output
#[derive(Debug, Clone, Serialize)]
pub struct ValidationReportOutput {
    pub challenge: Option<String>,
    pub passed: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub success_count: usize,
    pub checks: Vec<ValidationCheckOutput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationCheckOutput {
    pub name: String,
    pub passed: bool,
    pub severity: String, // "error", "warning", "info"
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    pub fixable: bool,
}

/// Single-stage test run output.
#[derive(Debug, Clone, Serialize)]
pub struct TestRunOutput {
    pub stage: String,
    pub passed: bool,
    pub exit_code: i32,
    pub duration_secs: f64,
    pub output: String,
}

/// Stage-level result in an all-stages run.
#[derive(Debug, Clone, Serialize)]
pub struct TestStageRunOutput {
    pub slug: String,
    pub name: String,
    pub passed: bool,
    pub exit_code: i32,
    pub duration_secs: f64,
    pub output: String,
}

/// Aggregate result for all-stages test run.
#[derive(Debug, Clone, Serialize)]
pub struct TestAllStagesOutput {
    pub challenge: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub duration_secs: f64,
    pub stages: Vec<TestStageRunOutput>,
}
