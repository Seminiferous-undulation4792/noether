use crate::stage::schema::StageId;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

#[derive(Debug, thiserror::Error)]
pub enum SigningError {
    #[error("invalid signature hex encoding")]
    InvalidSignatureEncoding,
    #[error("invalid signature bytes")]
    InvalidSignature,
    #[error("invalid public key hex encoding")]
    InvalidPublicKeyEncoding,
    #[error("invalid public key")]
    InvalidPublicKey,
}

/// Sign a StageId with the given Ed25519 signing key.
/// Returns the hex-encoded signature.
pub fn sign_stage_id(stage_id: &StageId, signing_key: &SigningKey) -> String {
    let sig = signing_key.sign(stage_id.0.as_bytes());
    hex::encode(sig.to_bytes())
}

/// Verify an Ed25519 signature over a StageId.
pub fn verify_stage_signature(
    stage_id: &StageId,
    signature_hex: &str,
    public_key_hex: &str,
) -> Result<bool, SigningError> {
    let sig_bytes =
        hex::decode(signature_hex).map_err(|_| SigningError::InvalidSignatureEncoding)?;
    let sig = Signature::from_slice(&sig_bytes).map_err(|_| SigningError::InvalidSignature)?;

    let pk_bytes =
        hex::decode(public_key_hex).map_err(|_| SigningError::InvalidPublicKeyEncoding)?;
    let pk_array: [u8; 32] = pk_bytes
        .try_into()
        .map_err(|_| SigningError::InvalidPublicKey)?;
    let pk = VerifyingKey::from_bytes(&pk_array).map_err(|_| SigningError::InvalidPublicKey)?;

    Ok(pk.verify(stage_id.0.as_bytes(), &sig).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    #[test]
    fn sign_verify_round_trip() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let stage_id = StageId("abcdef1234567890".into());
        let signature = sign_stage_id(&stage_id, &signing_key);
        let public_key = hex::encode(signing_key.verifying_key().to_bytes());

        let valid = verify_stage_signature(&stage_id, &signature, &public_key).unwrap();
        assert!(valid);
    }

    #[test]
    fn wrong_key_fails_verification() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let other_key = SigningKey::generate(&mut OsRng);
        let stage_id = StageId("abcdef1234567890".into());
        let signature = sign_stage_id(&stage_id, &signing_key);
        let wrong_public_key = hex::encode(other_key.verifying_key().to_bytes());

        let valid = verify_stage_signature(&stage_id, &signature, &wrong_public_key).unwrap();
        assert!(!valid);
    }

    #[test]
    fn tampered_id_fails_verification() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let stage_id = StageId("abcdef1234567890".into());
        let signature = sign_stage_id(&stage_id, &signing_key);
        let public_key = hex::encode(signing_key.verifying_key().to_bytes());

        let tampered_id = StageId("tampered".into());
        let valid = verify_stage_signature(&tampered_id, &signature, &public_key).unwrap();
        assert!(!valid);
    }

    #[test]
    fn invalid_hex_returns_error() {
        let stage_id = StageId("test".into());
        assert!(verify_stage_signature(&stage_id, "not_hex!!!", "also_bad!!!").is_err());
    }
}
