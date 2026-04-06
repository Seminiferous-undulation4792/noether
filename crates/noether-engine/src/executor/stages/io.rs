use crate::executor::ExecutionError;
use noether_core::stage::StageId;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::io::Read;

fn fail(stage: &str, msg: impl Into<String>) -> ExecutionError {
    ExecutionError::StageFailed {
        stage_id: StageId(stage.into()),
        message: msg.into(),
    }
}

// ── File I/O ────────────────────────────────────────────────────────────────

pub fn read_file(input: &Value) -> Result<Value, ExecutionError> {
    let path = input
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| fail("read_file", "missing field 'path'"))?;

    let content = std::fs::read_to_string(path).map_err(|e| fail("read_file", e.to_string()))?;
    let size = content.len() as u64;
    Ok(json!({"content": content, "size_bytes": size}))
}

pub fn write_file(input: &Value) -> Result<Value, ExecutionError> {
    let path = input
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| fail("write_file", "missing field 'path'"))?;
    let content = input
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| fail("write_file", "missing field 'content'"))?;

    std::fs::write(path, content).map_err(|e| fail("write_file", e.to_string()))?;
    Ok(json!({"path": path, "bytes_written": content.len()}))
}

pub fn stdin_read(_input: &Value) -> Result<Value, ExecutionError> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| fail("stdin_read", e.to_string()))?;
    Ok(Value::String(buf))
}

pub fn stdout_write(input: &Value) -> Result<Value, ExecutionError> {
    let text = input
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| fail("stdout_write", "missing field 'text'"))?;
    print!("{text}");
    Ok(Value::Null)
}

pub fn env_get(input: &Value) -> Result<Value, ExecutionError> {
    let name = input
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| fail("env_get", "missing field 'name'"))?;
    match std::env::var(name) {
        Ok(val) => Ok(Value::String(val)),
        Err(std::env::VarError::NotPresent) => Ok(Value::Null),
        Err(e) => Err(fail("env_get", e.to_string())),
    }
}

// ── HTTP ─────────────────────────────────────────────────────────────────────

fn collect_headers(resp: &reqwest::blocking::Response) -> Value {
    let mut map = Map::new();
    for (name, value) in resp.headers() {
        if let Ok(v) = value.to_str() {
            map.insert(name.as_str().to_string(), Value::String(v.to_string()));
        }
    }
    Value::Object(map)
}

fn build_request_headers(input: &Value) -> HashMap<String, String> {
    input
        .get("headers")
        .and_then(|v| v.as_object())
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

pub fn http_get(input: &Value) -> Result<Value, ExecutionError> {
    let url = input
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| fail("http_get", "missing field 'url'"))?;

    let client = reqwest::blocking::Client::new();
    let mut req = client.get(url);
    for (k, v) in build_request_headers(input) {
        req = req.header(k, v);
    }

    let resp = req.send().map_err(|e| fail("http_get", e.to_string()))?;
    let status = resp.status().as_u16() as i64;
    let headers = collect_headers(&resp);
    let body = resp.text().map_err(|e| fail("http_get", e.to_string()))?;
    Ok(json!({"status": status, "body": body, "headers": headers}))
}

pub fn http_post(input: &Value) -> Result<Value, ExecutionError> {
    let url = input
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| fail("http_post", "missing field 'url'"))?;
    let body = input
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or_else(|| fail("http_post", "missing field 'body'"))?;
    let content_type = input
        .get("content_type")
        .and_then(|v| v.as_str())
        .unwrap_or("application/json");

    let client = reqwest::blocking::Client::new();
    let mut req = client
        .post(url)
        .header("content-type", content_type)
        .body(body.to_string());
    for (k, v) in build_request_headers(input) {
        req = req.header(k, v);
    }

    let resp = req.send().map_err(|e| fail("http_post", e.to_string()))?;
    let status = resp.status().as_u16() as i64;
    let headers = collect_headers(&resp);
    let body = resp.text().map_err(|e| fail("http_post", e.to_string()))?;
    Ok(json!({"status": status, "body": body, "headers": headers}))
}

pub fn http_put(input: &Value) -> Result<Value, ExecutionError> {
    let url = input
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| fail("http_put", "missing field 'url'"))?;
    let body = input
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or_else(|| fail("http_put", "missing field 'body'"))?;
    let content_type = input
        .get("content_type")
        .and_then(|v| v.as_str())
        .unwrap_or("application/json");

    let client = reqwest::blocking::Client::new();
    let mut req = client
        .put(url)
        .header("content-type", content_type)
        .body(body.to_string());
    for (k, v) in build_request_headers(input) {
        req = req.header(k, v);
    }

    let resp = req.send().map_err(|e| fail("http_put", e.to_string()))?;
    let status = resp.status().as_u16() as i64;
    let headers = collect_headers(&resp);
    let body = resp.text().map_err(|e| fail("http_put", e.to_string()))?;
    Ok(json!({"status": status, "body": body, "headers": headers}))
}
