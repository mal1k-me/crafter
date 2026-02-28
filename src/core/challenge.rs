//! Challenge repository management.

use crate::core::config::ConfigManager;
use crate::core::git::GitManager;
use crate::types::{ChallengeEntry, CrafterError, Result};
use crate::utils::{env, fs, slug};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const COMPILED_STARTERS_DIR: &str = "compiled_starters";
const DOCKERFILES_DIR: &str = "dockerfiles";
const COURSE_DEFINITION_FILE: &str = "course-definition.yml";

pub struct ChallengeManager {
    config_manager: Arc<ConfigManager>,
    git_manager: Arc<GitManager>,
}

impl ChallengeManager {
    #[must_use] 
    pub const fn new(config_manager: Arc<ConfigManager>, git_manager: Arc<GitManager>) -> Self {
        Self {
            config_manager,
            git_manager,
        }
    }

    async fn clone_repository(&self, repository: &str, dest: &Path) -> Result<()> {
        // Use UFCS to disambiguate from `Arc::clone`.
        GitManager::clone(&self.git_manager, repository, dest).await
    }

    /// Get challenge directory path.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn get_challenge_dir(&self, challenge: &str) -> Result<PathBuf> {
        Ok(env::challenges_dir()?.join(challenge))
    }

    /// Check if challenge is downloaded.
    #[must_use] 
    pub fn is_downloaded(&self, challenge: &str) -> bool {
        self.get_challenge_dir(challenge)
            .map(|path| fs::exists(&path))
            .unwrap_or(false)
    }

    /// Download challenge repository.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub async fn download(&self, challenge: &str) -> Result<()> {
        use crate::output::compat as output;

        let entry = self.get_challenge_info(challenge)?;

        let dest = self.get_challenge_dir(challenge)?;

        if fs::exists(&dest) {
            output::warn(&format!(
                "Challenge repository already exists at {}",
                output::format_path(&dest)
            ));
            return Ok(());
        }

        output::step(&format!("Downloading challenge repository from {}...", entry.repository));
        
        // Verbose: show git clone details
        if output::is_verbose() {
            output::verbose(&format!("Repository: {}", entry.repository));
            output::verbose(&format!("Destination: {}", output::format_path(&dest)));
        }

        self.clone_repository(&entry.repository, &dest).await?;

        Ok(())
    }

    /// Update challenge repository.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn update(&self, challenge: &str) -> Result<()> {
        use crate::output::compat as output;

        let dest = self.get_challenge_dir(challenge)?;

        if !fs::exists(&dest) {
            return Err(CrafterError::other(format!(
                "Challenge {challenge} not downloaded. Run 'crafter challenge init {challenge} <language>' first."
            )));
        }

        output::step(&format!("Updating {challenge} challenge..."));
        
        self.git_manager.pull(&dest)?;

        output::success("Challenge updated");

        Ok(())
    }

    /// Get available languages for a challenge.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn get_available_languages(&self, challenge: &str) -> Result<Vec<String>> {
        let challenge_dir = self.get_challenge_dir(challenge)?;
        let starters_dir = challenge_dir.join(COMPILED_STARTERS_DIR);

        if !fs::exists(&starters_dir) {
            return Err(CrafterError::other(format!(
                "Challenge {challenge} not downloaded or missing compiled_starters/"
            )));
        }

        let mut languages = Vec::new();

        for entry in std::fs::read_dir(&starters_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    languages.push(name.to_string());
                }
            }
        }

        languages.sort();
        Ok(languages)
    }

    /// Get starter directory for a language.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn get_starter_dir(&self, challenge: &str, language: &str) -> Result<PathBuf> {
        let challenge_dir = self.get_challenge_dir(challenge)?;
        let starter_dir = challenge_dir.join(COMPILED_STARTERS_DIR).join(language);

        if !fs::exists(&starter_dir) {
            let error_msg = format!(
                "Language '{language}' not available for challenge '{challenge}'"
            );
            let suggestion = format!(
                "\n\nSuggestion:\n  Run 'crafter challenge languages {challenge}' to see available languages"
            );
            return Err(CrafterError::other(format!("{error_msg}{suggestion}")));
        }

        Ok(starter_dir)
    }

    /// Get dockerfile path for a buildpack.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn get_dockerfile(&self, challenge: &str, buildpack: &str) -> Result<PathBuf> {
        let challenge_dir = self.get_challenge_dir(challenge)?;
        let dockerfile = challenge_dir
            .join(DOCKERFILES_DIR)
            .join(format!("{buildpack}.Dockerfile"));

        if !fs::exists(&dockerfile) {
            return Err(CrafterError::other(format!(
                "Dockerfile for buildpack '{buildpack}' not found"
            )));
        }

        Ok(dockerfile)
    }

    /// List all configured challenges.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn list_challenges(&self) -> Result<Vec<ChallengeEntry>> {
        let config = self.config_manager.get_challenges()?;
        Ok(config.challenges)
    }

    /// List downloaded challenges.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn list_downloaded(&self) -> Result<Vec<String>> {
        let all_challenges = self.list_challenges()?;
        let downloaded: Vec<String> = all_challenges
            .iter()
            .filter_map(|entry| {
                let name = slug::challenge_from_url(&entry.repository)?;
                self.is_downloaded(&name).then_some(name)
            })
            .collect();

        Ok(downloaded)
    }

    /// Get challenge metadata.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn get_challenge_info(&self, challenge: &str) -> Result<ChallengeEntry> {
        let config = self.config_manager.get_challenges()?;
        config
            .find_challenge(challenge)
            .cloned()
            .ok_or_else(|| CrafterError::ChallengeNotFound(challenge.to_string()))
    }

    /// Parse course definition.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn get_course_definition(&self, challenge: &str) -> Result<crate::types::CourseDefinition> {
        let challenge_dir = self.get_challenge_dir(challenge)?;
        let course_def_path = challenge_dir.join(COURSE_DEFINITION_FILE);

        if !fs::exists(&course_def_path) {
            return Err(CrafterError::other(format!(
                "{COURSE_DEFINITION_FILE} not found for challenge '{challenge}'"
            )));
        }

        let content = fs::read_to_string(&course_def_path)?;
        let course_def: crate::types::CourseDefinition = serde_yaml::from_str(&content)
            .map_err(|e| CrafterError::other(format!("Failed to parse {COURSE_DEFINITION_FILE}: {e}")))?;

        Ok(course_def)
    }

    /// Get first stage slug.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn get_first_stage_slug(&self, challenge: &str) -> Result<String> {
        let course_def = self.get_course_definition(challenge)?;
        course_def
            .first_stage_slug()
            .ok_or_else(|| CrafterError::other(format!("No stages found for challenge '{challenge}'")))
    }

    /// Get all stages for a challenge.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn get_stages(&self, challenge: &str) -> Result<Vec<crate::types::Stage>> {
        let course_def = self.get_course_definition(challenge)?;
        Ok(course_def.stages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_paths() {
        // Just test path construction logic
        let challenges_dir = PathBuf::from("/tmp/challenges");
        let shell_dir = challenges_dir.join("shell");
        assert_eq!(shell_dir.to_str().unwrap(), "/tmp/challenges/shell");
    }

    #[test]
    fn test_challenge_name_from_repository() {
        assert_eq!(
            slug::challenge_from_url("https://github.com/codecrafters-io/build-your-own-shell"),
            Some("shell".to_string())
        );
        assert_eq!(
            slug::challenge_from_url("https://github.com/example-org/other-repo"),
            None
        );
    }
}

