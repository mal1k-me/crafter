//! Output configuration loading and merge logic.

use crate::output::{Format, Level, OutputPolicy};
use crate::types::{Config, OutputPreferences};
use std::env;
use std::path::PathBuf;

/// Loader for CLI/env/file/default output preferences.
pub struct ConfigLoader;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliVerbosity {
    Default,
    Quiet,
    Verbose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliFlag {
    Default,
    Enabled,
}

#[derive(Debug, Clone, Copy)]
pub struct CliOutputArgs {
    pub format: Option<Format>,
    pub verbosity: CliVerbosity,
    pub raw_sizes: CliFlag,
    pub full_paths: CliFlag,
}

impl ConfigLoader {
    /// Load output configuration from all sources.
    #[must_use] 
    pub fn load_output_config(cli: CliOutputArgs) -> OutputPolicy {
        let file_prefs = Self::load_file_preferences();
        let env_prefs = Self::load_env_preferences();

        Self::merge_configs(cli, &env_prefs, &file_prefs)
    }

    /// Load preferences from config file.
    fn load_file_preferences() -> OutputPreferences {
        // Try to load config file
        if let Ok(config_dir) = Self::get_config_dir() {
            let config_path = config_dir.join("config.json");

            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<Config>(&content) {
                    return config.output;
                }
            }
        }

        // Return defaults if file doesn't exist or can't be read
        OutputPreferences::default()
    }

    /// Load preferences from environment variables.
    fn load_env_preferences() -> OutputPreferences {
        OutputPreferences {
            format: env::var("CRAFTER_FORMAT").ok(),
            verbosity: env::var("CRAFTER_VERBOSITY").ok(),
            raw_sizes: Self::parse_bool_env("CRAFTER_RAW_SIZES").unwrap_or(false),
            full_paths: Self::parse_bool_env("CRAFTER_FULL_PATHS").unwrap_or(false),
            force_color: Self::parse_bool_env("CRAFTER_FORCE_COLOR").unwrap_or(false),
        }
    }

    /// Merge CLI, environment, file, and default values.
    fn merge_configs(cli: CliOutputArgs, env_prefs: &OutputPreferences, file_prefs: &OutputPreferences) -> OutputPolicy {
        // Determine format: CLI > Env > File > Default
        let format = cli
            .format
            .or_else(|| Self::parse_format(env_prefs.format.as_deref()))
            .or_else(|| Self::parse_format(file_prefs.format.as_deref()))
            .unwrap_or(Format::Human);

        // Determine verbosity level: CLI > Env > File > Default
        let level = match cli.verbosity {
            CliVerbosity::Quiet => Level::Silent,
            CliVerbosity::Verbose => Level::Verbose,
            CliVerbosity::Default => {
                Self::parse_level(env_prefs.verbosity.as_deref())
                    .or_else(|| Self::parse_level(file_prefs.verbosity.as_deref()))
                    .unwrap_or(Level::Normal)
            }
        };

        // Boolean flags: CLI (if true) > Env > File > Default (false)
        let raw_sizes = matches!(cli.raw_sizes, CliFlag::Enabled) || env_prefs.raw_sizes || file_prefs.raw_sizes;
        let full_paths = matches!(cli.full_paths, CliFlag::Enabled) || env_prefs.full_paths || file_prefs.full_paths;
        let force_color = env_prefs.force_color || file_prefs.force_color;

        OutputPolicy::new()
            .with_format(format)
            .with_level(level)
            .with_raw_sizes(raw_sizes)
            .with_full_paths(full_paths)
            .with_force_color(force_color)
    }

    /// Parse boolean from environment variable.
    fn parse_bool_env(var_name: &str) -> Option<bool> {
        env::var(var_name).ok().and_then(|v| {
            if matches!(v.as_str(), "1")
                || v.eq_ignore_ascii_case("true")
                || v.eq_ignore_ascii_case("yes")
                || v.eq_ignore_ascii_case("on")
            {
                Some(true)
            } else if matches!(v.as_str(), "0")
                || v.eq_ignore_ascii_case("false")
                || v.eq_ignore_ascii_case("no")
                || v.eq_ignore_ascii_case("off")
            {
                Some(false)
            } else {
                None
            }
        })
    }

    /// Parse output format value.
    fn parse_format(s: Option<&str>) -> Option<Format> {
        s.and_then(|v| {
            if v.eq_ignore_ascii_case("json") {
                Some(Format::Json)
            } else if v.eq_ignore_ascii_case("human") {
                Some(Format::Human)
            } else if v.eq_ignore_ascii_case("simple") {
                Some(Format::Simple)
            } else {
                None
            }
        })
    }

