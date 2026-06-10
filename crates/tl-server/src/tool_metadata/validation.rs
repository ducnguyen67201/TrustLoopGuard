//! Server-edge validation for tool metadata payloads. Enum values
//! (side effects, param roles, source origins) are already enforced by
//! serde at parse time; this checks the constraints serde cannot.

use tl_core::ToolMetadata;

pub(super) fn validate_metadata(metadata: &ToolMetadata) -> Result<(), String> {
    if metadata.tool.trim().is_empty() {
        return Err("tool is required".into());
    }
    let mut seen_paths = std::collections::HashSet::new();
    for param in &metadata.params {
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
    }
    if let Some(approval) = &metadata.approval {
        if approval.approver_roles.iter().any(|r| r.trim().is_empty()) {
            return Err("approval approver_roles must not contain blank entries".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::{AllowedSource, ApprovalRule, Origin, ParamRole, ParamSpec, SideEffectClass};

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
}
