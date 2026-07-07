//! Pure threshold math. Integer-only — no floats anywhere near money.

use tl_core::BudgetAlertThresholdType;

/// Has spend crossed the configured threshold of the window's cap?
///
/// - `percent`: fires when `spent * 100 >= cap * threshold`. The
///   integer cross-multiplication means fractional boundaries round
///   toward firing on the next whole unit (cap 3 at 80% ⇒ threshold
///   is 2.4 ⇒ fires at spent 3).
/// - `absolute`: fires when the remaining budget (`cap - spent`)
///   drops to `threshold_value` or below.
///
/// A cap of zero (or less) never fires: the hard limit already blocks
/// everything, so there is nothing to warn about.
pub fn crossed(
    threshold_type: BudgetAlertThresholdType,
    threshold_value: i64,
    cap_minor: i64,
    spent_minor: i64,
) -> bool {
    if cap_minor <= 0 {
        return false;
    }
    match threshold_type {
        // i128 so `spent * 100` cannot overflow near i64::MAX.
        BudgetAlertThresholdType::Percent => {
            i128::from(spent_minor) * 100 >= i128::from(cap_minor) * i128::from(threshold_value)
        }
        BudgetAlertThresholdType::Absolute => {
            i128::from(cap_minor) - i128::from(spent_minor) <= i128::from(threshold_value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use BudgetAlertThresholdType::{Absolute, Percent};

    #[test]
    fn percent_fires_exactly_at_the_boundary() {
        // cap 5000, threshold 80% → boundary at 4000.
        assert!(crossed(Percent, 80, 5000, 4000));
        assert!(crossed(Percent, 80, 5000, 4001));
        assert!(crossed(Percent, 80, 5000, 5000));
    }

    #[test]
    fn percent_below_the_boundary_does_not_fire() {
        assert!(!crossed(Percent, 80, 5000, 3999));
        assert!(!crossed(Percent, 80, 5000, 0));
    }

    #[test]
    fn cap_zero_never_fires() {
        assert!(!crossed(Percent, 80, 0, 0));
        assert!(!crossed(Percent, 80, 0, 100));
        assert!(!crossed(Absolute, 1000, 0, 0));
        assert!(!crossed(Absolute, 1000, -1, 0));
    }

    #[test]
    fn percent_integer_math_rounds_toward_the_next_whole_unit() {
        // cap 3 at 80%: the fractional boundary is 2.4, so the alert
        // fires at spent 3, not spent 2.
        assert!(!crossed(Percent, 80, 3, 2));
        assert!(crossed(Percent, 80, 3, 3));
    }

    #[test]
    fn percent_does_not_overflow_near_i64_max() {
        assert!(crossed(Percent, 80, i64::MAX, i64::MAX));
        assert!(!crossed(Percent, 100, i64::MAX, i64::MAX - 1));
    }

    #[test]
    fn absolute_fires_when_remaining_reaches_the_threshold() {
        // cap 5000, threshold 1000: fires once remaining <= 1000.
        assert!(crossed(Absolute, 1000, 5000, 4000));
        assert!(crossed(Absolute, 1000, 5000, 4200));
        assert!(!crossed(Absolute, 1000, 5000, 3999));
    }

    #[test]
    fn absolute_fires_past_the_cap() {
        assert!(crossed(Absolute, 0, 5000, 5000));
        assert!(crossed(Absolute, 0, 5000, 6000));
        assert!(!crossed(Absolute, 0, 5000, 4999));
    }
}
