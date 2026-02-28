// Test types

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct TestOptions {
    pub project_dir: PathBuf,
    pub challenge: String,
    pub stage: String,
    pub buildpack: String,
    pub debug: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestOutput {
    pub success: bool,
    pub duration_secs: f64,
    pub stdout: String,
    pub stderr: String,
}

impl TestOutput {
    #[must_use] 
    pub const fn failed(stderr: String) -> Self {
        Self {
            success: false,
            duration_secs: 0.0,
            stdout: String::new(),
            stderr,
        }
    }
}
