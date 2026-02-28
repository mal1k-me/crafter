//! Challenge command handlers.

use super::args::ChallengeAction;
use crafter::core::challenge::ChallengeManager;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct ChallengeInitResponse {
    success: bool,
    path: String,
}

#[derive(Debug, Serialize)]
struct ChallengeUpdateResponse {
    success: bool,
    challenge: Option<String>,
}

/// Ensure a challenge repository is available locally.
async fn ensure_challenge_downloaded(
    challenge_mgr: &ChallengeManager,
    challenge: &str,
    announce_download: bool,
) -> crafter::types::Result<()> {
    use crafter::output::compat;

    if !challenge_mgr.is_downloaded(challenge) {
        if announce_download {
            compat::step(&format!("Downloading {challenge} challenge repository..."));
        }
        challenge_mgr.download(challenge).await?;
    }

    Ok(())
}

fn render_challenge_list(names: Vec<String>, installed: bool) -> crafter::types::Result<()> {
    use crafter::output::compat;
    use crafter::output::Format;

    if compat::is_json() {
        compat::emit_json(&names)?;
    } else if matches!(compat::get_format(), Format::Simple) && !compat::is_quiet() {
        compat::list(&names);
    } else if !compat::is_quiet() {
        use crafter::output::formatter::Formatter;
        use crafter::output::formatters::ChallengeListFormatter;
        compat::with_stdout(|stdout| ChallengeListFormatter::new(names, installed).format(stdout))?;
    }

    Ok(())
}

fn render_stages_output(
    challenge_name: &str,
    stages: Vec<crafter::types::StageInfo>,
) -> crafter::types::Result<()> {
    use crafter::output::compat;
    use crafter::types::StagesOutput;

    let output_data = StagesOutput {
        challenge: challenge_name.to_string(),
        total: stages.len(),
        stages: stages.clone(),
    };

    if compat::is_json() {
        compat::emit_json(&output_data)?;
    } else if !compat::is_quiet() {
        use crafter::output::formatter::Formatter;
        use crafter::output::formatters::{StageEntry, StagesFormatter};

        let entries: Vec<StageEntry> = stages
            .iter()
            .map(|s| StageEntry {
                slug: s.slug.clone(),
                name: s.name.clone(),
                difficulty: s.difficulty.clone(),
                extension_slug: s.extension_slug.clone(),
                extension_name: s.extension.clone(),
            })
            .collect();

        compat::with_stdout(|stdout| {
            StagesFormatter::new(challenge_name.to_string(), entries).format(stdout)
        })?;
    }

    Ok(())
}

fn render_status_output(status: crafter::types::StatusOutput) -> crafter::types::Result<()> {
    use crafter::output::compat;

    if compat::is_json() {
        compat::emit_json(&status)?;
    } else if !compat::is_quiet() {
        use crafter::output::formatter::Formatter;
        use crafter::output::formatters::StatusFormatter;
        compat::with_stdout(|stdout| {
            StatusFormatter::new(status)
                .with_raw_sizes(compat::use_raw_sizes())
                .format(stdout)
        })?;
    }

    Ok(())
}

