//! Project validation checks.

use crate::core::challenge::ChallengeManager;
use crate::core::detector::ChallengeDetector;
use crate::core::tester::TesterManager;
use crate::constants::project;
use crate::types::Result;
use crate::utils::fs;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;

const KNOWN_BROKEN_BUILDPACKS: &[(&str, &str)] = &[
    ("zig-0.14", "Missing curl - use zig-0.13 or fix Dockerfile"),
    ("zig-0.15", "Missing curl - use zig-0.13 or fix Dockerfile"),
    (
        "zig-0.15.2",
        "Missing curl - use zig-0.13 or fix Dockerfile",
    ),
];

/// Result for a single validation check.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub check_name: String,
    pub passed: bool,
    pub message: String,
    pub suggestion: Option<String>,
    pub severity: ValidationSeverity,
    pub fixable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationSeverity {
    Error,   // Must fix before testing
    Warning, // Should fix but not blocking
    Info,    // Informational only
}

impl ValidationResult {
    #[must_use] 
    pub fn success(check_name: &str, message: &str) -> Self {
        Self {
            check_name: check_name.to_string(),
            passed: true,
            message: message.to_string(),
            suggestion: None,
            severity: ValidationSeverity::Info,
            fixable: false,
        }
    }

    #[must_use] 
    pub fn error(check_name: &str, message: &str) -> Self {
        Self {
            check_name: check_name.to_string(),
            passed: false,
            message: message.to_string(),
            suggestion: None,
            severity: ValidationSeverity::Error,
            fixable: false,
        }
    }

    #[must_use] 
    pub fn warning(check_name: &str, message: &str) -> Self {
        Self {
            check_name: check_name.to_string(),
            passed: false,
            message: message.to_string(),
            suggestion: None,
            severity: ValidationSeverity::Warning,
            fixable: false,
        }
    }

    #[must_use] 
    pub fn with_suggestion(mut self, suggestion: &str) -> Self {
        self.suggestion = Some(suggestion.to_string());
        self
    }

    #[must_use] 
    pub const fn with_fixable(mut self, fixable: bool) -> Self {
        self.fixable = fixable;
        self
    }
}

/// Project validator.
pub struct Validator {
    challenge_mgr: Arc<ChallengeManager>,
    tester_mgr: Arc<TesterManager>,
    detector: Arc<ChallengeDetector>,
}

impl Validator {
        fn read_project_config(project_dir: &Path) -> Option<crate::types::CodecraftersConfig> {
            let yml_path = project_dir.join(project::CODECRAFTERS_YML);
            fs::read_to_string(&yml_path)
                .ok()
                .and_then(|content| serde_yaml::from_str::<crate::types::CodecraftersConfig>(&content).ok())
        }

    pub const fn new(
        challenge_mgr: Arc<ChallengeManager>,
        tester_mgr: Arc<TesterManager>,
        detector: Arc<ChallengeDetector>,
    ) -> Self {
        Self {
            challenge_mgr,
            tester_mgr,
            detector,
        }
    }

    /// Run all validation checks.
    /// # Errors
    /// Returns an error if the underlying operation fails.
    pub fn validate_all(&self, project_dir: &Path, verbose: bool) -> Result<ValidationReport> {
        let mut results = Vec::new();

        if verbose {
            // Caller is responsible for emitting "Running validation checks..." progress
        }

        // Essential checks
        results.push(Self::validate_codecrafters_yml(project_dir));
        results.push(Self::validate_project_structure(project_dir));

        // Challenge detection (needed for other checks)
        let challenge = match self.detector.detect(project_dir) {
            Ok(ch) => {
                results.push(ValidationResult::success(
                    "Challenge Detection",
                    &format!("Detected challenge: {ch}"),
                ));
                Some(ch)
            }
            Err(e) => {
                results.push(
                    ValidationResult::error("Challenge Detection", &format!("Failed to detect challenge: {e}"))
                        .with_suggestion("Run 'crafter challenge list' to see available challenges\nEnsure you're in a CodeCrafters project directory"),
                );
                None
            }
        };

        // Continue with checks that need challenge info
        if let Some(ref challenge) = challenge {
            results.push(self.validate_buildpack(project_dir, challenge));
            results.push(self.validate_tester(challenge));
            results.push(self.validate_buildpack_version(project_dir, challenge));
            results.push(Self::validate_known_issues(project_dir, challenge));
        }

        // Environment checks
        results.push(Self::validate_docker());
        results.push(Self::validate_git(project_dir));

        Ok(ValidationReport { results, challenge })
    }

