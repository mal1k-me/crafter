//! Formatter for validation reports.

use crate::core::validator::{ValidationReport, ValidationSeverity};
use crate::output::formatter::Formatter;
use crate::output::primitives::{BracketedLine, Level, Section, SummaryBlock};
use std::io;
use termcolor::{Color, WriteColor};

/// Formats validation results with optional verbosity.
pub struct ValidationFormatter<'a> {
    report: &'a ValidationReport,
    verbose: bool,
}

impl<'a> ValidationFormatter<'a> {
    /// Create formatter.
    #[must_use] 
    pub const fn new(report: &'a ValidationReport) -> Self {
        Self {
            report,
            verbose: false,
        }
    }

    /// Show successful checks as well.
    #[must_use] 
    pub const fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Convert severity to output level.
    const fn severity_to_level(severity: &ValidationSeverity, passed: bool) -> Level {
        if passed {
            Level::Ok
        } else {
            match severity {
                ValidationSeverity::Error => Level::Error,
                ValidationSeverity::Warning => Level::Warn,
                ValidationSeverity::Info => Level::Info,
            }
        }
    }

    /// Build bracketed output line for a check.
    const fn create_line<'b>(
        level: Level,
        label: &'b str,
        message: &'b str,
        suggestion: Option<&'b str>,
    ) -> BracketedLine<'b> {
        let line = match level {
            Level::Ok => BracketedLine::ok(label, message),
            Level::Info => BracketedLine::info(label, message),
            Level::Warn => BracketedLine::warn(label, message),
            Level::Error => BracketedLine::error(label, message),
        };

        if let Some(sugg) = suggestion {
            line.with_suggestion(sugg)
        } else {
            line
        }
    }
}

