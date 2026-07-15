use std::sync::Arc;

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use tl_core::{
    AgenticPaymentAuthorizationResponse, AgenticPaymentAuthorizeRequest,
    AgenticPaymentCommitRequest, AgenticPaymentRecord, AgenticPaymentRollbackRequest,
    AuthorityRequirement, AuthorizationCapabilityId, AuthorizationDecision, AuthorizationDomain,
    AuthorizationEffect, AuthorizationFinding, AuthorizationGrantScope, AuthorizationIntentStatus,
    AuthorizationSubject, CompleteAuthorizationLeaseRequest, CounterpartyRef,
    CreateFinancialActionRequest, CreateFinancialPolicyRequest, FinancialAction,
    FinancialActionKind, FinancialActionListResponse, FinancialActionOutcome,
    FinancialActionOutcomeStatus, FinancialActionPrecondition, FinancialActionRecord,
    FinancialExecutionStatus, FinancialGrantScope, FinancialOutcomeListResponse,
    FinancialPolicyListResponse, FinancialPolicyRecord, FinancialPolicySelector, FinancialRail,
    FinancialReceipt, LeaseStatus, Severity, X402NormalizedPaymentRequirement,
    DEFAULT_ENVIRONMENT_ID,
};
use tl_policy::{validate_family_policy, FamilyPolicy, FinancialPolicy, FinancialWhen};

use super::{
    validation::validate_create_action, x402, AgenticPaymentBudgetReservationRequest,
    FinancialBudgetConstraint, FinancialBudgetReservationOutcome,
    FinancialBudgetReservationRequest, FinancialBudgetWindow, FinancialExecutor,
    FinancialLedgerEntryKind, FinancialStore, FinancialStoreError,
};
use crate::auth::WorkspaceKeyContext;
use crate::authorization::{
    adapters::AuthorizationAdapterRegistry, AuthorizationCoordinator, AuthorizationError,
    AuthorizationEvaluationRequest, MemoryAuthorizationStore,
};
use crate::budget_alerts::BudgetAlertRuntime;
use crate::policies::{MemoryPolicyStore, PolicyStore};

#[derive(Clone)]
pub struct FinancialAuthorizationService {
    store: Arc<dyn FinancialStore>,
    policy_store: Arc<dyn PolicyStore>,
    authorization: AuthorizationCoordinator,
    executor: Option<Arc<dyn FinancialExecutor>>,
    budget_alerts: Option<BudgetAlertRuntime>,
}

impl FinancialAuthorizationService {
    pub fn new(store: Arc<dyn FinancialStore>) -> Self {
        let policies: Arc<dyn PolicyStore> = Arc::new(MemoryPolicyStore::new());
        let authorization = AuthorizationCoordinator::new(
            Arc::new(MemoryAuthorizationStore::new()),
            policies.clone(),
            Arc::new(AuthorizationAdapterRegistry::new()),
        );
        Self {
            store,
            policy_store: policies,
            authorization,
            executor: None,
            budget_alerts: None,
        }
    }

    pub fn with_policy_store(
        store: Arc<dyn FinancialStore>,
        policy_store: Arc<dyn PolicyStore>,
    ) -> Self {
        let authorization = AuthorizationCoordinator::new(
            Arc::new(MemoryAuthorizationStore::new()),
            policy_store.clone(),
            Arc::new(AuthorizationAdapterRegistry::new()),
        );
        Self {
            store,
            policy_store,
            authorization,
            executor: None,
            budget_alerts: None,
        }
    }

    pub fn with_policy_store_and_executor(
        store: Arc<dyn FinancialStore>,
        policy_store: Arc<dyn PolicyStore>,
        authorization: AuthorizationCoordinator,
        executor: Arc<dyn FinancialExecutor>,
    ) -> Self {
        Self {
            store,
            policy_store,
            authorization,
            executor: Some(executor),
            budget_alerts: None,
        }
    }

    pub fn with_budget_alerts(mut self, runtime: BudgetAlertRuntime) -> Self {
        self.budget_alerts = Some(runtime);
        self
    }

    pub async fn create_action(
        &self,
        workspace_id: &str,
        input: CreateFinancialActionRequest,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.create_action_in_environment(workspace_id, DEFAULT_ENVIRONMENT_ID, input)
            .await
    }

    pub async fn create_action_in_environment(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: CreateFinancialActionRequest,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let execute = input.execute;
        let (action, decision) = self
            .create_and_authorize(workspace_id, environment_id, input, execute)
            .await?;
        if execute && decision.effect.is_executable() {
            self.execute_authorized(workspace_id, environment_id, action, decision)
                .await
        } else {
            Ok(action)
        }
    }

    async fn create_and_authorize(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: CreateFinancialActionRequest,
        executable_attempt: bool,
    ) -> Result<(FinancialActionRecord, AuthorizationDecision), FinancialStoreError> {
        validate_create_action(&input)?;
        let claim = input.authorization.clone();
        let action = self
            .store
            .create_action(workspace_id, environment_id, input)
            .await?;
        let attempt_id = executable_attempt.then(|| format!("financial:{}:execute", action.id));
        let decision = self
            .evaluate_action(workspace_id, environment_id, &action, claim, attempt_id)
            .await?;
        let mut projected = self
            .store
            .update_authorization(
                workspace_id,
                environment_id,
                &action.id,
                decision.intent_id.as_deref(),
                decision.receipt_id.as_deref(),
                decision.effect,
                decision
                    .status
                    .unwrap_or(AuthorizationIntentStatus::Evaluating),
            )
            .await?;
        projected.authorization = Some(decision.clone());
        Ok((projected, decision))
    }

