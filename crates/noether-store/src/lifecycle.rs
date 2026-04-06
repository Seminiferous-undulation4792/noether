use noether_core::stage::StageLifecycle;

/// Validate a lifecycle transition.
///
/// Allowed transitions:
/// - Draft → Active (stage passes test harness)
/// - Active → Deprecated (superseded by successor)
/// - Active → Tombstone (security removal)
pub fn validate_transition(from: &StageLifecycle, to: &StageLifecycle) -> Result<(), String> {
    match (from, to) {
        (StageLifecycle::Draft, StageLifecycle::Active) => Ok(()),
        (StageLifecycle::Active, StageLifecycle::Deprecated { .. }) => Ok(()),
        (StageLifecycle::Active, StageLifecycle::Tombstone) => Ok(()),
        (from, to) => Err(format!("invalid lifecycle transition: {from:?} → {to:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use noether_core::stage::StageId;

    #[test]
    fn valid_transitions() {
        assert!(validate_transition(&StageLifecycle::Draft, &StageLifecycle::Active).is_ok());
        assert!(validate_transition(
            &StageLifecycle::Active,
            &StageLifecycle::Deprecated {
                successor_id: StageId("abc".into())
            }
        )
        .is_ok());
        assert!(validate_transition(&StageLifecycle::Active, &StageLifecycle::Tombstone).is_ok());
    }

    #[test]
    fn invalid_transitions() {
        assert!(validate_transition(&StageLifecycle::Draft, &StageLifecycle::Tombstone).is_err());
        assert!(validate_transition(
            &StageLifecycle::Deprecated {
                successor_id: StageId("x".into())
            },
            &StageLifecycle::Active
        )
        .is_err());
        assert!(validate_transition(&StageLifecycle::Tombstone, &StageLifecycle::Active).is_err());
        assert!(validate_transition(&StageLifecycle::Draft, &StageLifecycle::Draft).is_err());
    }
}
