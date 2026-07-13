use std::sync::Arc;

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tl_core::{
    AgenticPaymentAuthorizationResponse, AgenticPaymentAuthorizeRequest,
    AgenticPaymentCommitRequest, AgenticPaymentDecision, AgenticPaymentMandateScope,
    AgenticPaymentRecord, AgenticPaymentReservationStatus, AgenticPaymentRollbackRequest,
    ApprovalRequirement, ApproveMatchingFinancialActionsRequest,
    ApproveMatchingFinancialActionsResponse, CounterpartyRef, CreateFinancialActionRequest,
    CreateFinancialMandateRequest, CreateFinancialPolicyRequest, EvidenceRef, FinancialAction,
    FinancialActionDecision, FinancialActionDecisionReceipt, FinancialActionKind,
    FinancialActionListResponse, FinancialActionOutcome, FinancialActionOutcomeStatus,
    FinancialActionPrecondition, FinancialActionRecord, FinancialActionStatus,
    FinancialApprovalEnvelope, FinancialApprovalRequestListResponse,
    FinancialApprovalRequestStatus, FinancialAuthorizationScopeProof, FinancialDecisionRisk,
    FinancialDecisionRiskCode, FinancialEligibilityStatus, FinancialEvidenceProof,
    FinancialExecutionProof, FinancialExecutionProofStatus, FinancialMandate,
    FinancialMandateListResponse, FinancialMandateStatus, FinancialOutcomeListResponse,
    FinancialPolicyListResponse, FinancialPolicyRecord, FinancialPolicySelector, FinancialRail,
    FinancialReceipt, PolicyAction, RecoveryStatus, ReversalCapability, Severity, Verdict,
    X402NormalizedPaymentRequirement, X402SettlementProof, DEFAULT_ENVIRONMENT_ID,
};
use tl_engine::{evaluate_financial_policies, financial_matches, financial_windowed_verdict};
use tl_policy::{validate_family_policy, Action, FamilyPolicy, FinancialPolicy, FinancialWhen};

use super::{
    validation::validate_create_action, x402, AgenticPaymentBudgetReservationRequest,
    FinancialBudgetConstraint, FinancialBudgetReservationOutcome,
    FinancialBudgetReservationRequest, FinancialBudgetViolation, FinancialBudgetWindow,
    FinancialExecutionError, FinancialExecutionResult, FinancialExecutor, FinancialLedgerEntryKind,
    FinancialStore, FinancialStoreError,
};
use crate::auth::WorkspaceKeyContext;
use crate::budget_alerts::BudgetAlertRuntime;
use crate::policies::PolicyStore;

#[derive(Clone)]
pub struct FinancialAuthorizationService {
    store: Arc<dyn FinancialStore>,
    policy_store: Option<Arc<dyn PolicyStore>>,
    executor: Option<Arc<dyn FinancialExecutor>>,
    /// Budget alert evaluation at spend-record time. `None` = alerts
    /// off (tests, minimal wiring); the spend path is unaffected.
    budget_alerts: Option<BudgetAlertRuntime>,
}

struct BudgetReservationDecision {
    decision: Option<(Verdict, String)>,
}

impl FinancialAuthorizationService {
    pub fn new(store: Arc<dyn FinancialStore>) -> Self {
        Self {
            store,
            policy_store: None,
            executor: None,
            budget_alerts: None,
        }
    }

    pub fn with_policy_store(
        store: Arc<dyn FinancialStore>,
        policy_store: Arc<dyn PolicyStore>,
    ) -> Self {
        Self {
            store,
            policy_store: Some(policy_store),
            executor: None,
            budget_alerts: None,
        }
    }

    pub fn with_policy_store_and_executor(
        store: Arc<dyn FinancialStore>,
        policy_store: Arc<dyn PolicyStore>,
        executor: Arc<dyn FinancialExecutor>,
    ) -> Self {
        Self {
            store,
            policy_store: Some(policy_store),
            executor: Some(executor),
            budget_alerts: None,
        }
    }

