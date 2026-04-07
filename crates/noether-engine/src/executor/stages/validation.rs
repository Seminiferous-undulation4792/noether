//! Rust-native implementations for the stage submission validation pipeline.
//!
//! These stages execute entirely within the Rust process (via `InlineExecutor`)
//! — no Python, no Nix subprocess.  Each stage operates on the raw JSON
//! representation of a `Stage` so they can be composed into a standard Noether
//! `Parallel + Sequential` graph without any special casing in the registry.

use super::super::ExecutionError;
use noether_core::stage::StageId;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

// ── helpers ────────────────────────────────────────────────────────────────

fn err(name: &str, message: impl Into<String>) -> ExecutionError {
    ExecutionError::StageFailed { stage_id: StageId(name.into()), message: message.into() }
}

// ── individual check stages ─────────────────────────────────────────────────

/// Verify that the stage's `id` field equals SHA-256 of its canonical
/// `signature` JSON (the Noether content-addressing invariant).
pub fn verify_stage_content_hash(input: &Value) -> Result<Value, ExecutionError> {
    let stage_id = input["id"].as_str().unwrap_or("");

    let sig_json = serde_json::to_string(&input["signature"]).map_err(|e| {
        err("verify_stage_content_hash", format!("cannot serialise signature: {e}"))
    })?;

    let computed = hex::encode(Sha256::digest(sig_json.as_bytes()));

    if stage_id == computed {
        Ok(json!({
            "passed": true,
            "stage_id": stage_id,
            "computed": computed,
            "error": null
        }))
    } else {
        Ok(json!({
            "passed": false,
            "stage_id": stage_id,
            "computed": computed,
            "error": format!(
                "content hash mismatch: stage.id={} computed={}",
                stage_id, computed
            )
        }))
    }
}

/// Verify the Ed25519 signature of a stage.
///
/// If the stage is unsigned, the check **passes** (with a warning) — unsigned
/// stages are allowed; promotion to Active is blocked by the lifecycle rules.
pub fn verify_stage_ed25519(input: &Value) -> Result<Value, ExecutionError> {
    let sig_hex = input["ed25519_signature"].as_str();
    let pub_hex = input["signer_public_key"].as_str();

    match (sig_hex, pub_hex) {
        (Some(sig), Some(pub_key)) => {
            let stage_id = StageId(input["id"].as_str().unwrap_or("").to_string());
            match noether_core::stage::verify_stage_signature(&stage_id, sig, pub_key) {
                Ok(true) => Ok(json!({ "passed": true, "signed": true, "warning": null })),
                Ok(false) => Ok(json!({
                    "passed": false,
                    "signed": true,
                    "warning": "Ed25519 signature verification failed — stage may have been tampered with"
                })),
                Err(e) => Ok(json!({
                    "passed": false,
                    "signed": true,
                    "warning": format!("signature decode error: {e}")
                })),
            }
        }
        (None, None) => Ok(json!({
            "passed": true,
            "signed": false,
            "warning": "stage is unsigned — consider signing before promoting to Active"
        })),
        _ => Ok(json!({
            "passed": false,
            "signed": false,
            "warning": "malformed: exactly one of ed25519_signature / signer_public_key is set"
        })),
    }
}

/// Check that the stage description field is non-empty.
pub fn check_stage_description(input: &Value) -> Result<Value, ExecutionError> {
    let desc = input["description"].as_str().unwrap_or("").trim();
    if desc.is_empty() {
        Ok(json!({ "passed": false, "error": "stage description must not be empty" }))
    } else {
        Ok(json!({ "passed": true, "error": null }))
    }
}

/// Check that the stage has at least one example.
///
/// Examples are optional but strongly recommended for semantic search quality.
/// Missing examples produce a warning, not a hard error.
pub fn check_stage_examples(input: &Value) -> Result<Value, ExecutionError> {
    let count = input["examples"].as_array().map(|a| a.len()).unwrap_or(0);
    let warning: Value = if count == 0 {
        Value::String(
            "no examples provided — semantic search quality will be reduced".into(),
        )
    } else {
        Value::Null
    };
    Ok(json!({ "passed": true, "count": count, "warning": warning }))
}

// ── aggregation stage ───────────────────────────────────────────────────────

/// Aggregate the results of the four parallel validation checks into a single
/// `ValidationReport`.
///
/// Input: a Record produced by the `Parallel` operator, with keys
/// `hash_check`, `sig_check`, `desc_check`, `examples_check`.
///
/// Output: `{ passed: Bool, errors: [Text], warnings: [Text] }`
pub fn merge_validation_checks(input: &Value) -> Result<Value, ExecutionError> {
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    for key in &["hash_check", "sig_check", "desc_check", "examples_check"] {
        let check = &input[key];

        let passed = check["passed"].as_bool().unwrap_or(false);

        if !passed {
            // Hard error — collect from "error" or "warning" field.
            for field in &["error", "warning"] {
                if let Some(msg) = check[field].as_str() {
                    if !msg.is_empty() {
                        errors.push(msg.to_string());
                    }
                }
            }
        } else {
            // Soft warning — collect from "warning" field.
            if let Some(msg) = check["warning"].as_str() {
                if !msg.is_empty() {
                    warnings.push(msg.to_string());
                }
            }
        }
    }

    Ok(json!({
        "passed": errors.is_empty(),
        "errors": errors,
        "warnings": warnings
    }))
}
