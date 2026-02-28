// Container service - Orchestrate Docker-based test execution

use crate::container::config::ContainerConfig;
use crate::container::docker::{DockerRunner, RunResult};
use crate::types::Result;

/// Orchestrates container-based test execution.
pub struct TestService {
    runner: DockerRunner,
}

impl TestService {
    #[must_use] 
    pub const fn new() -> Self {
        Self {
            runner: DockerRunner::new(),
        }
    }

    /// Check whether Docker is available.
    #[must_use] 
    pub fn is_available(&self) -> bool {
        self.runner.is_available()
    }

    /// Run a test in Docker (build image, then execute tester).
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn run_test(&self, config: &ContainerConfig) -> Result<RunResult> {
        // Step 1: Build image using CodeCrafters' dockerfile
        self.runner.build(config)?;

        // Step 2: Run tests using CodeCrafters' tester test.sh
        let result = self.runner.run(config)?;

        Ok(result)
    }

    /// Cleanup test artifacts.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn cleanup(&self, config: &ContainerConfig) -> Result<()> {
        self.runner.cleanup(config)
    }
}

impl Default for TestService {
    fn default() -> Self {
        Self::new()
    }
}
