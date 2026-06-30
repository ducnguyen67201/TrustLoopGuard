//! Durable persistence for the payment gate: per-owner caps (`pay_policy`)
//! and the `pay_decision` log (audit + hold registry + the windowed totals
//! that enforce daily/monthly caps).

use chrono::{DateTime, Datelike, TimeZone, Utc};
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::RunQueryDsl;
use tl_core::PayPolicy;
use uuid::Uuid;

use crate::models::{NewPayDecision, NewPayPolicy, PayDecisionRecord, PayPolicyRecord};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{pay_decision, pay_policy};
use crate::StorageError;

/// Materialised view of one `pay_decision` row — what the audit export returns.
#[derive(Debug, Clone)]
pub struct PayDecisionRow {
    pub decision_id: String,
    pub owner: String,
    pub amount_minor: i64,
    pub merchant: String,
    pub category: String,
    pub status: String,
    pub resolution: Option<bool>,
    pub created_at: DateTime<Utc>,
}

/// Per-owner spend-cap policy storage.
#[derive(Clone)]
pub struct PayPolicyRepo {
    pool: DbPool,
}

impl PayPolicyRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Insert or replace the caps for `(workspace_id, policy.owner)`.
    pub async fn upsert(&self, workspace_id: &str, policy: &PayPolicy) -> Result<(), StorageError> {
        let row = NewPayPolicy {
            workspace_id: workspace_id.to_string(),
            owner: policy.owner.clone(),
            per_transaction_minor: policy.per_transaction_minor,
            daily_minor: policy.daily_minor,
            monthly_minor: policy.monthly_minor,
            hold_above_minor: policy.hold_above_minor,
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(pay_policy::table)
            .values(&row)
            .on_conflict((pay_policy::workspace_id, pay_policy::owner))
            .do_update()
            .set((
                pay_policy::per_transaction_minor.eq(excluded(pay_policy::per_transaction_minor)),
                pay_policy::daily_minor.eq(excluded(pay_policy::daily_minor)),
                pay_policy::monthly_minor.eq(excluded(pay_policy::monthly_minor)),
                pay_policy::hold_above_minor.eq(excluded(pay_policy::hold_above_minor)),
                pay_policy::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("pay_policy upsert: {e}")))?;
        Ok(())
    }

    /// Fetch the caps for an owner, or `None` if none set (the gate then fails closed).
    pub async fn get(
        &self,
        workspace_id: &str,
        owner: &str,
    ) -> Result<Option<PayPolicy>, StorageError> {
        let mut conn = self.connection().await?;
        let row = pay_policy::table
            .filter(pay_policy::workspace_id.eq(workspace_id))
            .filter(pay_policy::owner.eq(owner))
            .select(PayPolicyRecord::as_select())
            .first::<PayPolicyRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("pay_policy get: {e}")))?;
        Ok(row.map(|r| PayPolicy {
            owner: r.owner,
            per_transaction_minor: r.per_transaction_minor,
            daily_minor: r.daily_minor,
            monthly_minor: r.monthly_minor,
            hold_above_minor: r.hold_above_minor,
        }))
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

/// Append-only decision log: every allow/block/hold, the hold registry, and
/// the source of the windowed spend totals.
#[derive(Clone)]
pub struct PayDecisionRepo {
    pool: DbPool,
}

