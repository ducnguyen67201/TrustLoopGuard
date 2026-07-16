use serde_json::json;
use tl_core::{
    AuthorizationEffect, AuthorizationGrantScope, AuthorizationSubject, Channel, EventKind,
    SideEffectClass, ToolIdentity,
};
use tl_engine::{evaluate_tool_policies, ToolPolicyError};
use tl_policy::{load_any_str, AnyPolicy, FamilyPolicy};

fn policy(yaml: &str) -> FamilyPolicy {
    let AnyPolicy::Family(policy) = load_any_str(yaml).expect("policy parses") else {
        panic!("expected family policy");
    };
    policy
}

fn subject(command: &str) -> AuthorizationSubject {
    AuthorizationSubject::Tool {
        invocation_id: "tool-use-1".into(),
        operation: "Bash".into(),
        tool_identity: ToolIdentity {
            server_id: "claude-code".into(),
            tool_name: "Bash".into(),
            schema_hash: "sha256:test".into(),
        },
        parameters: json!({
            "command": command,
            "shell": "bash",
            "cwd": "/workspace/project",
            "workspace_root": "/workspace/project",
            "run_in_background": false
        }),
        side_effect: SideEffectClass::ShellExec,
    }
}

#[test]
fn fact_policy_denies_nested_destructive_shell_command() {
    let family = policy(
        r#"
family: tool
id: block-system-delete
severity: critical
when: { side_effects: [shell_exec] }
match:
  all:
    - fact: { key: shell.risk, equals: filesystem_recursive_delete }
    - fact: { key: shell.target_scope, one_of: [root, system] }
action: deny
reason: System deletion is prohibited.
remediation: Use a disposable workspace path.
"#,
    );

    let outcome = evaluate_tool_policies("coding-agent", &subject("bash -c 'rm -rf /'"), [&family])
        .expect("evaluate");

    assert_eq!(outcome.findings.len(), 1);
    assert_eq!(outcome.findings[0].effect, AuthorizationEffect::Deny);
    assert_eq!(
        outcome.findings[0].policy_id.as_deref(),
        Some("block-system-delete")
    );
    assert!(outcome.requirements.is_empty());
}

#[test]
fn parameter_regex_policy_matches_without_shell_facts() {
    let family = policy(
        r#"
family: tool
id: block-company-destroy
when: { side_effects: [shell_exec] }
match:
  parameter: { path: /command, regex: '(?i)acme-prod\s+destroy' }
action: deny
reason: Production destroy is prohibited.
"#,
    );

    let outcome = evaluate_tool_policies(
        "coding-agent",
        &subject("acme-prod destroy service"),
        [&family],
    )
    .expect("evaluate");
    assert_eq!(outcome.findings[0].effect, AuthorizationEffect::Deny);
}

#[test]
fn approval_policy_requires_exact_non_reusable_scope() {
    let family = policy(
        r#"
family: tool
id: approve-workspace-delete
when: { side_effects: [shell_exec] }
match:
  fact: { key: shell.risk, equals: filesystem_recursive_delete }
action: require_approval
reason: Review destructive workspace changes.
approver_roles: [owner, admin]
max_grant_ttl_seconds: 600
"#,
    );
    let proposed = subject("rm -rf ./build");

    let outcome = evaluate_tool_policies("coding-agent", &proposed, [&family]).expect("evaluate");

    assert_eq!(
        outcome.findings[0].effect,
        AuthorizationEffect::RequireApproval
    );
    assert_eq!(outcome.requirements.len(), 1);
    let requirement = &outcome.requirements[0];
    assert!(!requirement.reusable_allowed);
    assert_eq!(requirement.max_grant_ttl_seconds, Some(600));
    let AuthorizationGrantScope::Action(scope) = &requirement.required_scope else {
        panic!("expected exact action scope");
    };
    assert_eq!(
        scope.parameters.as_ref(),
        Some(match &proposed {
            AuthorizationSubject::Tool { parameters, .. } => parameters,
            _ => unreachable!(),
        })
    );
}

#[test]
fn incomplete_analysis_defers_unproven_fact_policy() {
    let family = policy(
        r#"
family: tool
id: block-device-write
when: { side_effects: [shell_exec] }
match:
  fact: { key: shell.risk, equals: disk_overwrite }
action: deny
reason: Device writes are prohibited.
"#,
    );

    let outcome = evaluate_tool_policies("coding-agent", &subject("bash -c 'dd if=x"), [&family])
        .expect("evaluate");
    assert_eq!(outcome.findings[0].effect, AuthorizationEffect::Defer);
    assert_eq!(
        outcome.findings[0].policy_id.as_deref(),
        Some("block-device-write")
    );
}

#[test]
fn dynamic_target_defers_a_target_specific_fact_policy() {
    let family = policy(
        r#"
family: tool
id: block-root-delete
when: { side_effects: [shell_exec] }
match:
  all:
    - fact: { key: shell.risk, equals: filesystem_recursive_delete }
    - fact: { key: shell.target_scope, equals: root }
action: deny
reason: Root deletion is prohibited.
"#,
    );

    let outcome = evaluate_tool_policies("coding-agent", &subject("rm -rf $TARGET"), [&family])
        .expect("evaluate");
    assert_eq!(outcome.findings[0].effect, AuthorizationEffect::Defer);
    assert_eq!(
        outcome.findings[0].policy_id.as_deref(),
        Some("block-root-delete")
    );
}

