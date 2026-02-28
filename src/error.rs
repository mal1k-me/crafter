//! Error types with Unix exit codes
//!
//! Following Unix conventions for process exit codes

use serde::Serialize;
use std::fmt;

/// Unix-style exit codes
pub mod codes {
    pub const SUCCESS: i32 = 0;
    pub const GENERAL_ERROR: i32 = 1;
    pub const INVALID_USAGE: i32 = 2;
    pub const DOCKER_ERROR: i32 = 3;
    pub const NETWORK_ERROR: i32 = 4;
    pub const NOT_FOUND: i32 = 5;
    pub const VALIDATION_FAILED: i32 = 6;
    pub const TEST_FAILED: i32 = 7;
    pub const INTERRUPTED: i32 = 130;
}

/// Main error type with exit codes
#[derive(Debug)]
pub enum CrafterError {
    /// Docker is not available or not running
    Docker {
        message: String,
        hint: Option<String>,
    },

    /// Network or download error
    Network {
        message: String,
        url: Option<String>,
    },

    /// Resource not found
    NotFound {
        resource: String,
        name: String,
        suggestion: Option<String>,
    },

    /// Validation failed
    ValidationFailed { failures: Vec<ValidationFailure> },

    /// Test execution failed
    TestFailed {
        stage: String,
        exit_code: i32,
        output: Option<String>,
    },

    /// Invalid command line usage
    InvalidUsage { message: String },

    /// Configuration error
    Config {
        message: String,
        path: Option<std::path::PathBuf>,
    },

    /// IO error
    Io(std::io::Error),

    /// YAML parsing error
    Yaml(serde_yaml::Error),

    /// JSON parsing error
    Json(serde_json::Error),

    /// HTTP request error
    Http(reqwest::Error),

    /// Generic error
    Other(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationFailure {
    pub check: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl CrafterError {
    /// Get Unix exit code
    #[must_use] 
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::Docker { .. } => codes::DOCKER_ERROR,
            Self::Network { .. } => codes::NETWORK_ERROR,
            Self::NotFound { .. } => codes::NOT_FOUND,
            Self::ValidationFailed { .. } => codes::VALIDATION_FAILED,
            Self::TestFailed { .. } => codes::TEST_FAILED,
            Self::InvalidUsage { .. } => codes::INVALID_USAGE,
            Self::Config { .. } | Self::Io(_) | Self::Yaml(_) | Self::Json(_) | Self::Http(_) => {
                codes::GENERAL_ERROR
            }
            Self::Other(_) => codes::GENERAL_ERROR,
        }
    }

    /// Get machine-readable error type
    #[must_use] 
    pub const fn error_type(&self) -> &str {
        match self {
            Self::Docker { .. } => "docker",
            Self::Network { .. } => "network",
            Self::NotFound { .. } => "not_found",
            Self::ValidationFailed { .. } => "validation",
            Self::TestFailed { .. } => "test_failed",
            Self::InvalidUsage { .. } => "invalid_usage",
            Self::Config { .. } => "config",
            Self::Io(_) => "io",
            Self::Yaml(_) => "yaml",
            Self::Json(_) => "json",
            Self::Http(_) => "http",
            Self::Other(_) => "error",
        }
    }

    /// Convert to JSON for machine-readable output
    #[must_use] 
    pub fn to_json(&self) -> serde_json::Value {
        use serde_json::json;

        match self {
            Self::Docker { message, hint } => json!({
                "type": "docker",
                "error": "docker",
                "message": message,
                "hint": hint,
                "exit_code": self.exit_code(),
            }),
            Self::Network { message, url } => json!({
                "type": "network",
                "error": "network",
                "message": message,
                "url": url,
                "exit_code": self.exit_code(),
            }),
            Self::NotFound {
                resource,
                name,
                suggestion,
            } => json!({
                "type": "not_found",
                "error": "not_found",
                "message": format!("{resource} '{name}' not found"),
                "resource": resource,
                "name": name,
                "suggestion": suggestion,
                "exit_code": self.exit_code(),
            }),
            Self::ValidationFailed { failures } => json!({
                "type": "validation",
                "error": "validation",
                "message": format!("Validation failed ({} check(s) failed)", failures.len()),
                "failures": failures,
                "exit_code": self.exit_code(),
            }),
            Self::TestFailed {
                stage,
                exit_code,
                output,
            } => json!({
                "type": "test_failed",
                "error": "test_failed",
                "message": format!("Test failed for stage '{stage}' (exit code: {exit_code})"),
                "stage": stage,
                "test_exit_code": exit_code,
                "output": output,
                "exit_code": self.exit_code(),
            }),
            _ => json!({
                "type": self.error_type(),
                "error": self.error_type(),
                "message": self.to_string(),
                "exit_code": self.exit_code(),
            }),
        }
    }

    // Convenience constructors
    pub fn docker(message: impl Into<String>) -> Self {
        Self::Docker {
            message: message.into(),
            hint: Some("Ensure Docker is installed and running".to_string()),
        }
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
            url: None,
        }
    }

