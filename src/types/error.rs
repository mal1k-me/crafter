// Error types for Crafter

use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CrafterError>;

#[derive(Error, Debug)]
pub enum CrafterError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Challenge not found: {0}")]
    ChallengeNotFound(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Docker error: {0}")]
    Docker(String),

    #[error("Tester error: {0}")]
    Tester(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),

    #[error("Command failed: {0}")]
    CommandFailed(String),

    #[error("Not initialized. Run 'crafter base setup' first")]
    NotInitialized,

    #[error("{0}")]
    Other(String),

    /// Error with suggestion for how to fix it
    #[error("{message}")]
    WithSuggestion { message: String, suggestion: String },
}

impl CrafterError {
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    pub fn git(msg: impl Into<String>) -> Self {
        Self::Git(msg.into())
    }

    pub fn docker(msg: impl Into<String>) -> Self {
        Self::Docker(msg.into())
    }

    pub fn tester(msg: impl Into<String>) -> Self {
        Self::Tester(msg.into())
    }

    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }

    pub fn with_suggestion(message: impl Into<String>, suggestion: impl Into<String>) -> Self {
        Self::WithSuggestion {
            message: message.into(),
            suggestion: suggestion.into(),
        }
    }

    /// Get suggestion if available
    #[must_use] 
    pub fn suggestion(&self) -> Option<String> {
        match self {
            Self::WithSuggestion { suggestion, .. } => Some(suggestion.clone()),
            Self::NotInitialized => Some("Run: crafter base setup".to_string()),
            Self::ChallengeNotFound(_) => {
                Some("Run 'crafter challenge list' to see available challenges".to_string())
            }
            Self::Docker(msg) => {
                // Parse Docker error message for context-aware suggestions
                if msg.contains("returned a non-zero code")
                    || msg.contains("returned non-zero")
                    || msg.contains("command not found")
                    || msg.contains("RUN")
                {
                    // Build failed, likely Dockerfile issue
                    Some(
                        "Docker build failed. Check your Dockerfile for:\n  \
                        - Missing package installations (e.g., curl, git)\n  \
                        - Incorrect RUN commands\n  \
                        - Base image compatibility"
                            .to_string(),
                    )
                } else if msg.contains("Cannot connect") || msg.contains("daemon") {
                    // Docker daemon not running
                    Some(
                        "Docker daemon is not running. Start it with:\n  \
                        sudo systemctl start docker\n  \
                        Or ensure Docker Desktop is running"
                            .to_string(),
                    )
                } else if msg.contains("permission denied") || msg.contains("Permission denied") {
                    // Permission issue
                    Some(
                        "Docker permission denied. Add your user to docker group:\n  \
                        sudo usermod -aG docker $USER\n  \
                        Then log out and back in"
                            .to_string(),
                    )
                } else if msg.contains("not found") && msg.contains("image") {
                    // Image not found
                    Some(
                        "Docker image not found. Try rebuilding:\n  \
                        crafter test --rebuild"
                            .to_string(),
                    )
                } else {
                    // Generic Docker issue
                    Some(
                        "Docker error occurred. Ensure Docker is installed and running:\n  \
                        https://docs.docker.com/get-docker/"
                            .to_string(),
                    )
                }
            }
            Self::Git(msg) if msg.contains("not a git repository") => {
                Some("Initialize a git repository:\n  git init".to_string())
            }
            Self::Git(msg) if msg.contains("Failed to execute git") => {
                Some("Ensure git is installed:\n  https://git-scm.com/downloads".to_string())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q_a_display_formats_stay_human_readable() {
        let cases = [
            (
                CrafterError::config("bad value"),
                "Configuration error: bad value",
            ),
            (
                CrafterError::git("not a git repository"),
                "Git error: not a git repository",
            ),
            (
                CrafterError::docker("daemon not running"),
                "Docker error: daemon not running",
            ),
            (CrafterError::other("something unexpected"), "something unexpected"),
        ];

        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }

    #[test]
    fn q_a_explicit_suggestion_variant_wins() {
        let e = CrafterError::with_suggestion("oops", "do X");
        assert_eq!(e.suggestion(), Some("do X".to_string()));
    }

    #[test]
    fn q_a_core_suggestions_are_exposed() {
        let not_init = CrafterError::NotInitialized;
        assert!(not_init.suggestion().unwrap().contains("crafter base setup"));

        let missing = CrafterError::ChallengeNotFound("redis".to_string());
        assert!(missing.suggestion().unwrap().contains("crafter challenge list"));

        let git = CrafterError::git("not a git repository");
        assert!(git.suggestion().unwrap().contains("git init"));
    }

    #[test]
    fn q_a_docker_suggestions_cover_key_paths() {
        let cases = [
            ("permission denied for socket", "docker group"),
            ("Cannot connect to Docker daemon", "systemctl start docker"),
            ("RUN apt-get returned a non-zero code: 100", "Dockerfile"),
            ("image not found in registry", "rebuild"),
        ];

        for (message, expected) in cases {
            let suggestion = CrafterError::docker(message).suggestion().unwrap();
            assert!(suggestion.contains(expected));
        }
    }

    #[test]
    fn q_a_non_mapped_errors_have_no_suggestion() {
        assert_eq!(CrafterError::config("missing key").suggestion(), None);
        assert_eq!(CrafterError::other("random error").suggestion(), None);
    }
}
