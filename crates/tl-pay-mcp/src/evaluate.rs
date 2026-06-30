//! Pure pay decision: caps + hold band. No I/O, no clock — the caller fetches
//! the windowed spend totals and passes them in.

use tl_core::{PayPolicy, PayStatus};

/// First-match-wins evaluation mirroring the standalone gate's order (v1
/// scope): per-transaction → daily → monthly → hold band → allow. Merchant
/// and category lists are phase 2 and intentionally absent.
///
/// All amounts are minor units. Caps are inclusive (a value *equal* to a cap
/// passes); the hold band triggers at or above `hold_above_minor`. `None` on
/// any axis means no limit there.
pub fn evaluate(
    policy: &PayPolicy,
    spent_today_minor: i64,
    spent_month_minor: i64,
    amount_minor: i64,
) -> (PayStatus, String) {
    if let Some(cap) = policy.per_transaction_minor {
        if amount_minor > cap {
            return (
                PayStatus::Block,
                format!("over per-transaction cap ({cap})"),
            );
        }
    }
    if let Some(cap) = policy.daily_minor {
        // saturating: a spend total large enough to overflow i64 is far beyond
        // any cap anyway, so saturating to i64::MAX still blocks correctly.
        if spent_today_minor.saturating_add(amount_minor) > cap {
            return (PayStatus::Block, format!("over daily cap ({cap})"));
        }
    }
    if let Some(cap) = policy.monthly_minor {
        if spent_month_minor.saturating_add(amount_minor) > cap {
            return (PayStatus::Block, format!("over monthly cap ({cap})"));
        }
    }
    if let Some(threshold) = policy.hold_above_minor {
        if amount_minor >= threshold {
            return (
                PayStatus::Hold,
                format!("at or above hold threshold ({threshold})"),
            );
        }
    }
    (PayStatus::Allow, "within policy".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Policy from the launch-video demo: per-transaction 100.00, hold 50.00.
    fn demo_policy() -> PayPolicy {
        PayPolicy {
            owner: "me".into(),
            per_transaction_minor: Some(10_000),
            daily_minor: None,
            monthly_minor: None,
            hold_above_minor: Some(5_000),
        }
    }

    #[test]
    fn small_spend_within_policy_allows() {
        let (status, _) = evaluate(&demo_policy(), 0, 0, 4_000);
        assert_eq!(status, PayStatus::Allow);
    }

    #[test]
    fn over_per_transaction_cap_blocks() {
        let (status, reason) = evaluate(&demo_policy(), 0, 0, 80_000);
        assert_eq!(status, PayStatus::Block);
        assert!(reason.contains("per-transaction"));
    }

    #[test]
    fn borderline_spend_holds() {
        let (status, _) = evaluate(&demo_policy(), 0, 0, 6_000);
        assert_eq!(status, PayStatus::Hold);
    }

    #[test]
    fn amount_equal_to_cap_is_allowed_caps_are_inclusive() {
        let policy = PayPolicy {
            owner: "me".into(),
            per_transaction_minor: Some(10_000),
            daily_minor: None,
            monthly_minor: None,
            hold_above_minor: None,
        };
        let (status, _) = evaluate(&policy, 0, 0, 10_000);
        assert_eq!(status, PayStatus::Allow);
    }

    #[test]
    fn amount_at_hold_threshold_holds() {
        let policy = PayPolicy {
            owner: "me".into(),
            per_transaction_minor: None,
            daily_minor: None,
            monthly_minor: None,
            hold_above_minor: Some(5_000),
        };
        let (status, _) = evaluate(&policy, 0, 0, 5_000);
        assert_eq!(status, PayStatus::Hold);
    }

    #[test]
    fn daily_cap_counts_prior_spend() {
        let policy = PayPolicy {
            owner: "me".into(),
            per_transaction_minor: None,
            daily_minor: Some(10_000),
            monthly_minor: None,
            hold_above_minor: None,
        };
        // 8_000 already spent today, +3_000 = 11_000 > 10_000 → block.
        let (status, reason) = evaluate(&policy, 8_000, 0, 3_000);
        assert_eq!(status, PayStatus::Block);
        assert!(reason.contains("daily"));
        // +2_000 = 10_000, exactly the cap → allowed (inclusive).
        let (status, _) = evaluate(&policy, 8_000, 0, 2_000);
        assert_eq!(status, PayStatus::Allow);
    }

    #[test]
    fn monthly_cap_counts_prior_spend() {
        let policy = PayPolicy {
            owner: "me".into(),
            per_transaction_minor: None,
            daily_minor: None,
            monthly_minor: Some(50_000),
            hold_above_minor: None,
        };
        let (status, reason) = evaluate(&policy, 0, 49_000, 2_000);
        assert_eq!(status, PayStatus::Block);
        assert!(reason.contains("monthly"));
    }

    #[test]
    fn no_caps_allows_everything() {
        let policy = PayPolicy {
            owner: "me".into(),
            per_transaction_minor: None,
            daily_minor: None,
            monthly_minor: None,
            hold_above_minor: None,
        };
        let (status, _) = evaluate(&policy, 1_000_000, 1_000_000, 999_999);
        assert_eq!(status, PayStatus::Allow);
    }
}