#[test]
fn policy_scope_requires_every_configured_selector_to_match() {
    let family = policy(
        r#"
family: tool
id: scoped-policy
when:
  agents: [another-agent]
  operations: [OtherTool]
  side_effects: [network_call]
  tools:
    - server_id: another-server
      tool_name: OtherTool
      schema_hash: sha256:other
match:
  parameter: { path: /command, equals: "rm -rf /" }
action: deny
reason: This policy is outside the invocation scope.
"#,
    );

    let outcome =
        evaluate_tool_policies("coding-agent", &subject("rm -rf /"), [&family]).expect("evaluate");

    assert!(outcome.findings.is_empty());
    assert!(outcome.requirements.is_empty());
}

#[test]
fn any_and_all_clauses_preserve_boolean_match_semantics() {
    let any = policy(
        r#"
family: tool
id: any-match
when: { side_effects: [shell_exec] }
match:
  any:
    - parameter: { path: /missing, equals: value }
    - parameter: { path: /command, equals: "echo safe" }
action: deny
reason: One matcher is enough.
"#,
    );
    let all = policy(
        r#"
family: tool
id: all-match
when: { side_effects: [shell_exec] }
match:
  all:
    - parameter: { path: /command, one_of: ["echo safe", "echo other"] }
    - parameter: { path: /shell, equals: bash }
action: deny
reason: Every matcher is required.
"#,
    );
    let all_not_matched = policy(
        r#"
family: tool
id: all-not-matched
when: { side_effects: [shell_exec] }
match:
  all:
    - parameter: { path: /command, equals: "echo safe" }
    - parameter: { path: /shell, equals: zsh }
action: deny
reason: A false matcher short-circuits the clause.
"#,
    );

    let outcome = evaluate_tool_policies(
        "coding-agent",
        &subject("echo safe"),
        [&any, &all, &all_not_matched],
    )
    .expect("evaluate");
    let policy_ids = outcome
        .findings
        .iter()
        .filter_map(|finding| finding.policy_id.as_deref())
        .collect::<Vec<_>>();

    assert_eq!(policy_ids, vec!["any-match", "all-match"]);
}

#[test]
fn approval_defaults_and_non_string_parameters_are_supported() {
    let family = policy(
        r#"
family: tool
id: approve-zero-timeout
when: { side_effects: [shell_exec] }
match:
  parameter: { path: /timeout_ms, equals: "0" }
action: require_approval
reason: Review commands with an unbounded timeout.
"#,
    );
    let mut proposed = subject("echo safe");
    let AuthorizationSubject::Tool { parameters, .. } = &mut proposed else {
        unreachable!();
    };
    parameters["timeout_ms"] = json!(0);

    let outcome = evaluate_tool_policies("coding-agent", &proposed, [&family]).expect("evaluate");
    let requirement = &outcome.requirements[0];

    assert_eq!(requirement.approver_roles, vec!["owner", "admin"]);
    assert_eq!(requirement.max_grant_ttl_seconds, Some(900));
}

#[test]
fn invalid_subjects_and_shell_parameters_fail_closed() {
    let family = policy(
        r#"
family: tool
id: fact-policy
when: { side_effects: [shell_exec] }
match:
  fact: { key: shell.risk, equals: disk_overwrite }
action: deny
reason: Device writes are prohibited.
"#,
    );
    let content = AuthorizationSubject::Content {
        event_kind: EventKind::OutputProposed,
        channel: Channel::Chat,
        input: String::new(),
        output: String::new(),
    };
    assert!(matches!(
        evaluate_tool_policies("coding-agent", &content, [&family]),
        Err(ToolPolicyError::DomainMismatch)
    ));

    let mut invalid = subject("echo safe");
    let AuthorizationSubject::Tool { parameters, .. } = &mut invalid else {
        unreachable!();
    };
    *parameters = json!({ "shell": "bash" });
    assert!(matches!(
        evaluate_tool_policies("coding-agent", &invalid, [&family]),
        Err(ToolPolicyError::InvalidShellParameters(_))
    ));
}

#[test]
fn invalid_tool_identity_cannot_produce_an_authorization_capability() {
    let family = policy(
        r#"
family: tool
id: command-policy
when: { side_effects: [shell_exec] }
match:
  parameter: { path: /command, equals: "echo safe" }
action: deny
reason: Test policy.
"#,
    );
    let mut proposed = subject("echo safe");
    let AuthorizationSubject::Tool { tool_identity, .. } = &mut proposed else {
        unreachable!();
    };
    tool_identity.server_id = "invalid server".into();

    assert!(matches!(
        evaluate_tool_policies("coding-agent", &proposed, [&family]),
        Err(ToolPolicyError::InvalidCapability(_))
    ));
}
