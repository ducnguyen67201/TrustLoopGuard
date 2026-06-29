//! Parameter-source authorization: right tool, wrong source.

use tl_core::{AllowedSource, EventKind, GuardEvent, Origin, ParamRole, ToolResolution, Verdict};

use super::origin_str;
use crate::event_pipeline::{Checker, CheckerFinding};

pub const PARAMETER_AUTH_CHECKER_ID: &str = "parameter_auth";

const FAILURE_WRONG_SOURCE: &str = "wrong_source";
const FAILURE_MISSING_PROVENANCE: &str = "missing_provenance";
const FAILURE_UNVERIFIED_PARAMETERS: &str = "unverified_parameters";

/// Presence lookup for a dot-separated parameter path over the action's
/// JSON parameters. Object-key traversal only — array indexing and keys
/// containing a literal `.` are unsupported. This is a presence test: the
/// provenance join below uses the exact path string, mirroring how
/// `ProvenancePropagator` joins paths.
pub(crate) fn lookup_param<'a>(
    parameters: &'a serde_json::Value,
    path: &str,
) -> Option<&'a serde_json::Value> {
    let mut current = parameters;
    for segment in path.split('.') {
        current = current.as_object()?.get(segment)?;
    }
    Some(current)
}

/// One allowed-source rule matches one event source when the origin is
/// equal and every set optional field narrows further: `source_id` (if
/// set) must equal the source id, `kind` (if set) must equal the source
/// kind. Unset optionals mean "any".
pub(crate) fn source_matches_allowed(source: &tl_core::Source, allowed: &AllowedSource) -> bool {
    if allowed.origin != source.origin {
        return false;
    }
    if let Some(id) = &allowed.source_id {
        if id != &source.id {
            return false;
        }
    }
    if let Some(kind) = &allowed.kind {
        if source.kind.as_deref() != Some(kind.as_str()) {
            return false;
        }
    }
    true
}

enum ViolationKind {
    /// At least one contributing source has no allowed-source proof.
    DisallowedSource,
    /// The parameter was supplied but has no provenance entry.
    MissingProvenance,
}

struct ParamViolation {
    path: String,
    kind: ViolationKind,
    expected: Vec<AllowedSource>,
    /// Contributors with no allowed-source proof: (id, resolved origin).
    /// A provenance id with no matching event source resolves to
    /// `Origin::Unknown` — it can prove nothing.
    offending: Vec<(String, Origin)>,
}

/// The authorization rule, per authority-bearing parameter of the
/// resolved tool:
///
/// ```text
/// param path -> provenance source ids -> source origins
///            -> compare against allowed_sources
///            -> if no allowed proof: violation
/// ```
///
/// Conservative posture throughout: a missing or empty provenance entry
/// is missing proof, never clean; a dangling source id authorizes
/// nothing; an empty `allowed_sources` list on an authority-bearing spec
/// is the registry author's explicit lockdown and accepts no source.
/// Content-bearing parameters are never evaluated. A parameter absent
/// from the call is skipped — nothing was supplied to authorize.
fn evaluate(event: &GuardEvent) -> Vec<ParamViolation> {
    let Some(ToolResolution::Resolved { metadata }) = &event.resolution else {
        return vec![];
    };

    let mut violations = Vec::new();
    for spec in &metadata.params {
        if spec.role != ParamRole::AuthorityBearing {
            continue;
        }
        if lookup_param(&event.action.parameters, &spec.path).is_none() {
            continue;
        }

        let source_ids = event
            .provenance
            .0
            .get(&spec.path)
            .filter(|ids| !ids.is_empty());
        let Some(source_ids) = source_ids else {
            violations.push(ParamViolation {
                path: spec.path.clone(),
                kind: ViolationKind::MissingProvenance,
                expected: spec.allowed_sources.clone(),
                offending: vec![],
            });
            continue;
        };

        let offending: Vec<(String, Origin)> = source_ids
            .iter()
            .filter_map(
                |id| match event.sources.iter().find(|source| &source.id == id) {
                    Some(source) => {
                        let allowed = spec
                            .allowed_sources
                            .iter()
                            .any(|rule| source_matches_allowed(source, rule));
                        (!allowed).then(|| (id.clone(), source.origin))
                    }
                    None => Some((id.clone(), Origin::Unknown)),
                },
            )
            .collect();

        if !offending.is_empty() {
            violations.push(ParamViolation {
                path: spec.path.clone(),
                kind: ViolationKind::DisallowedSource,
                expected: spec.allowed_sources.clone(),
                offending,
            });
        }
    }
    violations
}