    /// Enable budget alert evaluation after each recorded spend.
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
        validate_create_action(&input)?;
        let input = self
            .attach_reusable_approval_mandate(workspace_id, input)
            .await?;
        let should_execute = input.execute;
        let action = self.store.create_action(workspace_id, input).await?;
        if action.status != FinancialActionStatus::Proposed {
            return Ok(action);
        }
        let action = self.enforce_mandate(workspace_id, action).await?;
        if action.status != FinancialActionStatus::Proposed {
            return Ok(action);
        }
        let action = self
            .apply_financial_policies(workspace_id, environment_id, action, should_execute)
            .await?;
        if should_execute && action.status == FinancialActionStatus::Proposed {
            let authorized = self
                .transition_action(
                    workspace_id,
                    &action.id,
                    FinancialActionStatus::Authorized,
                    "authorized",
                )
                .await?;
            return self.execute_action(workspace_id, &authorized.id).await;
        }
        Ok(action)
    }

    pub async fn get_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.store.get_action(workspace_id, action_id).await
    }

    pub async fn get_approval_envelope(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialApprovalEnvelope, FinancialStoreError> {
        let action = self.store.get_action(workspace_id, action_id).await?;
        if action.status != FinancialActionStatus::Held {
            return Err(FinancialStoreError::Conflict);
        }
        approval_envelope(&action)
    }

    pub async fn list_actions(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialActionListResponse, FinancialStoreError> {
        self.store.list_actions(workspace_id).await
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
        let reservation_expires_at = match input.reservation_expires_at.as_deref() {
            Some(value) => parse_rfc3339("reservation_expires_at", value)?,
            None => Utc::now() + Duration::minutes(15),
        };
        let session_limit_minor = input
            .session_limit_minor
            .unwrap_or(normalized.amount.amount_minor);
        let action_input = CreateFinancialActionRequest {
            idempotency_key: input.idempotency_key.clone(),
            execute: false,
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
                mandate: input.mandate.clone(),
                memo: Some("x402 agentic payment authorization".into()),
                metadata: agentic_payment_metadata(
                    &input,
                    &normalized,
                    principal_id.as_str(),
                    runtime_key.as_ref(),
                    session_limit_minor,
                    reservation_expires_at,
                ),
            },
            evidence: input.evidence.clone(),
        };

        validate_create_action(&action_input)?;
        let action_input = self
            .attach_reusable_approval_mandate(workspace_id, action_input)
            .await?;
        let mut action = self.store.create_action(workspace_id, action_input).await?;
        if action.status == FinancialActionStatus::Proposed {
            action = self.enforce_mandate(workspace_id, action).await?;
        }
        if action.status == FinancialActionStatus::Proposed {
            action = self
                .apply_financial_policies(workspace_id, environment_id, action, true)
                .await?;
        }

        let mut reservation = None;
        if matches!(
            action.status,
            FinancialActionStatus::Proposed | FinancialActionStatus::Held
        ) {
            let reserve_result = self
                .store
                .try_reserve_agentic_payment_budget(AgenticPaymentBudgetReservationRequest {
                    workspace_id: workspace_id.to_string(),
                    session_id: input.session_id.clone(),
                    principal_id: principal_id.clone(),
                    action_id: action.id.clone(),
                    payment_requirement_hash: normalized.payment_requirement_hash.clone(),
                    amount: normalized.amount.clone(),
                    session_limit_minor,
                    expires_at: reservation_expires_at,
                    metadata: serde_json::json!({
                        "source": "financial_authorization_service",
                        "protocol": "x402",
                        "action_id": action.id,
                        "idempotency_key": input.idempotency_key,
                        "normalized_requirement": normalized,
                    }),
                })
                .await;
            let reserved = match reserve_result {
                Ok(reserved) => reserved,
                Err(error) => {
                    if self
                        .action_budget_is_reserved(workspace_id, &action)
                        .await?
                    {
                        self.record_action_ledger_entry(
                            workspace_id,
                            &action,
                            FinancialLedgerEntryKind::Released,
                            "released",
                        )
                        .await?;
                    }
                    if action.status == FinancialActionStatus::Held {
                        self.store
                            .resolve_pending_approval_requests(
                                workspace_id,
                                &action.id,
                                FinancialApprovalRequestStatus::Denied,
                                None,
                            )
                            .await?;
                    }
                    self.transition_action(
                        workspace_id,
                        &action.id,
                        FinancialActionStatus::Failed,
                        "x402_reservation_failed",
                    )
                    .await?;
                    return Err(error);
                }
            };
            reservation = Some(reserved);
            if action.status == FinancialActionStatus::Proposed {
                action = self
                    .transition_action(
                        workspace_id,
                        &action.id,
                        FinancialActionStatus::Authorized,
                        "x402_reserved",
                    )
                    .await?;
            }
        }

        if reservation.is_none() {
            reservation = match self
                .store
                .get_agentic_payment_reservation(workspace_id, &action.id)
                .await
            {
                Ok(reservation) => Some(reservation),
                Err(FinancialStoreError::NotFound) => None,
                Err(error) => return Err(error),
            };
        }
        let decision = agentic_payment_decision(action.status);
        let reason =
            agentic_payment_authorization_reason(action.status, action.status_reason.as_deref());
        Ok(AgenticPaymentAuthorizationResponse {
            decision,
            signable: matches!(
                reservation.as_ref().map(|item| item.status),
                Some(AgenticPaymentReservationStatus::Reserved)
            ) && action.status == FinancialActionStatus::Authorized,
            reason,
            record: AgenticPaymentRecord {
                id: action.id.clone(),
                decision,
                action,
                normalized_requirement: normalized,
                reservation,
                proof: None,
                receipt_id: None,
            },
            decision_receipt_id: None,
        })
    }

    pub async fn get_agentic_payment(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<AgenticPaymentRecord, FinancialStoreError> {
        let action = self.store.get_action(workspace_id, action_id).await?;
        let normalized = normalized_requirement_from_action(&action)?;
        let reservation = match self
            .store
            .get_agentic_payment_reservation(workspace_id, action_id)
            .await
        {
            Ok(reservation) => Some(reservation),
            Err(FinancialStoreError::NotFound) => None,
            Err(error) => return Err(error),
        };
        let receipt_id = match self.store.get_receipt(workspace_id, action_id).await {
            Ok(receipt) => Some(receipt.id),
            Err(FinancialStoreError::NotFound) => None,
            Err(error) => return Err(error),
        };
        Ok(AgenticPaymentRecord {
            id: action.id.clone(),
            decision: agentic_payment_decision(action.status),
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
        action_id: &str,
        runtime_key: Option<WorkspaceKeyContext>,
        input: AgenticPaymentCommitRequest,
    ) -> Result<AgenticPaymentRecord, FinancialStoreError> {
        let current = self.store.get_action(workspace_id, action_id).await?;
        ensure_agentic_payment_principal(&current, runtime_key.as_ref())?;
        let normalized = normalized_requirement_from_action(&current)?;
        x402::verify_settlement_proof(&normalized, &input.proof)?;
        match current.status {
            FinancialActionStatus::Authorized => {}
            FinancialActionStatus::Held => {
                return Err(FinancialStoreError::Validation(
                    "agentic payment requires approval before commit".into(),
                ));
            }
            FinancialActionStatus::Executed => {
                let receipt_id = match self.store.get_receipt(workspace_id, action_id).await {
                    Ok(receipt) => Some(receipt.id),
                    Err(FinancialStoreError::NotFound) => None,
                    Err(error) => return Err(error),
                };
                let reservation = self
                    .store
                    .get_agentic_payment_reservation(workspace_id, action_id)
                    .await
                    .ok();
                return Ok(AgenticPaymentRecord {
                    id: current.id.clone(),
                    decision: AgenticPaymentDecision::Committed,
                    action: current,
                    normalized_requirement: normalized,
                    reservation,
                    proof: Some(input.proof),
                    receipt_id,
                });
            }
            _ => {
                return Err(FinancialStoreError::Validation(format!(
                    "agentic payment with status `{}` cannot be committed",
                    financial_status_label(current.status)
                )))
            }
        }

        let proof_value = serde_json::to_value(&input.proof)
            .map_err(|e| FinancialStoreError::Internal(format!("x402 proof encode: {e}")))?;
        let reservation = self
            .store
            .commit_agentic_payment_reservation(workspace_id, action_id, proof_value)
            .await?;
        let executed = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Executed,
                "x402_committed",
            )
            .await?;
        let mut ledger_event_ids = Vec::new();
        ledger_event_ids.push(
            self.record_action_ledger_entry(
                workspace_id,
                &executed,
                FinancialLedgerEntryKind::Executed,
                "executed",
            )
            .await?,
        );
        if self
            .action_budget_is_reserved(workspace_id, &current)
            .await?
        {
            ledger_event_ids.push(
                self.record_action_ledger_entry(
                    workspace_id,
                    &current,
                    FinancialLedgerEntryKind::Released,
                    "released",
                )
                .await?,
            );
        }
        self.notify_budget_alerts(workspace_id, &executed).await;
        let provider_result = Some(agentic_payment_execution_result(&input.proof));
        let receipt = self
            .create_execution_receipt(
                workspace_id,
                DEFAULT_ENVIRONMENT_ID,
                &executed,
                &ledger_event_ids,
                &provider_result,
            )
            .await?;
        self.record_provider_success(
            workspace_id,
            &executed,
            agentic_payment_execution_result(&input.proof),
        )
        .await?;
        Ok(AgenticPaymentRecord {
            id: executed.id.clone(),
            decision: AgenticPaymentDecision::Committed,
            action: executed,
            normalized_requirement: normalized,
            reservation: Some(reservation),
            proof: Some(input.proof),
            receipt_id: Some(receipt.id),
        })
    }

    pub async fn rollback_agentic_payment(
        &self,
        workspace_id: &str,
        action_id: &str,
        runtime_key: Option<WorkspaceKeyContext>,
        input: AgenticPaymentRollbackRequest,
    ) -> Result<AgenticPaymentRecord, FinancialStoreError> {
        let current = self.store.get_action(workspace_id, action_id).await?;
        ensure_agentic_payment_principal(&current, runtime_key.as_ref())?;
        let normalized = normalized_requirement_from_action(&current)?;
        if current.status == FinancialActionStatus::Executed {
            return Err(FinancialStoreError::Validation(
                "agentic payment rollback only releases an unsettled reservation; settled x402 payments cannot be reverted here".into(),
            ));
        }
        let reservation = self
            .store
            .release_agentic_payment_reservation(
                workspace_id,
                action_id,
                &input.reason,
                serde_json::json!({
                    "provider_error": input.provider_error,
                    "idempotency_key": input.idempotency_key,
                    "metadata": input.metadata,
                    "source": "financial_authorization_service",
                }),
            )
            .await?;
        if self
            .action_budget_is_reserved(workspace_id, &current)
            .await?
        {
            self.record_action_ledger_entry(
                workspace_id,
                &current,
                FinancialLedgerEntryKind::Released,
                "released",
            )
            .await?;
        }
        let failed = match current.status {
            FinancialActionStatus::Proposed
            | FinancialActionStatus::Authorized
            | FinancialActionStatus::Held => {
                self.transition_action(
                    workspace_id,
                    action_id,
                    FinancialActionStatus::Failed,
                    "x402_rolled_back",
                )
                .await?
            }
            FinancialActionStatus::Denied
            | FinancialActionStatus::Failed
            | FinancialActionStatus::Reversed
            | FinancialActionStatus::Expired => current,
            FinancialActionStatus::Executed => unreachable!(),
        };
        self.record_provider_failure(workspace_id, &failed, input.reason)
            .await?;
        Ok(AgenticPaymentRecord {
            id: failed.id.clone(),
            decision: AgenticPaymentDecision::RolledBack,
            action: failed,
            normalized_requirement: normalized,
            reservation: Some(reservation),
            proof: None,
            receipt_id: None,
        })
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
            let message = issues
                .into_iter()
                .map(|issue| format!("{}: {}", issue.path, issue.message))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(FinancialStoreError::Validation(message));
        }
        let source_yaml = serde_yaml::to_string(&family)
            .map_err(|e| FinancialStoreError::Internal(format!("financial policy yaml: {e}")))?;
        self.policy_store()?
            .upsert_family(workspace_id, environment_id, &family, &source_yaml)
            .await
            .map_err(|e| FinancialStoreError::Internal(format!("financial policy upsert: {e}")))?;
        Ok(financial_policy_record(&policy, true))
    }

    pub async fn list_financial_policies(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<FinancialPolicyListResponse, FinancialStoreError> {
        let families = self
            .policy_store()?
            .list_enabled_families(workspace_id, environment_id)
            .await
            .map_err(|e| FinancialStoreError::Internal(format!("financial policy list: {e}")))?;
        let policies = families
            .iter()
            .filter_map(|family| match family.as_ref() {
                FamilyPolicy::Financial(policy) => Some(financial_policy_record(policy, true)),
                _ => None,
            })
            .collect();
        Ok(FinancialPolicyListResponse { policies })
    }

    pub async fn create_mandate(
        &self,
        workspace_id: &str,
        input: CreateFinancialMandateRequest,
    ) -> Result<FinancialMandate, FinancialStoreError> {
        let input = normalize_create_mandate_request(input)?;
        self.store.create_mandate(workspace_id, input).await
    }

    pub async fn list_mandates(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialMandateListResponse, FinancialStoreError> {
        self.store.list_mandates(workspace_id).await
    }

    pub async fn get_mandate(
        &self,
        workspace_id: &str,
        mandate_id: &str,
        version: Option<i32>,
    ) -> Result<FinancialMandate, FinancialStoreError> {
        self.store
            .get_mandate(workspace_id, mandate_id, version)
            .await
    }

    pub async fn revoke_mandate(
        &self,
        workspace_id: &str,
        mandate_id: &str,
    ) -> Result<FinancialMandate, FinancialStoreError> {
        self.store.revoke_mandate(workspace_id, mandate_id).await
    }

    pub async fn get_receipt(
        &self,
        workspace_id: &str,
        receipt_id: &str,
    ) -> Result<FinancialReceipt, FinancialStoreError> {
        self.store.get_receipt(workspace_id, receipt_id).await
    }

    pub async fn get_decision_receipt(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionDecisionReceipt, FinancialStoreError> {
        let action = self.store.get_action(workspace_id, action_id).await?;
        let families = self
            .matching_financial_families(workspace_id, environment_id, &action.action)
            .await?;
        let pure = evaluate_financial_policies(&action.action, families.iter().map(Arc::as_ref));
        let windowed = self
            .evaluate_ledger_windows(workspace_id, &action, &families)
            .await?;
        let (evidence, mut risks) = financial_evidence_proofs(&action, &families);
        risks.extend(pure.triggered.iter().map(|triggered| {
            risk_from_reason(
                &triggered.reason,
                triggered.severity,
                Some(triggered.id.clone()),
                "financial_policy",
            )
        }));
        if let Some((_, reason)) = &windowed {
            risks.push(risk_from_reason(
                reason,
                Severity::High,
                policy_id_from_reason(reason),
                "financial_ledger",
            ));
        }

        let authorization_scope = self
            .authorization_scope_proof(workspace_id, &action, &families)
            .await?;
        if authorization_scope.result == FinancialEligibilityStatus::Missing {
            risks.push(FinancialDecisionRisk {
                code: FinancialDecisionRiskCode::MissingAuthorizationScope,
                severity: Severity::High,
                reason: authorization_scope
                    .reason
                    .clone()
                    .unwrap_or_else(|| "authorization scope required before execution".into()),
                policy_id: None,
                source: "authorization_scope".into(),
            });
        } else if authorization_scope.result == FinancialEligibilityStatus::Failed {
            risks.push(FinancialDecisionRisk {
                code: FinancialDecisionRiskCode::AuthorizationScopeInvalid,
                severity: Severity::High,
                reason: authorization_scope
                    .reason
                    .clone()
                    .unwrap_or_else(|| "authorization scope invalid".into()),
                policy_id: None,
                source: "authorization_scope".into(),
            });
        }

        let approval = self
            .store
            .list_approval_requests(workspace_id)
            .await?
            .approval_requests
            .into_iter()
            .find(|request| {
                request.action_id == action.id
                    && request.status == FinancialApprovalRequestStatus::Pending
            })
            .map(|request| ApprovalRequirement {
                required: true,
                approver_roles: request.approver_roles,
                reason: request.reason,
                expires_at: request.expires_at,
            });
        let execution = self.execution_proof(workspace_id, &action).await?;
        let policy_decision = compose_policy_decisions(
            pure.verdict.map(|verdict| {
                (
                    verdict,
                    pure.reason
                        .clone()
                        .unwrap_or_else(|| "financial policy matched".to_string()),
                )
            }),
            windowed,
        );
        let decision = action_decision(action.status, policy_decision.as_ref());
        let reason = decision_receipt_reason(&action, decision, &risks, approval.as_ref());

        Ok(FinancialActionDecisionReceipt {
            schema: "financial_action_decision_receipt.v1".into(),
            action_id: action.id.clone(),
            decision,
            status: action.status,
            reason,
            amount: action.action.amount.clone(),
            operation: action.action.operation.clone(),
            principal_id: action.action.principal_id.clone(),
            counterparty: action.action.counterparty.clone(),
            authorization_scope,
            evidence,
            risks,
            approval,
            execution,
            created_at: action.created_at,
            updated_at: action.updated_at,
        })
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

    pub async fn list_approval_requests(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialApprovalRequestListResponse, FinancialStoreError> {
        self.store.list_approval_requests(workspace_id).await
    }

    pub async fn hold_action(
        &self,
        workspace_id: &str,
        action_id: &str,
        approval: ApprovalRequirement,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let action = self.store.get_action(workspace_id, action_id).await?;
        let families = self
            .enabled_financial_families(workspace_id, DEFAULT_ENVIRONMENT_ID)
            .await?;
        let reservation = self
            .reserve_action_budget(workspace_id, &action, &families)
            .await?;
        if let Some((Verdict::Block | Verdict::Rewrite, reason)) = reservation.decision {
            return self
                .store
                .transition_action_with_reason(
                    workspace_id,
                    action_id,
                    FinancialActionStatus::Denied,
                    "policy_denied",
                    &reason,
                )
                .await;
        }
        let held = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Held,
                "approval_required",
            )
            .await?;
        self.store
            .create_approval_request(workspace_id, action_id, approval)
            .await?;
        Ok(held)
    }

    pub async fn approve_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.approve_action_as(workspace_id, action_id, None).await
    }

    pub async fn approve_action_as(
        &self,
        workspace_id: &str,
        action_id: &str,
        actor_id: Option<&str>,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let current = self.store.get_action(workspace_id, action_id).await?;
        let current = self.enforce_mandate(workspace_id, current).await?;
        if current.status == FinancialActionStatus::Denied {
            return Ok(current);
        }
        if !self
            .action_budget_is_reserved(workspace_id, &current)
            .await?
        {
            let families = self
                .enabled_financial_families(workspace_id, DEFAULT_ENVIRONMENT_ID)
                .await?;
            let decision = self
                .current_policy_decision(workspace_id, &current, &families)
                .await?;
            if let Some((Verdict::Block | Verdict::Rewrite, reason)) = decision {
                return self
                    .store
                    .transition_action_with_reason(
                        workspace_id,
                        action_id,
                        FinancialActionStatus::Denied,
                        "policy_denied",
                        &reason,
                    )
                    .await;
            }
            let reservation = self
                .reserve_action_budget(workspace_id, &current, &families)
                .await?;
            if let Some((Verdict::Block | Verdict::Rewrite, reason)) = reservation.decision {
                return self
                    .store
                    .transition_action_with_reason(
                        workspace_id,
                        action_id,
                        FinancialActionStatus::Denied,
                        "policy_denied",
                        &reason,
                    )
                    .await;
            }
        }
        let approved = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Authorized,
                "approved",
            )
            .await?;
        self.store
            .resolve_pending_approval_requests(
                workspace_id,
                action_id,
                FinancialApprovalRequestStatus::Approved,
                actor_id,
            )
            .await?;
        Ok(approved)
    }

    pub async fn approve_matching_actions_as(
        &self,
        workspace_id: &str,
        action_id: &str,
        actor_id: Option<&str>,
        input: ApproveMatchingFinancialActionsRequest,
    ) -> Result<ApproveMatchingFinancialActionsResponse, FinancialStoreError> {
        let action = self.store.get_action(workspace_id, action_id).await?;
        if action.status != FinancialActionStatus::Held {
            return Err(FinancialStoreError::Conflict);
        }
        let envelope = approval_envelope(&action)?;
        if input.action_fingerprint != envelope.action_fingerprint {
            return Err(FinancialStoreError::Conflict);
        }
        if input.max_amount_minor < action.action.amount.amount_minor || input.max_amount_minor <= 0
        {
            return Err(FinancialStoreError::Validation(
                "max_amount_minor must cover the held action and be greater than zero".into(),
            ));
        }
        let now = Utc::now();
        let expires_at = parse_rfc3339("expires_at", &input.expires_at)?;
        if expires_at <= now {
            return Err(FinancialStoreError::Validation(
                "expires_at must be in the future".into(),
            ));
        }
        if expires_at > now + Duration::days(30) {
            return Err(FinancialStoreError::Validation(
                "expires_at cannot be more than 30 days in the future".into(),
            ));
        }

        let mandate_id = reusable_mandate_id(&envelope.action_fingerprint);
        let previous = match self
            .store
            .get_mandate(workspace_id, &mandate_id, None)
            .await
        {
            Ok(mandate) => Some(mandate),
            Err(FinancialStoreError::NotFound) => None,
            Err(error) => return Err(error),
        };
        if previous.as_ref().is_some_and(|mandate| {
            reusable_mandate_fingerprint(mandate) != Some(envelope.action_fingerprint.as_str())
        }) {
            return Err(FinancialStoreError::Conflict);
        }
        let version = previous
            .as_ref()
            .map(|mandate| mandate.version)
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| FinancialStoreError::Internal("mandate version overflow".into()))?;
        let payment_scope = reusable_payment_scope(&action, input.max_amount_minor)?;
        let approved = self
            .approve_action_as(workspace_id, action_id, actor_id)
            .await?;
        if approved.status != FinancialActionStatus::Authorized {
            return Err(FinancialStoreError::Conflict);
        }
        let mandate = self
            .create_mandate(
                workspace_id,
                CreateFinancialMandateRequest {
                    id: Some(mandate_id.clone()),
                    version: Some(version),
                    principal_id: action.action.principal_id.clone(),
                    scope: serde_json::Value::Null,
                    payment_scope: Some(payment_scope),
                    metadata: serde_json::json!({
                        "mode": "approval_reuse",
                        "action_fingerprint": envelope.action_fingerprint,
                        "fingerprint_version": envelope.fingerprint_version,
                        "source_action_id": action.id,
                        "approved_by": actor_id,
                    }),
                    starts_at: None,
                    expires_at: Some(expires_at.to_rfc3339()),
                },
            )
            .await?;
        Ok(ApproveMatchingFinancialActionsResponse {
            action: approved,
            mandate,
            approval_envelope: envelope,
        })
    }

    async fn attach_reusable_approval_mandate(
        &self,
        workspace_id: &str,
        mut input: CreateFinancialActionRequest,
    ) -> Result<CreateFinancialActionRequest, FinancialStoreError> {
        if input.action.mandate.is_some() {
            return Ok(input);
        }
        let fingerprint = action_fingerprint(&input.action)?;
        let mandate_id = reusable_mandate_id(&fingerprint);
        let latest = match self
            .store
            .get_mandate(workspace_id, &mandate_id, None)
            .await
        {
            Ok(mandate) => mandate,
            Err(FinancialStoreError::NotFound) => return Ok(input),
            Err(error) => return Err(error),
        };
        if reusable_mandate_fingerprint(&latest) != Some(fingerprint.as_str()) {
            return Err(FinancialStoreError::Conflict);
        }
        let candidate = FinancialActionRecord {
            id: input.action.id.clone().unwrap_or_default(),
            workspace_id: workspace_id.to_string(),
            status: FinancialActionStatus::Proposed,
            status_reason: None,
            action: input.action.clone(),
            evidence: input.evidence.clone(),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };
        if mandate_denial_reason(&latest, &candidate)?.is_none() {
            input.action.mandate = Some(tl_core::MandateRef {
                id: latest.id.clone(),
                version: Some(latest.version),
            });
        }
        Ok(input)
    }

    pub async fn authorize_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let current = self.store.get_action(workspace_id, action_id).await?;
        let current = self.enforce_mandate(workspace_id, current).await?;
        if current.status == FinancialActionStatus::Denied {
            return Ok(current);
        }
        let prepared = self
            .apply_financial_policies(workspace_id, DEFAULT_ENVIRONMENT_ID, current, true)
            .await?;
        if prepared.status != FinancialActionStatus::Proposed {
            return Ok(prepared);
        }
        self.transition_action(
            workspace_id,
            action_id,
            FinancialActionStatus::Authorized,
            "authorized",
        )
        .await
    }

    pub async fn execute_held_action_retryable(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionExecutionAttempt, FinancialStoreError> {
        let current = self.store.get_action(workspace_id, action_id).await?;
        if current.status == FinancialActionStatus::Executed {
            return Ok(FinancialActionExecutionAttempt::Executed(current));
        }
        if current.status != FinancialActionStatus::Held {
            let executed = self.execute_action(workspace_id, action_id).await?;
            return Ok(FinancialActionExecutionAttempt::Executed(executed));
        }

        let provider_result = match self
            .execute_provider_if_required(workspace_id, &current)
            .await
        {
            Ok(result) => result,
            Err(reason) => {
                self.record_provider_failure(workspace_id, &current, reason.clone())
                    .await?;
                return Ok(FinancialActionExecutionAttempt::Failed {
                    action: current,
                    reason,
                });
            }
        };
        let executed = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Executed,
                "executed",
            )
            .await?;
        let mut ledger_event_ids = Vec::new();
        ledger_event_ids.push(
            self.record_action_ledger_entry(
                workspace_id,
                &executed,
                FinancialLedgerEntryKind::Executed,
                "executed",
            )
            .await?,
        );
        if self
            .action_budget_is_reserved(workspace_id, &current)
            .await?
        {
            ledger_event_ids.push(
                self.record_action_ledger_entry(
                    workspace_id,
                    &current,
                    FinancialLedgerEntryKind::Released,
                    "released",
                )
                .await?,
            );
        }
        self.store
            .resolve_pending_approval_requests(
                workspace_id,
                action_id,
                FinancialApprovalRequestStatus::Approved,
                None,
            )
            .await?;
        self.create_execution_receipt(
            workspace_id,
            DEFAULT_ENVIRONMENT_ID,
            &executed,
            &ledger_event_ids,
            &provider_result,
        )
        .await?;
        if let Some(provider_result) = provider_result {
            self.record_provider_success(workspace_id, &executed, provider_result)
                .await?;
        }
        Ok(FinancialActionExecutionAttempt::Executed(executed))
    }

    pub async fn deny_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.deny_action_as(workspace_id, action_id, None).await
    }

    pub async fn deny_action_as(
        &self,
        workspace_id: &str,
        action_id: &str,
        actor_id: Option<&str>,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let current = self.store.get_action(workspace_id, action_id).await?;
        let denied = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Denied,
                "denied",
            )
            .await?;
        self.store
            .resolve_pending_approval_requests(
                workspace_id,
                action_id,
                FinancialApprovalRequestStatus::Denied,
                actor_id,
            )
            .await?;
        if current.status == FinancialActionStatus::Held {
            self.record_action_ledger_entry(
                workspace_id,
                &current,
                FinancialLedgerEntryKind::Released,
                "released",
            )
            .await?;
        }
        Ok(denied)
    }

    pub async fn execute_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let current = self.store.get_action(workspace_id, action_id).await?;
        let current = self.enforce_mandate(workspace_id, current).await?;
        if current.status == FinancialActionStatus::Denied {
            return Ok(current);
        }
        match current.status {
            FinancialActionStatus::Authorized => {}
            FinancialActionStatus::Held => {
                return Err(FinancialStoreError::Validation(
                    "financial action requires approval before execution".into(),
                ));
            }
            FinancialActionStatus::Proposed => {
                return Err(FinancialStoreError::Validation(
                    "financial action requires authorization before execution".into(),
                ));
            }
            FinancialActionStatus::Executed
            | FinancialActionStatus::Denied
            | FinancialActionStatus::Failed
            | FinancialActionStatus::Reversed
            | FinancialActionStatus::Expired => {
                return Err(FinancialStoreError::Validation(format!(
                    "financial action with status `{}` cannot be executed",
                    financial_status_label(current.status)
                )));
            }
        }
        if !self
            .action_budget_is_reserved(workspace_id, &current)
            .await?
        {
            return Err(FinancialStoreError::Validation(
                "financial action requires a current budget reservation before execution".into(),
            ));
        }
        let families = self
            .enabled_financial_families(workspace_id, DEFAULT_ENVIRONMENT_ID)
            .await?;
        if let Some((verdict, reason)) = self
            .current_commit_policy_decision(workspace_id, &current, &families)
            .await?
        {
            let approval_satisfies_escalation = verdict == Verdict::Escalate
                && self
                    .action_has_approved_review(workspace_id, &current)
                    .await?;
            if matches!(verdict, Verdict::Block | Verdict::Rewrite)
                || (verdict == Verdict::Escalate && !approval_satisfies_escalation)
            {
                return self
                    .deny_reserved_action(workspace_id, &current, &reason)
                    .await;
            }
        }
        let provider_result = match self
            .execute_provider_if_required(workspace_id, &current)
            .await
        {
            Ok(result) => result,
            Err(reason) => {
                if self
                    .action_budget_is_reserved(workspace_id, &current)
                    .await?
                {
                    self.record_action_ledger_entry(
                        workspace_id,
                        &current,
                        FinancialLedgerEntryKind::Released,
                        "released",
                    )
                    .await?;
                }
                let failed = self
                    .transition_action(
                        workspace_id,
                        action_id,
                        FinancialActionStatus::Failed,
                        "provider_failed",
                    )
                    .await?;
                self.record_provider_failure(workspace_id, &failed, reason)
                    .await?;
                return Ok(failed);
            }
        };
        let executed = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Executed,
                "executed",
            )
            .await?;
        let mut ledger_event_ids = Vec::new();
        ledger_event_ids.push(
            self.record_action_ledger_entry(
                workspace_id,
                &executed,
                FinancialLedgerEntryKind::Executed,
                "executed",
            )
            .await?,
        );
        if self
            .action_budget_is_reserved(workspace_id, &current)
            .await?
        {
            ledger_event_ids.push(
                self.record_action_ledger_entry(
                    workspace_id,
                    &current,
                    FinancialLedgerEntryKind::Released,
                    "released",
                )
                .await?,
            );
        }
        // Budget alert thresholds are checked right after the spend
        // lands in the ledger. Never fails or delays the spend beyond
        // one indexed config lookup — errors are logged and swallowed.
        self.notify_budget_alerts(workspace_id, &executed).await;
        if current.status == FinancialActionStatus::Held {
            self.store
                .resolve_pending_approval_requests(
                    workspace_id,
                    action_id,
                    FinancialApprovalRequestStatus::Approved,
                    None,
                )
                .await?;
        }
        self.create_execution_receipt(
            workspace_id,
            DEFAULT_ENVIRONMENT_ID,
            &executed,
            &ledger_event_ids,
            &provider_result,
        )
        .await?;
        if let Some(provider_result) = provider_result {
            self.record_provider_success(workspace_id, &executed, provider_result)
                .await?;
        }
        Ok(executed)
    }

    async fn transition_action(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: FinancialActionStatus,
        event_type: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.store
            .transition_action(workspace_id, action_id, status, event_type)
            .await
    }

    /// Spend-time budget alert hook. Evaluates enabled alert configs
    /// against the acting principal's ledger window sums and the caps
    /// from the matching financial policies (execute runs in the
    /// default environment, like receipts). Infallible by design:
    /// alerting must never fail a spend.
    async fn notify_budget_alerts(&self, workspace_id: &str, action: &FinancialActionRecord) {
        let Some(runtime) = &self.budget_alerts else {
            return;
        };
        let Some(policy_store) = &self.policy_store else {
            return;
        };
        let principal_id = &action.action.principal_id;
        let currency = &action.action.amount.currency;
        crate::budget_alerts::evaluate_spend_alerts(
            crate::budget_alerts::SpendAlertEvaluation {
                runtime,
                policy_store: policy_store.as_ref(),
                workspace_id,
                environment_id: DEFAULT_ENVIRONMENT_ID,
                principal_id,
                currency,
                meter: tl_core::SpendMeter::Actions,
            },
            |financial| financial_matches(financial, &action.action),
            |window_start, now| async move {
                self.store
                    .net_spend_minor(workspace_id, principal_id, currency, window_start, now)
                    .await
                    .map_err(|error| error.to_string())
            },
        )
        .await;
    }

    async fn record_action_ledger_entry(
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
                &ledger_idempotency_key(&action.id, suffix),
                serde_json::json!({
                    "action_id": action.id,
                    "financial_status": action.status,
                    "source": "financial_authorization_service"
                }),
            )
            .await
    }

    async fn ledger_entry_exists(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        suffix: &str,
    ) -> Result<bool, FinancialStoreError> {
        self.store
            .ledger_entry_exists(workspace_id, &ledger_idempotency_key(&action.id, suffix))
            .await
    }

    async fn action_budget_is_reserved(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
    ) -> Result<bool, FinancialStoreError> {
        if !self
            .ledger_entry_exists(workspace_id, action, "reserved")
            .await?
        {
            return Ok(false);
        }
        Ok(!self
            .ledger_entry_exists(workspace_id, action, "released")
            .await?)
    }

    async fn create_execution_receipt(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action: &FinancialActionRecord,
        ledger_event_ids: &[String],
        provider_result: &Option<FinancialExecutionResult>,
    ) -> Result<FinancialReceipt, FinancialStoreError> {
        let proof = self
            .execution_receipt_proof(
                workspace_id,
                environment_id,
                action,
                ledger_event_ids,
                provider_result,
            )
            .await?;
        self.store
            .create_receipt(
                workspace_id,
                &action.id,
                None,
                ledger_event_ids.to_vec(),
                proof,
            )
            .await
    }

    async fn execution_receipt_proof(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action: &FinancialActionRecord,
        ledger_event_ids: &[String],
        provider_result: &Option<FinancialExecutionResult>,
    ) -> Result<serde_json::Value, FinancialStoreError> {
        let mandate_snapshot = match &action.action.mandate {
            Some(reference) => Some(
                self.store
                    .get_mandate(workspace_id, &reference.id, reference.version)
                    .await?,
            ),
            None => None,
        };
        let approval_requests = self
            .store
            .list_approval_requests(workspace_id)
            .await?
            .approval_requests
            .into_iter()
            .filter(|request| request.action_id == action.id)
            .collect::<Vec<_>>();
        let policy_snapshots = self
            .matching_policy_snapshots(workspace_id, environment_id, &action.action)
            .await?;

        Ok(serde_json::json!({
            "schema": "financial_execution_receipt.v1",
            "action_id": action.id,
            "action_status": "executed",
            "action_snapshot": action.action,
            "amount": action.action.amount,
            "counterparty": action.action.counterparty,
            "mandate_ref": action.action.mandate,
            "mandate_snapshot": mandate_snapshot,
            "approval_requests": approval_requests,
            "evidence_refs": action.evidence,
            "policy_snapshots": policy_snapshots,
            "ledger_source": "financial_ledger_entries",
            "ledger_event_ids": ledger_event_ids,
            "provider": provider_proof(provider_result),
            "receipt_source": "financial_authorization_service"
        }))
    }

    async fn matching_policy_snapshots(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action: &FinancialAction,
    ) -> Result<Vec<serde_json::Value>, FinancialStoreError> {
        let Some(policy_store) = &self.policy_store else {
            return Ok(vec![]);
        };
        let families = policy_store
            .list_enabled_families(workspace_id, environment_id)
            .await
            .map_err(|e| FinancialStoreError::Internal(format!("financial policies: {e}")))?;
        Ok(families
            .iter()
            .filter(|family| receipt_policy_matches(family.as_ref(), action))
            .map(|family| {
                serde_json::json!({
                    "id": family.id(),
                    "policy": family.as_ref()
                })
            })
            .collect())
    }

    async fn matching_financial_families(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action: &FinancialAction,
    ) -> Result<Vec<Arc<FamilyPolicy>>, FinancialStoreError> {
        let Some(policy_store) = &self.policy_store else {
            return Ok(vec![]);
        };
        let families = policy_store
            .list_enabled_families(workspace_id, environment_id)
            .await
            .map_err(|e| FinancialStoreError::Internal(format!("financial policies: {e}")))?;
        Ok(families
            .into_iter()
            .filter(|family| receipt_policy_matches(family.as_ref(), action))
            .collect())
    }

    async fn authorization_scope_proof(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        families: &[Arc<FamilyPolicy>],
    ) -> Result<FinancialAuthorizationScopeProof, FinancialStoreError> {
        let scope_required = families.iter().any(|family| match family.as_ref() {
            FamilyPolicy::Financial(financial) => financial.mandate_required,
            _ => false,
        });
        let Some(scope_ref) = &action.action.mandate else {
            return Ok(FinancialAuthorizationScopeProof {
                checked: false,
                result: if scope_required {
                    FinancialEligibilityStatus::Missing
                } else {
                    FinancialEligibilityStatus::Passed
                },
                scope_ref: None,
                scope_snapshot: None,
                source: Some("financial_authorization_service".into()),
                mandate_hash: None,
                normalized_scope: None,
                reason: Some(if scope_required {
                    "authorization scope required before execution".into()
                } else {
                    "no authorization scope required by matching policy".into()
                }),
            });
        };
        let snapshot = match self
            .store
            .get_mandate(workspace_id, &scope_ref.id, scope_ref.version)
            .await
        {
            Ok(snapshot) => snapshot,
            Err(FinancialStoreError::NotFound) => {
                return Ok(FinancialAuthorizationScopeProof {
                    checked: true,
                    result: FinancialEligibilityStatus::Missing,
                    scope_ref: Some(scope_ref.clone()),
                    scope_snapshot: None,
                    source: Some("financial_authorization_service".into()),
                    mandate_hash: None,
                    normalized_scope: None,
                    reason: Some("authorization scope not found".into()),
                });
            }
            Err(error) => return Err(error),
        };
        let denial = mandate_denial_reason(&snapshot, action)?;
        let reason = denial
            .clone()
            .or_else(|| authorization_scope_summary(&snapshot));
        Ok(FinancialAuthorizationScopeProof {
            checked: true,
            result: if denial.is_some() {
                FinancialEligibilityStatus::Failed
            } else {
                FinancialEligibilityStatus::Passed
            },
            scope_ref: Some(scope_ref.clone()),
            mandate_hash: Some(mandate_hash(&snapshot.scope)?),
            normalized_scope: Some(snapshot.scope.clone()),
            scope_snapshot: Some(snapshot),
            source: Some("financial_authorization_service".into()),
            reason,
        })
    }

    async fn execution_proof(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
    ) -> Result<FinancialExecutionProof, FinancialStoreError> {
        match self.store.get_receipt(workspace_id, &action.id).await {
            Ok(receipt) => Ok(FinancialExecutionProof {
                status: FinancialExecutionProofStatus::Executed,
                receipt_id: Some(receipt.id),
                ledger_event_ids: receipt.ledger_event_ids,
            }),
            Err(FinancialStoreError::NotFound) => Ok(FinancialExecutionProof {
                status: match action.status {
                    FinancialActionStatus::Executed => {
                        FinancialExecutionProofStatus::ReceiptMissing
                    }
                    FinancialActionStatus::Failed => FinancialExecutionProofStatus::Failed,
                    _ => FinancialExecutionProofStatus::NotStarted,
                },
                receipt_id: None,
                ledger_event_ids: vec![],
            }),
            Err(error) => Err(error),
        }
    }

    fn policy_store(&self) -> Result<&Arc<dyn PolicyStore>, FinancialStoreError> {
        self.policy_store.as_ref().ok_or_else(|| {
            FinancialStoreError::Internal("financial policy store unavailable".into())
        })
    }

    async fn execute_provider_if_required(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
    ) -> Result<Option<FinancialExecutionResult>, String> {
        if action.action.rail != FinancialRail::PaymentHttp {
            return Ok(None);
        }
        let Some(executor) = &self.executor else {
            return Ok(None);
        };
        executor
            .execute(workspace_id, action, &action.id)
            .await
            .map(Some)
            .map_err(|error| match error {
                FinancialExecutionError::NoProvider => error.to_string(),
                FinancialExecutionError::Failed(reason) => reason,
            })
    }

    async fn record_provider_success(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        result: FinancialExecutionResult,
    ) -> Result<(), FinancialStoreError> {
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
                    metadata: serde_json::json!({
                        "provider_response": result.provider_response,
                        "source": "financial_authorization_service"
                    }),
                },
            )
            .await?;
        Ok(())
    }

    async fn record_provider_failure(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        reason: String,
    ) -> Result<(), FinancialStoreError> {
        self.store
            .record_action_outcome(
                workspace_id,
                &action.id,
                FinancialActionOutcome {
                    action_id: action.id.clone(),
                    status: FinancialActionOutcomeStatus::Failed,
                    reversal_capability: ReversalCapability::None,
                    recovery_status: RecoveryStatus::NotAvailable,
                    provider_status: Some("failed".into()),
                    provider_reference: None,
                    final_loss_amount: None,
                    occurred_at: Utc::now().to_rfc3339(),
                    metadata: serde_json::json!({
                        "reason": reason,
                        "source": "financial_authorization_service"
                    }),
                },
            )
            .await?;
        Ok(())
    }

    async fn enforce_mandate(
        &self,
        workspace_id: &str,
        action: FinancialActionRecord,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let Some(reference) = &action.action.mandate else {
            return Ok(action);
        };
        let mandate = match self
            .store
            .get_mandate(workspace_id, &reference.id, reference.version)
            .await
        {
            Ok(mandate) => mandate,
            Err(FinancialStoreError::NotFound) => {
                return self
                    .deny_for_mandate(workspace_id, &action, "mandate_not_found")
                    .await;
            }
            Err(error) => return Err(error),
        };

        if let Some(reason) = mandate_denial_reason(&mandate, &action)? {
            return self.deny_for_mandate(workspace_id, &action, &reason).await;
        }
        Ok(action)
    }

    async fn deny_for_mandate(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        reason: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        if self.action_budget_is_reserved(workspace_id, action).await? {
            self.record_action_ledger_entry(
                workspace_id,
                action,
                FinancialLedgerEntryKind::Released,
                "released",
            )
            .await?;
        }
        self.transition_action(
            workspace_id,
            &action.id,
            FinancialActionStatus::Denied,
            reason,
        )
        .await
    }

    async fn apply_financial_policies(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action: FinancialActionRecord,
        reserve_if_allowed: bool,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let families = self
            .enabled_financial_families(workspace_id, environment_id)
            .await?;
        let mut decision = self
            .current_policy_decision(workspace_id, &action, &families)
            .await?;
        if matches!(
            decision.as_ref(),
            Some((Verdict::Block | Verdict::Rewrite, _))
        ) {
            return deny_for_policy(self.store.as_ref(), workspace_id, &action.id, decision).await;
        }
        if reserve_if_allowed || matches!(decision.as_ref(), Some((Verdict::Escalate, _))) {
            let reservation = self
                .reserve_action_budget(workspace_id, &action, &families)
                .await?;
            decision = compose_policy_decisions(decision, reservation.decision);
        }

        match decision {
            Some((Verdict::Block | Verdict::Rewrite, reason)) => {
                self.store
                    .transition_action_with_reason(
                        workspace_id,
                        &action.id,
                        FinancialActionStatus::Denied,
                        "policy_denied",
                        &reason,
                    )
                    .await
            }
            Some((Verdict::Escalate, reason)) => {
                let approver_roles = financial_approver_roles(&families, &action.action);
                self.hold_reserved_action(
                    workspace_id,
                    &action.id,
                    ApprovalRequirement {
                        required: true,
                        approver_roles,
                        reason,
                        expires_at: None,
                    },
                )
                .await
            }
            Some((Verdict::Allow, _)) | None => Ok(action),
        }
    }

    async fn enabled_financial_families(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Vec<Arc<FamilyPolicy>>, FinancialStoreError> {
        let Some(policy_store) = &self.policy_store else {
            return Ok(vec![]);
        };
        policy_store
            .list_enabled_families(workspace_id, environment_id)
            .await
            .map_err(|error| FinancialStoreError::Internal(format!("financial policies: {error}")))
    }

    async fn current_policy_decision(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        families: &[Arc<FamilyPolicy>],
    ) -> Result<Option<(Verdict, String)>, FinancialStoreError> {
        let pure = self
            .current_per_action_policy_decision(workspace_id, action, families)
            .await?;
        let windowed = self
            .evaluate_ledger_windows(workspace_id, action, families)
            .await?;
        let eligibility = financial_eligibility_decision(action, families);
        let decision = compose_policy_decisions(pure, windowed);
        Ok(compose_policy_decisions(decision, eligibility))
    }

    async fn current_commit_policy_decision(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        families: &[Arc<FamilyPolicy>],
    ) -> Result<Option<(Verdict, String)>, FinancialStoreError> {
        let pure = self
            .current_per_action_policy_decision(workspace_id, action, families)
            .await?;
        let windowed = self
            .evaluate_reserved_ledger_windows(workspace_id, action, families)
            .await?;
        let eligibility = financial_eligibility_decision(action, families);
        Ok(compose_policy_decisions(
            compose_policy_decisions(pure, windowed),
            eligibility,
        ))
    }

    async fn current_per_action_policy_decision(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        families: &[Arc<FamilyPolicy>],
    ) -> Result<Option<(Verdict, String)>, FinancialStoreError> {
        let pure = evaluate_financial_policies(&action.action, families.iter().map(Arc::as_ref));
        let reusable_approval = self
            .action_uses_reusable_approval_mandate(workspace_id, action)
            .await?;
        Ok(pure.verdict.and_then(|verdict| {
            if reusable_approval && verdict == Verdict::Escalate {
                None
            } else {
                Some((
                    verdict,
                    pure.reason
                        .unwrap_or_else(|| "financial policy matched".to_string()),
                ))
            }
        }))
    }

    async fn action_uses_reusable_approval_mandate(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
    ) -> Result<bool, FinancialStoreError> {
        let Some(reference) = &action.action.mandate else {
            return Ok(false);
        };
        let mandate = match self
            .store
            .get_mandate(workspace_id, &reference.id, reference.version)
            .await
        {
            Ok(mandate) => mandate,
            Err(FinancialStoreError::NotFound) => return Ok(false),
            Err(error) => return Err(error),
        };
        let fingerprint = action_fingerprint(&action.action)?;
        Ok(reusable_mandate_fingerprint(&mandate) == Some(fingerprint.as_str()))
    }

    async fn reserve_action_budget(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        families: &[Arc<FamilyPolicy>],
    ) -> Result<BudgetReservationDecision, FinancialStoreError> {
        let now = Utc::now();
        let (day_start, week_start, month_start) = financial_window_starts(now)?;
        let mut constraints = Vec::new();
        for family in families {
            let FamilyPolicy::Financial(financial) = family.as_ref() else {
                continue;
            };
            if !financial_matches(financial, &action.action) {
                continue;
            }
            let block_on_breach = financial.on_breach != Action::Escalate;
            for (window, cap_minor) in [
                (FinancialBudgetWindow::Day, financial.daily_minor),
                (FinancialBudgetWindow::Week, financial.weekly_minor),
                (FinancialBudgetWindow::Month, financial.monthly_minor),
            ] {
                if let Some(cap_minor) = cap_minor {
                    constraints.push(FinancialBudgetConstraint {
                        policy_id: financial.id.clone(),
                        window,
                        cap_minor,
                        block_on_breach,
                    });
                }
            }
        }
        let outcome = self
            .store
            .try_reserve_action_budget(FinancialBudgetReservationRequest {
                workspace_id: workspace_id.to_string(),
                action_id: action.id.clone(),
                principal_id: action.action.principal_id.clone(),
                amount: action.action.amount.clone(),
                idempotency_key: ledger_idempotency_key(&action.id, "reserved"),
                day_start,
                week_start,
                month_start,
                now,
                constraints,
                metadata: serde_json::json!({
                    "action_id": action.id,
                    "financial_status": action.status,
                    "source": "financial_authorization_service",
                    "reservation": "policy_budget"
                }),
            })
            .await?;
        let violations = match outcome {
            FinancialBudgetReservationOutcome::Reserved { violations, .. }
            | FinancialBudgetReservationOutcome::Denied { violations } => violations,
        };
        Ok(BudgetReservationDecision {
            decision: budget_violation_decision(&violations),
        })
    }

    async fn hold_reserved_action(
        &self,
        workspace_id: &str,
        action_id: &str,
        approval: ApprovalRequirement,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let held = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Held,
                "approval_required",
            )
            .await?;
        self.store
            .create_approval_request(workspace_id, action_id, approval)
            .await?;
        Ok(held)
    }

    async fn action_has_approved_review(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
    ) -> Result<bool, FinancialStoreError> {
        self.store
            .has_current_approved_request(workspace_id, &action.id)
            .await
    }

    async fn deny_reserved_action(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        reason: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.record_action_ledger_entry(
            workspace_id,
            action,
            FinancialLedgerEntryKind::Released,
            "released",
        )
        .await?;
        self.store
            .transition_action_with_reason(
                workspace_id,
                &action.id,
                FinancialActionStatus::Denied,
                "policy_denied_before_execution",
                reason,
            )
            .await
    }

    async fn evaluate_ledger_windows(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        families: &[Arc<FamilyPolicy>],
    ) -> Result<Option<(Verdict, String)>, FinancialStoreError> {
        let now = Utc::now();
        let (day_start, week_start, month_start) = financial_window_starts(now)?;
        let mut decision = None;
        let mut spend_cache = None;
        for family in families {
            let FamilyPolicy::Financial(financial) = family.as_ref() else {
                continue;
            };
            if !financial_matches(financial, &action.action) {
                continue;
            }
            if financial.daily_minor.is_none()
                && financial.weekly_minor.is_none()
                && financial.monthly_minor.is_none()
            {
                continue;
            }
            let (spent_today, spent_week, spent_month) = match spend_cache {
                Some(spend) => spend,
                None => {
                    let spend = (
                        self.store
                            .net_spend_minor(
                                workspace_id,
                                &action.action.principal_id,
                                &action.action.amount.currency,
                                day_start,
                                now,
                            )
                            .await?,
                        self.store
                            .net_spend_minor(
                                workspace_id,
                                &action.action.principal_id,
                                &action.action.amount.currency,
                                week_start,
                                now,
                            )
                            .await?,
                        self.store
                            .net_spend_minor(
                                workspace_id,
                                &action.action.principal_id,
                                &action.action.amount.currency,
                                month_start,
                                now,
                            )
                            .await?,
                    );
                    spend_cache = Some(spend);
                    spend
                }
            };
            let next = financial_windowed_verdict(
                financial,
                spent_today,
                spent_week,
                spent_month,
                action.action.amount.amount_minor,
            );
            let Some(next) = next else { continue };
            decision = compose_policy_decisions(decision, Some(next));
        }
        Ok(decision)
    }

    async fn evaluate_reserved_ledger_windows(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        families: &[Arc<FamilyPolicy>],
    ) -> Result<Option<(Verdict, String)>, FinancialStoreError> {
        let now = Utc::now();
        let (day_start, week_start, month_start) = financial_window_starts(now)?;
        let spent = (
            self.store
                .net_spend_minor(
                    workspace_id,
                    &action.action.principal_id,
                    &action.action.amount.currency,
                    day_start,
                    now,
                )
                .await?,
            self.store
                .net_spend_minor(
                    workspace_id,
                    &action.action.principal_id,
                    &action.action.amount.currency,
                    week_start,
                    now,
                )
                .await?,
            self.store
                .net_spend_minor(
                    workspace_id,
                    &action.action.principal_id,
                    &action.action.amount.currency,
                    month_start,
                    now,
                )
                .await?,
        );
        let mut violations = Vec::new();
        for family in families {
            let FamilyPolicy::Financial(financial) = family.as_ref() else {
                continue;
            };
            if !financial_matches(financial, &action.action) {
                continue;
            }
            let block_on_breach = financial.on_breach != Action::Escalate;
            for (window, committed_minor, cap_minor) in [
                (FinancialBudgetWindow::Day, spent.0, financial.daily_minor),
                (FinancialBudgetWindow::Week, spent.1, financial.weekly_minor),
                (
                    FinancialBudgetWindow::Month,
                    spent.2,
                    financial.monthly_minor,
                ),
            ] {
                if let Some(cap_minor) = cap_minor.filter(|cap| committed_minor > *cap) {
                    violations.push(FinancialBudgetViolation {
                        policy_id: financial.id.clone(),
                        window,
                        cap_minor,
                        committed_minor,
                        requested_minor: 0,
                        block_on_breach,
                    });
                }
            }
        }
        Ok(budget_violation_decision(&violations))
    }
}

