//! CI gate — every stdlib stage either verifies behaviourally against its
//! declared examples or is transparently skipped via an honest effect tag.
//!
//! Running this test is what makes `noether stage test` a permanent
//! property of the stdlib rather than a one-time audit: any future stdlib
//! contribution that ships an example that doesn't match its
//! implementation, or that is falsely marked `Pure` while having side
//! effects, will fail CI here.

use noether_core::stdlib::load_stdlib;
use noether_engine::executor::inline::InlineExecutor;
use noether_engine::stage_test::{verify_stage, ExampleOutcome, ReportOutcome};
use noether_store::{MemoryStore, StageStore};

#[test]
fn every_stdlib_stage_passes_or_is_honestly_skipped() {
    // Seed an in-memory store with the full stdlib, then build the
    // InlineExecutor against it. Deterministic Rust-native stages
    // dispatch through this executor; anything with side effects is
    // filtered out by `verify_stage`'s skip logic.
    let mut store = MemoryStore::new();
    for stage in load_stdlib() {
        store.put(stage).unwrap();
    }
    let executor = InlineExecutor::from_store(&store);

    let mut failures: Vec<String> = Vec::new();
    let mut passed = 0usize;
    let mut skipped = 0usize;

    for stage in load_stdlib() {
        let report = verify_stage(&stage, &executor);
        match &report.outcome {
            ReportOutcome::Skipped { .. } => skipped += 1,
            ReportOutcome::Tested { examples } => {
                let mismatches: Vec<(usize, String)> = examples
                    .iter()
                    .enumerate()
                    .filter_map(|(i, outcome)| match outcome {
                        ExampleOutcome::Ok => None,
                        ExampleOutcome::Mismatch { expected, actual } => Some((
                            i,
                            format!(
                                "mismatch: expected {}, got {}",
                                serde_json::to_string(expected).unwrap_or_default(),
                                serde_json::to_string(actual).unwrap_or_default(),
                            ),
                        )),
                        ExampleOutcome::Errored { message } => {
                            Some((i, format!("error: {message}")))
                        }
                    })
                    .collect();
                if mismatches.is_empty() {
                    passed += 1;
                } else {
                    for (i, msg) in mismatches {
                        failures.push(format!("  {} (example {}): {}", stage.description, i, msg));
                    }
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} stdlib stage example(s) do not match their implementation. \
         Either fix the example, fix the implementation, or — if the \
         stage legitimately depends on ambient state — add the \
         corresponding effect (NonDeterministic, Process, Network, Llm) \
         so `stage test` skips it.\n\n{}",
        failures.len(),
        failures.join("\n")
    );

    // Sanity: the stdlib should have a non-trivial pass count.
    // If this drops to zero it means everything is being skipped, which
    // means the effect annotations are way too pessimistic.
    assert!(
        passed >= 30,
        "only {passed} stdlib stages verified — at least 30 should have \
         deterministic, testable behaviour. Did effect annotations drift?"
    );

    eprintln!("stdlib verification: {passed} passed, {skipped} skipped (effect-guarded), 0 failed");
}
