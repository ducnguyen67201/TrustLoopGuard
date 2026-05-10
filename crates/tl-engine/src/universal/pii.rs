//! PII detectors. All run against the agent's *proposed output* — the
//! goal is preventing leaks from the agent to the customer. Inputs from
//! the customer (which often legitimately contain PII) are not screened
//! here.
//!
//! Detectors are deliberately conservative on false positives:
//! - SSN matches the format only; we don't try to validate area numbers
//!   (the regex crate doesn't support lookarounds, and a benign 9-digit
//!   sequence in 3-2-4 form is rare in business writing).
//! - Phone matches the common North American format with optional
//!   country code, parentheses, and separators.
//! - Credit card requires a Luhn check after the format match — naked
//!   regexes here have a brutal false-positive rate.
//! - IPv4 octets are bounded 0-255 in the regex itself.

use std::sync::OnceLock;

use regex::Regex;
use tl_core::{Severity, Verdict};

use super::UniversalHit;

fn email() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}")
            .expect("compile email regex")
    })
}

fn us_ssn() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("compile ssn regex"))
}

fn us_phone() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]\d{3}[-.\s]\d{4}")
            .expect("compile phone regex")
    })
}

fn ipv4() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(
            r"\b(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\b",
        )
        .expect("compile ipv4 regex")
    })
}

fn cc_candidate() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // 13-19 digit candidates with optional spaces/dashes between.
        Regex::new(r"\b(?:\d[ -]?){12,18}\d\b").expect("compile cc regex")
    })
}

pub fn detect(text: &str) -> Vec<UniversalHit> {
    let mut hits = vec![];

    if email().is_match(text) {
        hits.push(UniversalHit {
            id: "universal:pii.email".into(),
            severity: Severity::Medium,
            message: "draft contains an email address".into(),
            verdict: Verdict::Block,
        });
    }
    if us_ssn().is_match(text) {
        hits.push(UniversalHit {
            id: "universal:pii.ssn".into(),
            severity: Severity::High,
            message: "draft contains a US SSN-shaped sequence".into(),
            verdict: Verdict::Block,
        });
    }
    if us_phone().is_match(text) {
        hits.push(UniversalHit {
            id: "universal:pii.phone".into(),
            severity: Severity::Medium,
            message: "draft contains a US phone number".into(),
            verdict: Verdict::Block,
        });
    }
    if ipv4().is_match(text) {
        hits.push(UniversalHit {
            id: "universal:pii.ipv4".into(),
            severity: Severity::Low,
            message: "draft contains an IPv4 address".into(),
            verdict: Verdict::Block,
        });
    }
    if let Some(len) = first_valid_credit_card(text) {
        hits.push(UniversalHit {
            id: "universal:pii.credit_card".into(),
            severity: Severity::High,
            message: format!("draft contains a Luhn-valid {len}-digit number"),
            verdict: Verdict::Block,
        });
    }

    hits
}

fn first_valid_credit_card(text: &str) -> Option<usize> {
    for m in cc_candidate().find_iter(text) {
        let digits: String = m.as_str().chars().filter(char::is_ascii_digit).collect();
        if digits.len() < 13 || digits.len() > 19 {
            continue;
        }
        if luhn_valid(&digits) {
            return Some(digits.len());
        }
    }
    None
}

fn luhn_valid(digits: &str) -> bool {
    let mut sum: u32 = 0;
    let mut alt = false;
    for ch in digits.chars().rev() {
        let mut d = match ch.to_digit(10) {
            Some(d) => d,
            None => return false,
        };
        if alt {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
        alt = !alt;
    }
    sum % 10 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(text: &str) -> Vec<String> {
        detect(text).into_iter().map(|h| h.id).collect()
    }

    #[test]
    fn detects_email() {
        assert!(ids("contact me at alice@example.com").contains(&"universal:pii.email".into()));
    }

    #[test]
    fn ignores_text_without_email() {
        assert!(detect("plain greeting, no addresses").is_empty());
    }

    #[test]
    fn detects_ssn_format() {
        assert!(ids("ssn 123-45-6789 on file").contains(&"universal:pii.ssn".into()));
    }

    #[test]
    fn ignores_partial_ssn() {
        // 4-2-4 isn't ssn-shaped
        assert!(!ids("number 1234-56-7890").contains(&"universal:pii.ssn".into()));
    }

    #[test]
    fn detects_phone_with_dashes() {
        assert!(ids("call 415-555-1212").contains(&"universal:pii.phone".into()));
    }

    #[test]
    fn detects_phone_with_parens() {
        assert!(ids("call (415) 555-1212").contains(&"universal:pii.phone".into()));
    }

    #[test]
    fn detects_phone_with_country_code() {
        assert!(ids("dial +1 415-555-1212 today").contains(&"universal:pii.phone".into()));
    }

    #[test]
    fn detects_ipv4() {
        assert!(ids("server 192.168.1.42 is down").contains(&"universal:pii.ipv4".into()));
    }

    #[test]
    fn rejects_invalid_ipv4_octet() {
        assert!(!ids("port 1.2.3.999 is bad").contains(&"universal:pii.ipv4".into()));
    }

    #[test]
    fn detects_luhn_valid_credit_card() {
        // Test card number — passes Luhn.
        assert!(ids("card: 4111111111111111").contains(&"universal:pii.credit_card".into()));
    }

    #[test]
    fn detects_luhn_valid_credit_card_with_spaces() {
        assert!(ids("card 4111 1111 1111 1111").contains(&"universal:pii.credit_card".into()));
    }

    #[test]
    fn rejects_luhn_invalid_candidate() {
        // Same length, last digit wrong → fails Luhn → not flagged.
        assert!(!ids("ref 4111111111111112").contains(&"universal:pii.credit_card".into()));
    }

    #[test]
    fn rejects_short_digit_runs() {
        // 12 digits — below CC range → no flag even if Luhn passes.
        assert!(!ids("invoice 411111111111").contains(&"universal:pii.credit_card".into()));
    }

    #[test]
    fn returns_multiple_hits() {
        let hits = detect("alice@example.com ssn 123-45-6789 ip 10.0.0.1");
        let ids: Vec<_> = hits.iter().map(|h| h.id.as_str()).collect();
        assert!(ids.contains(&"universal:pii.email"));
        assert!(ids.contains(&"universal:pii.ssn"));
        assert!(ids.contains(&"universal:pii.ipv4"));
    }
}