async fn deny_for_policy(
    store: &dyn FinancialStore,
    workspace_id: &str,
    action_id: &str,
    decision: Option<(Verdict, String)>,
) -> Result<FinancialActionRecord, FinancialStoreError> {
    let reason = decision
        .map(|(_, reason)| reason)
        .ok_or_else(|| FinancialStoreError::Internal("missing policy denial reason".into()))?;
    store
        .transition_action_with_reason(
            workspace_id,
            action_id,
            FinancialActionStatus::Denied,
            "policy_denied",
            &reason,
        )
        .await
}

type FinancialWindowStarts = (DateTime<Utc>, DateTime<Utc>, DateTime<Utc>);

fn financial_window_starts(
    now: DateTime<Utc>,
) -> Result<FinancialWindowStarts, FinancialStoreError> {
    let day_start = Utc
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .single()
        .ok_or_else(|| FinancialStoreError::Internal("invalid day window".into()))?;
    // ponytail: week starts Monday UTC; make configurable if a customer asks
    let week_start = day_start - Duration::days(i64::from(now.weekday().num_days_from_monday()));
    let month_start = Utc
        .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .ok_or_else(|| FinancialStoreError::Internal("invalid month window".into()))?;
    Ok((day_start, week_start, month_start))
}

fn budget_violation_decision(violations: &[FinancialBudgetViolation]) -> Option<(Verdict, String)> {
    violations.iter().fold(None, |decision, violation| {
        let verdict = if violation.block_on_breach {
            Verdict::Block
        } else {
            Verdict::Escalate
        };
        let window = match violation.window {
            FinancialBudgetWindow::Day => "daily",
            FinancialBudgetWindow::Week => "weekly",
            FinancialBudgetWindow::Month => "monthly",
        };
        compose_policy_decisions(
            decision,
            Some((
                verdict,
                format!(
                    "financial policy `{}`: {window} spend would exceed cap {}",
                    violation.policy_id, violation.cap_minor
                ),
            )),
        )
    })
}

