use serde_json::json;
use tl_core::{
    CheckRequest, CreateRunEventRequest, DataHandlingMode, RedactionInfo, RedactionMode,
    RedactionStatus, RunEventKind,
};

use super::{apply_server_redaction, requires_redaction_rejection, should_apply_server_redaction};

fn base_request(input: &str, proposed_output: &str) -> CheckRequest {
    CheckRequest {
        agent_id: "tax-document-agent".into(),
        input: input.into(),
        proposed_output: proposed_output.into(),
        context: json!({}),
        ..Default::default()
    }
}

fn applied_info(mode: RedactionMode) -> RedactionInfo {
    RedactionInfo {
        mode,
        status: RedactionStatus::Applied,
        entities: Vec::new(),
        input_redacted: true,
        proposed_output_redacted: true,
        context_redacted: false,
    }
}

#[test]
fn raw_allowed_workspace_never_rejects() {
    let req = base_request("My SIN is 123-456-789.", "ok");
    assert!(!requires_redaction_rejection(
        DataHandlingMode::RawAllowed,
        &req
    ));
}

#[test]
fn redacted_only_rejects_raw_input_without_redaction_info() {
    let req = base_request("My SIN is 123-456-789.", "ok");
    assert!(requires_redaction_rejection(
        DataHandlingMode::RedactedOnly,
        &req
    ));
}

#[test]
fn redacted_only_rejects_when_client_asserts_status_but_data_is_raw() {
    // The client cannot be trusted: even with status == Applied, if raw
    // sensitive data is present the gate must reject.
    let mut req = base_request("Email alice@example.com.", "ok");
    req.redaction = Some(applied_info(RedactionMode::SdkLocal));
    assert!(requires_redaction_rejection(
        DataHandlingMode::RedactedOnly,
        &req
    ));
}

#[test]
fn redacted_only_passes_clean_payload() {
    let req = base_request("workflow stage 3", "[EMAIL_1] confirmed");
    assert!(!requires_redaction_rejection(
        DataHandlingMode::RedactedOnly,
        &req
    ));
}

#[test]
fn redacted_only_skips_rejection_when_server_will_redact() {
    let mut req = base_request("SIN 123-456-789.", "Email alice@example.com.");
    req.redaction = Some(RedactionInfo {
        mode: RedactionMode::Server,
        status: RedactionStatus::NotRequested,
        entities: Vec::new(),
        input_redacted: false,
        proposed_output_redacted: false,
        context_redacted: false,
    });
    assert!(!requires_redaction_rejection(
        DataHandlingMode::RedactedOnly,
        &req
    ));
    assert!(should_apply_server_redaction(&req));
}

#[test]
fn passthrough_context_keys_do_not_trigger_rejection() {
    let mut req = base_request("clean", "clean");
    req.context = json!({
        "workflow_step": "alice@example.com",
        "notes": "ok",
    });
    assert!(!requires_redaction_rejection(
        DataHandlingMode::RedactedOnly,
        &req
    ));
}

#[test]
fn non_passthrough_context_value_triggers_rejection() {
    let mut req = base_request("clean", "clean");
    req.context = json!({ "notes": "alice@example.com" });
    assert!(requires_redaction_rejection(
        DataHandlingMode::RedactedOnly,
        &req
    ));
}

#[test]
fn apply_server_redaction_strips_email_sin_and_income() {
    let mut req = base_request(
        "Alice Example earned $82,000.",
        "Reach alice@example.com about SIN 123-456-789.",
    );
    apply_server_redaction(&mut req);

    assert!(!req.input.contains("$82,000"));
    assert!(!req.proposed_output.contains("alice@example.com"));
    assert!(!req.proposed_output.contains("123-456-789"));
    assert!(req.input.contains("[INCOME_AMOUNT_1]"));
    assert!(req.proposed_output.contains("[EMAIL_1]"));
    assert!(req.proposed_output.contains("[SIN_1]"));
    // PERSON_NAME is intentionally not redacted server-side because the
    // current regex over-matches benign proper-noun pairs. Leave the
    // person name in place; SDK-local redaction handles it.
    assert!(req.input.contains("Alice Example"));
    assert!(!req.input.contains("[PERSON_NAME"));

    let info = req.redaction.expect("redaction info populated");
    assert_eq!(info.mode, RedactionMode::Server);
    assert_eq!(info.status, RedactionStatus::Applied);
    assert!(info.input_redacted);
    assert!(info.proposed_output_redacted);
}

#[test]
fn apply_server_redaction_dedupes_repeated_values() {
    let mut req = base_request(
        "Reach alice@example.com or alice@example.com.",
        "Confirm with alice@example.com.",
    );
    apply_server_redaction(&mut req);

    let info = req.redaction.expect("redaction info populated");
    let email = info
        .entities
        .iter()
        .find(|entity| entity.entity_type == "EMAIL")
        .expect("EMAIL entity present");
    assert_eq!(email.token, "[EMAIL_1]");
    assert_eq!(email.count, 3);
}

#[test]
fn apply_server_redaction_preserves_passthrough_context_keys() {
    let mut req = base_request("ok", "ok");
    req.context = json!({
        "workflow_step": "alice@example.com",
        "notes": "alice@example.com",
    });
    apply_server_redaction(&mut req);

    assert_eq!(req.context["workflow_step"], json!("alice@example.com"));
    assert_eq!(req.context["notes"], json!("[EMAIL_1]"));
    assert!(req.redaction.as_ref().unwrap().context_redacted);
}

#[test]
fn apply_server_redaction_scrubs_run_event_summaries() {
    let mut req = base_request("ok", "ok");
    req.run_event = Some(CreateRunEventRequest {
        kind: RunEventKind::ToolCall,
        sequence: None,
        label: None,
        input_summary: Some("alice@example.com asked about pricing".into()),
        output_summary: Some("Sent to alice@example.com.".into()),
        metadata: serde_json::Value::Null,
        occurred_at: None,
    });
    apply_server_redaction(&mut req);

    let event = req.run_event.expect("event preserved");
    assert!(!event.input_summary.as_deref().unwrap().contains("alice@"));
    assert!(event
        .input_summary
        .as_deref()
        .unwrap()
        .contains("[EMAIL_1]"));
    assert!(event
        .output_summary
        .as_deref()
        .unwrap()
        .contains("[EMAIL_1]"));
}
