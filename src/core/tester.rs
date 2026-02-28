//! Tester download and cache management.

use crate::core::config::ConfigManager;
use crate::types::{BuildOptions, CrafterError, GitHubRelease, Result, TesterInfo};
use crate::utils::{env, fs, slug};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

const GITHUB_API_USER_AGENT: &str = "crafter-cli";
const TEST_WRAPPER_SCRIPT: &str = "#!/bin/sh\nexec \"$(dirname \"$0\")/tester\" \"$@\"\n";

pub struct TesterManager {
    config_manager: Arc<ConfigManager>,
    cache: RwLock<HashMap<String, TesterInfo>>,
}

impl TesterManager {
    #[must_use] 
    pub fn new(config_manager: Arc<ConfigManager>) -> Self {
        Self {
            config_manager,
            cache: RwLock::new(HashMap::new()),
        }
    }

    fn fallback_testers_dir() -> PathBuf {
        if let Some(home) = home::home_dir() {
            home.join(".local").join("share").join("crafter").join("testers")
        } else {
            PathBuf::from(".local/share/crafter/testers")
        }
    }

    fn invalidate_cache(&self, challenge: &str) -> Result<()> {
        let mut cache = self
            .cache
            .write()
            .map_err(|e| CrafterError::other(format!("Cache lock poisoned: {e}")))?;
        cache.remove(challenge);
        Ok(())
    }

    /// Get tester directory for a challenge.
    pub fn get_tester_dir(&self, challenge: &str) -> PathBuf {
        env::testers_dir()
            .unwrap_or_else(|_| Self::fallback_testers_dir())
            .join(challenge)
    }

    /// Check whether tester binary and wrapper are present.
    pub fn is_available(&self, challenge: &str) -> bool {
        let dir = self.get_tester_dir(challenge);
        let tester_binary = dir.join("tester");
        let test_script = dir.join("test.sh");

        fs::exists(&tester_binary) && fs::exists(&test_script)
    }

    /// Ensure tester is available.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub async fn ensure_available(&self, challenge: &str) -> Result<()> {
        use crate::output::compat as output;

        if self.is_available(challenge) {
            return Ok(());
        }

        output::step(&format!("Ensuring {challenge} tester is available..."));