fn agentic_payment_principal(
    requested_principal_id: &str,
    runtime_key: Option<&WorkspaceKeyContext>,
) -> Result<String, FinancialStoreError> {
    let requested = requested_principal_id.trim();
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

fn ensure_agentic_payment_principal(
    action: &FinancialActionRecord,
    runtime_key: Option<&WorkspaceKeyContext>,
) -> Result<(), FinancialStoreError> {
    let Some(runtime_key) = runtime_key else {
        return Ok(());
    };
    let bound = runtime_key_principal(runtime_key);
    if action.action.principal_id != bound {
        return Err(FinancialStoreError::Validation(
            "runtime API key principal cannot operate on this payment".into(),
        ));
    }
    Ok(())
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
            "scheme": normalized.scheme,
            "resource": normalized.resource,
            "method": normalized.method,
            "host": normalized.host,
            "facilitator": normalized.facilitator,
        }),
    }
}

fn agentic_payment_metadata(
    input: &AgenticPaymentAuthorizeRequest,
    normalized: &X402NormalizedPaymentRequirement,
    principal_id: &str,
    runtime_key: Option<&WorkspaceKeyContext>,
    session_limit_minor: i64,
    reservation_expires_at: DateTime<Utc>,
) -> serde_json::Value {
    serde_json::json!({
        "agentic_payment": {
            "protocol": "x402",
            "session_id": input.session_id,
            "requested_principal_id": input.principal_id,
            "principal_id": principal_id,
            "session_limit_minor": session_limit_minor,
            "reservation_expires_at": reservation_expires_at.to_rfc3339(),
            "payment_requirement": input.payment_requirement,
            "normalized_requirement": normalized,
            "traceparent": input.traceparent,
            "tracestate": input.tracestate,
            "runtime_api_key_id": runtime_key.map(|key| key.api_key_id.as_str()),
        },
        "x402": {
            "session_id": input.session_id,
            "payment_requirement": input.payment_requirement,
            "normalized_requirement": normalized,
        },
        "customer_metadata": input.metadata,
    })
}