impl Formatter for ValidationFormatter<'_> {
    fn format(&self, w: &mut dyn WriteColor) -> io::Result<()> {
        // Section header — Section::write adds its own leading + trailing blank
        Section::new("Validation Results").write(w)?;

        // Individual check results
        for result in &self.report.results {
            // Skip successful checks in non-verbose mode
            if result.passed && !self.verbose {
                continue;
            }

            let level = Self::severity_to_level(&result.severity, result.passed);

            Self::create_line(
                level,
                &result.check_name,
                &result.message,
                result.suggestion.as_deref(),
            )
            .write(w)?;
        }

        // Summary — SummaryBlock::write adds its own leading blank, item block, and trailing blank
        let passed = self.report.success_count();
        let warnings = self.report.warning_count();
        let errors = self.report.error_count();

        let mut summary = SummaryBlock::new("Validation Summary").add(
            "Passed",
            passed.to_string(),
            Some(Color::Green),
        );

        if warnings > 0 {
            summary = summary.add("Warnings", warnings.to_string(), Some(Color::Yellow));
        }

        if errors > 0 {
            summary = summary.add("Errors", errors.to_string(), Some(Color::Red));
        }

        summary.write(w)?;

        // Final status message
        if self.report.has_errors() {
            BracketedLine::error("Validation Failed", "Fix errors above before testing")
                .write(w)?;
        } else if self.report.has_warnings() {
            BracketedLine::warn("Validation Passed", "Some warnings present").write(w)?;
        } else {
            BracketedLine::ok("Success", "All validations passed").write(w)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::validator::ValidationResult;
    use termcolor::Buffer;

    #[test]
    fn test_validation_formatter_all_pass() {
        let report = ValidationReport {
            challenge: Some("redis".to_string()),
            results: vec![
                ValidationResult {
                    check_name: "Config exists".to_string(),
                    passed: true,
                    message: "crafter.toml found".to_string(),
                    suggestion: None,
                    severity: ValidationSeverity::Error,
                    fixable: false,
                },
                ValidationResult {
                    check_name: "Git repo".to_string(),
                    passed: true,
                    message: "Git repository initialized".to_string(),
                    suggestion: None,
                    severity: ValidationSeverity::Error,
                    fixable: false,
                },
            ],
        };

        let mut buffer = Buffer::no_color();
        ValidationFormatter::new(&report)
            .format(&mut buffer)
            .unwrap();

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("Validation Results"));
        assert!(output.contains("Summary"));
        assert!(output.contains("Passed"));
        assert!(output.contains("Success"));
        assert!(output.contains("Validation Results"));
        assert!(output.contains("Validation Summary"));
    }

    #[test]
    fn test_validation_formatter_snapshot_like_contract() {
        let report = ValidationReport {
            challenge: Some("redis".to_string()),
            results: vec![ValidationResult {
                check_name: "Config".to_string(),
                passed: false,
                message: "Missing codecrafters.yml".to_string(),
                suggestion: Some("Run crafter challenge init".to_string()),
                severity: ValidationSeverity::Error,
                fixable: false,
            }],
        };

        let mut buffer = Buffer::no_color();
        ValidationFormatter::new(&report)
            .format(&mut buffer)
            .unwrap();

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("[ERR]"));
        assert!(output.contains("Config - Missing codecrafters.yml"));
        assert!(output.contains("Run crafter challenge init"));
        assert!(output.contains("Validation Failed"));
    }

    #[test]
    fn test_validation_formatter_with_errors() {
        let report = ValidationReport {
            challenge: Some("git".to_string()),
            results: vec![
                ValidationResult {
                    check_name: "Config exists".to_string(),
                    passed: false,
                    message: "crafter.toml not found".to_string(),
                    suggestion: Some("Run 'crafter init' to create config".to_string()),
                    severity: ValidationSeverity::Error,
                    fixable: true,
                },
                ValidationResult {
                    check_name: "Git repo".to_string(),
                    passed: true,
                    message: "Git repository initialized".to_string(),
                    suggestion: None,
                    severity: ValidationSeverity::Error,
                    fixable: false,
                },
            ],
        };

        let mut buffer = Buffer::no_color();
        ValidationFormatter::new(&report)
            .format(&mut buffer)
            .unwrap();

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("Validation Results"));
        assert!(output.contains("Config exists"));
        assert!(output.contains("crafter.toml not found"));
        assert!(output.contains("Run 'crafter init'"));
        assert!(output.contains("Errors"));
        assert!(output.contains("Validation Failed"));
    }

    #[test]
    fn test_validation_formatter_verbose_mode() {
        let report = ValidationReport {
            challenge: Some("redis".to_string()),
            results: vec![
                ValidationResult {
                    check_name: "Check 1".to_string(),
                    passed: true,
                    message: "All good".to_string(),
                    suggestion: None,
                    severity: ValidationSeverity::Error,
                    fixable: false,
                },
                ValidationResult {
                    check_name: "Check 2".to_string(),
                    passed: true,
                    message: "Also good".to_string(),
                    suggestion: None,
                    severity: ValidationSeverity::Error,
                    fixable: false,
                },
            ],
        };

        // Non-verbose: should skip successful checks
        let mut buffer = Buffer::no_color();
        ValidationFormatter::new(&report)
            .format(&mut buffer)
            .unwrap();
        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(!output.contains("Check 1"));
        assert!(!output.contains("Check 2"));

        // Verbose: should show all checks
        let mut buffer = Buffer::no_color();
        ValidationFormatter::new(&report)
            .with_verbose(true)
            .format(&mut buffer)
            .unwrap();
        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("Check 1"));
        assert!(output.contains("Check 2"));
    }

    #[test]
    fn test_validation_formatter_with_warnings() {
        let report = ValidationReport {
            challenge: Some("http-server".to_string()),
            results: vec![
                ValidationResult {
                    check_name: "Config valid".to_string(),
                    passed: true,
                    message: "Configuration is valid".to_string(),
                    suggestion: None,
                    severity: ValidationSeverity::Error,
                    fixable: false,
                },
                ValidationResult {
                    check_name: "README exists".to_string(),
                    passed: false,
                    message: "README.md not found".to_string(),
                    suggestion: Some("Consider adding a README".to_string()),
                    severity: ValidationSeverity::Warning,
                    fixable: false,
                },
            ],
        };

        let mut buffer = Buffer::no_color();
        ValidationFormatter::new(&report)
            .format(&mut buffer)
            .unwrap();

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("README exists"));
        assert!(output.contains("Warnings"));
        assert!(output.contains("Validation Passed"));
        assert!(!output.contains("Validation Failed"));
    }
}