    /// Validate `codecrafters.yml`.
    fn validate_codecrafters_yml(project_dir: &Path) -> ValidationResult {
        let yml_path = project_dir.join(project::CODECRAFTERS_YML);

        if !fs::exists(&yml_path) {
            return ValidationResult::error(
                project::CODECRAFTERS_YML,
                "codecrafters.yml not found",
            )
            .with_suggestion(
                "This doesn't appear to be a CodeCrafters project.\nRun: crafter challenge init <challenge> <language>",
            );
        }

        // Try to parse it
        match fs::read_to_string(&yml_path) {
            Ok(content) => {
                match serde_yaml::from_str::<crate::types::CodecraftersConfig>(&content) {
                    Ok(_) => {
                        ValidationResult::success(project::CODECRAFTERS_YML, "codecrafters.yml is valid")
                    }
                    Err(e) => ValidationResult::error(
                        project::CODECRAFTERS_YML,
                        &format!("codecrafters.yml is invalid YAML: {e}"),
                    )
                    .with_suggestion("Check YAML syntax and ensure 'buildpack' field is present"),
                }
            }
            Err(e) => ValidationResult::error(
                project::CODECRAFTERS_YML,
                &format!("Cannot read codecrafters.yml: {e}"),
            ),
        }
    }

    /// Validate project directory structure.
    fn validate_project_structure(project_dir: &Path) -> ValidationResult {
        let codecrafters_dir = project_dir.join(".codecrafters");

        if !fs::exists(&codecrafters_dir) {
            return ValidationResult::warning(
                "Project Structure",
                ".codecrafters/ directory not found",
            )
            .with_suggestion("This may cause issues during testing.\nEnsure you initialized the project with 'crafter challenge init'");
        }

        // Check for compile.sh
        let compile_sh = codecrafters_dir.join("compile.sh");
        if !fs::exists(&compile_sh) {
            return ValidationResult::warning(
                "Project Structure",
                ".codecrafters/compile.sh not found",
            );
        }

        ValidationResult::success("Project Structure", "Project structure looks good")
    }

    /// Validate buildpack dockerfile existence.
    fn validate_buildpack(&self, project_dir: &Path, challenge: &str) -> ValidationResult {
        let Some(config) = Self::read_project_config(project_dir) else {
            return ValidationResult::error(
                "Buildpack",
                "Cannot read buildpack from codecrafters.yml",
            );
        };
        let buildpack = config.buildpack;

        let Ok(challenge_dir) = self.challenge_mgr.get_challenge_dir(challenge) else {
            return ValidationResult::error(
                "Buildpack",
                &format!("Challenge '{challenge}' not downloaded"),
            )
            .with_suggestion(&format!(
                "Run: crafter challenge init {challenge} <language>"
            ))
            .with_fixable(true);
        };

        let dockerfile = challenge_dir
            .join("dockerfiles")
            .join(format!("{buildpack}.Dockerfile"));

        if !fs::exists(&dockerfile) {
            // Find available dockerfiles
            let suggestion = match Self::find_available_dockerfiles(&challenge_dir, &buildpack) {
                Ok(available) if !available.is_empty() => {
                    let mut msg = format!(
                        "Dockerfile not found for buildpack '{buildpack}'\n\nAvailable buildpacks:\n"
                    );
                    for bp in &available {
                        writeln!(&mut msg, "  - {bp}").expect("writing to String cannot fail");
                    }
                    let Some(last) = available.last() else {
                        return ValidationResult::error("Buildpack", &msg).with_fixable(true);
                    };
                    write!(&mut msg, "\nUpdate codecrafters.yml:\n  buildpack: {last}")
                        .expect("writing to String cannot fail");
                    msg
                }
                _ => format!("Dockerfile not found for buildpack '{buildpack}'"),
            };

            return ValidationResult::error("Buildpack", &suggestion).with_fixable(true);
        }

        ValidationResult::success(
            "Buildpack",
            &format!("Buildpack '{buildpack}' dockerfile available"),
        )
    }