fn normalize_create_mandate_request(
    mut input: CreateFinancialMandateRequest,
) -> Result<CreateFinancialMandateRequest, FinancialStoreError> {
    let Some(payment_scope) = input.payment_scope.as_ref() else {
        return Ok(input);
    };
    validate_payment_mandate_scope(payment_scope)?;
    let normalized_scope = payment_scope_json(payment_scope)?;
    if !json_scope_is_empty(&input.scope) && input.scope != normalized_scope {
        return Err(FinancialStoreError::Validation(
            "provide either scope or payment_scope, not conflicting mandate scopes".into(),
        ));
    }
    input.scope = normalized_scope;
    Ok(input)
}

fn validate_payment_mandate_scope(
    scope: &AgenticPaymentMandateScope,
) -> Result<(), FinancialStoreError> {
    if scope.action_kinds.is_empty() {
        return Err(FinancialStoreError::Validation(
            "payment_scope.action_kinds must include at least one action kind".into(),
        ));
    }
    if scope
        .currency
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        return Err(FinancialStoreError::Validation(
            "payment_scope.currency is required".into(),
        ));
    }
    match scope.max_amount_minor {
        Some(amount) if amount > 0 => {}
        _ => {
            return Err(FinancialStoreError::Validation(
                "payment_scope.max_amount_minor must be greater than zero".into(),
            ))
        }
    }
    Ok(())
}

