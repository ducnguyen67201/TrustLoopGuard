//! Server-edge validation for tool metadata payloads. Enum values
//! (side effects, param roles, source origins) are already enforced by
//! serde at parse time; this checks the constraints serde cannot.

use tl_core::ToolMetadata;

/// Size bounds for registry payloads. Resolved metadata is cached on the
/// per-event hot path and re-embedded in every resolved trace event, so
/// oversized entries amplify memory and storage cost well beyond the
/// upsert itself.
const MAX_TOOL_NAME_LEN: usize = 256;
const MAX_PARAMS: usize = 100;
const MAX_PARAM_PATH_LEN: usize = 512;
const MAX_APPROVER_ROLES: usize = 32;
const MAX_APPROVER_ROLE_LEN: usize = 256;
const MAX_APPROVAL_REASON_BYTES: usize = 1024;
const MAX_SANDBOX_HINT_BYTES: usize = 4096;

pub(super) fn validate_metadata(metadata: &ToolMetadata) -> Result<(), String> {
    if metadata.tool.trim().is_empty() {
        return Err("tool is required".into());
    }
    if metadata.tool.len() > MAX_TOOL_NAME_LEN {
        return Err(format!("tool name exceeds {MAX_TOOL_NAME_LEN} bytes"));
    }
    if metadata.tool.contains('\0') {
        return Err("tool name must not contain NUL bytes".into());
    }
    if metadata.params.len() > MAX_PARAMS {
        return Err(format!("params exceeds {MAX_PARAMS} entries"));
    }
    let mut seen_paths = std::collections::HashSet::new();
    for param in &metadata.params {
        if param.path.len() > MAX_PARAM_PATH_LEN {
            return Err(format!("param path exceeds {MAX_PARAM_PATH_LEN} bytes"));
        }
        let path = param.path.trim();
        if path.is_empty() {
            return Err("param path must not be empty".into());
        }
        if !seen_paths.insert(path) {
            return Err(format!("duplicate param path `{path}`"));
        }
        for source in &param.allowed_sources {
            if source
                .source_id
                .as_deref()
                .is_some_and(|s| s.trim().is_empty())
            {
                return Err(format!(
                    "param `{path}`: allowed source_id must not be blank"
                ));
            }
            if source.kind.as_deref().is_some_and(|s| s.trim().is_empty()) {
                return Err(format!(
                    "param `{path}`: allowed source kind must not be blank"
                ));
            }
        }
        // A value limit that sets no bound is a no-op that reads as
        // protection, and an inverted range is unsatisfiable; both are
        // operator mistakes we reject at registration rather than letting
        // them silently mis-guard a money parameter.
        if let Some(limit) = &param.limit {
            match (limit.min, limit.max) {
                (None, None) => {
                    return Err(format!(
                        "param `{path}`: limit must set at least one of min or max"
                    ));
                }
                (Some(min), Some(max)) if min > max => {
                    return Err(format!(
                        "param `{path}`: limit min ({min}) must not exceed max ({max})"
                    ));
                }
                _ => {}
            }
        }
    }
    if let Some(approval) = &metadata.approval {
        if approval.approver_roles.len() > MAX_APPROVER_ROLES {
            return Err(format!(
                "approval approver_roles exceeds {MAX_APPROVER_ROLES} entries"
            ));
        }
        if approval.approver_roles.iter().any(|r| r.trim().is_empty()) {
            return Err("approval approver_roles must not contain blank entries".into());
        }
        // Roles and reason flow verbatim into decision remediation and
        // escalation webhook payloads; bound them like sandbox_hint.
        if approval
            .approver_roles
            .iter()
            .any(|r| r.len() > MAX_APPROVER_ROLE_LEN)
        {
            return Err(format!(
                "approval approver_roles entries must be at most {MAX_APPROVER_ROLE_LEN} bytes"
            ));
        }
        if approval
            .reason
            .as_deref()
            .is_some_and(|r| r.len() > MAX_APPROVAL_REASON_BYTES)
        {
            return Err(format!(
                "approval reason exceeds {MAX_APPROVAL_REASON_BYTES} bytes"
            ));
        }
    }
    if let Some(hint) = &metadata.sandbox_hint {
        let serialized = serde_json::to_string(hint)
            .map_err(|e| format!("sandbox_hint is not serializable: {e}"))?;
        if serialized.len() > MAX_SANDBOX_HINT_BYTES {
            return Err(format!(
                "sandbox_hint exceeds {MAX_SANDBOX_HINT_BYTES} serialized bytes"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::{
        AllowedSource, ApprovalRule, LimitAction, Origin, ParamLimit, ParamRole, ParamSpec,
        SideEffectClass,
    };

    fn metadata() -> ToolMetadata {
        ToolMetadata {
            tool: "send_email".into(),
            side_effect: SideEffectClass::ExternalCommunication,
            reversible: false,
            params: vec![ParamSpec {
                path: "recipient".into(),
                role: ParamRole::AuthorityBearing,
                allowed_sources: vec![AllowedSource {
                    origin: Origin::User,
                    source_id: None,
                    kind: None,
                }],
                limit: None,
            }],
            approval: None,
            sandbox_hint: None,
        }
    }

    #[test]
    fn accepts_valid_metadata() {
        assert_eq!(validate_metadata(&metadata()), Ok(()));
    }

    #[test]
    fn rejects_blank_tool_name() {
        let mut m = metadata();
        m.tool = "  ".into();
        assert!(validate_metadata(&m).unwrap_err().contains("tool"));
    }

    #[test]
    fn rejects_empty_param_path() {
        let mut m = metadata();
        m.params[0].path = "".into();
        assert!(validate_metadata(&m).unwrap_err().contains("param path"));
    }

    #[test]
    fn rejects_duplicate_param_paths() {
        let mut m = metadata();
        m.params.push(m.params[0].clone());
        assert!(validate_metadata(&m).unwrap_err().contains("duplicate"));
    }

    #[test]
    fn rejects_blank_allowed_source_id() {
        let mut m = metadata();
        m.params[0].allowed_sources[0].source_id = Some(" ".into());
        assert!(validate_metadata(&m).unwrap_err().contains("source_id"));
    }

    #[test]
    fn rejects_oversized_tool_name() {
        let mut m = metadata();
        m.tool = "a".repeat(MAX_TOOL_NAME_LEN + 1);
        assert!(validate_metadata(&m).unwrap_err().contains("exceeds"));
    }

    #[test]
    fn rejects_nul_byte_in_tool_name() {
        let mut m = metadata();
        m.tool = "send\0email".into();
        assert!(validate_metadata(&m).unwrap_err().contains("NUL"));
    }

    #[test]
    fn rejects_too_many_params() {
        let mut m = metadata();
        m.params = (0..=MAX_PARAMS)
            .map(|i| ParamSpec {
                path: format!("p{i}"),
                role: ParamRole::ContentBearing,
                allowed_sources: vec![],
                limit: None,
            })
            .collect();
        assert!(validate_metadata(&m).unwrap_err().contains("params"));
    }

    #[test]
    fn rejects_oversized_param_path() {
        let mut m = metadata();
        m.params[0].path = "p".repeat(MAX_PARAM_PATH_LEN + 1);
        assert!(validate_metadata(&m).unwrap_err().contains("param path"));
    }

    #[test]
    fn rejects_oversized_param_path_with_whitespace_padding() {
        let mut m = metadata();
        m.params[0].path = format!("p{}", " ".repeat(MAX_PARAM_PATH_LEN));
        assert!(validate_metadata(&m).unwrap_err().contains("param path"));
    }

    #[test]
    fn rejects_oversized_sandbox_hint() {
        let mut m = metadata();
        m.sandbox_hint = Some(serde_json::json!({
            "blob": "x".repeat(MAX_SANDBOX_HINT_BYTES)
        }));
        assert!(validate_metadata(&m).unwrap_err().contains("sandbox_hint"));
    }

    #[test]
    fn rejects_too_many_approver_roles() {
        let mut m = metadata();
        m.approval = Some(ApprovalRule {
            required: true,
            approver_roles: (0..=MAX_APPROVER_ROLES).map(|i| format!("r{i}")).collect(),
            reason: None,
        });
        assert!(validate_metadata(&m)
            .unwrap_err()
            .contains("approver_roles"));
    }

    #[test]
    fn rejects_blank_approver_role() {
        let mut m = metadata();
        m.approval = Some(ApprovalRule {
            required: true,
            approver_roles: vec!["admin".into(), "".into()],
            reason: None,
        });
        assert!(validate_metadata(&m)
            .unwrap_err()
            .contains("approver_roles"));
    }

    #[test]
    fn rejects_oversized_approver_role() {
        let mut m = metadata();
        m.approval = Some(ApprovalRule {
            required: true,
            approver_roles: vec!["r".repeat(MAX_APPROVER_ROLE_LEN + 1)],
            reason: None,
        });
        assert!(validate_metadata(&m)
            .unwrap_err()
            .contains("approver_roles"));
    }

    #[test]
    fn rejects_oversized_approval_reason() {
        let mut m = metadata();
        m.approval = Some(ApprovalRule {
            required: true,
            approver_roles: vec!["admin".into()],
            reason: Some("x".repeat(MAX_APPROVAL_REASON_BYTES + 1)),
        });
        assert!(validate_metadata(&m).unwrap_err().contains("reason"));
    }

    fn with_limit(limit: ParamLimit) -> ToolMetadata {
        let mut m = metadata();
        m.params[0].limit = Some(limit);
        m
    }

    #[test]
    fn accepts_valid_value_limit() {
        let m = with_limit(ParamLimit {
            max: Some(500),
            min: Some(1),
            on_breach: LimitAction::Block,
        });
        assert_eq!(validate_metadata(&m), Ok(()));
    }

    #[test]
    fn accepts_max_only_value_limit() {
        let m = with_limit(ParamLimit {
            max: Some(500),
            min: None,
            on_breach: LimitAction::Block,
        });
        assert_eq!(validate_metadata(&m), Ok(()));
    }

    #[test]
    fn rejects_value_limit_with_no_bounds() {
        let m = with_limit(ParamLimit {
            max: None,
            min: None,
            on_breach: LimitAction::Block,
        });
        assert!(validate_metadata(&m).unwrap_err().contains("at least one"));
    }

    #[test]
    fn rejects_inverted_value_limit() {
        let m = with_limit(ParamLimit {
            max: Some(100),
            min: Some(500),
            on_breach: LimitAction::Block,
        });
        let err = validate_metadata(&m).unwrap_err();
        assert!(err.contains("min"));
        assert!(err.contains("max"));
    }
}