    pub async fn get_action(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.store
            .get_action(workspace_id, environment_id, action_id)
            .await
    }

    pub async fn list_actions(
        &self,
        workspace_id: &str,
        environment_id: Option<&str>,
    ) -> Result<FinancialActionListResponse, FinancialStoreError> {
        self.store.list_actions(workspace_id, environment_id).await
    }

    pub async fn execute_action(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action_id: &str,
        input: tl_core::ExecuteFinancialActionRequest,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let action = self
            .store
            .get_action(workspace_id, environment_id, action_id)
            .await?;
        if action.execution_status == FinancialExecutionStatus::Succeeded {
            return Ok(action);
        }
        let decision = self
            .evaluate_action(
                workspace_id,
                environment_id,
                &action,
                input.authorization,
                input.attempt_id.or_else(|| {
                    Some(format!(
                        "financial:{action_id}:execute:{}",
                        uuid::Uuid::now_v7()
                    ))
                }),
            )
            .await?;
        let mut action = self
            .store
            .update_authorization(
                workspace_id,
                environment_id,
                action_id,
                decision.intent_id.as_deref(),
                decision.receipt_id.as_deref(),
                decision.effect,
                decision
                    .status
                    .unwrap_or(AuthorizationIntentStatus::Evaluating),
            )
            .await?;
        action.authorization = Some(decision.clone());
        if !decision.effect.is_executable() {
            return Ok(action);
        }
        self.execute_authorized(workspace_id, environment_id, action, decision)
            .await
    }

    async fn execute_authorized(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action: FinancialActionRecord,
        decision: AuthorizationDecision,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let authorization_receipt_id = decision.receipt_id.clone().ok_or_else(|| {
            FinancialStoreError::Internal("authorization decision has no receipt".into())
        })?;
        let mut ledger_ids = Vec::new();
        let reserve = self.reserve_action_budget(workspace_id, &action).await?;
        match reserve {
            FinancialBudgetReservationOutcome::Denied { .. } => {
                self.complete_decision_lease(workspace_id, environment_id, &decision, false)
                    .await?;
                return Err(FinancialStoreError::Conflict);
            }
            FinancialBudgetReservationOutcome::Reserved {
                ledger_entry_id, ..
            } => ledger_ids.push(ledger_entry_id),
        }
        let executing = self
            .store
            .transition_execution(
                workspace_id,
                environment_id,
                &action.id,
                FinancialExecutionStatus::Executing,
                None,
            )
            .await?;
        let executor = self.executor.as_ref().ok_or_else(|| {
            FinancialStoreError::Internal("financial executor unavailable".into())
        })?;
        let execution = executor
            .execute(
                workspace_id,
                &executing,
                &format!("financial-execute:{}", action.id),
            )
            .await;
        match execution {
            Ok(result) => {
                ledger_ids.push(
                    self.record_ledger(
                        workspace_id,
                        &action,
                        FinancialLedgerEntryKind::Released,
                        "released_after_execution",
                    )
                    .await?,
                );
                ledger_ids.push(
                    self.record_ledger(
                        workspace_id,
                        &action,
                        FinancialLedgerEntryKind::Executed,
                        "executed",
                    )
                    .await?,
                );
                let mut succeeded = self
                    .store
                    .transition_execution(
                        workspace_id,
                        environment_id,
                        &action.id,
                        FinancialExecutionStatus::Succeeded,
                        None,
                    )
                    .await?;
                succeeded.authorization = Some(decision.clone());
                self.store
                    .create_receipt(
                        workspace_id,
                        &action.id,
                        &authorization_receipt_id,
                        Some(&decision.trace_id),
                        ledger_ids,
                        result.provider_response.clone(),
                    )
                    .await?;
                self.store
                    .record_action_outcome(
                        workspace_id,
                        &action.id,
                        FinancialActionOutcome {
                            action_id: action.id.clone(),
                            status: FinancialActionOutcomeStatus::Succeeded,
                            reversal_capability: result.reversal_capability,
                            recovery_status: result.recovery_status,
                            provider_status: result.provider_status,
                            provider_reference: result.provider_reference,
                            final_loss_amount: None,
                            occurred_at: Utc::now().to_rfc3339(),
                            metadata: serde_json::json!({}),
                        },
                    )
                    .await?;
                self.notify_budget_alerts(workspace_id, environment_id, &succeeded)
                    .await;
                self.complete_decision_lease(workspace_id, environment_id, &decision, true)
                    .await?;
                Ok(succeeded)
            }
            Err(error) => {
                let _ = self
                    .record_ledger(
                        workspace_id,
                        &action,
                        FinancialLedgerEntryKind::Released,
                        "released_after_failure",
                    )
                    .await;
                let mut failed = self
                    .store
                    .transition_execution(
                        workspace_id,
                        environment_id,
                        &action.id,
                        FinancialExecutionStatus::Failed,
                        Some(&error.to_string()),
                    )
                    .await?;
                failed.authorization = Some(decision.clone());
                self.complete_decision_lease(workspace_id, environment_id, &decision, false)
                    .await?;
                Ok(failed)
            }
        }
    }

