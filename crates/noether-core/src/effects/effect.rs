use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "effect")]
pub enum Effect {
    Cost { cents: u64 },
    Fallible,
    Llm { model: String },
    Network,
    NonDeterministic,
    Pure,
    Unknown,
}

/// An ordered set of effects declared on a stage.
///
/// Uses `BTreeSet` for deterministic serialization order, which is
/// critical for canonical JSON hashing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectSet {
    effects: BTreeSet<Effect>,
}

impl EffectSet {
    pub fn unknown() -> Self {
        Self {
            effects: BTreeSet::from([Effect::Unknown]),
        }
    }

    pub fn pure() -> Self {
        Self {
            effects: BTreeSet::from([Effect::Pure]),
        }
    }

    pub fn new(effects: impl IntoIterator<Item = Effect>) -> Self {
        Self {
            effects: effects.into_iter().collect(),
        }
    }

    pub fn contains(&self, effect: &Effect) -> bool {
        self.effects.contains(effect)
    }

    pub fn is_unknown(&self) -> bool {
        self.effects.contains(&Effect::Unknown)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Effect> {
        self.effects.iter()
    }
}

impl Default for EffectSet {
    fn default() -> Self {
        Self::unknown()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_unknown() {
        let es = EffectSet::default();
        assert!(es.is_unknown());
        assert!(es.contains(&Effect::Unknown));
    }

    #[test]
    fn pure_does_not_contain_unknown() {
        let es = EffectSet::pure();
        assert!(!es.is_unknown());
        assert!(es.contains(&Effect::Pure));
    }

    #[test]
    fn serde_round_trip() {
        let es = EffectSet::new([
            Effect::Network,
            Effect::Fallible,
            Effect::Llm {
                model: "claude-sonnet-4".into(),
            },
        ]);
        let json = serde_json::to_string(&es).unwrap();
        let deserialized: EffectSet = serde_json::from_str(&json).unwrap();
        assert_eq!(es, deserialized);
    }
}