    /// Parse verbosity level value.
    fn parse_level(s: Option<&str>) -> Option<Level> {
        s.and_then(|v| {
            if v.eq_ignore_ascii_case("silent") || v.eq_ignore_ascii_case("quiet") {
                Some(Level::Silent)
            } else if v.eq_ignore_ascii_case("normal") {
                Some(Level::Normal)
            } else if v.eq_ignore_ascii_case("verbose") {
                Some(Level::Verbose)
            } else if v.eq_ignore_ascii_case("debug") {
                Some(Level::Debug)
            } else {
                None
            }
        })
    }

    /// Resolve config directory path.
    fn get_config_dir() -> Result<PathBuf, std::io::Error> {
        crate::utils::env::config_dir()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q_a_parse_helpers() {
        assert_eq!(ConfigLoader::parse_format(Some("json")), Some(Format::Json));
        assert_eq!(ConfigLoader::parse_format(Some("invalid")), None);
        assert_eq!(ConfigLoader::parse_level(Some("quiet")), Some(Level::Silent));
        assert_eq!(ConfigLoader::parse_level(Some("debug")), Some(Level::Debug));
        assert_eq!(ConfigLoader::parse_level(Some("invalid")), None);

        let cases = [
            ("true", Some(true)),
            ("1", Some(true)),
            ("yes", Some(true)),
            ("false", Some(false)),
            ("0", Some(false)),
            ("invalid", None),
        ];

        for (index, (value, expected)) in cases.iter().enumerate() {
            let key = format!("CRAFTER_TEST_BOOL_{index}");
            env::set_var(&key, value);
            assert_eq!(ConfigLoader::parse_bool_env(&key), *expected);
            env::remove_var(&key);
        }
    }

    #[test]
    fn q_a_precedence_cli_env_file_default() {
        let env_prefs = OutputPreferences {
            format: Some("simple".to_string()),
            verbosity: Some("debug".to_string()),
            raw_sizes: false,
            full_paths: true,
            force_color: false,
        };
        let file_prefs = OutputPreferences {
            format: Some("human".to_string()),
            verbosity: Some("normal".to_string()),
            raw_sizes: true,
            full_paths: false,
            force_color: true,
        };

        // Q: CLI format + raw_sizes enabled, env full_paths true, file force_color true.
        // A: format/raw_sizes from CLI, full_paths from env, force_color from file.
        let config = ConfigLoader::merge_configs(
            CliOutputArgs {
                format: Some(Format::Json),
                verbosity: CliVerbosity::Default,
                raw_sizes: CliFlag::Enabled,
                full_paths: CliFlag::Default,
            },
            &env_prefs,
            &file_prefs,
        );

        assert_eq!(config.format, Format::Json);
        assert_eq!(config.level, Level::Debug);
        assert!(config.raw_sizes);
        assert!(config.full_paths);
    }

    #[test]
    fn q_a_defaults_when_nothing_is_set() {
        let config = ConfigLoader::merge_configs(
            CliOutputArgs {
                format: None,
                verbosity: CliVerbosity::Default,
                raw_sizes: CliFlag::Default,
                full_paths: CliFlag::Default,
            },
            &OutputPreferences::default(),
            &OutputPreferences::default(),
        );

        assert_eq!(config.format, Format::Human);
        assert_eq!(config.level, Level::Normal);
        assert!(!config.raw_sizes);
        assert!(!config.full_paths);
    }

    #[test]
    fn q_a_cli_verbosity_modes_override_all() {
        let env_prefs = OutputPreferences {
            verbosity: Some("debug".to_string()),
            ..Default::default()
        };
        let file_prefs = OutputPreferences {
            verbosity: Some("normal".to_string()),
            ..Default::default()
        };

        let quiet_cfg = ConfigLoader::merge_configs(
            CliOutputArgs {
                format: None,
                verbosity: CliVerbosity::Quiet,
                raw_sizes: CliFlag::Default,
                full_paths: CliFlag::Default,
            },
            &env_prefs,
            &file_prefs,
        );

        let verbose_cfg = ConfigLoader::merge_configs(
            CliOutputArgs {
                format: None,
                verbosity: CliVerbosity::Verbose,
                raw_sizes: CliFlag::Default,
                full_paths: CliFlag::Default,
            },
            &env_prefs,
            &file_prefs,
        );

        assert_eq!(quiet_cfg.level, Level::Silent);
        assert_eq!(verbose_cfg.level, Level::Verbose);
    }
}
