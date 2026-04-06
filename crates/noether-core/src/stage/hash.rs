use crate::stage::schema::{StageId, StageSignature};
use sha2::{Digest, Sha256};

/// Produce the canonical JSON bytes for a StageSignature.
///
/// Determinism is guaranteed by:
/// - `BTreeMap` for Record fields (sorted keys)
/// - `BTreeSet` for EffectSet (sorted elements)
/// - `serde_json::to_vec` (compact, no whitespace)
pub fn canonical_json(sig: &StageSignature) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(sig)
}

/// Compute the content-addressed StageId from a StageSignature.
///
/// The identity is the hex-encoded SHA-256 of the canonical JSON
/// serialization of the signature.
pub fn compute_stage_id(sig: &StageSignature) -> Result<StageId, serde_json::Error> {
    let bytes = canonical_json(sig)?;
    let hash = Sha256::digest(&bytes);
    Ok(StageId(hex::encode(hash)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::EffectSet;
    use crate::types::NType;

    fn sample_sig() -> StageSignature {
        StageSignature {
            input: NType::Text,
            output: NType::Number,
            effects: EffectSet::pure(),
            implementation_hash: "abc123".into(),
        }
    }

    #[test]
    fn hash_is_deterministic() {
        let sig = sample_sig();
        let id1 = compute_stage_id(&sig).unwrap();
        let id2 = compute_stage_id(&sig).unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn different_signatures_produce_different_ids() {
        let sig1 = sample_sig();
        let mut sig2 = sample_sig();
        sig2.output = NType::Text;
        let id1 = compute_stage_id(&sig1).unwrap();
        let id2 = compute_stage_id(&sig2).unwrap();
        assert_ne!(id1, id2);
    }

    #[test]
    fn hash_is_64_hex_chars() {
        let id = compute_stage_id(&sample_sig()).unwrap();
        assert_eq!(id.0.len(), 64);
        assert!(id.0.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn canonical_json_round_trip_preserves_hash() {
        let sig = sample_sig();
        let json = canonical_json(&sig).unwrap();
        let deserialized: StageSignature = serde_json::from_slice(&json).unwrap();
        let json2 = canonical_json(&deserialized).unwrap();
        assert_eq!(json, json2);
    }
}
