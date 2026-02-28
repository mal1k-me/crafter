// Container configuration - Parse codecrafters.yml and build container config

use crate::types::{CodecraftersConfig, CrafterError, Result};
use crate::utils::fs;
use crate::constants::project;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Container configuration used for tests.
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    // Project settings
    pub buildpack: String,
    pub debug: bool,

    // Paths
    pub project_dir: PathBuf,
    pub tester_dir: PathBuf,
    pub challenge_dir: PathBuf,
    pub dockerfile_path: PathBuf,

    // Docker settings
    pub image_name: String,
    pub network_mode: String,
    pub memory_limit: String,
    pub cpu_limit: String,
    pub cap_add: Vec<String>,
    pub environment_vars: HashMap<String, String>,

    // Test settings
    pub stage_slug: String,
}

impl ContainerConfig {
    /// Build container config from project state.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn from_project(
        project_dir: &Path,
        tester_dir: &Path,
        challenge_dir: &Path,
        stage_slug: &str,
    ) -> Result<Self> {
        // Parse codecrafters.yml
        let cc_config = parse_codecrafters_yml(project_dir)?;

        // Build dockerfile path
        let dockerfile_path = challenge_dir
            .join("dockerfiles")
            .join(format!("{}.Dockerfile", cc_config.buildpack));

        if !fs::exists(&dockerfile_path) {
            // Try to find available dockerfiles for this language
            let suggestion = match find_available_dockerfiles(challenge_dir, &cc_config.buildpack) {
                Ok(available) if !available.is_empty() => {
                    let mut msg = format!(
                        "Available {} buildpacks:\n",
                        extract_language(&cc_config.buildpack)
                    );
                    for bp in &available {
                        writeln!(&mut msg, "  - {bp}").expect("writing to String cannot fail");
                    }
                    if let Some(last) = available.last() {
                        write!(
                            &mut msg,
                            "\nUpdate codecrafters.yml with one of these:\n  buildpack: {last}"
                        )
                        .expect("writing to String cannot fail");
                    }
                    msg
                }
                _ => {
                    format!(
                        "Check available dockerfiles in:\n  {}/dockerfiles/",
                        crate::output::compat::format_path(challenge_dir)
                    )
                }
            };

            return Err(CrafterError::with_suggestion(
                format!(
                    "Dockerfile not found for buildpack '{}' at {}",
                    cc_config.buildpack,
                    crate::output::compat::format_path(&dockerfile_path)
                ),
                suggestion,
            ));
        }

        // Generate image name
        let challenge_name = challenge_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let image_name = format!("crafter-{}-{}", challenge_name, cc_config.buildpack);

        // Build environment variables (CodeCrafters convention)
        let mut env_vars = HashMap::new();
        env_vars.insert(
            "CODECRAFTERS_SUBMISSION_DIR".to_string(),
            "/app".to_string(),
        );
        env_vars.insert(
            "CODECRAFTERS_REPOSITORY_DIR".to_string(),
            "/app".to_string(),
        );
        env_vars.insert("TESTER_DIR".to_string(), "/tester".to_string());

        // Add test cases JSON (required by tester)
        let test_cases_json = format!(
            r#"[{{"slug":"{stage_slug}","tester_log_prefix":"tester::#{stage_slug}","title":"Stage {stage_slug}"}}]"#
        );
        env_vars.insert("CODECRAFTERS_TEST_CASES_JSON".to_string(), test_cases_json);

        Ok(Self {
            buildpack: cc_config.buildpack,
            debug: cc_config.debug,
            project_dir: project_dir.to_path_buf(),
            tester_dir: tester_dir.to_path_buf(),
            challenge_dir: challenge_dir.to_path_buf(),
            dockerfile_path,
            image_name,
            network_mode: "none".to_string(),
            memory_limit: "2g".to_string(),
            cpu_limit: "0.5".to_string(),
            cap_add: vec!["SYS_ADMIN".to_string()],
            environment_vars: env_vars,
            stage_slug: stage_slug.to_string(),
        })
    }
}

/// Parse `codecrafters.yml` from project directory.
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn parse_codecrafters_yml(project_dir: &Path) -> Result<CodecraftersConfig> {
    let yml_path = project_dir.join(project::CODECRAFTERS_YML);

    if !fs::exists(&yml_path) {
        return Err(CrafterError::other(
            project::MSG_YML_NOT_FOUND_IN_PROJECT_DIR,
        ));
    }

    let content = fs::read_to_string(&yml_path)?;
    let config: CodecraftersConfig = serde_yaml::from_str(&content)?;

    Ok(config)
}

/// Find available dockerfiles in challenge directory.
fn find_available_dockerfiles(challenge_dir: &Path, buildpack: &str) -> Result<Vec<String>> {
    let dockerfiles_dir = challenge_dir.join("dockerfiles");

    if !fs::exists(&dockerfiles_dir) {
        return Ok(Vec::new());
    }

    let language = extract_language(buildpack);
    let mut available = Vec::new();

    for entry in std::fs::read_dir(&dockerfiles_dir)? {
        let entry = entry?;
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        // Match pattern: {language}-{version}.Dockerfile
        if filename_str.starts_with(&format!("{language}-"))
            && filename_str.ends_with(".Dockerfile")
        {
            // Extract buildpack name (remove .Dockerfile extension)
            if let Some(bp) = filename_str.strip_suffix(".Dockerfile") {
                available.push(bp.to_string());
            }
        }
    }

    available.sort();
    Ok(available)
}

/// Extract language prefix from buildpack.
fn extract_language(buildpack: &str) -> String {
    // Split by '-' and take the first part
    buildpack.split('-').next().unwrap_or(buildpack).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_config_paths() {
        let project_dir = PathBuf::from("/tmp/project");
        let tester_dir = PathBuf::from("/tmp/tester");
        let challenge_dir = PathBuf::from("/tmp/challenge");

        // This will fail without actual files, but tests the path logic
        assert!(
            ContainerConfig::from_project(&project_dir, &tester_dir, &challenge_dir, "init")
                .is_err()
        );
    }
}
