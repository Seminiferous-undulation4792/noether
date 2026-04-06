use crate::executor::ExecutionError;
use noether_core::stage::StageId;
use serde_json::Value;

pub fn to_text(input: &Value) -> Result<Value, ExecutionError> {
    let text = match input {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => serde_json::to_string(other).unwrap_or_else(|_| format!("{other}")),
    };
    Ok(Value::String(text))
}

pub fn to_number(input: &Value) -> Result<Value, ExecutionError> {
    let n = match input {
        Value::Number(n) => return Ok(Value::Number(n.clone())),
        Value::Bool(true) => 1.0,
        Value::Bool(false) => 0.0,
        Value::String(s) => s.parse::<f64>().map_err(|_| ExecutionError::StageFailed {
            stage_id: StageId("to_number".into()),
            message: format!("cannot parse '{s}' as number"),
        })?,
        other => {
            return Err(ExecutionError::StageFailed {
                stage_id: StageId("to_number".into()),
                message: format!("cannot convert {other} to number"),
            })
        }
    };
    Ok(serde_json::json!(n))
}

pub fn to_bool(input: &Value) -> Result<Value, ExecutionError> {
    let b = match input {
        Value::Bool(b) => *b,
        Value::Null => false,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        _ => true,
    };
    Ok(Value::Bool(b))
}

pub fn parse_json(input: &Value) -> Result<Value, ExecutionError> {
    let s = input.as_str().ok_or_else(|| ExecutionError::StageFailed {
        stage_id: StageId("parse_json".into()),
        message: "input must be a string".into(),
    })?;
    serde_json::from_str(s).map_err(|e| ExecutionError::StageFailed {
        stage_id: StageId("parse_json".into()),
        message: format!("invalid JSON: {e}"),
    })
}

pub fn to_json(input: &Value) -> Result<Value, ExecutionError> {
    let json_str = serde_json::to_string(input).map_err(|e| ExecutionError::StageFailed {
        stage_id: StageId("to_json".into()),
        message: format!("serialization failed: {e}"),
    })?;
    Ok(Value::String(json_str))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_to_text() {
        assert_eq!(to_text(&json!(42)).unwrap(), json!("42"));
        assert_eq!(to_text(&json!(true)).unwrap(), json!("true"));
        assert_eq!(to_text(&json!(null)).unwrap(), json!("null"));
        assert_eq!(to_text(&json!("hello")).unwrap(), json!("hello"));
    }

    #[test]
    fn test_to_number() {
        assert_eq!(to_number(&json!("42")).unwrap(), json!(42.0));
        assert_eq!(to_number(&json!(true)).unwrap(), json!(1.0));
        assert_eq!(to_number(&json!(false)).unwrap(), json!(0.0));
        assert_eq!(to_number(&json!(99)).unwrap(), json!(99));
        assert!(to_number(&json!("not_a_number")).is_err());
    }

    #[test]
    fn test_to_bool() {
        assert_eq!(to_bool(&json!(true)).unwrap(), json!(true));
        assert_eq!(to_bool(&json!(false)).unwrap(), json!(false));
        assert_eq!(to_bool(&json!(0)).unwrap(), json!(false));
        assert_eq!(to_bool(&json!(1)).unwrap(), json!(true));
        assert_eq!(to_bool(&json!(null)).unwrap(), json!(false));
        assert_eq!(to_bool(&json!("")).unwrap(), json!(false));
        assert_eq!(to_bool(&json!("hello")).unwrap(), json!(true));
    }

    #[test]
    fn test_parse_json() {
        assert_eq!(parse_json(&json!("42")).unwrap(), json!(42));
        assert_eq!(parse_json(&json!(r#"{"a":1}"#)).unwrap(), json!({"a": 1}));
        assert!(parse_json(&json!("invalid{")).is_err());
    }

    #[test]
    fn test_to_json() {
        assert_eq!(to_json(&json!(42)).unwrap(), json!("42"));
        assert_eq!(to_json(&json!({"a": 1})).unwrap(), json!(r#"{"a":1}"#));
    }
}