        self.build(challenge, BuildOptions::default()).await
    }

    /// Download or build tester.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub async fn build(&self, challenge: &str, opts: BuildOptions) -> Result<()> {
        use crate::output::compat as output;

        self.invalidate_cache(challenge)?;

        // Get tester URL from config
        let challenges_config = self.config_manager.get_challenges()?;
        let entry = challenges_config
            .find_challenge(challenge)
            .ok_or_else(|| CrafterError::ChallengeNotFound(challenge.to_string()))?;

        let tester_dir = self.get_tester_dir(challenge);

        // Clean if force rebuild
        if opts.force && fs::exists(&tester_dir) {
            std::fs::remove_dir_all(&tester_dir)?;
        }

        // Create tester directory only after challenge is validated
        fs::ensure_dir(&tester_dir)?;

        // Try to download prebuilt release
        output::step(&format!("Fetching latest release for {challenge} tester..."));

        match self.download_release(&entry.tester, &tester_dir, opts.version.as_deref()).await {
            Ok(version) => {
                output::success(&format!(
                    "Tester ready: {} ({})",
                    output::format_path(&tester_dir),
                    version
                ));
                self.invalidate_cache(challenge)?;
                Ok(())
            }
            Err(e) => {
                output::warn(&format!("Failed to download prebuilt tester: {e}"));
                output::info("Falling back to building from source...");
                Self::build_from_source(&entry.tester, &tester_dir)
            }
        }
    }

    /// Download prebuilt tester from GitHub Releases.
    async fn download_release(
        &self,
        tester_url: &str,
        dest_dir: &PathBuf,
        version: Option<&str>,
    ) -> Result<String> {
        use crate::output::compat as output;

        // Extract owner/repo from URL
        let (owner, repo) = Self::parse_github_url(tester_url)?;

        // Get latest release or specific version
        let release = if let Some(ver) = version {
            self.get_release(&owner, &repo, ver).await?
        } else {
            self.get_latest_release(&owner, &repo).await?
        };

        output::step(&format!("Downloading {} ({})", release.tag_name, Self::get_platform_name()));
        
        // Verbose: show release details
        if output::is_verbose() {
            output::verbose(&format!("Release URL: https://github.com/{}/{}/releases/tag/{}", owner, repo, release.tag_name));
            output::verbose(&format!("Destination: {}", output::format_path(dest_dir)));
        }

        // Find matching asset for current platform
        // Assets are named: v{version}_{os}_{arch}.tar.gz
        let (os, arch) = slug::get_platform();
        let asset_name = format!("{}_{}_{}.", release.tag_name, os, arch);

        let asset = release
            .assets
            .iter()
            .find(|a| a.name.contains(&asset_name))
            .ok_or_else(|| {
                CrafterError::tester(format!(
                    "No prebuilt tester for platform {}-{}. Available assets: {:?}",
                    os,
                    arch,
                    release.assets.iter().map(|a| &a.name).collect::<Vec<_>>()
                ))
            })?;
        
        // Verbose: show download details
        if output::is_verbose() {
            output::verbose(&format!("Asset name: {}", asset.name));
            output::verbose(&format!("Download URL: {}", asset.browser_download_url));
        }

        // Download asset
        let client = reqwest::Client::new();
        let response = client
            .get(&asset.browser_download_url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(CrafterError::tester(format!(
                "Failed to download tester: HTTP {}",
                response.status()
            )));
        }

        let bytes = response.bytes().await?;
        
        // Verbose: download complete
        if output::is_verbose() {
            output::verbose(&format!("Downloaded {} bytes", bytes.len()));
        }

        // Save to temporary tar.gz file
        let tar_path = dest_dir.join("tester.tar.gz");
        std::fs::write(&tar_path, bytes)?;

        // Extract tar.gz
        let tar_gz = std::fs::File::open(&tar_path)?;
        let tar = flate2::read::GzDecoder::new(tar_gz);
        let mut archive = tar::Archive::new(tar);
        archive.unpack(dest_dir)?;

        // Remove tar.gz
        std::fs::remove_file(&tar_path)?;

        // The tester binary should now be at dest_dir/tester
        let tester_path = dest_dir.join("tester");

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&tester_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&tester_path, perms)?;
        }

        // Create test.sh wrapper (CodeCrafters testers expect this)
        let test_script = dest_dir.join("test.sh");
        fs::write_string(&test_script, TEST_WRAPPER_SCRIPT)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&test_script)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&test_script, perms)?;
        }

        // Save version info
        let version_file = dest_dir.join(".version");
        fs::write_string(&version_file, &release.tag_name)?;

        Ok(release.tag_name)
    }

    /// Fallback source-build path.
    fn build_from_source(_tester_url: &str, _dest_dir: &PathBuf) -> Result<()> {
        // For now, return error - building from source requires Go toolchain
        Err(CrafterError::tester(
            "Building from source not yet implemented. Please ensure prebuilt binaries are available.",
        ))
    }

    /// Fetch latest GitHub release.
    async fn get_latest_release(&self, owner: &str, repo: &str) -> Result<GitHubRelease> {
        let url = format!(
            "https://api.github.com/repos/{owner}/{repo}/releases/latest"
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("User-Agent", GITHUB_API_USER_AGENT)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(CrafterError::tester(format!(
                "Failed to fetch release: HTTP {}",
                response.status()
            )));
        }

        let release: GitHubRelease = response.json().await?;
        Ok(release)
    }

    /// Fetch release by tag.
    async fn get_release(&self, owner: &str, repo: &str, tag: &str) -> Result<GitHubRelease> {
        let url = format!(
            "https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}"
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("User-Agent", GITHUB_API_USER_AGENT)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(CrafterError::tester(format!(
                "Failed to fetch release {}: HTTP {}",
                tag,
                response.status()
            )));
        }

        let release: GitHubRelease = response.json().await?;
        Ok(release)
    }

    /// Parse GitHub URL into owner/repo.
    fn parse_github_url(url: &str) -> Result<(String, String)> {
        // e.g., "https://github.com/codecrafters-io/shell-tester" -> ("codecrafters-io", "shell-tester")
        let normalized = url.trim_end_matches('/');
        let parts: Vec<&str> = normalized.split('/').collect();

        if parts.len() < 2 {
            return Err(CrafterError::other("Invalid GitHub URL"));
        }

        let repo = parts[parts.len() - 1]
            .strip_suffix(".git")
            .unwrap_or(parts[parts.len() - 1])
            .to_string();
        let owner = parts[parts.len() - 2].to_string();

        Ok((owner, repo))
    }

    /// Get platform display name.
    fn get_platform_name() -> String {
        let (os, arch) = slug::get_platform();
        format!("{os}/{arch}")
    }

    /// Get tester info.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn get_info(&self, challenge: &str) -> Result<TesterInfo> {
        // Check cache first
        {
            let cache = self
                .cache
                .read()
                .map_err(|e| CrafterError::other(format!("Cache lock poisoned: {e}")))?;
            if let Some(info) = cache.get(challenge) {
                return Ok(info.clone());
            }
        }

        // Build info
        let tester_dir = self.get_tester_dir(challenge);
        let tester_binary = tester_dir.join("tester");
        let test_script = tester_dir.join("test.sh");
        let version_file = tester_dir.join(".version");

        let version = if fs::exists(&version_file) {
            fs::read_to_string(&version_file)?
        } else {
            "unknown".to_string()
        };

        let info = TesterInfo {
            challenge: challenge.to_string(),
            version,
            path: tester_dir,
            tester_binary,
            test_script,
        };

        // Cache it
        {
            let mut cache = self
                .cache
                .write()
                .map_err(|e| CrafterError::other(format!("Cache lock poisoned: {e}")))?;
            cache.insert(challenge.to_string(), info.clone());
        }

        Ok(info)
    }

    /// Clean tester files and cache entry.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn clean(&self, challenge: &str) -> Result<()> {
        let tester_dir = self.get_tester_dir(challenge);

        if fs::exists(&tester_dir) {
            std::fs::remove_dir_all(&tester_dir)?;
        }

        // Remove from cache
        self.invalidate_cache(challenge)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_url() {
        let (owner, repo) = TesterManager::parse_github_url("https://github.com/codecrafters-io/shell-tester")
            .unwrap();
        assert_eq!(owner, "codecrafters-io");
        assert_eq!(repo, "shell-tester");
    }

    #[test]
    fn test_parse_github_url_git_suffix() {
        let (owner, repo) = TesterManager::parse_github_url("https://github.com/codecrafters-io/shell-tester.git")
            .unwrap();
        assert_eq!(owner, "codecrafters-io");
        assert_eq!(repo, "shell-tester");
    }
}

