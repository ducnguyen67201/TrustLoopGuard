//! Unified authorization polling and guarded callback execution.

use std::{future::Future, time::Duration};

use tl_core::{
    ApprovalStatus, AuthorizationApproval, AuthorizationApprovalListResponse,
    AuthorizationDecision, AuthorizationEffect, AuthorizationGrant, AuthorizationGrantListResponse,
    AuthorizationReceipt, CompleteAuthorizationLeaseRequest, CreateAuthorizationGrantRequest,
    DecideAuthorizationApprovalRequest, DecideAuthorizationApprovalResponse, GuardEvent,
    LeaseStatus,
};

use crate::{Client, SdkError};

#[derive(Debug)]
pub struct AuthorizationResult<T> {
    pub decision: AuthorizationDecision,
    pub value: Option<T>,
}

impl<T> AuthorizationResult<T> {
    pub fn executed(&self) -> bool {
        self.value.is_some()
    }
}

impl Client {
    pub async fn get_approval(&self, id: &str) -> Result<AuthorizationApproval, SdkError> {
        let path = format!("/v1/authorization/approvals/{}", urlencoding::encode(id));
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    pub async fn list_approvals(&self) -> Result<AuthorizationApprovalListResponse, SdkError> {
        self.retry_loop("/v1/authorization/approvals", || {
            self.send_get("/v1/authorization/approvals")
        })
        .await
    }

    pub async fn decide_approval(
        &self,
        id: &str,
        request: &DecideAuthorizationApprovalRequest,
    ) -> Result<DecideAuthorizationApprovalResponse, SdkError> {
        let path = format!(
            "/v1/authorization/approvals/{}/decide",
            urlencoding::encode(id)
        );
        self.send_post_json(&path, request).await
    }

    pub async fn create_grant(
        &self,
        request: &CreateAuthorizationGrantRequest,
    ) -> Result<AuthorizationGrant, SdkError> {
        self.send_post_json("/v1/authorization/grants", request)
            .await
    }

    pub async fn list_grants(&self) -> Result<AuthorizationGrantListResponse, SdkError> {
        self.retry_loop("/v1/authorization/grants", || {
            self.send_get("/v1/authorization/grants")
        })
        .await
    }

    pub async fn revoke_grant(&self, id: &str) -> Result<AuthorizationGrant, SdkError> {
        let path = format!(
            "/v1/authorization/grants/{}/revoke",
            urlencoding::encode(id)
        );
        self.send_post_json(&path, &serde_json::json!({})).await
    }

    pub async fn complete_lease(
        &self,
        id: &str,
        request: &CompleteAuthorizationLeaseRequest,
    ) -> Result<tl_core::AuthorizationLease, SdkError> {
        let path = format!(
            "/v1/authorization/leases/{}/complete",
            urlencoding::encode(id)
        );
        self.send_post_json(&path, request).await
    }

    pub async fn get_authorization_receipt(
        &self,
        id: &str,
    ) -> Result<AuthorizationReceipt, SdkError> {
        let path = format!("/v1/authorization/receipts/{}", urlencoding::encode(id));
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    /// Wait for a reviewer grant, re-evaluate current policy, claim a
    /// one-attempt lease, execute the callback once, then consume the lease.
    pub async fn with_authorized_action<T, F, Fut>(
        &self,
        mut event: GuardEvent,
        timeout: Duration,
        execute: F,
    ) -> Result<AuthorizationResult<T>, SdkError>
    where
        F: FnOnce(serde_json::Value) -> Fut,
        Fut: Future<Output = Result<T, SdkError>>,
    {
        if event.action.invocation_id.is_none() {
            event.action.invocation_id = Some(uuid::Uuid::now_v7().to_string());
        }
        let mut decision = self.submit_event(&event).await?;
        if decision.effect == AuthorizationEffect::Permit {
            let lease = decision.lease.clone();
            let value = match execute(event.action.parameters.clone()).await {
                Ok(value) => value,
                Err(error) => {
                    if let Some(lease) = lease {
                        let _ = self
                            .complete_lease(
                                &lease.id,
                                &CompleteAuthorizationLeaseRequest {
                                    status: LeaseStatus::Canceled,
                                    outcome: serde_json::json!({ "success": false }),
                                },
                            )
                            .await;
                    }
                    return Err(error);
                }
            };
            if let Some(lease) = lease {
                self.complete_lease(
                    &lease.id,
                    &CompleteAuthorizationLeaseRequest {
                        status: LeaseStatus::Consumed,
                        outcome: serde_json::json!({ "success": true }),
                    },
                )
                .await?;
            }
            return Ok(AuthorizationResult {
                decision,
                value: Some(value),
            });
        }
        let Some(approval) = decision
            .approval
            .clone()
            .filter(|_| decision.effect == AuthorizationEffect::RequireApproval)
        else {
            return Ok(AuthorizationResult {
                decision,
                value: None,
            });
        };

        let attempt_id = uuid::Uuid::now_v7().to_string();
        let started = tokio::time::Instant::now();
        while started.elapsed() < timeout {
            let current = self.get_approval(&approval.id).await?;
            match current.status {
                ApprovalStatus::Approved => {
                    let Some(grant_id) = current.grant_id else {
                        return Ok(AuthorizationResult {
                            decision,
                            value: None,
                        });
                    };
                    let mut resumed = event.clone();
                    resumed.action.authorization = Some(tl_core::AuthorizationClaim {
                        grant_id,
                        attempt_id,
                    });
                    decision = self.submit_event(&resumed).await?;
                    let Some(lease) = decision
                        .lease
                        .clone()
                        .filter(|_| decision.effect == AuthorizationEffect::Permit)
                    else {
                        return Ok(AuthorizationResult {
                            decision,
                            value: None,
                        });
                    };
                    let value = match execute(event.action.parameters.clone()).await {
                        Ok(value) => value,
                        Err(error) => {
                            let _ = self
                                .complete_lease(
                                    &lease.id,
                                    &CompleteAuthorizationLeaseRequest {
                                        status: LeaseStatus::Canceled,
                                        outcome: serde_json::json!({ "success": false }),
                                    },
                                )
                                .await;
                            return Err(error);
                        }
                    };
                    self.complete_lease(
                        &lease.id,
                        &CompleteAuthorizationLeaseRequest {
                            status: LeaseStatus::Consumed,
                            outcome: serde_json::json!({ "success": true }),
                        },
                    )
                    .await?;
                    return Ok(AuthorizationResult {
                        decision,
                        value: Some(value),
                    });
                }
                ApprovalStatus::Pending => tokio::time::sleep(Duration::from_secs(1)).await,
                _ => {
                    return Ok(AuthorizationResult {
                        decision,
                        value: None,
                    })
                }
            }
        }
        Ok(AuthorizationResult {
            decision,
            value: None,
        })
    }
}