    async fn evaluate_action(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action: &FinancialActionRecord,
        claim: Option<tl_core::AuthorizationClaim>,
        attempt_id: Option<String>,
    ) -> Result<AuthorizationDecision, FinancialStoreError> {
        let families = self
            .policy_store
            .list_enabled_families(workspace_id, environment_id)
            .await
            .map_err(|error| FinancialStoreError::Internal(error.to_string()))?;
        let (findings, requirements, versions) =
            self.policy_inputs(workspace_id, action, &families).await?;
        self.authorization
            .evaluate(AuthorizationEvaluationRequest {
                workspace_id: workspace_id.to_string(),
                environment_id: environment_id.to_string(),
                principal_id: action.action.principal_id.clone(),
                subject: AuthorizationSubject::Financial {
                    action_id: action.id.clone(),
                    action: action.action.clone(),
                },
                findings,
                requirements,
                policy_versions: versions,
                claim,
                attempt_id,
                trace_id: uuid::Uuid::now_v7().to_string(),
                transformed_value: None,
                intent_expires_at: Some(Utc::now() + Duration::minutes(15)),
            })
            .await
            .map_err(authorization_error)
    }

    async fn policy_inputs(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        families: &[Arc<FamilyPolicy>],
    ) -> Result<
        (
            Vec<AuthorizationFinding>,
            Vec<AuthorityRequirement>,
            Vec<String>,
        ),
        FinancialStoreError,
    > {
        let capability = AuthorizationCapabilityId::parse(format!(
            "financial:{}",
            action
                .action
                .operation
                .to_ascii_lowercase()
                .replace(' ', "_")
        ))
        .map_err(|message| FinancialStoreError::Validation(message.into()))?;
        let mut findings = Vec::new();
        let mut requirements = Vec::new();
        let mut versions = Vec::new();
        let (day_start, week_start, month_start) = financial_window_starts(Utc::now())?;
        let mut spend = None;
        for family in families {
            let FamilyPolicy::Financial(policy) = family.as_ref() else {
                continue;
            };
            if !financial_matches(policy, &action.action) {
                continue;
            }
            versions.push(policy.id.to_string());
            let mut add = |rule: &str,
                           effect: AuthorizationEffect,
                           reason: String,
                           reusable: bool|
             -> Result<(), FinancialStoreError> {
                let requirement_id = (effect == AuthorizationEffect::RequireApproval)
                    .then(|| format!("financial:{}:{rule}", policy.id));
                if let Some(id) = requirement_id.clone() {
                    requirements.push(AuthorityRequirement {
                        id: id.clone(),
                        capability: capability.clone(),
                        required_scope: financial_scope(&action.action),
                        approver_roles: policy.approver_roles.clone(),
                        reason: reason.clone(),
                        reusable_allowed: reusable,
                        max_grant_ttl_seconds: Some(30 * 24 * 60 * 60),
                    });
                }
                findings.push(AuthorizationFinding {
                    id: format!("financial:{}:{rule}", policy.id),
                    source: "financial_policy".into(),
                    effect,
                    reason,
                    severity: policy.severity,
                    policy_id: Some(policy.id.to_string()),
                    requirement_id,
                    remediation: None,
                    evidence: serde_json::json!({ "action_id": action.id }),
                });
                Ok(())
            };

            if let Some(counterparty) = &action.action.counterparty {
                if policy.denied_counterparty_ids.contains(&counterparty.id) {
                    add(
                        "counterparty_denied",
                        AuthorizationEffect::Deny,
                        format!(
                            "counterparty `{}` is denied by `{}`",
                            counterparty.id, policy.id
                        ),
                        false,
                    )?;
                }
                if (!policy.allowed_counterparty_ids.is_empty()
                    && !policy.allowed_counterparty_ids.contains(&counterparty.id))
                    || (policy.require_approval_for_new_counterparty
                        && !policy.allowed_counterparty_ids.contains(&counterparty.id))
                {
                    add(
                        "counterparty_approval",
                        AuthorizationEffect::RequireApproval,
                        format!("counterparty `{}` requires approval", counterparty.id),
                        true,
                    )?;
                }
            } else if policy.require_approval_for_new_counterparty
                || !policy.allowed_counterparty_ids.is_empty()
            {
                add(
                    "counterparty_missing",
                    AuthorizationEffect::RequireApproval,
                    "missing counterparty requires approval".into(),
                    true,
                )?;
            }
            if policy.grant_required {
                add(
                    "grant_required",
                    AuthorizationEffect::RequireApproval,
                    format!("policy `{}` requires delegated authority", policy.id),
                    true,
                )?;
            }
            if policy
                .per_transaction_minor
                .is_some_and(|cap| action.action.amount.amount_minor > cap)
            {
                add(
                    "per_transaction_cap",
                    enforcing_effect(policy.on_breach),
                    format!("amount exceeds the per-transaction cap for `{}`", policy.id),
                    false,
                )?;
            }
            if policy
                .approval_threshold_minor
                .is_some_and(|threshold| action.action.amount.amount_minor >= threshold)
            {
                add(
                    "approval_threshold",
                    AuthorizationEffect::RequireApproval,
                    format!("amount reaches the approval threshold for `{}`", policy.id),
                    true,
                )?;
            }
            for precondition in &policy.required_preconditions {
                match evidence_bool(action, precondition_key(*precondition)) {
                    Some(true) => {}
                    Some(false) => {
                        let rule =
                            format!("precondition_{}_failed", precondition_key(*precondition));
                        add(
                            &rule,
                            enforcing_effect(policy.failed_precondition_effect),
                            format!("required precondition `{precondition:?}` failed"),
                            false,
                        )?
                    }
                    None => {
                        let rule =
                            format!("precondition_{}_missing", precondition_key(*precondition));
                        add(
                            &rule,
                            enforcing_effect(policy.missing_evidence_effect),
                            format!("evidence for `{precondition:?}` is missing"),
                            false,
                        )?
                    }
                }
            }
            if policy.daily_minor.is_some()
                || policy.weekly_minor.is_some()
                || policy.monthly_minor.is_some()
            {
                let (today, week, month) = match spend {
                    Some(value) => value,
                    None => {
                        let value = (
                            self.store
                                .net_spend_minor(
                                    workspace_id,
                                    &action.action.principal_id,
                                    &action.action.amount.currency,
                                    day_start,
                                    Utc::now(),
                                )
                                .await?,
                            self.store
                                .net_spend_minor(
                                    workspace_id,
                                    &action.action.principal_id,
                                    &action.action.amount.currency,
                                    week_start,
                                    Utc::now(),
                                )
                                .await?,
                            self.store
                                .net_spend_minor(
                                    workspace_id,
                                    &action.action.principal_id,
                                    &action.action.amount.currency,
                                    month_start,
                                    Utc::now(),
                                )
                                .await?,
                        );
                        spend = Some(value);
                        value
                    }
                };
                for (label, used, cap) in [
                    ("daily", today, policy.daily_minor),
                    ("weekly", week, policy.weekly_minor),
                    ("monthly", month, policy.monthly_minor),
                ] {
                    if cap.is_some_and(|cap| {
                        used.saturating_add(action.action.amount.amount_minor) > cap
                    }) {
                        add(
                            &format!("{label}_cap"),
                            enforcing_effect(policy.on_breach),
                            format!("{label} spend would exceed the cap for `{}`", policy.id),
                            false,
                        )?;
                    }
                }
            }
        }
        Ok((findings, requirements, versions))
    }

