use super::{ExecutionError, StageExecutor};
use crate::lagrange::CompositionNode;
use crate::trace::{CompositionTrace, StageStatus, StageTrace, TraceStatus};
use noether_core::stage::StageId;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::Instant;

/// Result of executing a composition graph.
#[derive(Debug)]
pub struct CompositionResult {
    pub output: Value,
    pub trace: CompositionTrace,
}

/// Execute a composition graph using the provided executor.
pub fn run_composition(
    node: &CompositionNode,
    input: &Value,
    executor: &impl StageExecutor,
    composition_id: &str,
) -> Result<CompositionResult, ExecutionError> {
    let start = Instant::now();
    let mut stage_traces = Vec::new();
    let mut step_counter = 0;

    let output = execute_node(node, input, executor, &mut stage_traces, &mut step_counter)?;

    let duration_ms = start.elapsed().as_millis() as u64;
    let has_failures = stage_traces
        .iter()
        .any(|t| matches!(t.status, StageStatus::Failed { .. }));

    let trace = CompositionTrace {
        composition_id: composition_id.into(),
        started_at: "2026-04-05T10:00:00Z".into(), // simplified for Phase 2
        duration_ms,
        status: if has_failures {
            TraceStatus::Failed
        } else {
            TraceStatus::Ok
        },
        stages: stage_traces,
    };

    Ok(CompositionResult { output, trace })
}

fn execute_node(
    node: &CompositionNode,
    input: &Value,
    executor: &impl StageExecutor,
    traces: &mut Vec<StageTrace>,
    step_counter: &mut usize,
) -> Result<Value, ExecutionError> {
    match node {
        CompositionNode::Stage { id } => execute_stage(id, input, executor, traces, step_counter),
        CompositionNode::Sequential { stages } => {
            let mut current = input.clone();
            for stage in stages {
                current = execute_node(stage, &current, executor, traces, step_counter)?;
            }
            Ok(current)
        }
        CompositionNode::Parallel { branches } => {
            let mut output_fields = serde_json::Map::new();
            for (name, branch) in branches {
                // Each branch gets its corresponding field from the input record
                let branch_input = if let Value::Object(ref obj) = input {
                    obj.get(name).cloned().unwrap_or(Value::Null)
                } else {
                    input.clone()
                };
                let branch_output =
                    execute_node(branch, &branch_input, executor, traces, step_counter)?;
                output_fields.insert(name.clone(), branch_output);
            }
            Ok(Value::Object(output_fields))
        }
        CompositionNode::Branch {
            predicate,
            if_true,
            if_false,
        } => {
            let pred_result = execute_node(predicate, input, executor, traces, step_counter)?;
            let condition = match &pred_result {
                Value::Bool(b) => *b,
                _ => false,
            };
            if condition {
                execute_node(if_true, input, executor, traces, step_counter)
            } else {
                execute_node(if_false, input, executor, traces, step_counter)
            }
        }
        CompositionNode::Fanout { source, targets } => {
            let source_output = execute_node(source, input, executor, traces, step_counter)?;
            let mut results = Vec::new();
            for target in targets {
                let result = execute_node(target, &source_output, executor, traces, step_counter)?;
                results.push(result);
            }
            Ok(Value::Array(results))
        }
        CompositionNode::Merge { sources, target } => {
            let mut merged = serde_json::Map::new();
            for (i, source) in sources.iter().enumerate() {
                let source_input = if let Value::Object(ref obj) = input {
                    obj.get(&format!("source_{i}"))
                        .cloned()
                        .unwrap_or(Value::Null)
                } else {
                    input.clone()
                };
                let result = execute_node(source, &source_input, executor, traces, step_counter)?;
                merged.insert(format!("source_{i}"), result);
            }
            execute_node(
                target,
                &Value::Object(merged),
                executor,
                traces,
                step_counter,
            )
        }
        CompositionNode::Retry {
            stage,
            max_attempts,
            ..
        } => {
            let mut last_err = None;
            for _ in 0..*max_attempts {
                match execute_node(stage, input, executor, traces, step_counter) {
                    Ok(output) => return Ok(output),
                    Err(e) => last_err = Some(e),
                }
            }
            Err(last_err.unwrap_or(ExecutionError::RetryExhausted {
                stage_id: StageId("unknown".into()),
                attempts: *max_attempts,
            }))
        }
    }
}

