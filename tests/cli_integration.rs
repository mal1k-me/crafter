// CLI integration tests using assert_cmd

use assert_cmd::Command;
use predicates::prelude::*;

fn crafter() -> Command {
    assert_cmd::cargo_bin_cmd!("crafter")
}

#[test]
fn q_a_help_shows_subcommands() {
    crafter()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("base"))
        .stdout(predicate::str::contains("challenge"))
        .stdout(predicate::str::contains("tester"));
}

#[test]
fn q_a_no_args_fails_with_usage() {
    crafter()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage:"));
}

#[test]
fn q_a_challenge_help_has_descriptions() {
    crafter()
        .args(["challenge", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialize a new challenge project"))
        .stdout(predicate::str::contains("List available challenges"));
}

#[test]
fn q_a_json_outputs_are_structured() {
    crafter()
        .args(["challenge", "list", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["))
        .stdout(predicate::str::contains("\"redis\""))
        .stderr(predicate::str::is_empty());

    crafter()
        .args(["config", "show", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"auto_update\""))
        .stdout(predicate::str::contains("\"output\""))
        .stderr(predicate::str::is_empty());
}

#[test]
fn q_a_quiet_mode_suppresses_stdout() {
    crafter()
        .args(["challenge", "list", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn q_a_simple_mode_has_no_ansi() {
    crafter()
        .args(["challenge", "list", "--format", "simple"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[").not());
}

#[test]
fn q_a_flag_conflicts_are_rejected() {
    let cases = [
        vec!["base", "--quiet", "--verbose", "status"],
        vec!["challenge", "list", "--raw-sizes"],
        vec!["challenge", "list", "--full-paths"],
        vec!["challenge", "list", "--verbose"],
        vec!["config", "get", "auto_update", "--verbose"],
        vec!["challenge", "languages", "redis", "--verbose"],
    ];

    for args in cases {
        crafter().args(args).assert().failure();
    }
}

#[test]
fn q_a_quiet_can_combine_with_format_and_display_flags() {
    crafter()
        .args(["base", "--quiet", "--format", "json", "status"])
        .assert()
        .success();

    crafter()
        .args(["tester", "list", "--quiet", "--raw-sizes"])
        .assert()
        .success();

    crafter()
        .args(["tester", "list", "--quiet", "--full-paths"])
        .assert()
        .success();
}

#[test]
fn q_a_test_flag_constraints_hold() {
    crafter()
        .args(["test", "--continue-on-failure"])
        .assert()
        .failure();

    crafter()
        .args(["test", "first", "--all"])
        .assert()
        .failure();
}

#[test]
fn q_a_unknown_inputs_are_rejected() {
    crafter().arg("totally-bogus-command").assert().failure();
    crafter()
        .args(["tester", "list", "--not-a-real-flag"])
        .assert()
        .failure();
    crafter()
        .args(["challenge", "init", "redis", "rust", "--bogus"])
        .assert()
        .failure();
}

#[test]
fn q_a_json_error_shape_stays_on_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("codecrafters-redis-rust")).unwrap();

    crafter()
        .args(["challenge", "init", "redis", "rust", "--format", "json"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"error\""))
        .stdout(predicate::str::contains("\"type\""))
        .stderr(predicate::str::is_empty());
}

#[test]
fn q_a_unknown_tester_build_does_not_create_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let tester_dir = tmp
        .path()
        .join(".local")
        .join("share")
        .join("crafter")
        .join("testers")
        .join("totally-bogus");

    crafter()
        .args(["tester", "build", "totally-bogus"])
        .env("HOME", tmp.path())
        .assert()
        .failure();

    assert!(!tester_dir.exists());
}
