use tl_core::SourceLabelPolicy;

/// Reject overrides that set nothing: a policy row must override at
/// least one label family or it has no effect. Origins and label values
/// are enums and are already validated by serde at the boundary.
pub(super) fn validate_policy(policy: &SourceLabelPolicy) -> Result<(), String> {
    if policy.trust.is_none() && policy.confidentiality.is_none() && policy.integrity.is_none() {
        return Err(
            "policy must set at least one of trust, confidentiality, integrity".to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::{Confidentiality, Integrity, Origin, Trust};

    fn policy(
        trust: Option<Trust>,
        confidentiality: Option<Confidentiality>,
        integrity: Option<Integrity>,
    ) -> SourceLabelPolicy {
        SourceLabelPolicy {
            origin: Origin::Web,
            trust,
            confidentiality,
            integrity,
        }
    }

    #[test]
    fn accepts_single_family_override() {
        assert!(validate_policy(&policy(Some(Trust::Untrusted), None, None)).is_ok());
        assert!(validate_policy(&policy(None, Some(Confidentiality::Private), None)).is_ok());
        assert!(validate_policy(&policy(None, None, Some(Integrity::Low))).is_ok());
    }

    #[test]
    fn accepts_full_override() {
        assert!(validate_policy(&policy(
            Some(Trust::Trusted),
            Some(Confidentiality::Secret),
            Some(Integrity::High),
        ))
        .is_ok());
    }

    #[test]
    fn rejects_empty_override() {
        let err = validate_policy(&policy(None, None, None)).unwrap_err();
        assert!(err.contains("at least one"));
    }
}
