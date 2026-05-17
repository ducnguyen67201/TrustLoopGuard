//! Cache key derivation. We hash the inputs that should produce the
//! same `Decision` if the request repeats.
//!
//! Included in the key:
//! - `domain` (defaults to `customer_support` so older clients without
//!   the field hash to the same bucket)
//! - `agent_id`
//! - `input`
//! - `proposed_output`
//! - the full `context` JSON (canonicalised)
//! - the request-level `policies` filter
//! - a caller-supplied policy scope fingerprint
//!
//! Excluded:
//! - `trace_id` — rotates per call, never identical across requests
//!
//! Hash function is BLAKE3 — fast, fixed-size, suitable for cache
//! keys. Collisions are not security-relevant; we're not authenticating.

use serde_json::Value;
use tl_core::CheckRequest;

const DEFAULT_DOMAIN: &str = "customer_support";

pub fn for_check_request(req: &CheckRequest) -> String {
    for_check_request_with_policy_scope(req, "")
}

pub fn for_check_request_with_policy_scope(req: &CheckRequest, policy_scope: &str) -> String {
    let domain = req.domain.as_deref().unwrap_or(DEFAULT_DOMAIN);
    let context = canonical_json(&req.context);
    let mut policies = req.policies.clone();
    policies.sort();

    // The format is intentionally `\u{1f}` (ASCII Unit Separator) between
    // segments so the digest can't be ambiguated by a tenant who happens
    // to embed our delimiter in their text.
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(req.agent_id.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(req.input.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(req.proposed_output.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(context.as_bytes());
    hasher.update(b"\x1f");
    for policy in policies {
        hasher.update(policy.as_bytes());
        hasher.update(b"\x1e");
    }
    hasher.update(b"\x1f");
    hasher.update(policy_scope.as_bytes());
    hasher.finalize().to_hex().to_string()
}

/// Render `value` to a deterministic JSON string. `serde_json` doesn't
/// guarantee object key order across runs, so we sort keys ourselves at
/// every level.
fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => {
            // serde_json::to_string handles all the escaping rules.
            let escaped = serde_json::to_string(s).unwrap_or_else(|_| String::from("\"\""));
            out.push_str(&escaped);
        }
        Value::Array(arr) => {
            out.push('[');
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(v, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let escaped = serde_json::to_string(k).unwrap_or_else(|_| String::from("\"\""));
                out.push_str(&escaped);
                out.push(':');
                write_canonical(&map[*k], out);
            }
            out.push('}');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tl_core::Channel;

    fn req(input: &str, output: &str, ctx: Value) -> CheckRequest {
        CheckRequest {
            workspace_id: None,
            run_id: None,
            run_event_id: None,
            run_event: None,
            agent_id: "agent-x".into(),
            channel: Channel::Chat,
            input: input.into(),
            proposed_output: output.into(),
            domain: None,
            policies: vec![],
            context: ctx,
            trace_id: None,
        }
    }

    #[test]
    fn identical_requests_hash_equal() {
        let a = req("hello", "hi", json!({"docs":["a","b"]}));
        let b = req("hello", "hi", json!({"docs":["a","b"]}));
        assert_eq!(for_check_request(&a), for_check_request(&b));
    }

    #[test]
    fn different_drafts_hash_differently() {
        let a = req("hello", "hi there", Value::Null);
        let b = req("hello", "hi friend", Value::Null);
        assert_ne!(for_check_request(&a), for_check_request(&b));
    }

    #[test]
    fn trace_id_does_not_affect_key() {
        let mut a = req("hello", "hi", Value::Null);
        let mut b = a.clone();
        a.trace_id = Some("t-1".into());
        b.trace_id = Some("t-2".into());
        assert_eq!(for_check_request(&a), for_check_request(&b));
    }

    #[test]
    fn context_object_key_order_does_not_affect_key() {
        // Object key order in the source JSON differs but canonical form
        // is the same → cache should hit.
        let a = req("hi", "ho", json!({"docs":["x"], "customer":"alice"}));
        let b = req("hi", "ho", json!({"customer":"alice", "docs":["x"]}));
        assert_eq!(for_check_request(&a), for_check_request(&b));
    }

    #[test]
    fn nested_objects_canonicalise_recursively() {
        let a = req(
            "hi",
            "ho",
            json!({"a": {"x":1, "y":2}, "b": [{"q":3, "p":4}]}),
        );
        let b = req(
            "hi",
            "ho",
            json!({"b": [{"p":4, "q":3}], "a": {"y":2, "x":1}}),
        );
        assert_eq!(for_check_request(&a), for_check_request(&b));
    }

    #[test]
    fn missing_domain_is_treated_as_default() {
        let mut a = req("hi", "ho", Value::Null);
        let mut b = a.clone();
        a.domain = None;
        b.domain = Some("customer_support".into());
        assert_eq!(for_check_request(&a), for_check_request(&b));
    }

    #[test]
    fn different_domain_changes_key() {
        let mut a = req("hi", "ho", Value::Null);
        let mut b = a.clone();
        a.domain = Some("customer_support".into());
        b.domain = Some("voice_agent".into());
        assert_ne!(for_check_request(&a), for_check_request(&b));
    }

    #[test]
    fn policy_filter_changes_key() {
        let a = req("hi", "ho", Value::Null);
        let mut b = a.clone();
        b.policies = vec!["refund-guarantee".into()];
        assert_ne!(for_check_request(&a), for_check_request(&b));
    }

    #[test]
    fn policy_scope_changes_key() {
        let req = req("hi", "ho", Value::Null);
        assert_ne!(
            for_check_request_with_policy_scope(&req, "bundle-a"),
            for_check_request_with_policy_scope(&req, "bundle-b")
        );
    }
}