    async fn reserve_action_budget(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
    ) -> Result<FinancialBudgetReservationOutcome, FinancialStoreError> {
        let families = self
            .policy_store
            .list_enabled_families(workspace_id, &action.environment_id)
            .await
            .map_err(|error| FinancialStoreError::Internal(error.to_string()))?;
        let mut constraints = Vec::new();
        for family in families {
            let FamilyPolicy::Financial(policy) = family.as_ref() else {
                continue;
            };
            if !financial_matches(policy, &action.action) {
                continue;
            }
            for (window, cap_minor) in [
                (FinancialBudgetWindow::Day, policy.daily_minor),
                (FinancialBudgetWindow::Week, policy.weekly_minor),
                (FinancialBudgetWindow::Month, policy.monthly_minor),
            ] {
                if let Some(cap_minor) = cap_minor {
                    constraints.push(FinancialBudgetConstraint {
                        policy_id: policy.id.to_string(),
                        window,
                        cap_minor,
                        block_on_breach: true,
                    });
                }
            }
        }
        let now = Utc::now();
        let (day_start, week_start, month_start) = financial_window_starts(now)?;
        self.store
            .try_reserve_action_budget(FinancialBudgetReservationRequest {
                workspace_id: workspace_id.to_string(),
                action_id: action.id.clone(),
                principal_id: action.action.principal_id.clone(),
                amount: action.action.amount.clone(),
                idempotency_key: format!("financial:{}:reserved", action.id),
                day_start,
                week_start,
                month_start,
                now,
                constraints,
                metadata: serde_json::json!({ "authorization_receipt_id": action.authorization_receipt_id }),
            })
            .await
    }

    async fn record_ledger(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        kind: FinancialLedgerEntryKind,
        suffix: &str,
    ) -> Result<String, FinancialStoreError> {
        self.store
            .record_ledger_entry(
                workspace_id,
                &action.id,
                kind,
                action.action.amount.amount_minor,
                &action.action.amount.currency,
                &format!("financial:{}:{suffix}", action.id),
                serde_json::json!({ "action_id": action.id }),
            )
            .await
    }

    async fn notify_budget_alerts(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action: &FinancialActionRecord,
    ) {
        let Some(runtime) = &self.budget_alerts else {
            return;
        };
        let principal_id = &action.action.principal_id;
        let currency = &action.action.amount.currency;
        crate::budget_alerts::evaluate_spend_alerts(
            crate::budget_alerts::SpendAlertEvaluation {
                runtime,
                policy_store: self.policy_store.as_ref(),
                workspace_id,
                environment_id,
                principal_id,
                currency,
                meter: tl_core::SpendMeter::Actions,
            },
            |policy| financial_matches(policy, &action.action),
            |window_start, now| async move {
                self.store
                    .net_spend_minor(workspace_id, principal_id, currency, window_start, now)
                    .await
                    .map_err(|error| error.to_string())
            },
        )
        .await;
    }

