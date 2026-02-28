//! Challenge auto-detection from project context.

use crate::core::git::GitManager;
use crate::types::{CrafterError, Result};
use crate::utils::slug;
use std::path::Path;
use std::sync::Arc;

const DETECTION_HINT: &str = "Ensure you're in a CodeCrafters project directory (e.g., codecrafters-shell-rust)\nor initialize a new project:\n  crafter challenge init <challenge> <language>";

pub struct ChallengeDetector {
    git_manager: Arc<GitManager>,
}

impl ChallengeDetector {
    #[must_use] 
    pub const fn new(git_manager: Arc<GitManager>) -> Self {
        Self { git_manager }
    }

    /// Detect challenge from project directory.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn detect(&self, project_dir: &Path) -> Result<String> {
        self.detect_from_git_remote(project_dir)
            .or_else(|| Self::detect_from_directory_name(project_dir))
            .or_else(|| Self::detect_from_parent_name(project_dir))
            .ok_or_else(Self::detection_error)
    }

    fn detection_error() -> CrafterError {
        CrafterError::with_suggestion(
            "Could not detect challenge from directory name or git remote",
            DETECTION_HINT,
        )
    }

    /// Try detection from git remote URL.
    fn detect_from_git_remote(&self, project_dir: &Path) -> Option<String> {
        if !self.git_manager.is_repo(project_dir) {
            return None;
        }

        let url = self.git_manager.get_remote_url(project_dir).ok()?;
        slug::challenge_from_url(&url)
    }

    /// Try detection from directory name.
    fn detect_from_directory_name(project_dir: &Path) -> Option<String> {
        let dir_name = project_dir.file_name()?.to_str()?;
        slug::challenge_from_dirname(dir_name)
    }

    /// Try detection from parent directory name.
    fn detect_from_parent_name(project_dir: &Path) -> Option<String> {
        let parent = project_dir.parent()?;
        let parent_name = parent.file_name()?.to_str()?;
        slug::challenge_from_dirname(parent_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_logic() {
        // Test slug extraction
        assert_eq!(
            slug::challenge_from_url("https://github.com/codecrafters-io/build-your-own-shell"),
            Some("shell".to_string())
        );
        assert_eq!(
            slug::challenge_from_dirname("codecrafters-shell-rust"),
            Some("shell".to_string())
        );
    }

    #[test]
    fn detect_from_directory_name_parses_codecrafters_pattern() {
        let path = Path::new("/tmp/codecrafters-redis-rust");
        assert_eq!(
            ChallengeDetector::detect_from_directory_name(path),
            Some("redis".to_string())
        );
    }

    #[test]
    fn detect_from_parent_name_parses_codecrafters_pattern() {
        let path = Path::new("/tmp/codecrafters-grep-rust/src");
        assert_eq!(
            ChallengeDetector::detect_from_parent_name(path),
            Some("grep".to_string())
        );
    }
}
