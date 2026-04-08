use crate::lagrange::CompositionGraph;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A cached composition entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedComposition {
    /// The original problem description (for display/debugging).
    pub problem: String,
    /// The resolved composition graph.
    pub graph: CompositionGraph,
    /// Unix timestamp of when this was cached.
    pub cached_at: u64,
    /// Which model produced this graph.
    pub model: String,
}

/// Persistent cache that maps normalized problem hashes to composition graphs.
///
/// Stored as JSON at `~/.noether/compositions.json` (or `NOETHER_HOME/compositions.json`).
/// `--force` bypasses the cache and always re-runs the LLM.
pub struct CompositionCache {
    path: PathBuf,
    entries: HashMap<String, CachedComposition>,
}

impl CompositionCache {
    /// Open or create the cache at `path`.
    pub fn open(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        let entries = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            HashMap::new()
        };
        Self { path, entries }
    }

    /// Look up a problem. Returns `None` if not cached or cache is empty.
    pub fn get(&self, problem: &str) -> Option<&CachedComposition> {
        let key = normalize_key(problem);
        self.entries.get(&key)
    }

    /// Store a new composition result.
    pub fn insert(&mut self, problem: &str, graph: CompositionGraph, model: &str) {
        let key = normalize_key(problem);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.entries.insert(
            key,
            CachedComposition {
                problem: problem.to_string(),
                graph,
                cached_at: now,
                model: model.to_string(),
            },
        );
        // Best-effort persist; failures are not fatal.
        let _ = self.save();
    }

    /// Number of cached compositions.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.entries).map_err(std::io::Error::other)?;
        std::fs::write(&self.path, json)
    }
}

/// SHA-256 of the lower-cased, whitespace-normalized problem text.
fn normalize_key(problem: &str) -> String {
    let normalized = problem
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    hex::encode(Sha256::digest(normalized.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lagrange::{parse_graph, CompositionGraph};
    use tempfile::NamedTempFile;

    fn dummy_graph() -> CompositionGraph {
        parse_graph(r#"{"description":"test","version":"0.1.0","root":{"op":"Stage","id":"abc"}}"#)
            .unwrap()
    }

    #[test]
    fn cache_roundtrip() {
        let tmp = NamedTempFile::new().unwrap();
        let mut cache = CompositionCache::open(tmp.path());
        assert!(cache.get("hello world").is_none());

        cache.insert("hello world", dummy_graph(), "test-model");
        let hit = cache.get("hello world").unwrap();
        assert_eq!(hit.problem, "hello world");
        assert_eq!(hit.model, "test-model");
    }

    #[test]
    fn cache_key_normalizes_whitespace_and_case() {
        let tmp = NamedTempFile::new().unwrap();
        let mut cache = CompositionCache::open(tmp.path());
        cache.insert("hello  WORLD", dummy_graph(), "m");

        // Different whitespace / case → same key
        assert!(cache.get("hello world").is_some());
        assert!(cache.get("HELLO WORLD").is_some());
        assert!(cache.get("  hello   world  ").is_some());
    }

    #[test]
    fn cache_persists_across_reopen() {
        let tmp = NamedTempFile::new().unwrap();
        {
            let mut cache = CompositionCache::open(tmp.path());
            cache.insert("persist me", dummy_graph(), "m");
        }
        let cache2 = CompositionCache::open(tmp.path());
        assert!(cache2.get("persist me").is_some());
    }
}
