use crate::executor::ExecutionError;
use noether_core::stage::StageId;
use serde_json::{json, Value};

fn fail(stage: &str, msg: impl Into<String>) -> ExecutionError {
    ExecutionError::StageFailed {
        stage_id: StageId(stage.into()),
        message: msg.into(),
    }
}

pub fn json_merge(input: &Value) -> Result<Value, ExecutionError> {
    let base = input
        .get("base")
        .ok_or_else(|| fail("json_merge", "missing field 'base'"))?;
    let patch = input
        .get("patch")
        .ok_or_else(|| fail("json_merge", "missing field 'patch'"))?;
    Ok(merge_deep(base, patch))
}

fn merge_deep(base: &Value, patch: &Value) -> Value {
    match (base, patch) {
        (Value::Object(base_map), Value::Object(patch_map)) => {
            let mut result = base_map.clone();
            for (key, patch_val) in patch_map {
                let merged = if let Some(base_val) = result.get(key) {
                    merge_deep(base_val, patch_val)
                } else {
                    patch_val.clone()
                };
                result.insert(key.clone(), merged);
            }
            Value::Object(result)
        }
        (_, patch) => patch.clone(),
    }
}

pub fn json_path(input: &Value) -> Result<Value, ExecutionError> {
    let data = input
        .get("data")
        .ok_or_else(|| fail("json_path", "missing field 'data'"))?;
    let path = input
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| fail("json_path", "path must be a string"))?;

    // Simple JSONPath: support $.field.field and $[index]
    let result = resolve_path(data, path)?;
    Ok(result)
}

fn resolve_path(data: &Value, path: &str) -> Result<Value, ExecutionError> {
    let path = path.strip_prefix('$').unwrap_or(path);

    let mut current = data.clone();
    let mut remaining = path;

    while !remaining.is_empty() {
        remaining = remaining.strip_prefix('.').unwrap_or(remaining);
        if remaining.is_empty() {
            break;
        }

        if remaining.starts_with('[') {
            // Array index
            let end = remaining
                .find(']')
                .ok_or_else(|| fail("json_path", "unclosed bracket"))?;
            let idx_str = &remaining[1..end];
            let idx: usize = idx_str
                .parse()
                .map_err(|_| fail("json_path", format!("invalid index: {idx_str}")))?;
            current = current
                .get(idx)
                .cloned()
                .ok_or_else(|| fail("json_path", format!("index {idx} out of bounds")))?;
            remaining = &remaining[end + 1..];
        } else {
            // Object field
            let end = remaining.find(['.', '[']).unwrap_or(remaining.len());
            let field = &remaining[..end];
            current = current
                .get(field)
                .cloned()
                .ok_or_else(|| fail("json_path", format!("field '{field}' not found")))?;
            remaining = &remaining[end..];
        }
    }

    Ok(current)
}

pub fn json_schema_validate(input: &Value) -> Result<Value, ExecutionError> {
    let data = input
        .get("data")
        .ok_or_else(|| fail("json_schema_validate", "missing field 'data'"))?;
    let schema = input
        .get("schema")
        .ok_or_else(|| fail("json_schema_validate", "missing field 'schema'"))?;

    // Simple type validation based on schema's "type" field
    let mut errors = Vec::new();
    if let Some(expected_type) = schema.get("type").and_then(|v| v.as_str()) {
        let actual_type = match data {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        };
        if actual_type != expected_type {
            errors.push(format!("expected {expected_type}, got {actual_type}"));
        }
    }

    Ok(json!({
        "valid": errors.is_empty(),
        "errors": errors,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_merge_objects() {
        let result = json_merge(&json!({"base": {"a": 1}, "patch": {"b": 2}})).unwrap();
        assert_eq!(result, json!({"a": 1, "b": 2}));
    }

    #[test]
    fn test_json_merge_deep() {
        let result =
            json_merge(&json!({"base": {"a": {"b": 1}}, "patch": {"a": {"c": 2}}})).unwrap();
        assert_eq!(result, json!({"a": {"b": 1, "c": 2}}));
    }

    #[test]
    fn test_json_merge_override() {
        let result = json_merge(&json!({"base": {"a": 1}, "patch": {"a": 2}})).unwrap();
        assert_eq!(result, json!({"a": 2}));
    }

    #[test]
    fn test_json_path_field() {
        let result = json_path(&json!({"data": {"a": {"b": 42}}, "path": "$.a.b"})).unwrap();
        assert_eq!(result, json!(42));
    }

    #[test]
    fn test_json_path_array() {
        let result = json_path(&json!({"data": [10, 20, 30], "path": "$[1]"})).unwrap();
        assert_eq!(result, json!(20));
    }

    #[test]
    fn test_json_path_nested() {
        let result =
            json_path(&json!({"data": {"items": [1, 2, 3]}, "path": "$.items[0]"})).unwrap();
        assert_eq!(result, json!(1));
    }

    #[test]
    fn test_json_schema_validate_pass() {
        let result =
            json_schema_validate(&json!({"data": "hello", "schema": {"type": "string"}})).unwrap();
        assert_eq!(result, json!({"valid": true, "errors": []}));
    }

    #[test]
    fn test_json_schema_validate_fail() {
        let result =
            json_schema_validate(&json!({"data": 42, "schema": {"type": "string"}})).unwrap();
        assert_eq!(result["valid"], false);
        assert!(!result["errors"].as_array().unwrap().is_empty());
    }
}