    /// Validate tester availability.
    fn validate_tester(&self, challenge: &str) -> ValidationResult {
        match self.tester_mgr.get_info(challenge) {
            Ok(info) => {
                // Check if tester binary exists
                let tester_binary = info.path.join("tester");
                let test_wrapper = info.path.join("test.sh");

                if !fs::exists(&tester_binary) {
                    return ValidationResult::error(
                        "Tester",
                        &format!("Tester binary not found for '{challenge}'"),
                    )
                    .with_suggestion(&format!("Run: crafter tester build {challenge}"))
                    .with_fixable(true);
                }

                if !fs::exists(&test_wrapper) {
                    return ValidationResult::warning(
                        "Tester",
                        "Tester wrapper (test.sh) not found",
                    )
                    .with_suggestion(&format!("Run: crafter tester build {challenge} --force"));
                }

                ValidationResult::success("Tester", &format!("Tester ready ({})", info.version))
            }
            Err(_) => ValidationResult::error(
                "Tester",
                &format!("Tester not available for '{challenge}'"),
            )
            .with_suggestion(&format!("Run: crafter tester build {challenge}"))
            .with_fixable(true),
        }
    }

    /// Validate Docker availability and daemon state.
    fn validate_docker() -> ValidationResult {
        use std::process::Command;

        // Check if docker command exists
        let docker_check = Command::new("docker").arg("--version").output();

        match docker_check {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                let version_str = version.trim().replace("Docker version ", "");

                // Check if daemon is running
                let daemon_check = Command::new("docker").arg("ps").output();

                match daemon_check {
                    Ok(output) if output.status.success() => ValidationResult::success(
                        "Docker",
                        &format!(
                            "Docker available ({})",
                            version_str.lines().next().unwrap_or("")
                        ),
                    ),
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        if stderr.contains("permission denied")
                            || stderr.contains("Permission denied")
                        {
                            ValidationResult::error("Docker", "Docker permission denied")
                                .with_suggestion("Add your user to docker group:\n  sudo usermod -aG docker $USER\n  Then log out and back in")
                        } else {
                            ValidationResult::error("Docker", "Docker daemon not running")
                                .with_suggestion("Start Docker:\n  sudo systemctl start docker\n  Or start Docker Desktop")
                        }
                    }
                    Err(_) => ValidationResult::error("Docker", "Cannot connect to Docker daemon")
                        .with_suggestion(
                            "Ensure Docker is running:\n  sudo systemctl start docker",
                        ),
                }
            }
            Ok(_) => ValidationResult::error("Docker", "Docker command failed"),
            Err(_) => ValidationResult::error("Docker", "Docker not installed")
                .with_suggestion("Install Docker:\n  https://docs.docker.com/get-docker/"),
        }
    }

    /// Validate git repository status.
    fn validate_git(project_dir: &Path) -> ValidationResult {
        use std::process::Command;

        // Check if git exists
        let git_check = Command::new("git").arg("--version").output();

        if git_check.is_err() {
            return ValidationResult::warning("Git", "Git not installed").with_suggestion(
                "Install git for version control:\n  https://git-scm.com/downloads",
            );
        }

        // Check if this is a git repo
        let is_repo = Command::new("git")
            .arg("rev-parse")
            .arg("--git-dir")
            .current_dir(project_dir)
            .output();

        match is_repo {
            Ok(output) if output.status.success() => {
                // Get current branch
                let branch_output = Command::new("git")
                    .arg("branch")
                    .arg("--show-current")
                    .current_dir(project_dir)
                    .output();

                if let Ok(output) = branch_output {
                    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if branch == "main" || branch == "master" {
                        return ValidationResult::warning(
                            "Git",
                            &format!("Working on '{branch}' branch"),
                        )
                        .with_suggestion(
                            "Consider creating a 'crafter' branch:\n  git checkout -b crafter",
                        );
                    }

                    return ValidationResult::success(
                        "Git",
                        &format!("Git repository (branch: {branch})"),
                    );
                }

                ValidationResult::success("Git", "Git repository initialized")
            }
            _ => ValidationResult::warning("Git", "Not a git repository").with_suggestion(
                "Initialize git:\n  git init\n  git add .\n  git commit -m 'init'",
            ),
        }
    }

    /// Check whether buildpack is latest available.
    fn validate_buildpack_version(&self, project_dir: &Path, challenge: &str) -> ValidationResult {
        let Some(config) = Self::read_project_config(project_dir) else {
            return ValidationResult::success("Buildpack Version", "Skipped");
        };
        let buildpack = config.buildpack;

        let Ok(challenge_dir) = self.challenge_mgr.get_challenge_dir(challenge) else {
            return ValidationResult::success("Buildpack Version", "Skipped");
        };

        match Self::find_available_dockerfiles(&challenge_dir, &buildpack) {
            Ok(available) if !available.is_empty() => {
                let Some(latest) = available.last() else {
                    return ValidationResult::success(
                        "Buildpack Version",
                        "Cannot determine latest version",
                    );
                };

                if latest != &buildpack {
                    return ValidationResult::warning(
                        "Buildpack Version",
                        &format!("Using '{buildpack}' (latest: '{latest}')"),
                    )
                    .with_suggestion(&format!("Update to latest:\n  buildpack: {latest}"))
                    .with_fixable(true);
                }
                ValidationResult::success(
                    "Buildpack Version",
                    &format!("Using latest ({buildpack})"),
                )
            }
            _ => ValidationResult::success("Buildpack Version", "Cannot determine latest version"),
        }
    }

    /// Check known buildpack issues.
    fn validate_known_issues(project_dir: &Path, _challenge: &str) -> ValidationResult {
        let buildpack = match Self::read_project_config(project_dir) {
            Some(config) => config.buildpack,
            None => return ValidationResult::success("Known Issues", "No known issues"),
        };

        for (broken_bp, issue) in KNOWN_BROKEN_BUILDPACKS {
            if buildpack == *broken_bp {
                return ValidationResult::warning(
                    "Known Issues",
                    &format!("Buildpack '{buildpack}' has known issues"),
                )
                .with_suggestion(issue);
            }
        }

        ValidationResult::success("Known Issues", "No known issues detected")
    }

    /// Find available dockerfiles for a buildpack.
    fn find_available_dockerfiles(challenge_dir: &Path, buildpack: &str) -> Result<Vec<String>> {
        let dockerfiles_dir = challenge_dir.join("dockerfiles");

        if !fs::exists(&dockerfiles_dir) {
            return Ok(Vec::new());
        }

        let language = buildpack.split('-').next().unwrap_or(buildpack);
        let mut available = Vec::new();

        for entry in std::fs::read_dir(&dockerfiles_dir)? {
            let entry = entry?;
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();

            if filename_str.starts_with(&format!("{language}-"))
                && filename_str.ends_with(".Dockerfile")
            {
                if let Some(bp) = filename_str.strip_suffix(".Dockerfile") {
                    available.push(bp.to_string());
                }
            }
        }

        // Sort buildpacks semantically by version number
        available.sort_by(|a, b| {
            // Extract version part after the language prefix (e.g., "c-23" -> "23")
            let ver_a = a.split('-').nth(1).unwrap_or("0");
            let ver_b = b.split('-').nth(1).unwrap_or("0");

            // Try to parse as numbers for proper version comparison
            // Handle versions like "9.2" vs "23"
            let parse_version = |v: &str| -> Vec<u32> {
                v.split('.').filter_map(|s| s.parse::<u32>().ok()).collect()
            };

            let parts_a = parse_version(ver_a);
            let parts_b = parse_version(ver_b);

            // Compare version parts
            for (pa, pb) in parts_a.iter().zip(parts_b.iter()) {
                if pa != pb {
                    return pa.cmp(pb);
                }
            }

            // If all compared parts are equal, longer version is greater (1.0.1 > 1.0)
            parts_a.len().cmp(&parts_b.len())
        });

        Ok(available)
    }
}