impl PayDecisionRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Record one decision. `status` is `"allow"`, `"block"`, or `"hold"`;
    /// `resolution` starts NULL and is set later by [`Self::resolve_hold`].
    pub async fn record(
        &self,
        workspace_id: &str,
        owner: &str,
        decision_id: &str,
        amount_minor: i64,
        merchant: &str,
        category: &str,
        status: &str,
    ) -> Result<(), StorageError> {
        let row = NewPayDecision {
            id: Uuid::now_v7(),
            workspace_id: workspace_id.to_string(),
            owner: owner.to_string(),
            decision_id: decision_id.to_string(),
            amount_minor,
            merchant: merchant.to_string(),
            category: category.to_string(),
            status: status.to_string(),
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(pay_decision::table)
            .values(&row)
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("pay_decision insert: {e}")))?;
        Ok(())
    }

    /// Spend counted toward caps since `since`: allowed spends plus approved
    /// holds. Blocks and unresolved/denied holds do not count.
    ///
    // ponytail: folds in Rust rather than SQL `sum()` (which returns Numeric,
    // dragging in bigdecimal). A per-owner daily/monthly window is small; move
    // to a SQL aggregate if an owner's volume ever makes this scan hurt.
    pub async fn sum_counted_since(
        &self,
        workspace_id: &str,
        owner: &str,
        since: DateTime<Utc>,
    ) -> Result<i64, StorageError> {
        let mut conn = self.connection().await?;
        let amounts: Vec<i64> = pay_decision::table
            .filter(pay_decision::workspace_id.eq(workspace_id))
            .filter(pay_decision::owner.eq(owner))
            .filter(pay_decision::created_at.ge(since))
            .filter(
                pay_decision::status.eq("allow").or(pay_decision::status
                    .eq("hold")
                    .and(pay_decision::resolution.eq(true))),
            )
            .select(pay_decision::amount_minor)
            .load::<i64>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("pay_decision sum: {e}")))?;
        Ok(amounts.into_iter().fold(0i64, i64::saturating_add))
    }

    /// Spend counted today (since 00:00 UTC).
    pub async fn sum_today(&self, workspace_id: &str, owner: &str) -> Result<i64, StorageError> {
        self.sum_counted_since(workspace_id, owner, start_of_day_utc())
            .await
    }

    /// Spend counted this month (since the 1st at 00:00 UTC).
    pub async fn sum_this_month(
        &self,
        workspace_id: &str,
        owner: &str,
    ) -> Result<i64, StorageError> {
        self.sum_counted_since(workspace_id, owner, start_of_month_utc())
            .await
    }

    /// Approve or deny a pending hold. Returns `true` if a matching hold was
    /// found and updated.
    pub async fn resolve_hold(
        &self,
        workspace_id: &str,
        decision_id: &str,
        approve: bool,
    ) -> Result<bool, StorageError> {
        let mut conn = self.connection().await?;
        let updated = diesel::update(
            pay_decision::table
                .filter(pay_decision::workspace_id.eq(workspace_id))
                .filter(pay_decision::decision_id.eq(decision_id))
                .filter(pay_decision::status.eq("hold")),
        )
        .set(pay_decision::resolution.eq(approve))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("resolve_hold: {e}")))?;
        Ok(updated > 0)
    }

    /// Every decision for an owner, oldest first (the audit export).
    pub async fn list_for_owner(
        &self,
        workspace_id: &str,
        owner: &str,
    ) -> Result<Vec<PayDecisionRow>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = pay_decision::table
            .filter(pay_decision::workspace_id.eq(workspace_id))
            .filter(pay_decision::owner.eq(owner))
            .select(PayDecisionRecord::as_select())
            .order(pay_decision::created_at.asc())
            .load::<PayDecisionRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("pay_decision list: {e}")))?;
        Ok(rows
            .into_iter()
            .map(|r| PayDecisionRow {
                decision_id: r.decision_id,
                owner: r.owner,
                amount_minor: r.amount_minor,
                merchant: r.merchant,
                category: r.category,
                status: r.status,
                resolution: r.resolution,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

fn start_of_day_utc() -> DateTime<Utc> {
    let today = Utc::now().date_naive();
    Utc.from_utc_datetime(&today.and_hms_opt(0, 0, 0).expect("midnight is valid"))
}

fn start_of_month_utc() -> DateTime<Utc> {
    let first = Utc::now().date_naive().with_day(1).expect("day 1 is valid");
    Utc.from_utc_datetime(&first.and_hms_opt(0, 0, 0).expect("midnight is valid"))
}
