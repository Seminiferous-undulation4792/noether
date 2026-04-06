use super::{ExecutionError, StageExecutor};
use noether_core::stage::StageId;
use noether_store::StageStore;
use serde_json::Value;
use std::collections::HashMap;

/// Mock executor that returns pre-configured or example-based outputs.
#[derive(Debug, Default)]
pub struct MockExecutor {
    outputs: HashMap<String, Value>,
}

impl MockExecutor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a fixed output for a specific stage.
    pub fn with_output(mut self, stage_id: &StageId, output: Value) -> Self {
        self.outputs.insert(stage_id.0.clone(), output);
        self
    }

    /// Pre-populate from a store: use each stage's first example output.
    pub fn from_store(store: &(impl StageStore + ?Sized)) -> Self {
        let mut outputs = HashMap::new();
        for stage in store.list(None) {
            if let Some(example) = stage.examples.first() {
                outputs.insert(stage.id.0.clone(), example.output.clone());
            }
        }
        Self { outputs }
    }
}

impl StageExecutor for MockExecutor {
    fn execute(&self, stage_id: &StageId, _input: &Value) -> Result<Value, ExecutionError> {
        match self.outputs.get(&stage_id.0) {
            Some(output) => Ok(output.clone()),
            None => Ok(Value::Null),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mock_returns_configured_output() {
        let id = StageId("abc".into());
        let executor = MockExecutor::new().with_output(&id, json!(42));
        let result = executor.execute(&id, &json!(null)).unwrap();
        assert_eq!(result, json!(42));
    }

    #[test]
    fn mock_returns_null_for_unknown() {
        let executor = MockExecutor::new();
        let result = executor
            .execute(&StageId("unknown".into()), &json!(null))
            .unwrap();
        assert_eq!(result, json!(null));
    }
}