fn payment_scope_json(
    scope: &AgenticPaymentMandateScope,
) -> Result<serde_json::Value, FinancialStoreError> {
    let mut value = serde_json::to_value(scope)
        .map_err(|e| FinancialStoreError::Internal(format!("payment scope encode: {e}")))?;
    if let Some(object) = value.as_object_mut() {
        object.retain(|_, value| match value {
            serde_json::Value::Null => false,
            serde_json::Value::Array(values) => !values.is_empty(),
            serde_json::Value::String(value) => !value.trim().is_empty(),
            _ => true,
        });
    }
    Ok(value)
}

fn json_scope_is_empty(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => true,
        serde_json::Value::Object(object) => object.is_empty(),
        _ => false,
    }
}

const APPROVAL_FINGERPRINT_VERSION: i32 = 1;

#[derive(Serialize)]
struct ApprovalFingerprintPayload {
    version: i32,
    principal_id: String,
    action_kind: FinancialActionKind,
    operation: String,
    rail: FinancialRail,
    currency: String,
    counterparty_id: Option<String>,
    counterparty_kind: Option<String>,
    counterparty_country: Option<String>,
    x402_host: Option<String>,
    x402_resource: Option<String>,
    x402_network: Option<String>,
    x402_asset: Option<String>,
    x402_pay_to: Option<String>,
}

fn action_fingerprint(action: &FinancialAction) -> Result<String, FinancialStoreError> {
    let normalized_x402 = action
        .metadata
        .get("agentic_payment")
        .and_then(|value| value.get("normalized_requirement"))
        .or_else(|| {
            action
                .metadata
                .get("x402")
                .and_then(|value| value.get("normalized_requirement"))
        })
        .cloned()
        .and_then(|value| serde_json::from_value::<X402NormalizedPaymentRequirement>(value).ok());
    let payload = ApprovalFingerprintPayload {
        version: APPROVAL_FINGERPRINT_VERSION,
        principal_id: action.principal_id.trim().to_string(),
        action_kind: action.kind,
        operation: action.operation.trim().to_string(),
        rail: action.rail,
        currency: action.amount.currency.trim().to_ascii_uppercase(),
        counterparty_id: action
            .counterparty
            .as_ref()
            .map(|counterparty| counterparty.id.trim().to_string()),
        counterparty_kind: action
            .counterparty
            .as_ref()
            .map(|counterparty| counterparty.kind.trim().to_ascii_lowercase()),
        counterparty_country: action
            .counterparty
            .as_ref()
            .and_then(|counterparty| counterparty.country.as_deref())
            .map(|country| country.trim().to_ascii_uppercase()),
        x402_host: normalized_x402
            .as_ref()
            .and_then(|requirement| requirement.host.clone()),
        x402_resource: normalized_x402
            .as_ref()
            .and_then(|requirement| requirement.resource.clone()),
        x402_network: normalized_x402
            .as_ref()
            .and_then(|requirement| requirement.network.clone()),
        x402_asset: normalized_x402
            .as_ref()
            .and_then(|requirement| requirement.asset.clone()),
        x402_pay_to: normalized_x402
            .as_ref()
            .and_then(|requirement| requirement.normalized_pay_to.clone())
            .or_else(|| {
                normalized_x402
                    .as_ref()
                    .map(|requirement| requirement.pay_to.clone())
            }),
    };
    let encoded = serde_json::to_vec(&payload).map_err(|error| {
        FinancialStoreError::Internal(format!("action fingerprint encode: {error}"))
    })?;
    let mut hasher = Sha256::new();
    hasher.update(encoded);
    let hash_hex = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!("sha256:v{APPROVAL_FINGERPRINT_VERSION}:{hash_hex}"))
}

fn approval_envelope(
    action: &FinancialActionRecord,
) -> Result<FinancialApprovalEnvelope, FinancialStoreError> {
    Ok(FinancialApprovalEnvelope {
        action_id: action.id.clone(),
        action_fingerprint: action_fingerprint(&action.action)?,
        fingerprint_version: APPROVAL_FINGERPRINT_VERSION,
        principal_id: action.action.principal_id.clone(),
        action_kind: action.action.kind,
        operation: action.action.operation.clone(),
        rail: action.action.rail,
        currency: action.action.amount.currency.to_ascii_uppercase(),
        counterparty_id: action
            .action
            .counterparty
            .as_ref()
            .map(|counterparty| counterparty.id.clone()),
        current_amount_minor: action.action.amount.amount_minor,
        recommended_max_amount_minor: action.action.amount.amount_minor,
    })
}

fn reusable_mandate_id(fingerprint: &str) -> String {
    let digest = fingerprint.rsplit(':').next().unwrap_or(fingerprint);
    let short = digest.get(..32).unwrap_or(digest);
    format!("approval-reuse-{short}")
}

fn reusable_mandate_fingerprint(mandate: &FinancialMandate) -> Option<&str> {
    if mandate
        .metadata
        .get("mode")
        .and_then(serde_json::Value::as_str)
        != Some("approval_reuse")
    {
        return None;
    }
    mandate
        .metadata
        .get("action_fingerprint")
        .and_then(serde_json::Value::as_str)
}

fn reusable_payment_scope(
    action: &FinancialActionRecord,
    max_amount_minor: i64,
) -> Result<AgenticPaymentMandateScope, FinancialStoreError> {
    let normalized_x402 = if action.action.rail == FinancialRail::X402 {
        Some(normalized_requirement_from_action(action)?)
    } else {
        None
    };
    Ok(AgenticPaymentMandateScope {
        intent_label: Some(format!(
            "Approved matching {} actions",
            action.action.operation
        )),
        action_kinds: vec![action.action.kind],
        operation: Some(action.action.operation.clone()),
        max_amount_minor: Some(max_amount_minor),
        currency: Some(action.action.amount.currency.to_ascii_uppercase()),
        rail: Some(action.action.rail),
        allowed_counterparty_ids: action
            .action
            .counterparty
            .as_ref()
            .map(|counterparty| vec![counterparty.id.clone()])
            .unwrap_or_default(),
        allowed_hosts: normalized_x402
            .as_ref()
            .and_then(|requirement| requirement.host.clone())
            .into_iter()
            .collect(),
        allowed_resources: normalized_x402
            .as_ref()
            .and_then(|requirement| requirement.resource.clone())
            .into_iter()
            .collect(),
        allowed_networks: normalized_x402
            .as_ref()
            .and_then(|requirement| requirement.network.clone())
            .into_iter()
            .collect(),
        allowed_assets: normalized_x402
            .as_ref()
            .and_then(|requirement| requirement.asset.clone())
            .into_iter()
            .collect(),
        allowed_pay_to: normalized_x402
            .as_ref()
            .map(|requirement| {
                requirement
                    .normalized_pay_to
                    .clone()
                    .unwrap_or_else(|| requirement.pay_to.clone())
            })
            .into_iter()
            .collect(),
        required_preconditions: vec![],
    })
}

fn mandate_hash(scope: &serde_json::Value) -> Result<String, FinancialStoreError> {
    let encoded = serde_json::to_vec(scope)
        .map_err(|e| FinancialStoreError::Internal(format!("mandate scope encode: {e}")))?;
    let mut hasher = Sha256::new();
    hasher.update(encoded);
    let hash_hex = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!("sha256:{hash_hex}"))
}

fn agentic_payment_decision(status: FinancialActionStatus) -> AgenticPaymentDecision {
    match status {
        FinancialActionStatus::Authorized | FinancialActionStatus::Proposed => {
            AgenticPaymentDecision::Authorized
        }
        FinancialActionStatus::Held => AgenticPaymentDecision::Held,
        FinancialActionStatus::Executed => AgenticPaymentDecision::Committed,
        FinancialActionStatus::Denied => AgenticPaymentDecision::Denied,
        FinancialActionStatus::Failed | FinancialActionStatus::Expired => {
            AgenticPaymentDecision::Failed
        }
        FinancialActionStatus::Reversed => AgenticPaymentDecision::RolledBack,
    }
}

