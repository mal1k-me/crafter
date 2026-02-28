//! Global configuration management.

use crate::types::{ChallengesConfig, Config, Result};
use crate::utils::{env, fs};
use std::path::PathBuf;

const CONFIG_FILE: &str = "config.json";
const CHALLENGES_FILE: &str = "challenges.json";

/// Paths resolved by `ConfigManager::initialize()`.
pub struct InitResult {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
}

pub struct ConfigManager {
    config_dir: PathBuf,
}

impl ConfigManager {
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn new() -> Result<Self> {
        let config_dir = env::config_dir()?;
        Ok(Self { config_dir })
    }

    fn ensure_config_dir(&self) -> Result<()> {
        fs::ensure_dir(&self.config_dir)
    }

    fn write_pretty_json<T: serde::Serialize>(&self, path: PathBuf, value: &T) -> Result<()> {
        let content = serde_json::to_string_pretty(value)?;
        fs::write_string(path, &content)?;
        Ok(())
    }

    /// Get `config.json` path.
    #[must_use] 
    pub fn config_path(&self) -> PathBuf {
        self.config_dir.join(CONFIG_FILE)
    }

    /// Get `challenges.json` path.
    fn challenges_path(&self) -> PathBuf {
        self.config_dir.join(CHALLENGES_FILE)
    }

    /// Load `config.json` or return default.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn get_config(&self) -> Result<Config> {
        let path = self.config_path();

        if !fs::exists(&path) {
            return Ok(Config::default());
        }

        let content = fs::read_to_string(&path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save `config.json`.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn save_config(&self, config: &Config) -> Result<()> {
        self.ensure_config_dir()?;
        self.write_pretty_json(self.config_path(), config)
    }

    /// Set `auto_update`.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn set_auto_update(&self, enabled: bool) -> Result<()> {
        let mut config = self.get_config()?;
        config.auto_update = enabled;
        self.save_config(&config)
    }

    /// Load `challenges.json` or return defaults.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn get_challenges(&self) -> Result<ChallengesConfig> {
        let path = self.challenges_path();

        if !fs::exists(&path) {
            return Ok(ChallengesConfig::default_challenges());
        }

        let content = fs::read_to_string(&path)?;
        let challenges: ChallengesConfig = serde_json::from_str(&content)?;
        Ok(challenges)
    }

    /// Save `challenges.json`.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn save_challenges(&self, challenges: &ChallengesConfig) -> Result<()> {
        self.ensure_config_dir()?;
        self.write_pretty_json(self.challenges_path(), challenges)
    }

    /// Initialize config/data directories and default files.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn initialize(&self) -> Result<InitResult> {
        // Create directories
        self.ensure_config_dir()?;
        let data_dir = env::data_dir()?;
        fs::ensure_dir(&data_dir)?;
        fs::ensure_dir(&env::challenges_dir()?)?;
        fs::ensure_dir(&env::testers_dir()?)?;

        // Create default config if it doesn't exist
        if !fs::exists(self.config_path()) {
            self.save_config(&Config::default())?;
        }

        // Create default challenges if it doesn't exist
        if !fs::exists(self.challenges_path()) {
            self.save_challenges(&ChallengesConfig::default_challenges())?;
        }

        Ok(InitResult {
            config_dir: self.config_dir.clone(),
            data_dir,
        })
    }

    /// Check whether crafter is initialized.
    #[must_use] 
    pub fn is_initialized(&self) -> bool {
        fs::exists(self.config_path()) && fs::exists(self.challenges_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.auto_update);
    }

    #[test]
    fn test_challenges_default() {
        let challenges = ChallengesConfig::default_challenges();
        assert!(!challenges.challenges.is_empty());
    }
}
