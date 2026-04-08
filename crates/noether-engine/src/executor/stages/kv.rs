use crate::executor::ExecutionError;
use noether_core::stage::StageId;
use rusqlite::{params, Connection};
use serde_json::Value;
use std::sync::{Mutex, OnceLock};

fn fail(stage: &str, msg: impl Into<String>) -> ExecutionError {
    ExecutionError::StageFailed {
        stage_id: StageId(stage.into()),
        message: msg.into(),
    }
}

// ── Shared SQLite connection ─────────────────────────────────────────────────

static KV_CONN: OnceLock<Mutex<Connection>> = OnceLock::new();

fn kv_path() -> std::path::PathBuf {
    std::env::var("NOETHER_KV_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            std::path::Path::new(&home).join(".noether").join("kv.db")
        })
}

fn with_conn<F, T>(f: F) -> Result<T, ExecutionError>
where
    F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
{
    let mutex = KV_CONN.get_or_init(|| {
        let path = kv_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(&path).expect("failed to open kv.db");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kv (
                namespace TEXT NOT NULL DEFAULT '',
                key       TEXT NOT NULL,
                value     TEXT NOT NULL,
                PRIMARY KEY (namespace, key)
            );",
        )
        .expect("failed to create kv table");
        Mutex::new(conn)
    });

    let conn = mutex.lock().map_err(|e| ExecutionError::StageFailed {
        stage_id: StageId("kv".into()),
        message: format!("kv lock poisoned: {e}"),
    })?;
    f(&conn).map_err(|e| ExecutionError::StageFailed {
        stage_id: StageId("kv".into()),
        message: e.to_string(),
    })
}

fn ns(input: &Value) -> &str {
    input
        .get("namespace")
        .and_then(|v| v.as_str())
        .unwrap_or("")
}

// ── Stage implementations ────────────────────────────────────────────────────

pub fn kv_set(input: &Value) -> Result<Value, ExecutionError> {
    let key = input
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| fail("kv_set", "missing field 'key'"))?;
    let value = input
        .get("value")
        .ok_or_else(|| fail("kv_set", "missing field 'value'"))?;
    let ns = ns(input);
    let serialized = serde_json::to_string(value).map_err(|e| fail("kv_set", e.to_string()))?;

    with_conn(|conn| {
        conn.execute(
            "INSERT INTO kv (namespace, key, value) VALUES (?1, ?2, ?3)
             ON CONFLICT(namespace, key) DO UPDATE SET value = excluded.value",
            params![ns, key, serialized],
        )?;
        Ok(Value::String("ok".into()))
    })
}

pub fn kv_get(input: &Value) -> Result<Value, ExecutionError> {
    let key = input
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| fail("kv_get", "missing field 'key'"))?;
    let ns = ns(input);

    with_conn(|conn| {
        let mut stmt = conn.prepare("SELECT value FROM kv WHERE namespace = ?1 AND key = ?2")?;
        let mut rows = stmt.query(params![ns, key])?;
        match rows.next()? {
            Some(row) => {
                let s: String = row.get(0)?;
                let v: Value = serde_json::from_str(&s).unwrap_or(Value::Null);
                Ok(v)
            }
            None => Ok(Value::Null),
        }
    })
}

pub fn kv_delete(input: &Value) -> Result<Value, ExecutionError> {
    let key = input
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| fail("kv_delete", "missing field 'key'"))?;
    let ns = ns(input);

    with_conn(|conn| {
        let changed = conn.execute(
            "DELETE FROM kv WHERE namespace = ?1 AND key = ?2",
            params![ns, key],
        )?;
        Ok(Value::Bool(changed > 0))
    })
}

pub fn kv_exists(input: &Value) -> Result<Value, ExecutionError> {
    let key = input
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| fail("kv_exists", "missing field 'key'"))?;
    let ns = ns(input);

    with_conn(|conn| {
        let mut stmt =
            conn.prepare("SELECT 1 FROM kv WHERE namespace = ?1 AND key = ?2 LIMIT 1")?;
        let exists = stmt.exists(params![ns, key])?;
        Ok(Value::Bool(exists))
    })
}