fn agentic_payment_authorization_reason(
    status: FinancialActionStatus,
    status_reason: Option<&str>,
) -> String {
    if let Some(reason) = status_reason.filter(|reason| !reason.is_empty()) {
        return reason.to_string();
    }
    match status {
        FinancialActionStatus::Authorized => {
            "x402 payment authorized; reservation is ready for signing".into()
        }
        FinancialActionStatus::Held => "x402 payment requires human approval before signing".into(),
        FinancialActionStatus::Denied => "x402 payment denied before signing".into(),
        FinancialActionStatus::Failed => "x402 payment authorization failed".into(),
        FinancialActionStatus::Proposed => "x402 payment passed initial checks".into(),
        FinancialActionStatus::Executed => "x402 payment already committed".into(),
        FinancialActionStatus::Reversed => "x402 payment was rolled back".into(),
        FinancialActionStatus::Expired => "x402 payment authorization expired".into(),
    }
}

fn normalized_requirement_from_action(
    action: &FinancialActionRecord,
) -> Result<X402NormalizedPaymentRequirement, FinancialStoreError> {
    if action.action.rail != FinancialRail::X402 {
        return Err(FinancialStoreError::Validation(
            "financial action is not an x402 agentic payment".into(),
        ));
    }
    let value = action
        .action
        .metadata
        .get("agentic_payment")
        .and_then(|value| value.get("normalized_requirement"))
        .or_else(|| {
            action
                .action
                .metadata
                .get("x402")
                .and_then(|value| value.get("normalized_requirement"))
        })
        .cloned()
        .ok_or_else(|| {
            FinancialStoreError::Internal(
                "x402 normalized payment requirement missing from action metadata".into(),
            )
        })?;
    serde_json::from_value(value)
        .map_err(|e| FinancialStoreError::Internal(format!("x402 metadata decode: {e}")))
}

fn agentic_payment_execution_result(proof: &X402SettlementProof) -> FinancialExecutionResult {
    FinancialExecutionResult {
        provider_status: Some("settled".into()),
        provider_reference: proof.settlement_reference.clone(),
        provider_response: serde_json::json!({
            "protocol": "x402",
            "provider": proof.provider,
            "payment_requirement_hash": proof.payment_requirement_hash,
            "payment_response": proof.payment_response,
            "raw": proof.raw,
        }),
        reversal_capability: ReversalCapability::None,
        recovery_status: RecoveryStatus::NotAvailable,
    }
}

pub enum FinancialActionExecutionAttempt {
    Executed(FinancialActionRecord),
    Failed {
        action: FinancialActionRecord,
        reason: String,
    },
}

fn financial_evidence_proofs(
    action: &FinancialActionRecord,
    families: &[Arc<FamilyPolicy>],
) -> (Vec<FinancialEvidenceProof>, Vec<FinancialDecisionRisk>) {
    let mut proofs = Vec::new();
    let mut risks = Vec::new();
    for family in families {
        let FamilyPolicy::Financial(financial) = family.as_ref() else {
            continue;
        };
        if !financial_matches(financial, &action.action) {
            continue;
        }
        for precondition in &financial.required_preconditions {
            if proofs
                .iter()
                .any(|proof: &FinancialEvidenceProof| proof.precondition == *precondition)
            {
                continue;
            }
            let key = precondition_key(*precondition);
            let evidence = evidence_for_key(action, key);
            let (status, reason) = match evidence.and_then(|item| item.metadata.get(key)) {
                Some(value) if value.as_bool() == Some(true) => {
                    (FinancialEligibilityStatus::Passed, None)
                }
                Some(value) if value.as_bool() == Some(false) => (
                    FinancialEligibilityStatus::Failed,
                    Some(format!("eligibility precondition `{key}` failed")),
                ),
                _ => (
                    FinancialEligibilityStatus::Missing,
                    Some(format!("missing eligibility evidence `{key}`")),
                ),
            };
            if status != FinancialEligibilityStatus::Passed {
                let reason = reason
                    .clone()
                    .unwrap_or_else(|| format!("eligibility precondition `{key}` failed"));
                risks.push(FinancialDecisionRisk {
                    code: if status == FinancialEligibilityStatus::Missing {
                        FinancialDecisionRiskCode::MissingEvidence
                    } else {
                        FinancialDecisionRiskCode::FailedEvidence
                    },
                    severity: financial.severity,
                    reason,
                    policy_id: Some(financial.id.clone()),
                    source: "eligibility".into(),
                });
            }
            proofs.push(FinancialEvidenceProof {
                precondition: *precondition,
                status,
                evidence_source_id: evidence.map(|item| item.source_id.clone()),
                reason,
            });
        }
    }
    (proofs, risks)
}

fn evidence_for_key<'a>(action: &'a FinancialActionRecord, key: &str) -> Option<&'a EvidenceRef> {
    action
        .evidence
        .iter()
        .find(|evidence| evidence.metadata.get(key).is_some())
}

fn risk_from_reason(
    reason: &str,
    severity: Severity,
    policy_id: Option<String>,
    source: &str,
) -> FinancialDecisionRisk {
    FinancialDecisionRisk {
        code: risk_code_from_reason(reason),
        severity,
        reason: reason.to_string(),
        policy_id,
        source: source.to_string(),
    }
}

fn risk_code_from_reason(reason: &str) -> FinancialDecisionRiskCode {
    if reason.contains("at or above hold threshold")
        || reason.contains("at or above approval threshold")
    {
        FinancialDecisionRiskCode::AmountAboveAutoApproveThreshold
    } else if reason.contains("over per-transaction cap") {
        FinancialDecisionRiskCode::AmountOverPerTransactionCap
    } else if reason.contains("mandate required") || reason.contains("authorization scope required")
    {
        FinancialDecisionRiskCode::MissingAuthorizationScope
    } else if reason.contains("denied counterparty") {
        FinancialDecisionRiskCode::CounterpartyDenied
    } else if reason.contains("is not allowed") || reason.contains("missing counterparty") {
        FinancialDecisionRiskCode::CounterpartyNotAllowed
    } else if reason.contains("new counterparty") {
        FinancialDecisionRiskCode::NewCounterparty
    } else if reason.contains("missing eligibility evidence") {
        FinancialDecisionRiskCode::MissingEvidence
    } else if reason.contains("eligibility precondition") && reason.contains("failed") {
        FinancialDecisionRiskCode::FailedEvidence
    } else if reason.contains("daily spend would exceed cap") {
        FinancialDecisionRiskCode::DailyCapExceeded
    } else if reason.contains("weekly spend would exceed cap") {
        FinancialDecisionRiskCode::WeeklyCapExceeded
    } else if reason.contains("monthly spend would exceed cap") {
        FinancialDecisionRiskCode::MonthlyCapExceeded
    } else if reason.contains("provider_failed") || reason.contains("provider failed") {
        FinancialDecisionRiskCode::ProviderFailed
    } else {
        FinancialDecisionRiskCode::Unknown
    }
}

fn policy_id_from_reason(reason: &str) -> Option<String> {
    let start = reason.find('`')?;
    let rest = &reason[start + 1..];
    let end = rest.find('`')?;
    Some(rest[..end].to_string())
}

fn action_decision(
    status: FinancialActionStatus,
    policy_decision: Option<&(Verdict, String)>,
) -> FinancialActionDecision {
    match status {
        FinancialActionStatus::Held => FinancialActionDecision::Hold,
        FinancialActionStatus::Denied | FinancialActionStatus::Failed => {
            FinancialActionDecision::Block
        }
        FinancialActionStatus::Proposed
        | FinancialActionStatus::Authorized
        | FinancialActionStatus::Executed => match policy_decision.map(|(verdict, _)| *verdict) {
            Some(Verdict::Block | Verdict::Rewrite) => FinancialActionDecision::Block,
            Some(Verdict::Escalate) => FinancialActionDecision::Escalate,
            Some(Verdict::Allow) | None => FinancialActionDecision::Allow,
        },
        FinancialActionStatus::Reversed | FinancialActionStatus::Expired => {
            FinancialActionDecision::Block
        }
    }
}

fn decision_receipt_reason(
    action: &FinancialActionRecord,
    decision: FinancialActionDecision,
    risks: &[FinancialDecisionRisk],
    approval: Option<&ApprovalRequirement>,
) -> String {
    if decision == FinancialActionDecision::Hold
        && risks
            .iter()
            .any(|risk| risk.code == FinancialDecisionRiskCode::AmountAboveAutoApproveThreshold)
    {
        return "valid refund, but above threshold so human approval required".into();
    }
    if let Some(reason) = action
        .status_reason
        .as_ref()
        .filter(|reason| !reason.is_empty())
    {
        return reason.clone();
    }
    if let Some(approval) = approval {
        return approval.reason.clone();
    }
    if let Some(risk) = risks.first() {
        return risk.reason.clone();
    }
    match decision {
        FinancialActionDecision::Allow => "financial action passed authorization checks".into(),
        FinancialActionDecision::Hold => "financial action requires human approval".into(),
        FinancialActionDecision::Block => "financial action blocked before execution".into(),
        FinancialActionDecision::Escalate => "financial action requires escalation".into(),
    }
}

fn authorization_scope_summary(scope: &FinancialMandate) -> Option<String> {
    let max_amount = scope
        .scope
        .get("max_amount_minor")
        .and_then(serde_json::Value::as_i64);
    let currency = scope
        .scope
        .get("currency")
        .and_then(serde_json::Value::as_str);
    match (max_amount, currency) {
        (Some(max_amount), Some(currency)) => Some(format!(
            "{} may spend up to {} {}",
            scope.principal_id,
            currency,
            money_major(max_amount)
        )),
        _ => Some("authorization scope passed".into()),
    }
}

fn money_major(amount_minor: i64) -> String {
    format!("{:.2}", amount_minor as f64 / 100.0)
}

fn compose_policy_decisions(
    current: Option<(Verdict, String)>,
    next: Option<(Verdict, String)>,
) -> Option<(Verdict, String)> {
    match (current, next) {
        (None, decision) | (decision, None) => decision,
        (Some((current_verdict, current_reason)), Some((next_verdict, next_reason))) => {
            let worst = current_verdict.worst_with(next_verdict);
            if worst == next_verdict && next_verdict != current_verdict {
                Some((worst, next_reason))
            } else {
                Some((worst, current_reason))
            }
        }
    }
}

fn ledger_idempotency_key(action_id: &str, suffix: &str) -> String {
    format!("{action_id}:{suffix}")
}

fn provider_proof(result: &Option<FinancialExecutionResult>) -> serde_json::Value {
    match result {
        Some(result) => serde_json::json!({
            "status": result.provider_status,
            "reference": result.provider_reference,
            "response": result.provider_response,
            "reversal_capability": result.reversal_capability,
            "recovery_status": result.recovery_status
        }),
        None => serde_json::Value::Null,
    }
}

fn receipt_policy_matches(policy: &FamilyPolicy, action: &FinancialAction) -> bool {
    match policy {
        FamilyPolicy::Financial(financial) => financial_matches(financial, action),
        _ => false,
    }
}

fn financial_approver_roles(
    families: &[Arc<FamilyPolicy>],
    action: &FinancialAction,
) -> Vec<String> {
    let mut roles = Vec::new();
    for family in families {
        let FamilyPolicy::Financial(financial) = family.as_ref() else {
            continue;
        };
        if !financial_matches(financial, action) {
            continue;
        }
        for role in &financial.approver_roles {
            if !roles.iter().any(|existing| existing == role) {
                roles.push(role.clone());
            }
        }
    }
    roles
}

