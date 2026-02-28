//! Integration tests for output and error systems

use crafter::error::{codes, CrafterError, ValidationFailure};
use crafter::types::output::{TestAllStagesOutput, TestRunOutput, TestStageRunOutput};

#[test]
fn test_error_exit_codes() {
    assert_eq!(
        CrafterError::docker("test").exit_code(),
        codes::DOCKER_ERROR
    );
    assert_eq!(
        CrafterError::network("test").exit_code(),
        codes::NETWORK_ERROR
    );
    assert_eq!(
        CrafterError::not_found("stage", "invalid").exit_code(),
        codes::NOT_FOUND
    );
    assert_eq!(
        CrafterError::test_failed("oo8", 1).exit_code(),
        codes::TEST_FAILED
    );
}

#[test]
fn test_error_json_serialization() {
    let err = CrafterError::ValidationFailed {
        failures: vec![ValidationFailure {
            check: "test".to_string(),
            message: "Test failed".to_string(),
            hint: Some("Fix it".to_string()),
        }],
    };

    let json = err.to_json();
    assert_eq!(json["error"], "validation");
    assert_eq!(json["type"], "validation");
    assert!(json["message"].is_string());
    assert_eq!(json["exit_code"], codes::VALIDATION_FAILED);
    assert!(json["failures"].is_array());
}

#[test]
fn test_error_json_not_found_has_stable_envelope() {
    let err = CrafterError::not_found("challenge", "redis");
    let json = err.to_json();

    assert_eq!(json["type"], "not_found");
    assert_eq!(json["error"], "not_found");
    assert!(json["message"].is_string());
    assert_eq!(json["resource"], "challenge");
    assert_eq!(json["name"], "redis");
    assert_eq!(json["exit_code"], codes::NOT_FOUND);
}

#[test]
fn test_typed_test_output_schema_contracts() {
    let single = TestRunOutput {
        stage: "oo8".to_string(),
        passed: true,
        exit_code: 0,
        duration_secs: 1.23,
        output: "ok".to_string(),
    };

    let all = TestAllStagesOutput {
        challenge: "shell".to_string(),
        total: 2,
        passed: 1,
        failed: 1,
        duration_secs: 2.34,
        stages: vec![TestStageRunOutput {
            slug: "oo8".to_string(),
            name: "Prompt".to_string(),
            passed: false,
            exit_code: 1,
            duration_secs: 0.5,
            output: "failed".to_string(),
        }],
    };

    let single_json = serde_json::to_value(single).unwrap();
    let all_json = serde_json::to_value(all).unwrap();

    assert_eq!(single_json["stage"], "oo8");
    assert!(single_json["passed"].is_boolean());
    assert!(single_json["exit_code"].is_i64());
    assert!(single_json["duration_secs"].is_number());
    assert!(single_json["output"].is_string());

    assert_eq!(all_json["challenge"], "shell");
    assert_eq!(all_json["total"], 2);
    assert_eq!(all_json["passed"], 1);
    assert_eq!(all_json["failed"], 1);
    assert!(all_json["duration_secs"].is_number());
    assert!(all_json["stages"].is_array());
    assert_eq!(all_json["stages"][0]["slug"], "oo8");
    assert_eq!(all_json["stages"][0]["name"], "Prompt");
}
