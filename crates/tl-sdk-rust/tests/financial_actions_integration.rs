//! Integration coverage for Rust SDK financial action helpers.

use std::time::Duration;

use tl_sdk_rust::{
    Client, CounterpartyRef, CreateFinancialActionRequest, CreateFinancialMandateRequest,
    CreateFinancialPolicyRequest, FinancialAction, FinancialActionDecision, FinancialActionKind,
    FinancialActionOutcome, FinancialActionOutcomeStatus, FinancialActionPrecondition,
    FinancialActionStatus, FinancialDecisionRiskCode, FinancialExecutionProofStatus,
    FinancialMandateStatus, FinancialPolicySelector, FinancialRail, MoneyAmount, PolicyAction,
    RecoveryStatus, RetryConfig, ReversalCapability, Severity, SpendMeter,
};
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn one_shot_retry() -> RetryConfig {
    RetryConfig {
        max_attempts: 1,
        total_budget: Duration::from_millis(50),
        base_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(2),
    }
}

fn refund_request() -> CreateFinancialActionRequest {
    CreateFinancialActionRequest {
        idempotency_key: "idem-refund-75".into(),
        execute: false,
        action: FinancialAction {
            id: None,
            kind: FinancialActionKind::Refund,
            operation: "issue_refund".into(),
            principal_id: "refund-bot".into(),
            amount: MoneyAmount {
                amount_minor: 7_500,
                currency: "USD".into(),
            },
            counterparty: Some(CounterpartyRef {
                id: "cust_456".into(),
                display_name: Some("Ada Customer".into()),
                kind: "customer".into(),
                country: Some("US".into()),
                metadata: serde_json::json!({}),
            }),
            rail: FinancialRail::PaymentHttp,
            mandate: None,
            memo: Some("refund damaged item".into()),
            metadata: serde_json::json!({
                "order_id": "order_123",
                "reason": "damaged_item"
            }),
        },
        evidence: vec![],
    }
}

fn action_body(id: &str, status: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "workspace_id": "default",
        "status": status,
        "action": {
            "id": id,
            "kind": "refund",
            "operation": "issue_refund",
            "principal_id": "refund-bot",
            "amount": { "amount_minor": 7500, "currency": "USD" },
            "counterparty": {
                "id": "cust_456",
                "display_name": "Ada Customer",
                "kind": "customer",
                "country": "US",
                "metadata": {}
            },
            "rail": "payment_http",
            "memo": "refund damaged item",
            "metadata": {
                "order_id": "order_123",
                "reason": "damaged_item"
            }
        },
        "evidence": [],
        "created_at": "2026-05-17T00:00:00Z",
        "updated_at": "2026-05-17T00:00:00Z"
    })
}

fn mandate_request() -> CreateFinancialMandateRequest {
    CreateFinancialMandateRequest {
        id: Some("mandate_refund_bot".into()),
        version: Some(1),
        principal_id: "refund-bot".into(),
        scope: serde_json::json!({
            "action_kinds": ["refund"],
            "max_amount_minor": 10_000,
            "currency": "USD"
        }),
        metadata: serde_json::json!({ "source": "rust_sdk_test" }),
        starts_at: None,
        expires_at: Some("2026-08-05T19:00:00Z".into()),
    }
}

fn mandate_body(status: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "mandate_refund_bot",
        "workspace_id": "default",
        "version": 1,
        "status": status,
        "principal_id": "refund-bot",
        "scope": {
            "action_kinds": ["refund"],
            "max_amount_minor": 10000,
            "currency": "USD"
        },
        "metadata": { "source": "rust_sdk_test" },
        "expires_at": "2026-08-05T19:00:00Z",
        "created_at": "2026-07-05T00:00:00Z",
        "updated_at": "2026-07-05T00:00:00Z"
    })
}

fn financial_policy_request() -> CreateFinancialPolicyRequest {
    CreateFinancialPolicyRequest {
        id: "refund-controls".into(),
        description: Some("Refund controls".into()),
        severity: Some(Severity::High),
        when: FinancialPolicySelector {
            agents: vec!["refund-bot".into()],
            action_kinds: vec![FinancialActionKind::Refund],
            operations: vec!["issue_refund".into()],
            currencies: vec!["USD".into()],
            rails: vec![FinancialRail::PaymentHttp],
        },
        meter: SpendMeter::Actions,
        per_transaction_minor: Some(10_000),
        hold_above_minor: Some(5_000),
        daily_minor: Some(50_000),
        weekly_minor: None,
        monthly_minor: Some(500_000),
        allowed_counterparty_ids: vec![],
        denied_counterparty_ids: vec![],
        hold_new_counterparty: false,
        mandate_required: false,
        approval_threshold_minor: None,
        approver_roles: vec![],
        refund_original_method_only: false,
        required_preconditions: vec![FinancialActionPrecondition::AmountLteRefundableBalance],
        missing_evidence_action: Some(PolicyAction::Escalate),
        failed_precondition_action: Some(PolicyAction::Block),
        on_breach: Some(PolicyAction::Block),
    }
}