fn expected_origins(allowed: &[AllowedSource]) -> String {
    if allowed.is_empty() {
        return "none".to_string();
    }
    allowed
        .iter()
        .map(|rule| origin_str(rule.origin))
        .collect::<Vec<_>>()
        .join(", ")
}

fn violation_finding(violation: ParamViolation) -> CheckerFinding {
    let expected = expected_origins(&violation.expected);
    let path = &violation.path;
    match violation.kind {
        ViolationKind::DisallowedSource => {
            let first_origin = violation
                .offending
                .first()
                .map(|(_, origin)| *origin)
                .unwrap_or(Origin::Unknown);
            // Offending source ids are caller-controlled strings; they
            // travel only in the structured `source_chain` field and are
            // never inlined into the reason sentence, which reaches HTTP
            // responses and dashboards verbatim.
            CheckerFinding {
                checker_id: PARAMETER_AUTH_CHECKER_ID.to_string(),
                verdict: Some(Verdict::Block),
                reason: format!(
                    "authority-bearing parameter '{path}' expects sources of origin \
                     {expected}, got {got}",
                    got = origin_str(first_origin),
                ),
                violated_rule: Some(format!("parameter_source.{path}")),
                remediation: Some(format!(
                    "re-derive '{path}' from an allowed source ({expected}) or update \
                     the tool's allowed_sources registry entry"
                )),
                source_chain: violation
                    .offending
                    .iter()
                    .map(|(id, _)| id.clone())
                    .collect(),
                risk_source: Some(origin_str(first_origin).to_string()),
                failure_mode: Some(FAILURE_WRONG_SOURCE.to_string()),
                harm_class: Some("integrity".to_string()),
            }
        }
        ViolationKind::MissingProvenance => CheckerFinding {
            checker_id: PARAMETER_AUTH_CHECKER_ID.to_string(),
            verdict: Some(Verdict::Escalate),
            reason: format!(
                "authority-bearing parameter '{path}' has no provenance; missing \
                 provenance is not proof of an allowed source"
            ),
            violated_rule: Some(format!("parameter_source.{path}")),
            remediation: Some(format!(
                "attach provenance showing '{path}' came from an allowed source ({expected})"
            )),
            source_chain: vec![],
            risk_source: None,
            failure_mode: Some(FAILURE_MISSING_PROVENANCE.to_string()),
            harm_class: Some("integrity".to_string()),
        },
    }
}

/// Deterministic parameter-source authorization over the resolved tool's
/// registry metadata:
///
/// - **parameter_source.{path}**: every authority-bearing parameter must
///   carry provenance whose sources all match the tool's
///   `allowed_sources`; wrong source blocks, missing proof escalates.
/// - An unregistered tool on an action event yields verdict-free
///   evidence: parameter sources exist but cannot be verified. Legacy
///   `output.proposed` events are exempt — their operation is never
///   registered and the evidence would be noise.
pub struct ParameterAuthChecker;

