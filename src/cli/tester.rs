//! Tester command handlers.

use super::args::TesterAction;
use crafter::core::tester::TesterManager;
use crafter::output::compat as output;
use crafter::types::BuildOptions;
use crafter::utils::{env, fs};
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct TesterListRow {
    challenge: String,
    version: String,
    size_bytes: u64,
    has_wrapper: bool,
    path: std::path::PathBuf,
}

#[derive(Debug, Serialize)]
struct TesterNotFoundResponse {
    challenge: String,
    removed: bool,
    error: String,
}

#[derive(Debug, Serialize)]
struct TesterCleanResponse {
    challenge: Option<String>,
    removed: usize,
    freed_bytes: u64,
    freed: String,
}

#[derive(Debug, Serialize)]
struct TesterBuildResponse {
    success: bool,
    challenge: String,
}

#[derive(Debug, Serialize)]
struct TesterListJsonEntry {
    challenge: String,
    version: String,
    size_mb: String,
    size_bytes: u64,
    has_wrapper: bool,
    path: String,
}

fn emit_tester_not_found(name: &str) -> crafter::types::Result<()> {
    if output::is_json() {
        output::emit_json(&TesterNotFoundResponse {
            challenge: name.to_string(),
            removed: false,
            error: "tester not found".to_string(),
        })?;
    } else {
        output::warn(&format!("Tester '{name}' not found"));
        output::suggestion("List installed testers with: crafter tester list");
    }
    Ok(())
}

fn emit_clean_none() -> crafter::types::Result<()> {
    if output::is_json() {
        output::emit_json(&TesterCleanResponse {
            challenge: None,
            removed: 0,
            freed_bytes: 0,
            freed: "0 B".to_string(),
        })?;
    } else {
        output::info("No testers to clean");
    }
    Ok(())
}

fn collect_testers(
    tester_mgr: &TesterManager,
    testers_dir: &std::path::Path,
) -> Vec<TesterListRow> {
    let mut testers = Vec::new();

    if let Ok(entries) = std::fs::read_dir(testers_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let challenge = entry.file_name().to_string_lossy().to_string();

                if let Ok(info) = tester_mgr.get_info(&challenge) {
                    let tester_size = fs::file_size(&info.tester_binary).unwrap_or(0);
                    let test_sh_exists = fs::exists(&info.test_script);

                    testers.push(TesterListRow {
                        challenge,
                        version: info.version,
                        size_bytes: tester_size,
                        has_wrapper: test_sh_exists,
                        path: info.path,
                    });
                }
            }
        }
    }

    testers.sort_by(|a, b| a.challenge.cmp(&b.challenge));
    testers
}

fn emit_tester_list_output(testers: &[TesterListRow]) -> crafter::types::Result<()> {
    use crafter::output::compat;

    if compat::is_json() {
        use crafter::output::utils::{format_path, size_as_mb};

        let json_testers: Vec<TesterListJsonEntry> = testers
            .iter()
            .map(|row| TesterListJsonEntry {
                challenge: row.challenge.clone(),
                version: row.version.clone(),
                size_mb: size_as_mb(row.size_bytes),
                size_bytes: row.size_bytes,
                has_wrapper: row.has_wrapper,
                path: format_path(&row.path, compat::use_full_paths()),
            })
            .collect();

        compat::emit_json(&json_testers)?;
    } else if !compat::is_quiet() {
        use crafter::output::formatter::Formatter;
        use crafter::output::formatters::{TesterEntry, TesterListFormatter};

        let entries: Vec<TesterEntry> = testers
            .iter()
            .map(|row| TesterEntry {
                challenge: row.challenge.clone(),
                version: row.version.clone(),
                size_bytes: row.size_bytes,
                has_wrapper: row.has_wrapper,
                path: row.path.clone(),
                raw_sizes: compat::use_raw_sizes(),
                full_paths: compat::use_full_paths(),
            })
            .collect();

        compat::with_stdout(|stdout| {
            TesterListFormatter::new(entries)
                .with_raw_sizes(compat::use_raw_sizes())
                .with_full_paths(compat::use_full_paths())
                .format(stdout)
        })?;
    }

    Ok(())
}

fn create_tester_manager() -> crafter::types::Result<Arc<TesterManager>> {
    let ctx = super::context::build_cli_context()?;
    Ok(ctx.tester_mgr)
}

