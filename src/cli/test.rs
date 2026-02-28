use crafter::container::{ContainerConfig, TestService};
use crafter::core::detector::ChallengeDetector;
use crafter::core::validator::Validator;
use crafter::output::compat as output;
use crafter::types::output::{TestAllStagesOutput, TestRunOutput, TestStageRunOutput};
use crafter::utils::env;
use std::sync::Arc;

pub(crate) fn emit_validation_report(
    report: &crafter::core::validator::ValidationReport,
    all_checks: bool,
) -> crafter::types::Result<()> {
    use crafter::output::compat;
    if compat::is_json() {
        compat::emit_json(&report.to_output())?;
    } else if !compat::is_quiet() {
        report.display(all_checks);
    }
    Ok(())
}

fn format_test_output(output: &str) -> String {
    match crafter::output::compat::get_format() {
        crafter::output::Format::Human => output.to_owned(),
        _ => strip_ansi(output),
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for ch in chars.by_ref() {
                if ch.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[allow(clippy::too_many_lines)]
pub async fn handle_test(
    stage: Option<String>,
    all: bool,
    continue_on_failure: bool,
    skip_validation: bool,
) -> crafter::types::Result<()> {
    let project_dir = env::current_dir()?;

    let ctx = super::context::build_cli_context()?;
    let git_mgr = ctx.git_mgr;
    let challenge_mgr = ctx.challenge_mgr;
    let tester_mgr = ctx.tester_mgr;

    let detector = Arc::new(ChallengeDetector::new(git_mgr.clone()));
    let test_service = TestService::new();

    if !skip_validation {
        output::step("Validating project configuration...");

        let validator = Validator::new(challenge_mgr.clone(), tester_mgr.clone(), detector.clone());
        let report = validator.validate_all(&project_dir, false)?;

        if report.has_errors() {
            output::error("Validation failed - fix errors before testing");
            emit_validation_report(&report, false)?;
            return Err(crafter::types::CrafterError::other("Validation failed"));
        }

        if report.has_warnings() {
            output::warn("Validation passed with warnings");
            for result in &report.results {
                if !result.passed
                    && result.severity == crafter::core::validator::ValidationSeverity::Warning
                {
                    output::warn(&format!("{}: {}", result.check_name, result.message));
                }
            }
        }
    }

    if !test_service.is_available() {
        output::error("Docker is not available");
        output::suggestion(
            "Please install Docker and ensure it's running:\n  https://docs.docker.com/get-docker/",
        );
        return Err(crafter::types::CrafterError::docker("Docker not available"));
    }

    output::step("Detecting challenge from project directory...");
    let challenge = detector.detect(&project_dir)?;
    output::detail(&format!("Detected challenge: {challenge}"));

    tester_mgr.ensure_available(&challenge).await?;

    if !challenge_mgr.is_downloaded(&challenge) {
        output::step(&format!("Downloading {challenge} challenge repository..."));
        challenge_mgr.download(&challenge).await?;
    }

    let tester_info = tester_mgr.get_info(&challenge)?;
    let challenge_dir = challenge_mgr.get_challenge_dir(&challenge)?;

    if all {
        return run_all_stages(
            &project_dir,
            &challenge,
            challenge_mgr.as_ref(),
            &tester_info.path,
            &challenge_dir,
            &test_service,
            continue_on_failure,
        );
    }

    let stage = if let Some(s) = stage {
        s
    } else {
        output::step("No stage specified, using first stage from course definition...");
        challenge_mgr.get_first_stage_slug(&challenge)?
    };

    let course_def = challenge_mgr.get_course_definition(&challenge)?;
    if course_def.get_stage(&stage).is_none() {
        output::error(&format!(
            "Stage '{stage}' not found for challenge '{challenge}'"
        ));
        output::info("Available stages:");
        for s in &course_def.stages {
            output::step(&format!("  {} — {}", s.slug, s.name));
        }
        output::suggestion(&format!(
            "Run 'crafter challenge stages {challenge}' to see all available stages"
        ));
        return Err(crafter::types::CrafterError::other(format!(
            "Stage '{stage}' not found"
        )));
    }

    output::operation(&format!("Running tests for stage: {stage}"));

    let config = ContainerConfig::from_project(&project_dir, &tester_info.path, &challenge_dir, &stage)?;

    output::detail(&format!("Project: {}", output::format_path(&project_dir)));
    output::detail(&format!("Buildpack: {}", config.buildpack));
    output::detail(&format!("Stage: {stage}"));

    let result = test_service.run_test(&config)?;

    let passed = result.exit_code == 0;
    let duration = result.duration.as_secs_f64();

    if output::is_json() {
        output::emit_json(&TestRunOutput {
            stage: stage.clone(),
            passed,
            exit_code: result.exit_code,
            duration_secs: duration,
            output: format_test_output(&result.output).trim().to_string(),
        })?;
    } else if !output::is_quiet() {
        let display_output = format_test_output(&result.output);
        print!("\n{display_output}");
    }

    if passed {
        if !output::is_json() {
            output::success(&format!("Test passed! (took {duration:.2}s)"));
        }
        Ok(())
    } else {
        if !output::is_json() {
            output::error(&format!("Test failed (took {duration:.2}s)"));
        }
        Err(crafter::types::CrafterError::other("Test failed"))
    }
}

#[allow(clippy::too_many_lines)]
fn run_all_stages(
    project_dir: &std::path::Path,
    challenge: &str,
    challenge_mgr: &crafter::core::challenge::ChallengeManager,
    tester_path: &std::path::Path,
    challenge_dir: &std::path::Path,
    test_service: &crafter::container::TestService,
    continue_on_failure: bool,
) -> crafter::types::Result<()> {
    use std::time::Instant;

    let stages = challenge_mgr.get_stages(challenge)?;

    if stages.is_empty() {
        output::error("No stages found in course definition");
        return Err(crafter::types::CrafterError::other("No stages found"));
    }

    if !output::is_json() {
        output::operation(&format!("Running all {} stages for {}", stages.len(), challenge));
    }

    let mut passed = 0;
    let mut failed = 0;
    let mut failed_stages = Vec::new();
    let mut stage_results: Vec<TestStageRunOutput> = Vec::new();
    let total_start = Instant::now();

    for (index, stage) in stages.iter().enumerate() {
        let stage_num = index + 1;
        if !output::is_json() {
            output::step(&format!(
                "[{}/{}] {} — {}",
                stage_num,
                stages.len(),
                stage.slug,
                stage.name
            ));
        }

        let config = match ContainerConfig::from_project(project_dir, tester_path, challenge_dir, &stage.slug)
        {
            Ok(c) => c,
            Err(e) => {
                if !output::is_json() {
                    output::error(&format!("Failed to create config: {e}"));
                }
                failed += 1;
                failed_stages.push((stage.slug.clone(), stage.name.clone()));
                if output::is_json() {
                    stage_results.push(TestStageRunOutput {
                        slug: stage.slug.clone(),
                        name: stage.name.clone(),
                        passed: false,
                        exit_code: -1,
                        duration_secs: 0.0,
                        output: format!("Failed to create config: {e}"),
                    });
                }
                if !continue_on_failure {
                    break;
                }
                continue;
            }
        };

        let result = match test_service.run_test(&config) {
            Ok(r) => r,
            Err(e) => {
                if !output::is_json() {
                    output::error(&format!("Failed to run test: {e}"));
                }
                failed += 1;
                failed_stages.push((stage.slug.clone(), stage.name.clone()));
                if output::is_json() {
                    stage_results.push(TestStageRunOutput {
                        slug: stage.slug.clone(),
                        name: stage.name.clone(),
                        passed: false,
                        exit_code: -1,
                        duration_secs: 0.0,
                        output: format!("Failed to run test: {e}"),
                    });
                }
                if !continue_on_failure {
                    break;
                }
                continue;
            }
        };

        let stage_passed = result.exit_code == 0;
        let stage_duration = result.duration.as_secs_f64();

        if output::is_json() {
            stage_results.push(TestStageRunOutput {
                slug: stage.slug.clone(),
                name: stage.name.clone(),
                passed: stage_passed,
                exit_code: result.exit_code,
                duration_secs: stage_duration,
                output: format_test_output(&result.output).trim().to_string(),
            });
        }

        if stage_passed {
            if !output::is_json() {
                output::success(&format!("PASSED ({stage_duration:.2}s)"));
            }
            passed += 1;
        } else {
            if !output::is_json() {
                output::error(&format!("FAILED ({stage_duration:.2}s)"));
            }
            failed += 1;
            failed_stages.push((stage.slug.clone(), stage.name.clone()));

            if !output::is_json() && !output::is_quiet() {
                let display_output = format_test_output(&result.output);
                print!("{display_output}");
            }

            if !continue_on_failure {
                if !output::is_json() {
                    output::warn(
                        "Stopping on first failure. Use --continue-on-failure to run all stages.",
                    );
                }
                break;
            }
        }
    }

    let total_duration = total_start.elapsed();

    if output::is_json() {
        output::emit_json(&TestAllStagesOutput {
            challenge: challenge.to_string(),
            total: stages.len(),
            passed,
            failed,
            duration_secs: total_duration.as_secs_f64(),
            stages: stage_results,
        })?;
    } else {
        let total_str = stages.len().to_string();
        let passed_str = passed.to_string();
        let failed_str = failed.to_string();
        let duration_str = format!("{:.2}s", total_duration.as_secs_f64());

        output::summary(
            "Results",
            &[
                ("total", &total_str),
                ("passed", &passed_str),
                ("failed", &failed_str),
                ("duration", &duration_str),
            ],
        );

        if !failed_stages.is_empty() {
            let rows: Vec<Vec<String>> = failed_stages
                .iter()
                .map(|(slug, name)| vec![slug.clone(), name.clone()])
                .collect();
            output::table(&["STAGE", "NAME"], &rows);
        }
    }

    if failed > 0 {
        Err(crafter::types::CrafterError::other(format!(
            "{failed} stage(s) failed"
        )))
    } else {
        if !output::is_json() {
            output::success("All stages passed!");
        }
        Ok(())
    }
}