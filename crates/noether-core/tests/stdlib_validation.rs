use noether_core::stage::validation::{validate_all, validate_stage};
use noether_core::stdlib::load_stdlib;

#[test]
fn all_stdlib_stages_pass_validation() {
    let stages = load_stdlib();
    let results = validate_all(&stages, 5);

    let mut failures = Vec::new();
    for result in &results {
        if !result.is_ok() {
            failures.push(format!(
                "Stage '{}' failed validation:\n{}",
                result.stage_description,
                result
                    .errors
                    .iter()
                    .map(|e| format!("  - {e}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "Stdlib validation failures:\n{}",
        failures.join("\n\n")
    );
}

#[test]
fn each_stdlib_stage_has_at_least_5_examples() {
    let stages = load_stdlib();
    for stage in &stages {
        assert!(
            stage.examples.len() >= 5,
            "Stage '{}' has only {} examples (need 5+)",
            stage.description,
            stage.examples.len()
        );
    }
}

#[test]
fn stdlib_example_inputs_match_declared_types() {
    let stages = load_stdlib();
    for stage in &stages {
        let result = validate_stage(stage, 0);
        let input_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    noether_core::stage::validation::ValidationError::InputTypeMismatch { .. }
                )
            })
            .collect();
        assert!(
            input_errors.is_empty(),
            "Stage '{}' has input type mismatches: {:?}",
            stage.description,
            input_errors
        );
    }
}

#[test]
fn stdlib_example_outputs_match_declared_types() {
    let stages = load_stdlib();
    for stage in &stages {
        let result = validate_stage(stage, 0);
        let output_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    noether_core::stage::validation::ValidationError::OutputTypeMismatch { .. }
                )
            })
            .collect();
        assert!(
            output_errors.is_empty(),
            "Stage '{}' has output type mismatches: {:?}",
            stage.description,
            output_errors
        );
    }
}