fn financial_policy_body() -> serde_json::Value {
    serde_json::json!({
        "id": "refund-controls",
        "description": "Refund controls",
        "severity": "high",
        "when": {
            "agents": ["refund-bot"],
            "action_kinds": ["refund"],
            "operations": ["issue_refund"],
            "currencies": ["USD"],
            "rails": ["payment_http"]
        },
        "per_transaction_minor": 10000,
        "hold_above_minor": 5000,
        "daily_minor": 50000,
        "monthly_minor": 500000,
        "allowed_counterparty_ids": [],
        "denied_counterparty_ids": [],
        "hold_new_counterparty": false,
        "mandate_required": false,
        "approver_roles": [],
        "refund_original_method_only": false,
        "required_preconditions": ["amount_lte_refundable_balance"],
        "missing_evidence_action": "escalate",
        "failed_precondition_action": "block",
        "on_breach": "block",
        "enabled": true
    })
}

fn receipt_body(id: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "action_id": id,
        "trace_id": "018f4444-4444-7444-8444-444444444444",
        "ledger_event_ids": ["ledger_execute_1"],
        "proof": {
            "action_status": "executed",
            "provider_reference": "refund_123"
        },
        "created_at": "2026-07-05T00:00:00Z"
    })
}

fn decision_receipt_body(id: &str) -> serde_json::Value {
    serde_json::json!({
        "schema": "financial_action_decision_receipt.v1",
        "action_id": id,
        "decision": "hold",
        "status": "held",
        "reason": "valid refund, but above threshold so human approval required",
        "amount": { "amount_minor": 7500, "currency": "USD" },
        "operation": "issue_refund",
        "principal_id": "refund-bot",
        "counterparty": {
            "id": "cust_456",
            "display_name": "Ada Customer",
            "kind": "customer",
            "country": "US",
            "metadata": {}
        },
        "authorization_scope": {
            "checked": true,
            "result": "passed",
            "scope_ref": { "id": "mandate_refund_bot", "version": 1 },
            "source": "financial_authorization_service",
            "reason": "refund-bot may spend up to USD 100.00"
        },
        "evidence": [],
        "risks": [{
            "code": "amount_above_auto_approve_threshold",
            "severity": "high",
            "reason": "amount at or above hold threshold",
            "policy_id": "refund-controls",
            "source": "financial_policy"
        }],
        "execution": {
            "status": "not_started",
            "ledger_event_ids": []
        },
        "created_at": "2026-07-05T00:00:00Z",
        "updated_at": "2026-07-05T00:00:00Z"
    })
}

fn outcome(action_id: &str) -> FinancialActionOutcome {
    FinancialActionOutcome {
        action_id: action_id.into(),
        status: FinancialActionOutcomeStatus::Succeeded,
        reversal_capability: ReversalCapability::ManualRecovery,
        recovery_status: RecoveryStatus::ManualRequired,
        provider_status: Some("provider_status".into()),
        provider_reference: Some("provider_ref_123".into()),
        final_loss_amount: None,
        occurred_at: "2026-07-05T20:00:00Z".into(),
        metadata: serde_json::json!({ "source": "rust_sdk_test" }),
    }
}

#[tokio::test]
async fn verify_action_posts_typed_request_with_bearer_auth() {
    let server = MockServer::start().await;
    let request = refund_request();
    Mock::given(method("POST"))
        .and(path("/v1/financial/actions"))
        .and(header("authorization", "Bearer secret"))
        .and(body_json(&request))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(action_body("act_refund_75", "proposed")),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri())
        .with_api_key("secret")
        .with_retry(one_shot_retry());
    let action = client.verify_action(&request).await.unwrap();

    assert_eq!(action.id, "act_refund_75");
    assert_eq!(action.status, FinancialActionStatus::Proposed);
    assert_eq!(action.action.kind, FinancialActionKind::Refund);
}

#[tokio::test]
async fn guard_payment_aliases_verify_action_for_payment_ergonomics() {
    let server = MockServer::start().await;
    let request = refund_request();
    Mock::given(method("POST"))
        .and(path("/v1/financial/actions"))
        .and(body_json(&request))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(action_body("act_refund_75", "proposed")),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());
    let action = client.guard_payment(&request).await.unwrap();

    assert_eq!(action.id, "act_refund_75");
}

#[tokio::test]
async fn financial_action_helpers_encode_ids_and_parse_statuses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/financial/actions/action%2Fone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(action_body("action/one", "held")))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/financial/actions/action%2Fone/approve"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(action_body("action/one", "authorized")),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/financial/actions/action%2Fone/deny"))
        .respond_with(ResponseTemplate::new(200).set_body_json(action_body("action/one", "denied")))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/financial/actions/action%2Fone/execute"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(action_body("action/one", "executed")),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());

    let action = client.get_financial_action("action/one").await.unwrap();
    assert_eq!(action.status, FinancialActionStatus::Held);

    let approved = client.approve_action("action/one").await.unwrap();
    assert_eq!(approved.status, FinancialActionStatus::Authorized);

    let denied = client.deny_action("action/one").await.unwrap();
    assert_eq!(denied.status, FinancialActionStatus::Denied);

    let executed = client.execute_action("action/one").await.unwrap();
    assert_eq!(executed.status, FinancialActionStatus::Executed);
}