/// Validation report.
pub struct ValidationReport {
    pub results: Vec<ValidationResult>,
    pub challenge: Option<String>,
}

impl ValidationReport {
    #[must_use] 
    pub fn has_errors(&self) -> bool {
        self.results
            .iter()
            .any(|r| !r.passed && r.severity == ValidationSeverity::Error)
    }

    #[must_use] 
    pub fn has_warnings(&self) -> bool {
        self.results
            .iter()
            .any(|r| !r.passed && r.severity == ValidationSeverity::Warning)
    }

    #[must_use] 
    pub fn error_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| !r.passed && r.severity == ValidationSeverity::Error)
            .count()
    }

    #[must_use] 
    pub fn warning_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| !r.passed && r.severity == ValidationSeverity::Warning)
            .count()
    }

    #[must_use] 
    pub fn success_count(&self) -> usize {
        self.results.iter().filter(|r| r.passed).count()
    }

    /// Display validation results.
    pub fn display(&self, verbose: bool) {
        use crate::output::formatter::Formatter;
        use crate::output::formatters::ValidationFormatter;

        let mut stdout = crate::output::compat::stdout();
        let formatter = ValidationFormatter::new(self).with_verbose(verbose);

        if let Err(e) = formatter.format(&mut stdout) {
            eprintln!("Error formatting validation results: {e}");
        }
    }

    /// Convert to JSON-serializable output.
    #[must_use] 
    pub fn to_output(&self) -> crate::types::ValidationReportOutput {
        use crate::types::ValidationCheckOutput;

        crate::types::ValidationReportOutput {
            challenge: self.challenge.clone(),
            passed: !self.has_errors(),
            error_count: self.error_count(),
            warning_count: self.warning_count(),
            success_count: self.success_count(),
            checks: self
                .results
                .iter()
                .map(|r| ValidationCheckOutput {
                    name: r.check_name.clone(),
                    passed: r.passed,
                    severity: match r.severity {
                        ValidationSeverity::Error => "error".to_string(),
                        ValidationSeverity::Warning => "warning".to_string(),
                        ValidationSeverity::Info => "info".to_string(),
                    },
                    message: r.message.clone(),
                    suggestion: r.suggestion.clone(),
                    fixable: r.fixable,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ValidationReport counters and outputs ---

    fn make_report(results: Vec<ValidationResult>) -> ValidationReport {
        ValidationReport {
            results,
            challenge: None,
        }
    }

    #[test]
    fn q_a_empty_report_has_zero_counts() {
        let rep = make_report(vec![]);
        assert!(!rep.has_errors());
        assert!(!rep.has_warnings());
        assert_eq!(rep.error_count(), 0);
        assert_eq!(rep.warning_count(), 0);
        assert_eq!(rep.success_count(), 0);
    }

    #[test]
    fn q_a_report_counts_by_severity() {
        let rep = make_report(vec![
            ValidationResult::error("A", "err1"),
            ValidationResult::warning("B", "warn1"),
            ValidationResult::success("C", "ok"),
        ]);

        assert!(rep.has_errors());
        assert!(rep.has_warnings());
        assert_eq!(rep.error_count(), 1);
        assert_eq!(rep.warning_count(), 1);
        assert_eq!(rep.success_count(), 1);
    }

    #[test]
    fn q_a_report_output_shape_contains_expected_fields() {
        let rep = ValidationReport {
            results: vec![ValidationResult::error("A", "err1").with_fixable(true)],
            challenge: Some("shell".to_string()),
        };

        let out = rep.to_output();
        assert_eq!(out.challenge, Some("shell".to_string()));
        assert_eq!(out.error_count, 1);
        assert_eq!(out.warning_count, 0);
        assert_eq!(out.success_count, 0);
        assert_eq!(out.checks.len(), 1);
        assert!(out.checks[0].fixable);
        assert_eq!(out.checks[0].severity, "error");
    }

    // --- find_available_dockerfiles logic (via filesystem helpers) ---
    // We replicate the scanning logic here since the method is private,
    // verifying it through CodecraftersConfig::detect_latest_buildpack behaviour.

    #[test]
    fn q_a_dockerfile_scan_finds_latest_matching_buildpack() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let dockerfiles = dir.path().join("dockerfiles");
        fs::create_dir_all(&dockerfiles).unwrap();
        fs::write(dockerfiles.join("rust-1.80.Dockerfile"), "").unwrap();
        fs::write(dockerfiles.join("rust-1.92.Dockerfile"), "").unwrap();
        fs::write(dockerfiles.join("go-1.23.Dockerfile"), "").unwrap(); // should be ignored

        // Use the public CodecraftersConfig path that exercises the same scan
        let bp = crate::types::CodecraftersConfig::default_buildpack("rust", Some(dir.path()));
        assert_eq!(bp, "rust-1.92");
    }

    #[test]
    fn q_a_dockerfile_scan_falls_back_to_latest_suffix_when_missing() {
        use std::path::Path;
        let bp = crate::types::CodecraftersConfig::default_buildpack(
            "haskell",
            Some(Path::new("/tmp/__crafter_no_such_dir_xyz")),
        );
        assert_eq!(bp, "haskell-latest");
    }
}
