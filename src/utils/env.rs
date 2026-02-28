//! Environment and user-path utilities.

use crate::types::Result;
use std::path::PathBuf;

const CONFIG_SUBDIR: &str = ".config";
const LOCAL_SHARE_SUBDIR: &str = ".local/share";
const APP_DIR: &str = "crafter";
const CHALLENGES_DIR: &str = "challenges";
const TESTERS_DIR: &str = "testers";

fn required_home_dir() -> Result<PathBuf> {
    home::home_dir()
        .ok_or_else(|| crate::types::error::CrafterError::other("Could not determine home directory"))
}

/// Get config directory (~/.config/crafter)
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn config_dir() -> Result<PathBuf> {
    Ok(required_home_dir()?.join(CONFIG_SUBDIR).join(APP_DIR))
}

/// Get data directory (~/.local/share/crafter)
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn data_dir() -> Result<PathBuf> {
    Ok(required_home_dir()?.join(LOCAL_SHARE_SUBDIR).join(APP_DIR))
}

/// Get challenges directory (~/.local/share/crafter/challenges)
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn challenges_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join(CHALLENGES_DIR))
}

/// Get testers directory (~/.local/share/crafter/testers)
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn testers_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join(TESTERS_DIR))
}

/// Get current working directory
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn current_dir() -> Result<PathBuf> {
    std::env::current_dir().map_err(|e| {
        crate::types::error::CrafterError::other(format!("Could not get current directory: {e}"))
    })
}

/// Check if environment variable is set
#[must_use] 
pub fn has_env(key: &str) -> bool {
    std::env::var(key).is_ok()
}

/// Get environment variable or default
#[must_use] 
pub fn get_env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q_a_paths_resolve_under_expected_roots() {
        let cfg = config_dir().expect("home dir should be available in test env");
        let data = data_dir().expect("home dir should be available in test env");
        assert!(cfg.ends_with(".config/crafter"));
        assert!(data.ends_with(".local/share/crafter"));
        assert_eq!(challenges_dir().unwrap(), data.join("challenges"));
        assert_eq!(testers_dir().unwrap(), data.join("testers"));
    }

    #[test]
    fn q_a_current_dir_and_env_helpers_work() {
        assert!(current_dir().is_ok());

        std::env::set_var("CRAFTER_TEST_VAR", "hello");
        assert!(has_env("CRAFTER_TEST_VAR"));
        assert_eq!(get_env_or("CRAFTER_TEST_VAR", "default"), "hello");
        std::env::remove_var("CRAFTER_TEST_VAR");

        std::env::remove_var("CRAFTER_TEST_ABSENT_XYZ");
        assert!(!has_env("CRAFTER_TEST_ABSENT_XYZ"));
        assert_eq!(get_env_or("CRAFTER_TEST_ABSENT_XYZ", "fallback"), "fallback");
    }
}
