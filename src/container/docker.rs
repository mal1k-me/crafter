//! Docker command runner for build and test execution.

use crate::container::config::ContainerConfig;
use crate::types::{CrafterError, Result};
use std::ffi::OsStr;
use std::process::Command;
use std::time::{Duration, Instant};

/// Result of a container run.
#[derive(Debug)]
pub struct RunResult {
    pub exit_code: i32,
    pub output: String,
    pub duration: Duration,
}

/// Docker runner for build/run lifecycle.
pub struct DockerRunner;

impl Default for DockerRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl DockerRunner {
    #[must_use] 
    pub const fn new() -> Self {
        Self
    }

    fn run_docker<I, S>(&self, args: I) -> Result<std::process::Output>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Command::new("docker")
            .args(args)
            .output()
            .map_err(|e| CrafterError::docker(format!("Failed to execute docker: {e}")))
    }

    fn run_docker_in_dir<I, S>(
        &self,
        current_dir: &std::path::Path,
        args: I,
    ) -> Result<std::process::Output>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Command::new("docker")
            .current_dir(current_dir)
            .args(args)
            .output()
            .map_err(|e| CrafterError::docker(format!("Failed to execute docker: {e}")))
    }

    fn failure_details(prefix: &str, output: &std::process::Output) -> CrafterError {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

        let reason = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("docker exited with status {}", output.status)
        };

        CrafterError::docker(format!("{prefix}: {reason}"))
    }

    /// Check whether Docker is available.
    #[must_use] 
    pub fn is_available(&self) -> bool {
        self.run_docker(["version"])
            .is_ok_and(|output| output.status.success())
    }

    /// Build Docker image from configured Dockerfile.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn build(&self, config: &ContainerConfig) -> Result<()> {
        use crate::output::compat as output;

        output::step(&format!(
            "Building container from {}.Dockerfile...",
            config.buildpack
        ));

        // Verbose: show full docker build command
        if output::is_verbose() {
            let cmd = format!(
                "docker build -f {} -t {} {}",
                config.dockerfile_path.display(),
                config.image_name,
                config.project_dir.display()
            );
            output::verbose(&format!("Running: {cmd}"));
            output::verbose(&format!(
                "Dockerfile: {}",
                output::format_path(&config.dockerfile_path)
            ));
            output::verbose(&format!(
                "Project dir: {}",
                output::format_path(&config.project_dir)
            ));
        }

        let output_result = self.run_docker_in_dir(
            &config.project_dir,
            [
                OsStr::new("build"),
                OsStr::new("-f"),
                config.dockerfile_path.as_os_str(),
                OsStr::new("-t"),
                OsStr::new(&config.image_name),
                config.project_dir.as_os_str(),
            ],
        )?;

        if !output_result.status.success() {
            return Err(Self::failure_details("Docker build failed", &output_result));
        }

        Ok(())
    }

    /// Run tester inside Docker container.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn run(&self, config: &ContainerConfig) -> Result<RunResult> {
        use crate::output::compat as output;

        output::step("Running tests in isolated container (network disabled)...");

        let args = Self::build_run_args(config);

        // Verbose: show full docker run command
        if output::is_verbose() {
            let cmd = format!("docker {}", args.join(" "));
            output::verbose(&format!("Running: {cmd}"));
            output::verbose(&format!(
                "Project dir: {}",
                output::format_path(&config.project_dir)
            ));
            output::verbose(&format!(
                "Tester dir: {}",
                output::format_path(&config.tester_dir)
            ));
            output::verbose(&format!(
                "Dockerfile: {}",
                output::format_path(&config.dockerfile_path)
            ));
        }

        let start_time = Instant::now();
        let output_result = self.run_docker(&args)?;

        let duration = start_time.elapsed();

        let combined_output = format!(
            "{}{}",
            String::from_utf8_lossy(&output_result.stdout),
            String::from_utf8_lossy(&output_result.stderr)
        );

        let exit_code = output_result.status.code().unwrap_or(-1);

        Ok(RunResult {
            exit_code,
            output: combined_output,
            duration,
        })
    }

    /// Build docker run arguments.
    fn build_run_args(config: &ContainerConfig) -> Vec<String> {
        let mut args = vec!["run".to_string(), "--rm".to_string()];

        // Network isolation (security)
        args.push(format!("--network={}", config.network_mode));

        // Resource limits
        args.push(format!("--memory={}", config.memory_limit));
        args.push(format!("--cpus={}", config.cpu_limit));

        // Capabilities (needed for some challenges)
        for cap in &config.cap_add {
            args.push("--cap-add".to_string());
            args.push(cap.clone());
        }

        // Volume mounts - mount user's code and CodeCrafters' tester
        args.push("-v".to_string());
        args.push(format!("{}:/app", config.project_dir.display()));
        args.push("-v".to_string());
        args.push(format!("{}:/tester:ro", config.tester_dir.display()));

        // Environment variables expected by CodeCrafters' tester
        for (key, value) in &config.environment_vars {
            args.push("-e".to_string());
            args.push(format!("{key}={value}"));
        }

        // Working directory
        args.push("-w".to_string());
        args.push("/app".to_string());

        // Image name
        args.push(config.image_name.clone());

        // Command: Run CodeCrafters' tester test.sh script
        args.push("/tester/test.sh".to_string());

        args
    }

    /// Remove Docker image.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn cleanup(&self, config: &ContainerConfig) -> Result<()> {
        let _ = self.run_docker(["rmi", "-f", &config.image_name]);

        // Ignore errors - image might not exist or be in use
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_available() {
        let runner = DockerRunner::new();
        // This might fail in CI/CD without Docker
        // Just test that the function runs
        let _ = runner.is_available();
    }
}