    pub fn not_found(resource: impl Into<String>, name: impl Into<String>) -> Self {
        Self::NotFound {
            resource: resource.into(),
            name: name.into(),
            suggestion: None,
        }
    }

    pub fn test_failed(stage: impl Into<String>, exit_code: i32) -> Self {
        Self::TestFailed {
            stage: stage.into(),
            exit_code,
            output: None,
        }
    }

    pub fn invalid_usage(message: impl Into<String>) -> Self {
        Self::InvalidUsage {
            message: message.into(),
        }
    }

    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }
}

impl fmt::Display for CrafterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Docker { message, .. } | Self::InvalidUsage { message } => {
                write!(f, "{message}")
            }
            Self::Network { message, url } => {
                write!(f, "{message}")?;
                if let Some(u) = url {
                    write!(f, " ({u})")?;
                }
                Ok(())
            }
            Self::NotFound { resource, name, .. } => {
                write!(f, "{resource} '{name}' not found")
            }
            Self::TestFailed {
                stage, exit_code, ..
            } => {
                write!(
                    f,
                    "Test failed for stage '{stage}' (exit code: {exit_code})"
                )
            }
            Self::ValidationFailed { failures } => {
                write!(f, "Validation failed ({} check(s) failed)", failures.len())
            }
            Self::Config { message, path } => {
                write!(f, "{message}")?;
                if let Some(p) = path {
                    write!(f, " ({})", crate::output::compat::format_path(p))?;
                }
                Ok(())
            }
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Yaml(e) => write!(f, "YAML error: {e}"),
            Self::Json(e) => write!(f, "JSON error: {e}"),
            Self::Http(e) => write!(f, "HTTP error: {e}"),
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for CrafterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Yaml(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::Http(e) => Some(e),
            _ => None,
        }
    }
}

// Conversions from standard errors
impl From<std::io::Error> for CrafterError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_yaml::Error> for CrafterError {
    fn from(err: serde_yaml::Error) -> Self {
        Self::Yaml(err)
    }
}

impl From<serde_json::Error> for CrafterError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl From<reqwest::Error> for CrafterError {
    fn from(err: reqwest::Error) -> Self {
        Self::Http(err)
    }
}

/// Result type alias
pub type Result<T> = std::result::Result<T, CrafterError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_codes() {
        assert_eq!(
            CrafterError::docker("test").exit_code(),
            codes::DOCKER_ERROR
        );
        assert_eq!(
            CrafterError::not_found("stage", "oo8").exit_code(),
            codes::NOT_FOUND
        );
        assert_eq!(
            CrafterError::test_failed("oo8", 1).exit_code(),
            codes::TEST_FAILED
        );
    }

    #[test]
    fn test_error_types() {
        assert_eq!(CrafterError::docker("test").error_type(), "docker");
        assert_eq!(CrafterError::network("test").error_type(), "network");
    }

    #[test]
    fn test_json_output() {
        let err = CrafterError::not_found("stage", "invalid");
        let json = err.to_json();
        assert_eq!(json["error"], "not_found");
        assert_eq!(json["resource"], "stage");
        assert_eq!(json["name"], "invalid");
    }
}