fn handle_clean_single(tester_mgr: &TesterManager, name: String) -> crafter::types::Result<()> {
    let info = match tester_mgr.get_info(&name) {
        Ok(info) => info,
        Err(_) => {
            emit_tester_not_found(&name)?;
            return Ok(());
        }
    };

    if !fs::exists(&info.path) {
        emit_tester_not_found(&name)?;
        return Ok(());
    }

    let tester_size = fs::file_size(&info.tester_binary).unwrap_or(0);
    let test_sh_size = fs::file_size(&info.test_script).unwrap_or(0);
    let total_size = tester_size + test_sh_size;

    if output::is_json() {
        std::fs::remove_dir_all(&info.path)?;
        output::emit_json(&TesterCleanResponse {
            challenge: Some(name),
            removed: 1,
            freed_bytes: total_size,
            freed: fs::format_size(total_size),
        })?;
    } else {
        use crafter::output::utils::format_size;

        output::operation(&format!("Removing tester: {name}"));
        std::fs::remove_dir_all(&info.path).map_err(|e| {
            output::error(&format!("Failed to remove tester: {e}"));
            e
        })?;

        let path = output::format_path(&info.path);
        let size = format_size(total_size, output::use_raw_sizes());
        output::keyval(&[("version", &info.version), ("path", &path), ("size", &size)]);

        output::success(&format!(
            "Removed {} tester, freed {}",
            name,
            format_size(total_size, output::use_raw_sizes())
        ));
    }

    Ok(())
}

fn collect_cleanup_stats(testers_dir: &Path) -> (usize, u64) {
    let mut total_testers = 0usize;
    let mut total_size = 0u64;

    if let Ok(entries) = std::fs::read_dir(testers_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                total_testers += 1;
                if let Ok(size) = fs::dir_size(entry.path()) {
                    total_size += size;
                }
            }
        }
    }

    (total_testers, total_size)
}

fn handle_clean_all(testers_dir: &Path) -> crafter::types::Result<()> {
    if !fs::exists(testers_dir) {
        emit_clean_none()?;
        return Ok(());
    }

    let (total_testers, total_size) = collect_cleanup_stats(testers_dir);
    if total_testers == 0 {
        emit_clean_none()?;
        return Ok(());
    }

    if output::is_json() {
        std::fs::remove_dir_all(testers_dir)?;
        fs::ensure_dir(testers_dir)?;
        output::emit_json(&TesterCleanResponse {
            challenge: None,
            removed: total_testers,
            freed_bytes: total_size,
            freed: fs::format_size(total_size),
        })?;
    } else {
        use crafter::output::utils::format_size;

        output::operation(&format!(
            "Removing {} testers  ({})",
            total_testers,
            format_size(total_size, output::use_raw_sizes())
        ));
        let path = output::format_path(testers_dir);
        output::keyval(&[("path", &path)]);

        std::fs::remove_dir_all(testers_dir).map_err(|e| {
            output::error(&format!("Failed to remove testers: {e}"));
            e
        })?;

        fs::ensure_dir(testers_dir)?;

        output::success(&format!(
            "Removed {} testers, freed {}",
            total_testers,
            format_size(total_size, output::use_raw_sizes())
        ));
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
pub async fn handle_tester(action: TesterAction) -> crafter::types::Result<()> {
    match action {
        TesterAction::Build {
            output: _,
            challenge,
            force,
            version,
        } => {
            let tester_mgr = create_tester_manager()?;

            output::operation(&format!("Building tester for challenge: {challenge}"));

            let normalized_version = version.and_then(|v| {
                if v.eq_ignore_ascii_case("latest") {
                    None
                } else {
                    Some(v)
                }
            });

            let opts = BuildOptions {
                force,
                version: normalized_version,
                from_source: false,
            };

            tester_mgr.build(&challenge, opts).await?;

            if output::is_json() {
                output::emit_json(&TesterBuildResponse {
                    success: true,
                    challenge,
                })?;
            } else {
                output::success("Tester built successfully");
            }
            Ok(())
        }
        TesterAction::List { .. } => {
            let tester_mgr = create_tester_manager()?;

            let testers_dir = env::testers_dir()?;

            if !fs::exists(&testers_dir) {
                return emit_tester_list_output(&[]);
            }

            let testers = collect_testers(&tester_mgr, &testers_dir);
            emit_tester_list_output(&testers)?;

            Ok(())
        }
        TesterAction::Clean {
            output: _,
            challenge,
        } => {
            let testers_dir = env::testers_dir()?;
            let tester_mgr = create_tester_manager()?;

            if let Some(name) = challenge {
                handle_clean_single(&tester_mgr, name)?;
            } else {
                handle_clean_all(&testers_dir)?;
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TesterListRow;

    #[test]
    fn sort_rows_by_challenge_name() {
        let mut rows = [
            TesterListRow {
                challenge: "zlib".to_string(),
                version: "v1".to_string(),
                size_bytes: 1,
                has_wrapper: true,
                path: std::path::PathBuf::from("/tmp/zlib"),
            },
            TesterListRow {
                challenge: "redis".to_string(),
                version: "v1".to_string(),
                size_bytes: 1,
                has_wrapper: true,
                path: std::path::PathBuf::from("/tmp/redis"),
            },
        ];

        rows.sort_by(|a, b| a.challenge.cmp(&b.challenge));
        assert_eq!(rows[0].challenge, "redis");
        assert_eq!(rows[1].challenge, "zlib");
    }
}