fn financial_eligibility_decision(
    action: &FinancialActionRecord,
    families: &[Arc<FamilyPolicy>],
) -> Option<(Verdict, String)> {
    let mut decision = None;
    for family in families {
        let FamilyPolicy::Financial(financial) = family.as_ref() else {
            continue;
        };
        if !financial_matches(financial, &action.action) {
            continue;
        }
        for precondition in &financial.required_preconditions {
            let key = precondition_key(*precondition);
            let next = match evidence_bool(action, key) {
                Some(true) => None,
                Some(false) => Some((
                    policy_action_verdict(financial.failed_precondition_action),
                    format!(
                        "financial policy `{}`: eligibility precondition `{key}` failed",
                        financial.id
                    ),
                )),
                None => Some((
                    policy_action_verdict(financial.missing_evidence_action),
                    format!(
                        "financial policy `{}`: missing eligibility evidence `{key}`",
                        financial.id
                    ),
                )),
            };
            decision = compose_policy_decisions(decision, next);
        }
    }
    decision
}

fn financial_status_label(status: FinancialActionStatus) -> &'static str {
    match status {
        FinancialActionStatus::Proposed => "proposed",
        FinancialActionStatus::Authorized => "authorized",
        FinancialActionStatus::Held => "held",
        FinancialActionStatus::Executed => "executed",
        FinancialActionStatus::Denied => "denied",
        FinancialActionStatus::Failed => "failed",
        FinancialActionStatus::Reversed => "reversed",
        FinancialActionStatus::Expired => "expired",
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
        hold_above_minor: input.hold_above_minor,
        daily_minor: input.daily_minor,
        weekly_minor: input.weekly_minor,
        monthly_minor: input.monthly_minor,
        allowed_counterparty_ids: input.allowed_counterparty_ids,
        denied_counterparty_ids: input.denied_counterparty_ids,
        hold_new_counterparty: input.hold_new_counterparty,
        mandate_required: input.mandate_required,
        approval_threshold_minor: input.approval_threshold_minor,
        approver_roles: input.approver_roles,
        refund_original_method_only: input.refund_original_method_only,
        required_preconditions: input.required_preconditions,
        missing_evidence_action: enforcing_action(
            "missing_evidence_action",
            input
                .missing_evidence_action
                .unwrap_or(PolicyAction::Escalate),
        )?,
        failed_precondition_action: enforcing_action(
            "failed_precondition_action",
            input
                .failed_precondition_action
                .unwrap_or(PolicyAction::Block),
        )?,
        on_breach: enforcing_action("on_breach", input.on_breach.unwrap_or(PolicyAction::Block))?,
    })
}

fn financial_policy_record(policy: &FinancialPolicy, enabled: bool) -> FinancialPolicyRecord {
    FinancialPolicyRecord {
        id: policy.id.clone(),
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
        hold_above_minor: policy.hold_above_minor,
        daily_minor: policy.daily_minor,
        weekly_minor: policy.weekly_minor,
        monthly_minor: policy.monthly_minor,
        allowed_counterparty_ids: policy.allowed_counterparty_ids.clone(),
        denied_counterparty_ids: policy.denied_counterparty_ids.clone(),
        hold_new_counterparty: policy.hold_new_counterparty,
        mandate_required: policy.mandate_required,
        approval_threshold_minor: policy.approval_threshold_minor,
        approver_roles: policy.approver_roles.clone(),
        refund_original_method_only: policy.refund_original_method_only,
        required_preconditions: policy.required_preconditions.clone(),
        missing_evidence_action: policy_action(policy.missing_evidence_action),
        failed_precondition_action: policy_action(policy.failed_precondition_action),
        on_breach: policy_action(policy.on_breach),
        enabled,
    }
}

fn enforcing_action(field: &str, action: PolicyAction) -> Result<Action, FinancialStoreError> {
    match action {
        PolicyAction::Block => Ok(Action::Block),
        PolicyAction::Escalate => Ok(Action::Escalate),
        PolicyAction::Rewrite => Err(FinancialStoreError::Validation(format!(
            "{field}: must be block or escalate"
        ))),
    }
}

fn policy_action(action: Action) -> PolicyAction {
    match action {
        Action::Escalate => PolicyAction::Escalate,
        Action::Allow | Action::Block | Action::Rewrite => PolicyAction::Block,
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
        FinancialActionPrecondition::MandateValid => "mandate_valid",
        FinancialActionPrecondition::Custom => "custom",
    }
}

fn policy_action_verdict(action: Action) -> Verdict {
    match action {
        Action::Block => Verdict::Block,
        Action::Escalate => Verdict::Escalate,
        Action::Allow | Action::Rewrite => Verdict::Block,
    }
}

fn mandate_denial_reason(
    mandate: &FinancialMandate,
    action: &FinancialActionRecord,
) -> Result<Option<String>, FinancialStoreError> {
    if mandate.status != FinancialMandateStatus::Active {
        return Ok(Some("mandate_inactive".into()));
    }
    if mandate.principal_id != action.action.principal_id {
        return Ok(Some("mandate_principal_mismatch".into()));
    }
    let now = Utc::now();
    if let Some(starts_at) =
        parse_optional_rfc3339("mandate starts_at", mandate.starts_at.as_deref())?
    {
        if starts_at > now {
            return Ok(Some("mandate_not_started".into()));
        }
    }
    if let Some(expires_at) =
        parse_optional_rfc3339("mandate expires_at", mandate.expires_at.as_deref())?
    {
        if expires_at <= now {
            return Ok(Some("mandate_expired".into()));
        }
    }
    mandate_scope_denial_reason(mandate, action)
}

fn mandate_scope_denial_reason(
    mandate: &FinancialMandate,
    action: &FinancialActionRecord,
) -> Result<Option<String>, FinancialStoreError> {
    if let Some(action_kinds) = mandate.scope.get("action_kinds") {
        let expected = serde_json::to_value(action.action.kind)
            .map_err(|e| FinancialStoreError::Internal(format!("action kind encode: {e}")))?;
        let expected = expected
            .as_str()
            .ok_or_else(|| FinancialStoreError::Internal("action kind encode".into()))?;
        if !json_string_array_contains(action_kinds, expected)? {
            return Ok(Some("mandate_scope_action_kind_mismatch".into()));
        }
    }

    if let Some(operation) = mandate.scope.get("operation") {
        let Some(operation) = operation.as_str() else {
            return Ok(Some("mandate_scope_operation_invalid".into()));
        };
        if operation != action.action.operation {
            return Ok(Some("mandate_scope_operation_mismatch".into()));
        }
    }

    if let Some(rail) = mandate.scope.get("rail") {
        let expected = serde_json::to_value(action.action.rail)
            .map_err(|e| FinancialStoreError::Internal(format!("rail encode: {e}")))?;
        let expected = expected
            .as_str()
            .ok_or_else(|| FinancialStoreError::Internal("rail encode".into()))?;
        let Some(rail) = rail.as_str() else {
            return Ok(Some("mandate_scope_rail_invalid".into()));
        };
        if !rail.eq_ignore_ascii_case(expected) {
            return Ok(Some("mandate_scope_rail_mismatch".into()));
        }
    }

    if let Some(currency) = mandate.scope.get("currency") {
        let Some(currency) = currency.as_str() else {
            return Ok(Some("mandate_scope_currency_invalid".into()));
        };
        if !currency.eq_ignore_ascii_case(&action.action.amount.currency) {
            return Ok(Some("mandate_scope_currency_mismatch".into()));
        }
    }

    if let Some(currencies) = mandate.scope.get("currencies") {
        if !json_string_array_contains(currencies, &action.action.amount.currency)? {
            return Ok(Some("mandate_scope_currency_mismatch".into()));
        }
    }

    if let Some(max_amount) = mandate.scope.get("max_amount_minor") {
        let Some(max_amount) = max_amount.as_i64() else {
            return Ok(Some("mandate_scope_max_amount_invalid".into()));
        };
        if action.action.amount.amount_minor > max_amount {
            return Ok(Some("mandate_scope_amount_exceeded".into()));
        }
    }

    if let Some(counterparties) = mandate.scope.get("allowed_counterparty_ids") {
        let Some(counterparty) = action.action.counterparty.as_ref() else {
            return Ok(Some("mandate_scope_counterparty_mismatch".into()));
        };
        let mut candidates = vec![counterparty.id.as_str()];
        if let Some(display_name) = counterparty.display_name.as_deref() {
            candidates.push(display_name);
        }
        if !json_string_array_contains_any(counterparties, &candidates)? {
            return Ok(Some("mandate_scope_counterparty_mismatch".into()));
        }
    }

    let x402 = normalized_requirement_from_action(action).ok();
    if let Some(hosts) = mandate.scope.get("allowed_hosts") {
        let Some(host) = x402
            .as_ref()
            .and_then(|requirement| requirement.host.as_deref())
        else {
            return Ok(Some("mandate_scope_host_mismatch".into()));
        };
        if !json_string_array_contains(hosts, host)? {
            return Ok(Some("mandate_scope_host_mismatch".into()));
        }
    }

    if let Some(resources) = mandate.scope.get("allowed_resources") {
        let Some(resource) = x402
            .as_ref()
            .and_then(|requirement| requirement.resource.as_deref())
        else {
            return Ok(Some("mandate_scope_resource_mismatch".into()));
        };
        if !json_string_array_contains(resources, resource)? {
            return Ok(Some("mandate_scope_resource_mismatch".into()));
        }
    }

    if let Some(networks) = mandate.scope.get("allowed_networks") {
        let Some(network) = x402
            .as_ref()
            .and_then(|requirement| requirement.network.as_deref())
        else {
            return Ok(Some("mandate_scope_network_mismatch".into()));
        };
        if !json_string_array_contains(networks, network)? {
            return Ok(Some("mandate_scope_network_mismatch".into()));
        }
    }

    if let Some(assets) = mandate.scope.get("allowed_assets") {
        let candidates = x402
            .as_ref()
            .and_then(|requirement| requirement.asset.as_deref())
            .into_iter()
            .chain(std::iter::once(action.action.amount.currency.as_str()))
            .collect::<Vec<_>>();
        if !json_string_array_contains_any(assets, &candidates)? {
            return Ok(Some("mandate_scope_asset_mismatch".into()));
        }
    }

    if let Some(pay_to) = mandate.scope.get("allowed_pay_to") {
        let Some(requirement) = x402.as_ref() else {
            return Ok(Some("mandate_scope_pay_to_mismatch".into()));
        };
        let mut candidates = vec![requirement.pay_to.as_str()];
        if let Some(normalized_pay_to) = requirement.normalized_pay_to.as_deref() {
            candidates.push(normalized_pay_to);
        }
        if !json_string_array_contains_any(pay_to, &candidates)? {
            return Ok(Some("mandate_scope_pay_to_mismatch".into()));
        }
    }

    Ok(None)
}

fn parse_optional_rfc3339(
    field: &str,
    value: Option<&str>,
) -> Result<Option<DateTime<Utc>>, FinancialStoreError> {
    value
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .map_err(|e| FinancialStoreError::Validation(format!("{field}: {e}")))
        .map(|value| value.map(|dt| dt.with_timezone(&Utc)))
}

fn parse_rfc3339(field: &str, value: &str) -> Result<DateTime<Utc>, FinancialStoreError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|e| FinancialStoreError::Validation(format!("{field}: {e}")))
}

fn json_string_array_contains(
    value: &serde_json::Value,
    expected: &str,
) -> Result<bool, FinancialStoreError> {
    let Some(values) = value.as_array() else {
        return Ok(false);
    };
    for value in values {
        let Some(candidate) = value.as_str() else {
            return Err(FinancialStoreError::Validation(
                "mandate scope array values must be strings".into(),
            ));
        };
        if candidate.eq_ignore_ascii_case(expected) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn json_string_array_contains_any(
    value: &serde_json::Value,
    expected_values: &[&str],
) -> Result<bool, FinancialStoreError> {
    for expected in expected_values {
        if json_string_array_contains(value, expected)? {
            return Ok(true);
        }
    }
    Ok(false)
}