    async fn complete_decision_lease(
        &self,
        workspace_id: &str,
        environment_id: &str,
        decision: &AuthorizationDecision,
        success: bool,
    ) -> Result<(), FinancialStoreError> {
        if let Some(lease) = &decision.lease {
            self.authorization
                .complete_lease(
                    workspace_id,
                    environment_id,
                    &lease.id,
                    CompleteAuthorizationLeaseRequest {
                        status: if success {
                            LeaseStatus::Consumed
                        } else {
                            LeaseStatus::Canceled
                        },
                        outcome: serde_json::json!({ "success": success }),
                    },
                )
                .await
                .map_err(authorization_error)?;
        }
        Ok(())
    }

    pub async fn create_financial_policy(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: CreateFinancialPolicyRequest,
    ) -> Result<FinancialPolicyRecord, FinancialStoreError> {
        let policy = financial_policy_from_request(input)?;
        let family = FamilyPolicy::Financial(policy.clone());
        if let Err(issues) = validate_family_policy(&family) {
            return Err(FinancialStoreError::Validation(
                issues
                    .into_iter()
                    .map(|issue| format!("{}: {}", issue.path, issue.message))
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }
        let source_yaml = serde_yaml::to_string(&family)
            .map_err(|error| FinancialStoreError::Internal(error.to_string()))?;
        self.policy_store
            .upsert_family(workspace_id, environment_id, &family, &source_yaml)
            .await
            .map_err(|error| FinancialStoreError::Internal(error.to_string()))?;
        Ok(financial_policy_record(&policy, true))
    }

    pub async fn list_financial_policies(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<FinancialPolicyListResponse, FinancialStoreError> {
        let policies = self
            .policy_store
            .list_enabled_families(workspace_id, environment_id)
            .await
            .map_err(|error| FinancialStoreError::Internal(error.to_string()))?
            .into_iter()
            .filter_map(|family| match family.as_ref() {
                FamilyPolicy::Financial(policy) => Some(financial_policy_record(policy, true)),
                _ => None,
            })
            .collect();
        Ok(FinancialPolicyListResponse { policies })
    }

    pub async fn get_receipt(
        &self,
        workspace_id: &str,
        receipt_id: &str,
    ) -> Result<FinancialReceipt, FinancialStoreError> {
        self.store.get_receipt(workspace_id, receipt_id).await
    }

    pub async fn record_action_outcome(
        &self,
        workspace_id: &str,
        action_id: &str,
        outcome: FinancialActionOutcome,
    ) -> Result<FinancialActionOutcome, FinancialStoreError> {
        self.store
            .record_action_outcome(workspace_id, action_id, outcome)
            .await
    }

    pub async fn list_action_outcomes(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialOutcomeListResponse, FinancialStoreError> {
        self.store
            .list_action_outcomes(workspace_id, action_id)
            .await
    }

    pub async fn authorize_agentic_payment_in_environment(
        &self,
        workspace_id: &str,
        environment_id: &str,
        runtime_key: Option<WorkspaceKeyContext>,
        input: AgenticPaymentAuthorizeRequest,
    ) -> Result<AgenticPaymentAuthorizationResponse, FinancialStoreError> {
        let principal_id = agentic_payment_principal(&input.principal_id, runtime_key.as_ref())?;
        let normalized = x402::normalize_payment_requirement(&input.payment_requirement)?;
        let expires_at = input
            .reservation_expires_at
            .as_deref()
            .map(parse_rfc3339)
            .transpose()?
            .unwrap_or_else(|| Utc::now() + Duration::minutes(15));
        let session_limit_minor = input
            .session_limit_minor
            .unwrap_or(normalized.amount.amount_minor);
        let create = CreateFinancialActionRequest {
            idempotency_key: input.idempotency_key.clone(),
            execute: false,
            authorization: input.authorization.clone(),
            action: FinancialAction {
                id: None,
                kind: FinancialActionKind::Payment,
                operation: input
                    .operation
                    .clone()
                    .unwrap_or_else(|| "x402_payment".into()),
                principal_id: principal_id.clone(),
                amount: normalized.amount.clone(),
                counterparty: Some(agentic_payment_counterparty(&normalized)),
                rail: FinancialRail::X402,
                memo: Some("x402 agentic payment authorization".into()),
                metadata: agentic_payment_metadata(
                    &input,
                    &normalized,
                    session_limit_minor,
                    expires_at,
                ),
            },
            evidence: input.evidence.clone(),
        };
        let (action, decision) = self
            .create_and_authorize(workspace_id, environment_id, create, true)
            .await?;
        let reservation = if decision.effect.is_executable() {
            Some(
                self.store
                    .try_reserve_agentic_payment_budget(AgenticPaymentBudgetReservationRequest {
                        workspace_id: workspace_id.to_string(),
                        session_id: input.session_id,
                        principal_id,
                        action_id: action.id.clone(),
                        payment_requirement_hash: normalized.payment_requirement_hash.clone(),
                        amount: normalized.amount.clone(),
                        session_limit_minor,
                        expires_at,
                        metadata: serde_json::json!({ "authorization_receipt_id": decision.receipt_id }),
                    })
                    .await?,
            )
        } else {
            None
        };
        let signable = decision.effect.is_executable() && reservation.is_some();
        let reason = decision.reason.clone();
        let authorization_receipt_id = decision.receipt_id.clone();
        let record = AgenticPaymentRecord {
            id: action.id.clone(),
            authorization: decision.clone(),
            action,
            normalized_requirement: normalized,
            reservation,
            proof: None,
            receipt_id: None,
        };
        Ok(AgenticPaymentAuthorizationResponse {
            authorization: decision,
            signable,
            reason,
            record,
            authorization_receipt_id,
        })
    }

    pub async fn get_agentic_payment(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action_id: &str,
    ) -> Result<AgenticPaymentRecord, FinancialStoreError> {
        let action = self
            .store
            .get_action(workspace_id, environment_id, action_id)
            .await?;
        let normalized = normalized_requirement_from_action(&action)?;
        let reservation = self
            .store
            .get_agentic_payment_reservation(workspace_id, action_id)
            .await
            .ok();
        let receipt_id = self
            .store
            .get_receipt(workspace_id, action_id)
            .await
            .ok()
            .map(|r| r.id);
        Ok(AgenticPaymentRecord {
            id: action.id.clone(),
            authorization: decision_from_action(&action),
            action,
            normalized_requirement: normalized,
            reservation,
            proof: None,
            receipt_id,
        })
    }

    pub async fn commit_agentic_payment(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action_id: &str,
        runtime_key: Option<WorkspaceKeyContext>,
        input: AgenticPaymentCommitRequest,
    ) -> Result<AgenticPaymentRecord, FinancialStoreError> {
        let action = self
            .store
            .get_action(workspace_id, environment_id, action_id)
            .await?;
        ensure_agentic_payment_principal(&action, runtime_key.as_ref())?;
        if !action.authorization_effect.is_executable() {
            return Err(FinancialStoreError::Conflict);
        }
        let normalized = normalized_requirement_from_action(&action)?;
        x402::verify_settlement_proof(&normalized, &input.proof)?;
        let reservation = self
            .store
            .commit_agentic_payment_reservation(
                workspace_id,
                action_id,
                serde_json::to_value(&input.proof)
                    .map_err(|error| FinancialStoreError::Internal(error.to_string()))?,
            )
            .await?;
        if action.execution_status == FinancialExecutionStatus::NotStarted {
            self.store
                .transition_execution(
                    workspace_id,
                    environment_id,
                    action_id,
                    FinancialExecutionStatus::Executing,
                    None,
                )
                .await?;
        }
        let action = self
            .store
            .transition_execution(
                workspace_id,
                environment_id,
                action_id,
                FinancialExecutionStatus::Succeeded,
                None,
            )
            .await?;
        let ledger_id = self
            .record_ledger(
                workspace_id,
                &action,
                FinancialLedgerEntryKind::Executed,
                "x402_executed",
            )
            .await?;
        let authorization_receipt_id =
            action.authorization_receipt_id.as_deref().ok_or_else(|| {
                FinancialStoreError::Internal("agentic payment has no authorization receipt".into())
            })?;
        let receipt = self
            .store
            .create_receipt(
                workspace_id,
                action_id,
                authorization_receipt_id,
                None,
                vec![ledger_id],
                serde_json::to_value(&input.proof)
                    .map_err(|error| FinancialStoreError::Internal(error.to_string()))?,
            )
            .await?;
        Ok(AgenticPaymentRecord {
            id: action.id.clone(),
            authorization: decision_from_action(&action),
            action,
            normalized_requirement: normalized,
            reservation: Some(reservation),
            proof: Some(input.proof),
            receipt_id: Some(receipt.id),
        })
    }

    pub async fn rollback_agentic_payment(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action_id: &str,
        runtime_key: Option<WorkspaceKeyContext>,
        input: AgenticPaymentRollbackRequest,
    ) -> Result<AgenticPaymentRecord, FinancialStoreError> {
        let action = self
            .store
            .get_action(workspace_id, environment_id, action_id)
            .await?;
        ensure_agentic_payment_principal(&action, runtime_key.as_ref())?;
        let normalized = normalized_requirement_from_action(&action)?;
        let reservation = self
            .store
            .release_agentic_payment_reservation(
                workspace_id,
                action_id,
                &input.reason,
                input.metadata,
            )
            .await?;
        let action = if action.execution_status == FinancialExecutionStatus::NotStarted
            || action.execution_status == FinancialExecutionStatus::Executing
            || action.execution_status == FinancialExecutionStatus::Failed
        {
            self.store
                .transition_execution(
                    workspace_id,
                    environment_id,
                    action_id,
                    FinancialExecutionStatus::Canceled,
                    Some(&input.reason),
                )
                .await?
        } else {
            action
        };
        Ok(AgenticPaymentRecord {
            id: action.id.clone(),
            authorization: decision_from_action(&action),
            action,
            normalized_requirement: normalized,
            reservation: Some(reservation),
            proof: None,
            receipt_id: None,
        })
    }
}

fn financial_matches(policy: &FinancialPolicy, action: &FinancialAction) -> bool {
    policy.meter == tl_core::SpendMeter::Actions
        && (policy.when.agents.is_empty() || policy.when.agents.contains(&action.principal_id))
        && (policy.when.action_kinds.is_empty() || policy.when.action_kinds.contains(&action.kind))
        && (policy.when.operations.is_empty() || policy.when.operations.contains(&action.operation))
        && (policy.when.currencies.is_empty()
            || policy.when.currencies.contains(&action.amount.currency))
        && (policy.when.rails.is_empty() || policy.when.rails.contains(&action.rail))
}

fn financial_scope(action: &FinancialAction) -> AuthorizationGrantScope {
    AuthorizationGrantScope::Financial(FinancialGrantScope {
        action_kinds: vec![action.kind],
        operation: Some(action.operation.clone()),
        rail: Some(action.rail),
        currency: Some(action.amount.currency.clone()),
        maximum_amount_minor: Some(action.amount.amount_minor),
        counterparties: action
            .counterparty
            .as_ref()
            .map(|value| vec![value.id.clone()])
            .unwrap_or_default(),
        x402_hosts: Vec::new(),
        x402_resources: Vec::new(),
        x402_networks: Vec::new(),
        x402_assets: Vec::new(),
        x402_payees: Vec::new(),
        required_preconditions: Vec::new(),
    })
}

fn enforcing_effect(effect: AuthorizationEffect) -> AuthorizationEffect {
    match effect {
        AuthorizationEffect::Deny
        | AuthorizationEffect::RequireApproval
        | AuthorizationEffect::Defer => effect,
        AuthorizationEffect::Permit | AuthorizationEffect::Transform => AuthorizationEffect::Deny,
    }
}

type FinancialWindowStarts = (DateTime<Utc>, DateTime<Utc>, DateTime<Utc>);

fn financial_window_starts(
    now: DateTime<Utc>,
) -> Result<FinancialWindowStarts, FinancialStoreError> {
    let day = Utc
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .single()
        .ok_or_else(|| FinancialStoreError::Internal("invalid daily window".into()))?;
    let week = day - Duration::days(i64::from(now.weekday().num_days_from_monday()));
    let month = Utc
        .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .ok_or_else(|| FinancialStoreError::Internal("invalid monthly window".into()))?;
    Ok((day, week, month))
}

fn authorization_error(error: AuthorizationError) -> FinancialStoreError {
    match error {
        AuthorizationError::Conflict(_) => FinancialStoreError::Conflict,
        AuthorizationError::Invalid(message) => FinancialStoreError::Validation(message),
        other => FinancialStoreError::Internal(other.to_string()),
    }
}

fn financial_policy_from_request(
    input: CreateFinancialPolicyRequest,
) -> Result<FinancialPolicy, FinancialStoreError> {
    Ok(FinancialPolicy {
        id: input.id,
        description: input.description,
        severity: input.severity.unwrap_or(Severity::Medium),
        when: FinancialWhen {
            agents: input.when.agents,
            action_kinds: input.when.action_kinds,
            operations: input.when.operations,
            currencies: input.when.currencies,
            rails: input.when.rails,
        },
        meter: input.meter,
        per_transaction_minor: input.per_transaction_minor,
        daily_minor: input.daily_minor,
        weekly_minor: input.weekly_minor,
        monthly_minor: input.monthly_minor,
        allowed_counterparty_ids: input.allowed_counterparty_ids,
        denied_counterparty_ids: input.denied_counterparty_ids,
        require_approval_for_new_counterparty: input.require_approval_for_new_counterparty,
        grant_required: input.grant_required,
        approval_threshold_minor: input.approval_threshold_minor,
        approver_roles: input.approver_roles,
        refund_original_method_only: input.refund_original_method_only,
        required_preconditions: input.required_preconditions,
        missing_evidence_effect: enforcing_effect(
            input
                .missing_evidence_effect
                .unwrap_or(AuthorizationEffect::Defer),
        ),
        failed_precondition_effect: enforcing_effect(
            input
                .failed_precondition_effect
                .unwrap_or(AuthorizationEffect::Deny),
        ),
        on_breach: enforcing_effect(input.on_breach.unwrap_or(AuthorizationEffect::Deny)),
    })
}

fn financial_policy_record(policy: &FinancialPolicy, enabled: bool) -> FinancialPolicyRecord {
    FinancialPolicyRecord {
        id: policy.id.to_string(),
        description: policy.description.clone(),
        severity: policy.severity,
        when: FinancialPolicySelector {
            agents: policy.when.agents.clone(),
            action_kinds: policy.when.action_kinds.clone(),
            operations: policy.when.operations.clone(),
            currencies: policy.when.currencies.clone(),
            rails: policy.when.rails.clone(),
        },
        meter: policy.meter,
        per_transaction_minor: policy.per_transaction_minor,
        daily_minor: policy.daily_minor,
        weekly_minor: policy.weekly_minor,
        monthly_minor: policy.monthly_minor,
        allowed_counterparty_ids: policy.allowed_counterparty_ids.clone(),
        denied_counterparty_ids: policy.denied_counterparty_ids.clone(),
        require_approval_for_new_counterparty: policy.require_approval_for_new_counterparty,
        grant_required: policy.grant_required,
        approval_threshold_minor: policy.approval_threshold_minor,
        approver_roles: policy.approver_roles.clone(),
        refund_original_method_only: policy.refund_original_method_only,
        required_preconditions: policy.required_preconditions.clone(),
        missing_evidence_effect: policy.missing_evidence_effect,
        failed_precondition_effect: policy.failed_precondition_effect,
        on_breach: policy.on_breach,
        enabled,
    }
}

fn evidence_bool(action: &FinancialActionRecord, key: &str) -> Option<bool> {
    action.evidence.iter().find_map(|evidence| {
        evidence
            .metadata
            .get(key)
            .and_then(serde_json::Value::as_bool)
    })
}

fn precondition_key(precondition: FinancialActionPrecondition) -> &'static str {
    match precondition {
        FinancialActionPrecondition::OrderExists => "order_exists",
        FinancialActionPrecondition::PaymentCaptured => "payment_captured",
        FinancialActionPrecondition::RefundWindowOpen => "refund_window_open",
        FinancialActionPrecondition::AmountLteRefundableBalance => "amount_lte_refundable_balance",
        FinancialActionPrecondition::DestinationIsOriginalPaymentMethod => {
            "destination_is_original_payment_method"
        }
        FinancialActionPrecondition::NoDuplicateRefund => "no_duplicate_refund",
        FinancialActionPrecondition::InvoiceMatchesPo => "invoice_matches_po",
        FinancialActionPrecondition::VendorApproved => "vendor_approved",
        FinancialActionPrecondition::GrantValid => "grant_valid",
        FinancialActionPrecondition::Custom => "custom",
    }
}

fn agentic_payment_principal(
    requested: &str,
    runtime_key: Option<&WorkspaceKeyContext>,
) -> Result<String, FinancialStoreError> {
    let requested = requested.trim();
    if requested.is_empty() {
        return Err(FinancialStoreError::Validation(
            "principal_id must not be empty".into(),
        ));
    }
    let Some(runtime_key) = runtime_key else {
        return Ok(requested.to_string());
    };
    let bound = runtime_key_principal(runtime_key);
    if requested != bound {
        return Err(FinancialStoreError::Validation(
            "principal_id must match the runtime API key principal".into(),
        ));
    }
    Ok(bound)
}

fn runtime_key_principal(runtime_key: &WorkspaceKeyContext) -> String {
    runtime_key
        .principal_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&runtime_key.api_key_id)
        .to_string()
}

fn ensure_agentic_payment_principal(
    action: &FinancialActionRecord,
    runtime_key: Option<&WorkspaceKeyContext>,
) -> Result<(), FinancialStoreError> {
    if runtime_key.is_some_and(|key| action.action.principal_id != runtime_key_principal(key)) {
        return Err(FinancialStoreError::Validation(
            "runtime API key principal cannot operate on this payment".into(),
        ));
    }
    Ok(())
}

fn agentic_payment_counterparty(normalized: &X402NormalizedPaymentRequirement) -> CounterpartyRef {
    CounterpartyRef {
        id: normalized
            .normalized_pay_to
            .clone()
            .unwrap_or_else(|| normalized.pay_to.clone()),
        display_name: Some(normalized.pay_to.clone()),
        kind: "x402_pay_to".into(),
        country: None,
        metadata: serde_json::json!({
            "network": normalized.network,
            "asset": normalized.asset,
            "host": normalized.host,
            "resource": normalized.resource,
        }),
    }
}

fn agentic_payment_metadata(
    input: &AgenticPaymentAuthorizeRequest,
    normalized: &X402NormalizedPaymentRequirement,
    session_limit_minor: i64,
    expires_at: DateTime<Utc>,
) -> serde_json::Value {
    serde_json::json!({
        "agentic_payment": {
            "session_id": input.session_id,
            "session_limit_minor": session_limit_minor,
            "reservation_expires_at": expires_at.to_rfc3339(),
            "normalized_requirement": normalized,
        },
        "customer_metadata": input.metadata,
    })
}

fn normalized_requirement_from_action(
    action: &FinancialActionRecord,
) -> Result<X402NormalizedPaymentRequirement, FinancialStoreError> {
    if action.action.rail != FinancialRail::X402 {
        return Err(FinancialStoreError::Validation(
            "financial action is not an x402 payment".into(),
        ));
    }
    let value = action
        .action
        .metadata
        .get("agentic_payment")
        .and_then(|value| value.get("normalized_requirement"))
        .cloned()
        .ok_or_else(|| FinancialStoreError::Internal("x402 metadata is incomplete".into()))?;
    serde_json::from_value(value).map_err(|error| FinancialStoreError::Internal(error.to_string()))
}

fn decision_from_action(action: &FinancialActionRecord) -> AuthorizationDecision {
    AuthorizationDecision {
        trace_id: action.id.clone(),
        intent_id: action.authorization_intent_id.clone(),
        domain: AuthorizationDomain::Financial,
        effect: action.authorization_effect,
        status: Some(action.authorization_status),
        reason: action
            .status_reason
            .clone()
            .unwrap_or_else(|| "financial authorization projection".into()),
        findings: Vec::new(),
        transformed_value: None,
        approval: None,
        applied_grant: None,
        lease: None,
        receipt_id: action.authorization_receipt_id.clone(),
        latency_ms: 0,
    }
}

fn parse_rfc3339(value: &str) -> Result<DateTime<Utc>, FinancialStoreError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| FinancialStoreError::Validation(error.to_string()))
}