fn execute_stage(
    id: &StageId,
    input: &Value,
    executor: &impl StageExecutor,
    traces: &mut Vec<StageTrace>,
    step_counter: &mut usize,
) -> Result<Value, ExecutionError> {
    let step_index = *step_counter;
    *step_counter += 1;
    let start = Instant::now();

    let input_hash = hash_value(input);

    match executor.execute(id, input) {
        Ok(output) => {
            let output_hash = hash_value(&output);
            let duration_ms = start.elapsed().as_millis() as u64;
            traces.push(StageTrace {
                stage_id: id.clone(),
                step_index,
                status: StageStatus::Ok,
                duration_ms,
                input_hash: Some(input_hash),
                output_hash: Some(output_hash),
            });
            Ok(output)
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            traces.push(StageTrace {
                stage_id: id.clone(),
                step_index,
                status: StageStatus::Failed {
                    code: "EXECUTION_ERROR".into(),
                    message: format!("{e}"),
                },
                duration_ms,
                input_hash: Some(input_hash),
                output_hash: None,
            });
            Err(e)
        }
    }
}

fn hash_value(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let hash = Sha256::digest(&bytes);
    hex::encode(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::mock::MockExecutor;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn stage(id: &str) -> CompositionNode {
        CompositionNode::Stage {
            id: StageId(id.into()),
        }
    }

    #[test]
    fn run_single_stage() {
        let executor = MockExecutor::new().with_output(&StageId("a".into()), json!(42));
        let result = run_composition(&stage("a"), &json!("input"), &executor, "test_comp").unwrap();
        assert_eq!(result.output, json!(42));
        assert_eq!(result.trace.stages.len(), 1);
        assert!(matches!(result.trace.status, TraceStatus::Ok));
    }

    #[test]
    fn run_sequential() {
        let executor = MockExecutor::new()
            .with_output(&StageId("a".into()), json!("mid"))
            .with_output(&StageId("b".into()), json!("final"));
        let node = CompositionNode::Sequential {
            stages: vec![stage("a"), stage("b")],
        };
        let result = run_composition(&node, &json!("start"), &executor, "test").unwrap();
        assert_eq!(result.output, json!("final"));
        assert_eq!(result.trace.stages.len(), 2);
    }

    #[test]
    fn run_parallel() {
        let executor = MockExecutor::new()
            .with_output(&StageId("s1".into()), json!("r1"))
            .with_output(&StageId("s2".into()), json!("r2"));
        let node = CompositionNode::Parallel {
            branches: BTreeMap::from([("left".into(), stage("s1")), ("right".into(), stage("s2"))]),
        };
        let result = run_composition(&node, &json!({}), &executor, "test").unwrap();
        assert_eq!(result.output, json!({"left": "r1", "right": "r2"}));
    }

    #[test]
    fn run_branch_true() {
        let executor = MockExecutor::new()
            .with_output(&StageId("pred".into()), json!(true))
            .with_output(&StageId("yes".into()), json!("YES"))
            .with_output(&StageId("no".into()), json!("NO"));
        let node = CompositionNode::Branch {
            predicate: Box::new(stage("pred")),
            if_true: Box::new(stage("yes")),
            if_false: Box::new(stage("no")),
        };
        let result = run_composition(&node, &json!("input"), &executor, "test").unwrap();
        assert_eq!(result.output, json!("YES"));
    }

    #[test]
    fn run_branch_false() {
        let executor = MockExecutor::new()
            .with_output(&StageId("pred".into()), json!(false))
            .with_output(&StageId("yes".into()), json!("YES"))
            .with_output(&StageId("no".into()), json!("NO"));
        let node = CompositionNode::Branch {
            predicate: Box::new(stage("pred")),
            if_true: Box::new(stage("yes")),
            if_false: Box::new(stage("no")),
        };
        let result = run_composition(&node, &json!("input"), &executor, "test").unwrap();
        assert_eq!(result.output, json!("NO"));
    }

    #[test]
    fn run_fanout() {
        let executor = MockExecutor::new()
            .with_output(&StageId("src".into()), json!("data"))
            .with_output(&StageId("t1".into()), json!("r1"))
            .with_output(&StageId("t2".into()), json!("r2"));
        let node = CompositionNode::Fanout {
            source: Box::new(stage("src")),
            targets: vec![stage("t1"), stage("t2")],
        };
        let result = run_composition(&node, &json!("in"), &executor, "test").unwrap();
        assert_eq!(result.output, json!(["r1", "r2"]));
    }

    #[test]
    fn trace_has_input_output_hashes() {
        let executor = MockExecutor::new().with_output(&StageId("a".into()), json!(42));
        let result = run_composition(&stage("a"), &json!("input"), &executor, "test").unwrap();
        assert!(result.trace.stages[0].input_hash.is_some());
        assert!(result.trace.stages[0].output_hash.is_some());
    }
}
