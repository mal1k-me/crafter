// Configuration types

use serde::{Deserialize, Serialize};

/// Output preferences for CLI display
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutputPreferences {
    /// Preferred output format (json, human, simple)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    /// Preferred verbosity level (silent, normal, verbose, debug)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<String>,

    /// Show raw file sizes instead of human-readable
    #[serde(default)]
    pub raw_sizes: bool,

    /// Show full paths instead of abbreviated with ~
    #[serde(default)]
    pub full_paths: bool,

    /// Force colored output even when not a TTY
    #[serde(default)]
    pub force_color: bool,
}

/// Main configuration file (~/.config/crafter/config.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_auto_update")]
    pub auto_update: bool,

    /// Output preferences (format, verbosity, etc.)
    #[serde(default)]
    pub output: OutputPreferences,

    #[serde(default)]
    pub settings: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_update: true,
            output: OutputPreferences::default(),
            settings: std::collections::HashMap::new(),
        }
    }
}

const fn default_auto_update() -> bool {
    true
}

/// Challenge entry in challenges.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeEntry {
    pub repository: String,
    pub tester: String,
}

/// Challenges configuration file (~/.config/crafter/challenges.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengesConfig {
    pub challenges: Vec<ChallengeEntry>,
}

impl ChallengesConfig {
    #[must_use] 
    pub fn default_challenges() -> Self {
        Self {
            challenges: vec![
                ChallengeEntry {
                    repository: "https://github.com/codecrafters-io/build-your-own-shell"
                        .to_string(),
                    tester: "https://github.com/codecrafters-io/shell-tester".to_string(),
                },
                ChallengeEntry {
                    repository: "https://github.com/codecrafters-io/build-your-own-redis"
                        .to_string(),
                    tester: "https://github.com/codecrafters-io/redis-tester".to_string(),
                },
                ChallengeEntry {
                    repository: "https://github.com/codecrafters-io/build-your-own-grep"
                        .to_string(),
                    tester: "https://github.com/codecrafters-io/grep-tester".to_string(),
                },
                // Add more default challenges as needed
            ],
        }
    }

    #[must_use] 
    pub fn find_challenge(&self, name: &str) -> Option<&ChallengeEntry> {
        self.challenges
            .iter()
            .find(|c| c.repository.contains(&format!("build-your-own-{name}")))
    }
}

/// codecrafters.yml configuration in project directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodecraftersConfig {
    #[serde(default)]
    pub debug: bool,

    pub buildpack: String,
}

impl CodecraftersConfig {
    /// Get default buildpack for a language, optionally scanning available dockerfiles
    #[must_use] 
    pub fn default_buildpack(language: &str, challenge_dir: Option<&std::path::Path>) -> String {
        // Try hardcoded defaults first for common languages
        let hardcoded = match language {
            "rust" => Some("rust-1.92".to_string()),
            "go" => Some("go-1.23".to_string()),
            "python" => Some("python-3.13".to_string()),
            "javascript" | "typescript" => Some("node-23".to_string()),
            _ => None,
        };

        if let Some(bp) = hardcoded {
            return bp;
        }

        // For unknown languages, try to detect latest available version
        if let Some(dir) = challenge_dir {
            if let Ok(latest) = Self::detect_latest_buildpack(dir, language) {
                return latest;
            }
        }

        // Fallback: use language name with -latest suffix (will likely fail)
        format!("{language}-latest")
    }

    /// Detect the latest available buildpack for a language by scanning dockerfiles
    fn detect_latest_buildpack(
        challenge_dir: &std::path::Path,
        language: &str,
    ) -> Result<String, std::io::Error> {
        let dockerfiles_dir = challenge_dir.join("dockerfiles");

        if !dockerfiles_dir.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "dockerfiles directory not found",
            ));
        }

        let mut versions = Vec::new();

        for entry in std::fs::read_dir(&dockerfiles_dir)? {
            let entry = entry?;
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();

            // Match pattern: {language}-{version}.Dockerfile
            if filename_str.starts_with(&format!("{language}-"))
                && filename_str.ends_with(".Dockerfile")
            {
                // Extract buildpack name (remove .Dockerfile extension)
                if let Some(buildpack) = filename_str.strip_suffix(".Dockerfile") {
                    versions.push(buildpack.to_string());
                }
            }
        }

        if versions.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("No dockerfiles found for language '{language}'"),
            ));
        }

        // Sort versions (simple lexicographic sort - works for most cases)
        versions.sort();

        // Return the last one (highest version)
        versions.last().cloned().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("No dockerfiles found for language '{language}'"),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q_a_config_defaults_and_serde_round_trip() {
        let cfg = Config::default();
        assert!(cfg.auto_update);

        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.auto_update, cfg.auto_update);
    }

    #[test]
    fn q_a_challenge_lookup_known_and_unknown() {
        let cfg = ChallengesConfig::default_challenges();
        assert!(!cfg.challenges.is_empty());
        assert!(cfg.find_challenge("shell").is_some());
        assert!(cfg.find_challenge("nonexistent-xyz").is_none());
    }

    #[test]
    fn q_a_default_buildpack_known_languages() {
        let cases = [
            ("rust", "rust-1.92"),
            ("go", "go-1.23"),
            ("python", "python-3.13"),
            ("javascript", "node-23"),
            ("typescript", "node-23"),
        ];

        for (lang, expected) in cases {
            assert_eq!(CodecraftersConfig::default_buildpack(lang, None), expected);
        }
    }

    #[test]
    fn q_a_default_buildpack_scans_dockerfiles_when_available() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let dockerfiles = dir.path().join("dockerfiles");
        fs::create_dir_all(&dockerfiles).unwrap();
        fs::write(dockerfiles.join("zig-0.12.Dockerfile"), "").unwrap();
        fs::write(dockerfiles.join("zig-0.13.Dockerfile"), "").unwrap();

        let bp = CodecraftersConfig::default_buildpack("zig", Some(dir.path()));
        assert_eq!(bp, "zig-0.13");
    }

    #[test]
    fn q_a_default_buildpack_falls_back_to_latest() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let dockerfiles = dir.path().join("dockerfiles");
        fs::create_dir_all(&dockerfiles).unwrap();
        fs::write(dockerfiles.join("python-3.12.Dockerfile"), "").unwrap();

        assert_eq!(
            CodecraftersConfig::default_buildpack("zig", Some(dir.path())),
            "zig-latest"
        );
    }
}
