//! Git process helpers.

use crate::types::{CrafterError, Result};
use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command as StdCommand, Output as ProcessOutput};
use tokio::process::Command;

/// Service for git command execution.
pub struct GitManager;

impl Default for GitManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GitManager {
    #[must_use] 
    pub const fn new() -> Self {
        Self
    }

    fn run_sync(&self, path: Option<&Path>, args: &[&OsStr]) -> Result<ProcessOutput> {
        let mut cmd = StdCommand::new("git");
        if let Some(path) = path {
            cmd.arg("-C").arg(path);
        }
        cmd.args(args);
        cmd.output()
            .map_err(|e| CrafterError::git(format!("Failed to execute git: {e}")))
    }

    async fn run_async(&self, args: &[&OsStr]) -> Result<ProcessOutput> {
        Command::new("git")
            .args(args)
            .output()
            .await
            .map_err(|e| CrafterError::git(format!("Failed to execute git: {e}")))
    }

    fn ensure_success(&self, output: ProcessOutput, failure_prefix: &str) -> Result<ProcessOutput> {
        if output.status.success() {
            return Ok(output);
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let reason = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("git exited with status {}", output.status)
        };

        Err(CrafterError::git(format!("{failure_prefix}: {reason}")))
    }

    /// Check if git is available
    #[must_use] 
    pub fn is_available(&self) -> bool {
        StdCommand::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    /// Clone a repository
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub async fn clone(&self, url: &str, dest: &Path) -> Result<()> {
        let output = self
            .run_async(&[OsStr::new("clone"), OsStr::new(url), dest.as_os_str()])
            .await?;
        self.ensure_success(output, "Git clone failed")?;

        Ok(())
    }

    /// Check if directory is a git repository
    #[must_use] 
    pub fn is_repo(&self, path: &Path) -> bool {
        self.run_sync(Some(path), &[OsStr::new("rev-parse"), OsStr::new("--git-dir")])
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Get current branch name
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn get_current_branch(&self, path: &Path) -> Result<String> {
        let output = self.run_sync(
            Some(path),
            &[
                OsStr::new("rev-parse"),
                OsStr::new("--abbrev-ref"),
                OsStr::new("HEAD"),
            ],
        )?;
        let output = self.ensure_success(output, "Failed to get current branch")?;

        let branch = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        Ok(branch)
    }

    /// Create a new branch
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn create_branch(&self, path: &Path, name: &str) -> Result<()> {
        let output = self.run_sync(Some(path), &[OsStr::new("branch"), OsStr::new(name)])?;
        self.ensure_success(output, "Failed to create branch")?;

        Ok(())
    }

    /// Checkout a branch
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn checkout_branch(&self, path: &Path, name: &str) -> Result<()> {
        let output = self.run_sync(Some(path), &[OsStr::new("checkout"), OsStr::new(name)])?;
        self.ensure_success(output, "Failed to checkout branch")?;

        Ok(())
    }

    /// Initialize a new git repository
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn init(&self, path: &Path) -> Result<()> {
        let output = self.run_sync(Some(path), &[OsStr::new("init")])?;
        self.ensure_success(output, "Git init failed")?;

        Ok(())
    }

    /// Add files to staging
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn add_all(&self, path: &Path) -> Result<()> {
        let output = self.run_sync(Some(path), &[OsStr::new("add"), OsStr::new(".")])?;
        self.ensure_success(output, "Git add failed")?;

        Ok(())
    }

    /// Create a commit
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn commit(&self, path: &Path, message: &str) -> Result<()> {
        let output = self.run_sync(
            Some(path),
            &[OsStr::new("commit"), OsStr::new("-m"), OsStr::new(message)],
        )?;
        self.ensure_success(output, "Git commit failed")?;

        Ok(())
    }

    /// Get git remote URL
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn get_remote_url(&self, path: &Path) -> Result<String> {
        let output = self.run_sync(
            Some(path),
            &[OsStr::new("remote"), OsStr::new("get-url"), OsStr::new("origin")],
        )?;
        let output = self.ensure_success(output, "Failed to get remote URL")?;

        let url = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        Ok(url)
    }

    /// Pull latest changes
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn pull(&self, path: &Path) -> Result<()> {
        let output = self.run_sync(Some(path), &[OsStr::new("pull")])?;
        self.ensure_success(output, "Git pull failed")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_available() {
        let git = GitManager::new();
        // Should have git on most dev machines
        assert!(git.is_available());
    }
}

