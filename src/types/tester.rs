// Tester types

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct TesterInfo {
    pub challenge: String,
    pub version: String,
    pub path: PathBuf,
    pub tester_binary: PathBuf,
    pub test_script: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct BuildOptions {
    pub force: bool,
    pub version: Option<String>,
    pub from_source: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
}