pub fn kv_list(input: &Value) -> Result<Value, ExecutionError> {
    let prefix = input.get("prefix").and_then(|v| v.as_str()).unwrap_or("");
    let ns = ns(input);

    with_conn(|conn| {
        let pattern = format!("{prefix}%");
        let mut stmt =
            conn.prepare("SELECT key FROM kv WHERE namespace = ?1 AND key LIKE ?2 ORDER BY key")?;
        let keys: Result<Vec<Value>, _> = stmt
            .query_map(params![ns, pattern], |row| {
                let k: String = row.get(0)?;
                Ok(Value::String(k))
            })?
            .collect();
        Ok(Value::Array(keys?))
    })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};

    // Each test uses a unique namespace to stay isolated across runs.
    fn ns_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let pid = std::process::id();
        let seq = COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("test:{pid}:{ts}:{seq}")
    }

    #[test]
    fn set_and_get_string() {
        let ns = ns_id();
        kv_set(&json!({"key": "k1", "value": "hello", "namespace": ns})).unwrap();
        let v = kv_get(&json!({"key": "k1", "namespace": ns})).unwrap();
        assert_eq!(v, json!("hello"));
    }

    #[test]
    fn get_missing_returns_null() {
        let ns = ns_id();
        let v = kv_get(&json!({"key": "nope", "namespace": ns})).unwrap();
        assert_eq!(v, json!(null));
    }

    #[test]
    fn set_and_get_json_value() {
        let ns = ns_id();
        kv_set(&json!({"key": "obj", "value": {"x": 1, "y": [2,3]}, "namespace": ns})).unwrap();
        let v = kv_get(&json!({"key": "obj", "namespace": ns})).unwrap();
        assert_eq!(v, json!({"x": 1, "y": [2, 3]}));
    }

    #[test]
    fn set_overwrites_existing() {
        let ns = ns_id();
        kv_set(&json!({"key": "k", "value": 1, "namespace": ns})).unwrap();
        kv_set(&json!({"key": "k", "value": 2, "namespace": ns})).unwrap();
        let v = kv_get(&json!({"key": "k", "namespace": ns})).unwrap();
        assert_eq!(v, json!(2));
    }

    #[test]
    fn delete_existing_returns_true() {
        let ns = ns_id();
        kv_set(&json!({"key": "d1", "value": "x", "namespace": ns})).unwrap();
        let r = kv_delete(&json!({"key": "d1", "namespace": ns})).unwrap();
        assert_eq!(r, json!(true));
        let after = kv_get(&json!({"key": "d1", "namespace": ns})).unwrap();
        assert_eq!(after, json!(null));
    }

    #[test]
    fn delete_missing_returns_false() {
        let ns = ns_id();
        let r = kv_delete(&json!({"key": "nope", "namespace": ns})).unwrap();
        assert_eq!(r, json!(false));
    }

    #[test]
    fn exists_true_and_false() {
        let ns = ns_id();
        kv_set(&json!({"key": "e1", "value": 0, "namespace": ns})).unwrap();
        assert_eq!(
            kv_exists(&json!({"key": "e1", "namespace": ns})).unwrap(),
            json!(true)
        );
        assert_eq!(
            kv_exists(&json!({"key": "nope", "namespace": ns})).unwrap(),
            json!(false)
        );
    }

    #[test]
    fn list_by_prefix() {
        let ns = ns_id();
        kv_set(&json!({"key": "aa:1", "value": 1, "namespace": ns})).unwrap();
        kv_set(&json!({"key": "aa:2", "value": 2, "namespace": ns})).unwrap();
        kv_set(&json!({"key": "bb:1", "value": 3, "namespace": ns})).unwrap();

        let keys = kv_list(&json!({"prefix": "aa:", "namespace": ns})).unwrap();
        assert_eq!(keys, json!(["aa:1", "aa:2"]));

        let all = kv_list(&json!({"prefix": "", "namespace": ns})).unwrap();
        assert_eq!(all.as_array().unwrap().len(), 3);
    }

    #[test]
    fn namespaces_are_isolated() {
        let ns1 = ns_id();
        let ns2 = ns_id();
        kv_set(&json!({"key": "k", "value": "ns1", "namespace": ns1})).unwrap();
        let v1 = kv_get(&json!({"key": "k", "namespace": ns1})).unwrap();
        let v2 = kv_get(&json!({"key": "k", "namespace": ns2})).unwrap();
        assert_eq!(v1, json!("ns1"));
        assert_eq!(v2, json!(null));
    }
}