#[tokio::test]
async fn list_financial_actions_fetches_collection() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/financial/actions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "actions": [action_body("act_refund_75", "proposed")]
        })))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());
    let actions = client.list_financial_actions().await.unwrap();

    assert_eq!(actions.actions.len(), 1);
    assert_eq!(actions.actions[0].id, "act_refund_75");
}

#[tokio::test]
async fn financial_policy_helpers_create_and_list_controls() {
    let server = MockServer::start().await;
    let request = financial_policy_request();
    Mock::given(method("POST"))
        .and(path("/v1/financial/policies"))
        .and(body_json(&request))
        .respond_with(ResponseTemplate::new(201).set_body_json(financial_policy_body()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/financial/policies"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "policies": [financial_policy_body()]
        })))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());

    let policy = client.create_financial_policy(&request).await.unwrap();
    assert_eq!(policy.id, "refund-controls");

    let policies = client.list_financial_policies().await.unwrap();
    assert_eq!(policies.policies.len(), 1);
    assert_eq!(policies.policies[0].when.agents, vec!["refund-bot"]);
}

#[tokio::test]
async fn financial_mandate_helpers_create_list_and_revoke() {
    let server = MockServer::start().await;
    let request = mandate_request();
    Mock::given(method("POST"))
        .and(path("/v1/financial/mandates"))
        .and(body_json(&request))
        .respond_with(ResponseTemplate::new(201).set_body_json(mandate_body("active")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/financial/mandates"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "mandates": [mandate_body("active")]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/financial/mandates/mandate_refund_bot/revoke"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mandate_body("revoked")))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());

    let mandate = client.create_mandate(&request).await.unwrap();
    assert_eq!(mandate.status, FinancialMandateStatus::Active);

    let mandates = client.list_mandates().await.unwrap();
    assert_eq!(mandates.mandates.len(), 1);

    let revoked = client.revoke_mandate("mandate_refund_bot").await.unwrap();
    assert_eq!(revoked.status, FinancialMandateStatus::Revoked);
}

#[tokio::test]
async fn list_approval_requests_fetches_collection() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/financial/approval-requests"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "approval_requests": [{
                "id": "approval_1",
                "workspace_id": "default",
                "action_id": "act_refund_75",
                "status": "pending",
                "reason": "above threshold",
                "approver_roles": ["finance"],
                "metadata": {},
                "created_at": "2026-07-05T00:00:00Z",
                "updated_at": "2026-07-05T00:00:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());
    let approvals = client.list_approval_requests().await.unwrap();

    assert_eq!(approvals.approval_requests.len(), 1);
    assert_eq!(approvals.approval_requests[0].reason, "above threshold");
}

#[tokio::test]
async fn get_receipt_fetches_financial_proof() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/financial/receipts/receipt%2Fone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(receipt_body("receipt/one")))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());
    let receipt = client.get_receipt("receipt/one").await.unwrap();

    assert_eq!(receipt.id, "receipt/one");
    assert_eq!(receipt.action_id, "receipt/one");
    assert_eq!(receipt.proof["action_status"], "executed");
}

#[tokio::test]
async fn get_financial_decision_receipt_fetches_per_action_proof() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/financial/actions/action%2Fone/decision-receipt"))
        .respond_with(ResponseTemplate::new(200).set_body_json(decision_receipt_body("action/one")))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());
    let receipt = client
        .get_financial_decision_receipt("action/one")
        .await
        .unwrap();

    assert_eq!(receipt.decision, FinancialActionDecision::Hold);
    assert_eq!(
        receipt.risks[0].code,
        FinancialDecisionRiskCode::AmountAboveAutoApproveThreshold
    );
    assert_eq!(
        receipt.execution.status,
        FinancialExecutionProofStatus::NotStarted
    );
}

#[tokio::test]
async fn financial_outcome_helpers_record_and_list() {
    let server = MockServer::start().await;
    let expected_outcome = outcome("action/one");
    Mock::given(method("POST"))
        .and(path("/v1/financial/actions/action%2Fone/outcomes"))
        .and(body_json(&expected_outcome))
        .respond_with(ResponseTemplate::new(201).set_body_json(&expected_outcome))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/financial/actions/action%2Fone/outcomes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "outcomes": [expected_outcome]
        })))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());
    let recorded = client
        .record_action_outcome("action/one", &outcome("action/one"))
        .await
        .unwrap();
    let outcomes = client.list_action_outcomes("action/one").await.unwrap();

    assert_eq!(recorded.status, FinancialActionOutcomeStatus::Succeeded);
    assert_eq!(outcomes.outcomes.len(), 1);
    assert_eq!(
        outcomes.outcomes[0].reversal_capability,
        ReversalCapability::ManualRecovery
    );
}