impl Checker for ParameterAuthChecker {
    fn id(&self) -> &'static str {
        PARAMETER_AUTH_CHECKER_ID
    }

    fn check(&self, event: &GuardEvent) -> Vec<CheckerFinding> {
        match &event.resolution {
            Some(ToolResolution::Resolved { .. }) => {
                evaluate(event).into_iter().map(violation_finding).collect()
            }
            Some(ToolResolution::Unregistered) | Some(ToolResolution::ResolutionFailed)
                if event.kind != EventKind::OutputProposed =>
            {
                vec![CheckerFinding {
                    checker_id: PARAMETER_AUTH_CHECKER_ID.to_string(),
                    verdict: None,
                    reason: format!(
                        "tool '{}' is not registered; parameter sources cannot be verified",
                        event.action.operation
                    ),
                    violated_rule: None,
                    remediation: Some(format!(
                        "register tool metadata for '{}' so authority-bearing \
                         parameters can be authorized",
                        event.action.operation
                    )),
                    source_chain: vec![],
                    risk_source: None,
                    failure_mode: Some(FAILURE_UNVERIFIED_PARAMETERS.to_string()),
                    harm_class: None,
                }]
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{event, source};
    use super::*;
    use tl_core::{EventKind, Labels, ParamSpec, ProvenanceMap, SideEffectClass, ToolMetadata};

    fn allowed(origin: Origin) -> AllowedSource {
        AllowedSource {
            origin,
            source_id: None,
            kind: None,
        }
    }

    fn authority_param(path: &str, allowed_sources: Vec<AllowedSource>) -> ParamSpec {
        ParamSpec {
            path: path.into(),
            role: ParamRole::AuthorityBearing,
            allowed_sources,
            limit: None,
        }
    }

    fn content_param(path: &str) -> ParamSpec {
        ParamSpec {
            path: path.into(),
            role: ParamRole::ContentBearing,
            allowed_sources: vec![],
            limit: None,
        }
    }

    fn resolved(params: Vec<ParamSpec>) -> ToolResolution {
        ToolResolution::Resolved {
            metadata: ToolMetadata {
                tool: "send_email".into(),
                side_effect: SideEffectClass::ExternalCommunication,
                reversible: false,
                params,
                approval: None,
                sandbox_hint: None,
            },
        }
    }

    fn provenance(path: &str, ids: &[&str]) -> ProvenanceMap {
        let mut map = ProvenanceMap::default();
        map.insert(path, ids.iter().map(|id| id.to_string()).collect());
        map
    }

    fn tool_event(
        parameters: serde_json::Value,
        provenance: ProvenanceMap,
        sources: Vec<tl_core::Source>,
        resolution: ToolResolution,
    ) -> GuardEvent {
        let mut guard_event = event(
            EventKind::ToolCallProposed,
            Some(SideEffectClass::ExternalCommunication),
            sources,
            provenance,
        );
        guard_event.action.parameters = parameters;
        guard_event.resolution = Some(resolution);
        guard_event
    }

    #[test]
    fn lookup_finds_top_level_param() {
        let params = serde_json::json!({ "recipient": "a@b.c" });
        assert!(lookup_param(&params, "recipient").is_some());
    }

    #[test]
    fn lookup_finds_nested_param() {
        let params = serde_json::json!({ "transfer": { "dest_iban": "DE89" } });
        assert_eq!(
            lookup_param(&params, "transfer.dest_iban"),
            Some(&serde_json::json!("DE89"))
        );
    }

    #[test]
    fn lookup_absent_path_is_none() {
        let params = serde_json::json!({ "recipient": "a@b.c" });
        assert!(lookup_param(&params, "cc").is_none());
    }

    #[test]
    fn lookup_through_non_object_is_none() {
        let params = serde_json::json!({ "recipient": "a@b.c" });
        assert!(lookup_param(&params, "recipient.domain").is_none());
    }

    #[test]
    fn lookup_on_null_parameters_is_none() {
        assert!(lookup_param(&serde_json::Value::Null, "recipient").is_none());
    }

    #[test]
    fn origin_only_rule_matches_any_id_and_kind() {
        let mut src = source("src.user", Origin::User, Labels::default());
        src.kind = Some("prompt".into());
        assert!(source_matches_allowed(&src, &allowed(Origin::User)));
    }

    #[test]
    fn source_id_narrows_match() {
        let src = source("src.user", Origin::User, Labels::default());
        let mut rule = allowed(Origin::User);
        rule.source_id = Some("src.user".into());
        assert!(source_matches_allowed(&src, &rule));
        rule.source_id = Some("src.other".into());
        assert!(!source_matches_allowed(&src, &rule));
    }

    #[test]
    fn kind_narrows_match() {
        let mut rule = allowed(Origin::Api);
        rule.kind = Some("contact_lookup".into());
        // Source declares no kind: the rule's narrowing cannot be proven.
        let src = source("src.crm", Origin::Api, Labels::default());
        assert!(!source_matches_allowed(&src, &rule));
        let mut with_kind = source("src.crm", Origin::Api, Labels::default());
        with_kind.kind = Some("contact_lookup".into());
        assert!(source_matches_allowed(&with_kind, &rule));
    }

    #[test]
    fn origin_mismatch_never_matches() {
        let src = source("src.web", Origin::Web, Labels::default());
        assert!(!source_matches_allowed(&src, &allowed(Origin::User)));
    }

    #[test]
    fn correct_source_yields_no_findings() {
        let guard_event = tool_event(
            serde_json::json!({ "recipient": "a@b.c" }),
            provenance("recipient", &["src.user"]),
            vec![source("src.user", Origin::User, Labels::default())],
            resolved(vec![authority_param(
                "recipient",
                vec![allowed(Origin::User)],
            )]),
        );

        assert!(ParameterAuthChecker.check(&guard_event).is_empty());
    }

    #[test]
    fn wrong_origin_blocks_with_expected_vs_actual_evidence() {
        let guard_event = tool_event(
            serde_json::json!({ "recipient": "a@b.c" }),
            provenance("recipient", &["src.web"]),
            vec![source("src.web", Origin::Web, Labels::default())],
            resolved(vec![authority_param(
                "recipient",
                vec![allowed(Origin::User)],
            )]),
        );

        let findings = ParameterAuthChecker.check(&guard_event);
        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.checker_id, "parameter_auth");
        assert_eq!(finding.verdict, Some(Verdict::Block));
        assert_eq!(
            finding.violated_rule.as_deref(),
            Some("parameter_source.recipient")
        );
        assert!(finding.reason.contains("expects sources of origin user"));
        assert!(finding.reason.contains("got web"));
        assert!(!finding.reason.contains("src.web"));
        assert!(finding
            .remediation
            .as_deref()
            .is_some_and(|text| text.contains("recipient")));
        assert_eq!(finding.source_chain, vec!["src.web"]);
        assert_eq!(finding.risk_source.as_deref(), Some("web"));
        assert_eq!(finding.failure_mode.as_deref(), Some("wrong_source"));
        assert_eq!(finding.harm_class.as_deref(), Some("integrity"));
    }

    #[test]
    fn missing_provenance_escalates() {
        let guard_event = tool_event(
            serde_json::json!({ "recipient": "a@b.c" }),
            ProvenanceMap::default(),
            vec![source("src.user", Origin::User, Labels::default())],
            resolved(vec![authority_param(
                "recipient",
                vec![allowed(Origin::User)],
            )]),
        );

        let findings = ParameterAuthChecker.check(&guard_event);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].verdict, Some(Verdict::Escalate));
        assert_eq!(
            findings[0].failure_mode.as_deref(),
            Some("missing_provenance")
        );
        assert_eq!(
            findings[0].violated_rule.as_deref(),
            Some("parameter_source.recipient")
        );
        assert!(findings[0].source_chain.is_empty());
        assert!(findings[0].risk_source.is_none());
    }

    #[test]
    fn empty_provenance_source_list_escalates() {
        let guard_event = tool_event(
            serde_json::json!({ "recipient": "a@b.c" }),
            provenance("recipient", &[]),
            vec![source("src.user", Origin::User, Labels::default())],
            resolved(vec![authority_param(
                "recipient",
                vec![allowed(Origin::User)],
            )]),
        );

        let findings = ParameterAuthChecker.check(&guard_event);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].verdict, Some(Verdict::Escalate));
    }

    #[test]
    fn dangling_source_id_cannot_authorize() {
        let guard_event = tool_event(
            serde_json::json!({ "recipient": "a@b.c" }),
            provenance("recipient", &["src.ghost"]),
            vec![],
            resolved(vec![authority_param(
                "recipient",
                vec![allowed(Origin::User)],
            )]),
        );

        let findings = ParameterAuthChecker.check(&guard_event);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].verdict, Some(Verdict::Block));
        assert_eq!(findings[0].source_chain, vec!["src.ghost"]);
        assert_eq!(findings[0].risk_source.as_deref(), Some("unknown"));
    }

    #[test]
    fn mixed_contributors_one_bad_blocks() {
        let mut map = ProvenanceMap::default();
        map.insert("recipient", vec!["src.user".into(), "src.web".into()]);
        let guard_event = tool_event(
            serde_json::json!({ "recipient": "a@b.c" }),
            map,
            vec![
                source("src.user", Origin::User, Labels::default()),
                source("src.web", Origin::Web, Labels::default()),
            ],
            resolved(vec![authority_param(
                "recipient",
                vec![allowed(Origin::User)],
            )]),
        );

        let findings = ParameterAuthChecker.check(&guard_event);
        assert_eq!(findings.len(), 1);
        // Only the unproven contributor is offending evidence.
        assert_eq!(findings[0].source_chain, vec!["src.web"]);
    }

    #[test]
    fn content_bearing_params_are_ignored() {
        let guard_event = tool_event(
            serde_json::json!({ "body": "untrusted text" }),
            provenance("body", &["src.web"]),
            vec![source("src.web", Origin::Web, Labels::default())],
            resolved(vec![content_param("body")]),
        );

        assert!(ParameterAuthChecker.check(&guard_event).is_empty());
    }

    #[test]
    fn param_absent_from_call_is_skipped() {
        let guard_event = tool_event(
            serde_json::json!({ "recipient": "a@b.c" }),
            ProvenanceMap::default(),
            vec![],
            resolved(vec![authority_param("cc", vec![allowed(Origin::User)])]),
        );

        assert!(ParameterAuthChecker.check(&guard_event).is_empty());
    }

    #[test]
    fn empty_allowed_sources_rejects_all() {
        let guard_event = tool_event(
            serde_json::json!({ "recipient": "a@b.c" }),
            provenance("recipient", &["src.user"]),
            vec![source("src.user", Origin::User, Labels::default())],
            resolved(vec![authority_param("recipient", vec![])]),
        );

        let findings = ParameterAuthChecker.check(&guard_event);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].verdict, Some(Verdict::Block));
        assert!(findings[0].reason.contains("origin none"));
    }

    #[test]
    fn multiple_params_evaluated_independently() {
        let mut map = ProvenanceMap::default();
        map.insert("recipient", vec!["src.user".into()]);
        map.insert("transfer.dest_iban", vec!["src.web".into()]);
        let guard_event = tool_event(
            serde_json::json!({
                "recipient": "a@b.c",
                "transfer": { "dest_iban": "DE89" }
            }),
            map,
            vec![
                source("src.user", Origin::User, Labels::default()),
                source("src.web", Origin::Web, Labels::default()),
            ],
            resolved(vec![
                authority_param("recipient", vec![allowed(Origin::User)]),
                authority_param("transfer.dest_iban", vec![allowed(Origin::User)]),
            ]),
        );

        let findings = ParameterAuthChecker.check(&guard_event);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].violated_rule.as_deref(),
            Some("parameter_source.transfer.dest_iban")
        );
    }

    #[test]
    fn unregistered_tool_call_emits_evidence_only_finding() {
        for resolution in [
            ToolResolution::Unregistered,
            ToolResolution::ResolutionFailed,
        ] {
            let guard_event = tool_event(
                serde_json::json!({ "recipient": "a@b.c" }),
                provenance("recipient", &["src.web"]),
                vec![source("src.web", Origin::Web, Labels::default())],
                resolution,
            );

            let findings = ParameterAuthChecker.check(&guard_event);
            assert_eq!(findings.len(), 1);
            let finding = &findings[0];
            assert_eq!(finding.verdict, None);
            assert_eq!(
                finding.failure_mode.as_deref(),
                Some("unverified_parameters")
            );
            assert!(finding.violated_rule.is_none());
            assert!(finding.reason.contains("'op' is not registered"));
        }
    }

    #[test]
    fn unregistered_output_event_emits_nothing() {
        let mut guard_event = event(
            EventKind::OutputProposed,
            Some(SideEffectClass::None),
            vec![],
            ProvenanceMap::default(),
        );
        guard_event.resolution = Some(ToolResolution::Unregistered);

        assert!(ParameterAuthChecker.check(&guard_event).is_empty());
    }

    #[test]
    fn unresolved_pipeline_emits_nothing() {
        let guard_event = event(
            EventKind::ToolCallProposed,
            Some(SideEffectClass::ExternalCommunication),
            vec![source("src.web", Origin::Web, Labels::default())],
            provenance("recipient", &["src.web"]),
        );

        assert!(ParameterAuthChecker.check(&guard_event).is_empty());
    }
}
