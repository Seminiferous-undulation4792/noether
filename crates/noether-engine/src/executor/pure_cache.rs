//! In-memory cache for Pure stage outputs.
//!
//! A `Pure` stage is deterministic and side-effect-free: for the same input, it
//! always returns the same output.  We can therefore skip execution entirely when
//! we've already seen a `(stage_id, input_hash)` pair during the current run.
//!
//! The cache lives for the duration of a single `run_composition` call and is
//! never persisted to disk.

use noether_core::effects::Effect;
use noether_core::stage::StageId;
use noether_store::StageStore;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct PureStageCache {
    /// Set of stage IDs whose `EffectSet` contains `Effect::Pure`.
    pure_ids: HashSet<String>,
    /// Cache from `(stage_id, input_hash)` to output value.
    entries: HashMap<CacheKey, Value>,
    pub hits: u32,
    pub misses: u32,
}

#[derive(Hash, PartialEq, Eq)]
struct CacheKey {
    stage_id: String,
    input_hash: String,
}

impl PureStageCache {
    /// Build a cache pre-populated with the set of Pure stage IDs from the store.
    pub fn from_store(store: &dyn StageStore) -> Self {
        let pure_ids = store
            .list(None)
            .into_iter()
            .filter(|s| s.signature.effects.contains(&Effect::Pure))
            .map(|s| s.id.0.clone())
            .collect();

        Self {
            pure_ids,
            entries: HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }

    /// Returns `true` when the stage is declared Pure.
    pub fn is_pure(&self, stage_id: &StageId) -> bool {
        self.pure_ids.contains(&stage_id.0)
    }

    /// Look up a cached output. Returns `None` on a cache miss.
    pub fn get(&mut self, stage_id: &StageId, input: &Value) -> Option<&Value> {
        if !self.is_pure(stage_id) {
            return None;
        }
        let key = CacheKey {
            stage_id: stage_id.0.clone(),
            input_hash: hash_value(input),
        };
        if self.entries.contains_key(&key) {
            self.hits += 1;
            self.entries.get(&key)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Store an output in the cache. No-op for non-Pure stages.
    pub fn put(&mut self, stage_id: &StageId, input: &Value, output: Value) {
        if !self.is_pure(stage_id) {
            return;
        }
        let key = CacheKey {
            stage_id: stage_id.0.clone(),
            input_hash: hash_value(input),
        };
        self.entries.insert(key, output);
    }
}

fn hash_value(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    hex::encode(Sha256::digest(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use noether_core::stage::StageId;
    use serde_json::json;

    fn id(s: &str) -> StageId {
        StageId(s.into())
    }

    #[test]
    fn miss_on_non_pure_stage() {
        let mut cache = PureStageCache::default();
        // non_pure_ids not in pure set → always None
        assert!(cache.get(&id("anything"), &json!("input")).is_none());
    }

    #[test]
    fn hit_after_put() {
        let mut cache = PureStageCache::default();
        cache.pure_ids.insert("pure_stage".into());

        let stage = id("pure_stage");
        let input = json!("hello");
        let output = json!(42);

        assert!(cache.get(&stage, &input).is_none());
        cache.put(&stage, &input, output.clone());
        let cached = cache.get(&stage, &input).unwrap();
        assert_eq!(*cached, output);
        assert_eq!(cache.hits, 1);
    }

    #[test]
    fn different_inputs_produce_different_keys() {
        let mut cache = PureStageCache::default();
        cache.pure_ids.insert("pure_stage".into());

        let stage = id("pure_stage");
        cache.put(&stage, &json!("foo"), json!(1));
        cache.put(&stage, &json!("bar"), json!(2));

        assert_eq!(*cache.get(&stage, &json!("foo")).unwrap(), json!(1));
        assert_eq!(*cache.get(&stage, &json!("bar")).unwrap(), json!(2));
    }
}