fn render_languages_output(challenge: &str, languages: Vec<String>) -> crafter::types::Result<()> {
    use crafter::output::compat;

    if compat::is_json() {
        compat::emit_json(&languages)?;
    } else if !compat::is_quiet() {
        use crafter::output::formatter::Formatter;
        use crafter::output::formatters::LanguageListFormatter;
        compat::with_stdout(|stdout| LanguageListFormatter::new(challenge, languages).format(stdout))?;
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
pub async fn handle_challenge(action: ChallengeAction) -> crafter::types::Result<()> {
    use crafter::core::detector::ChallengeDetector;
    use crafter::constants::project;
    use crafter::output::compat as output;
    use crafter::types::CodecraftersConfig;
    use crafter::utils::{env, fs};
    use std::sync::Arc;

    let ctx = super::context::build_cli_context()?;
    let git_mgr = ctx.git_mgr;
    let challenge_mgr = ctx.challenge_mgr;
    let tester_mgr = ctx.tester_mgr;

    match action {
        ChallengeAction::Init {
            output: _,
            challenge,
            language,
            git: init_git,
        } => {
            output::operation(&format!("Initializing {challenge} challenge with {language}..."));

            let target_dir = env::current_dir()?.join(format!("codecrafters-{challenge}-{language}"));

            if fs::exists(&target_dir) {
                return Err(crafter::types::CrafterError::other(format!(
                    "Directory {} already exists",
                    output::format_path(&target_dir)
                )));
            }

            output::detail(&format!("Target: {}", output::format_path(&target_dir)));

            ensure_challenge_downloaded(challenge_mgr.as_ref(), &challenge, false).await?;

            let starter_dir = challenge_mgr.as_ref().get_starter_dir(&challenge, &language)?;
            output::step(&format!("Copying starter files from {language}..."));
            fs::copy_dir_all(&starter_dir, &target_dir)?;

            let challenge_dir = challenge_mgr.as_ref().get_challenge_dir(&challenge)?;
            let buildpack = CodecraftersConfig::default_buildpack(&language, Some(&challenge_dir));
            let cc_config = CodecraftersConfig {
                debug: false,
                buildpack: buildpack.clone(),
            };
            let cc_yaml = serde_yaml::to_string(&cc_config)?;
            fs::write_string(target_dir.join(project::CODECRAFTERS_YML), &cc_yaml)?;

            tester_mgr.as_ref().ensure_available(&challenge).await?;

            if init_git {
                output::step("Initializing git repository...");
                git_mgr.init(&target_dir)?;
                git_mgr.add_all(&target_dir)?;
                git_mgr.commit(&target_dir, "init [skip ci]")?;
            }

            if output::is_json() {
                output::emit_json(&ChallengeInitResponse {
                    success: true,
                    path: output::format_path(&target_dir),
                })?;
            } else {
                output::success("Challenge initialized successfully!");
                if !output::is_quiet() {
                    use crafter::output::formatter::Formatter;
                    use crafter::output::formatters::NextStepsFormatter;
                    output::with_stdout(|stdout| {
                        NextStepsFormatter::new(target_dir.clone())
                            .with_full_paths(output::use_full_paths())
                            .format(stdout)
                    })?;
                }
            }

            Ok(())
        }
        ChallengeAction::List {
            output: _,
            installed,
        } => {
            use crafter::utils::slug::challenge_from_url;

            let names: Vec<String> = if installed {
                challenge_mgr.as_ref().list_downloaded()?
            } else {
                challenge_mgr
                    .as_ref()
                    .list_challenges()?
                    .iter()
                    .filter_map(|entry| challenge_from_url(&entry.repository))
                    .collect()
            };

            render_challenge_list(names, installed)?;
            Ok(())
        }
        ChallengeAction::Update {
            output: _,
            challenge,
        } => {
            use crafter::utils::slug::challenge_from_url;

            if let Some(ref name) = challenge {
                challenge_mgr.update(name)?;
            } else {
                if !output::is_json() {
                    output::info("Updating all challenges...");
                }
                let challenges = challenge_mgr.as_ref().list_challenges()?;
                for entry in challenges {
                    // Preserve historical behavior: if parsing fails, fall back
                    // to an "unknown" probe key (effectively a no-op in practice).
                    let name = challenge_from_url(&entry.repository)
                        .unwrap_or_else(|| "unknown".to_string());

                    if challenge_mgr.as_ref().is_downloaded(&name) {
                        challenge_mgr.as_ref().update(&name)?;
                    }
                }
            }
            if output::is_json() {
                output::emit_json(&ChallengeUpdateResponse {
                    success: true,
                    challenge,
                })?;
            }
            Ok(())
        }
        ChallengeAction::Stages {
            output: _,
            challenge,
        } => {
            use crafter::core::detector::ChallengeDetector;
            use crafter::types::StageInfo;

            let challenge_name = if let Some(name) = challenge {
                name
            } else {
                let project_dir = env::current_dir()?;
                let detector = ChallengeDetector::new(git_mgr.clone());
                detector.detect(&project_dir)?
            };

            ensure_challenge_downloaded(challenge_mgr.as_ref(), &challenge_name, true).await?;

            let course_def = challenge_mgr.as_ref().get_course_definition(&challenge_name)?;

            let stages: Vec<StageInfo> = course_def
                .stages
                .iter()
                .map(|stage| StageInfo {
                    slug: stage.slug.clone(),
                    name: stage.name.clone(),
                    difficulty: stage.difficulty.clone(),
                    extension: stage
                        .primary_extension_slug
                        .as_ref()
                        .and_then(|ext_slug| course_def.get_extension_name(ext_slug)),
                    extension_slug: stage.primary_extension_slug.clone(),
                })
                .collect();

            render_stages_output(&challenge_name, stages)?;

            Ok(())
        }
        ChallengeAction::Status { .. } => {
            use crafter::output::{compat, utils};
            use crafter::types::{
                CodecraftersConfig, DockerfileStatus, RepoStatus, StatusOutput, TesterStatus,
            };

            let project_dir = env::current_dir()?;

            let cc_yaml_path = project_dir.join(project::CODECRAFTERS_YML);
            if !fs::exists(&cc_yaml_path) {
                return Err(crafter::types::CrafterError::with_suggestion(
                    project::MSG_NOT_PROJECT,
                    project::MSG_INIT_PROJECT_HINT,
                ));
            }

            let cc_content = fs::read_to_string(&cc_yaml_path)?;
            let cc_config: CodecraftersConfig = serde_yaml::from_str(&cc_content).map_err(|e| {
                crafter::types::CrafterError::with_suggestion(
                    format!("Invalid {}: {e}", project::CODECRAFTERS_YML),
                    project::MSG_INVALID_YML_HINT,
                )
            })?;

            let detector = ChallengeDetector::new(git_mgr.clone());
            let challenge_result = detector.detect(&project_dir);

            let challenge = challenge_result.as_ref().ok().cloned();
            let mut repo_status = None;
            let mut tester_status = None;
            let mut dockerfile_status = None;

            if let Some(ref chall) = challenge {
                let challenge_dir = challenge_mgr.as_ref().get_challenge_dir(chall)?;

                if fs::exists(&challenge_dir) {
                    let size = fs::dir_size(&challenge_dir).unwrap_or(0);
                    repo_status = Some(RepoStatus {
                        downloaded: true,
                        size_bytes: Some(size),
                        size_mb: Some(utils::size_as_mb(size)),
                        path: Some(utils::format_path(&challenge_dir, compat::use_full_paths())),
                    });

                    let dockerfile = challenge_dir
                        .join("dockerfiles")
                        .join(format!("{}.Dockerfile", cc_config.buildpack));
                    dockerfile_status = Some(DockerfileStatus {
                        found: fs::exists(&dockerfile),
                        path: if fs::exists(&dockerfile) {
                            Some(utils::format_path(&dockerfile, compat::use_full_paths()))
                        } else {
                            None
                        },
                    });
                } else {
                    repo_status = Some(RepoStatus {
                        downloaded: false,
                        size_bytes: None,
                        size_mb: None,
                        path: None,
                    });
                }

                let tester_info = tester_mgr.as_ref().get_info(chall);

                if let Ok(info) = tester_info {
                    let tester_size = fs::file_size(&info.tester_binary).unwrap_or(0);

                    tester_status = Some(TesterStatus {
                        downloaded: true,
                        version: Some(info.version.clone()),
                        size_bytes: Some(tester_size),
                        size_mb: Some(utils::size_as_mb(tester_size)),
                        path: Some(utils::format_path(&info.path, compat::use_full_paths())),
                    });
                } else {
                    tester_status = Some(TesterStatus {
                        downloaded: false,
                        version: None,
                        size_bytes: None,
                        size_mb: None,
                        path: None,
                    });
                }
            }

            let docker_status = {
                use crafter::container::TestService;
                let docker_version = if TestService::new().is_available() {
                    std::process::Command::new("docker")
                        .arg("--version")
                        .output()
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .map(|v| v.trim().replace("Docker version ", ""))
                } else {
                    None
                };
                Some(crafter::types::output::DockerStatus {
                    available: docker_version.is_some(),
                    version: docker_version,
                })
            };

            let status = StatusOutput {
                directory: utils::format_path(&project_dir, compat::use_full_paths()),
                challenge,
                buildpack: cc_config.buildpack.clone(),
                debug: cc_config.debug,
                challenge_repo: repo_status,
                tester: tester_status,
                dockerfile: dockerfile_status,
                docker: docker_status,
            };

            render_status_output(status)?;

            Ok(())
        }
        ChallengeAction::Languages {
            output: _,
            challenge,
        } => {
            use crafter::output::compat;

            ensure_challenge_downloaded(challenge_mgr.as_ref(), &challenge, true).await?;

            let languages = challenge_mgr.as_ref().get_available_languages(&challenge)?;

            if languages.is_empty() {
                compat::warn(&format!("No languages found for challenge '{challenge}'"));
                return Ok(());
            }

            render_languages_output(&challenge, languages)?;

            Ok(())
        }
        ChallengeAction::Validate {
            output: _,
            all_checks,
            fix,
        } => {
            use crafter::core::validator::Validator;
            use crafter::output::compat;

            let project_dir = std::env::current_dir()?;
            let detector = Arc::new(ChallengeDetector::new(git_mgr.clone()));

            let validator = Validator::new(challenge_mgr.clone(), tester_mgr.clone(), detector.clone());
            let mut final_report = validator.validate_all(&project_dir, all_checks)?;

            if fix {
                if !compat::is_json() {
                    compat::info("Attempting to auto-fix issues...");
                }

                if let Some(ref challenge) = final_report.challenge {
                    for result in &final_report.results {
                        if !result.passed && result.fixable && result.check_name == "Tester" {
                            if !compat::is_json() {
                                compat::step(&format!("Downloading tester for '{challenge}'..."));
                            }
                            match tester_mgr.as_ref().ensure_available(challenge).await {
                                Ok(()) => {
                                    if !compat::is_json() {
                                        compat::success("Tester downloaded successfully");
                                    }
                                }
                                Err(e) => {
                                    if !compat::is_json() {
                                        compat::error(&format!("Failed to download tester: {e}"));
                                    }
                                }
                            }
                        }
                    }
                }

                if !compat::is_json() {
                    compat::info("Re-running validation...");
                }
                final_report = validator.validate_all(&project_dir, all_checks)?;
            }

            crate::cli::test::emit_validation_report(&final_report, all_checks)?;

            if final_report.has_errors() {
                std::process::exit(1);
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crafter::utils::slug::challenge_from_url;

    #[test]
    fn parse_challenge_name_from_repo_url() {
        assert_eq!(
            challenge_from_url("https://github.com/codecrafters-io/build-your-own-shell"),
            Some("shell".to_string())
        );
    }

    #[test]
    fn parse_challenge_name_returns_none_for_non_codecrafters_repo() {
        assert_eq!(
            challenge_from_url("https://github.com/example-org/some-other-repo"),
            None
        );
    }
}