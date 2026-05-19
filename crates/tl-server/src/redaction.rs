use std::collections::BTreeMap;
use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value;
use tl_core::{
    CheckRequest, DataHandlingMode, RedactedEntity, RedactionInfo, RedactionMode, RedactionStatus,
};

#[cfg(test)]
mod tests;

const CONTEXT_PASSTHROUGH_KEYS: &[&str] = &[
    "workflow_step",
    "document_type",
    "confidence_bucket",
    "pii_types",
];

pub fn requires_redaction_rejection(mode: DataHandlingMode, req: &CheckRequest) -> bool {
    if mode != DataHandlingMode::RedactedOnly {
        return false;
    }
    if should_apply_server_redaction(req) {
        return false;
    }
    // Re-scan even when the client claims `status = Applied`. Trusting the
    // client-asserted status would let a misconfigured caller bypass
    // `redacted_only` by flipping the flag while still shipping raw values.
    contains_raw_sensitive_data(req)
}

pub fn should_apply_server_redaction(req: &CheckRequest) -> bool {
    req.redaction
        .as_ref()
        .is_some_and(|info| info.mode == RedactionMode::Server)
}

pub fn apply_server_redaction(req: &mut CheckRequest) {
    let mut redactor = Redactor::default();

    let input = redactor.redact_text(&req.input);
    let proposed_output = redactor.redact_text(&req.proposed_output);
    let context = redactor.redact_context(&req.context, None);
    let input_redacted = input != req.input;
    let proposed_output_redacted = proposed_output != req.proposed_output;
    let context_redacted = context != req.context;

    req.input = input;
    req.proposed_output = proposed_output;
    req.context = context;

    if let Some(run_event) = req.run_event.as_mut() {
        if let Some(input_summary) = run_event.input_summary.as_ref() {
            run_event.input_summary = Some(redactor.redact_text(input_summary));
        }
        if let Some(output_summary) = run_event.output_summary.as_ref() {
            run_event.output_summary = Some(redactor.redact_text(output_summary));
        }
    }

    req.redaction = Some(RedactionInfo {
        mode: RedactionMode::Server,
        status: RedactionStatus::Applied,
        entities: redactor.entities(),
        input_redacted,
        proposed_output_redacted,
        context_redacted,
    });
}

fn contains_raw_sensitive_data(req: &CheckRequest) -> bool {
    text_contains_sensitive_data(&req.input)
        || text_contains_sensitive_data(&req.proposed_output)
        || context_contains_sensitive_data(&req.context, None)
}

fn context_contains_sensitive_data(value: &Value, key: Option<&str>) -> bool {
    if key.is_some_and(is_passthrough_key) {
        return false;
    }
    match value {
        Value::String(text) => text_contains_sensitive_data(text),
        Value::Array(values) => values
            .iter()
            .any(|value| context_contains_sensitive_data(value, key)),
        Value::Object(map) => map
            .iter()
            .any(|(key, value)| context_contains_sensitive_data(value, Some(key))),
        _ => false,
    }
}

fn text_contains_sensitive_data(text: &str) -> bool {
    email_regex().is_match(text) || sin_regex().is_match(text) || income_regex().is_match(text)
}

#[derive(Default)]
struct Redactor {
    tokens_by_raw: BTreeMap<(String, String), String>,
    counts_by_token: BTreeMap<(String, String), u32>,
    next_by_type: BTreeMap<String, u32>,
}

impl Redactor {
    fn redact_context(&mut self, value: &Value, key: Option<&str>) -> Value {
        if key.is_some_and(is_passthrough_key) {
            return value.clone();
        }
        match value {
            Value::String(text) => Value::String(self.redact_text(text)),
            Value::Array(values) => Value::Array(
                values
                    .iter()
                    .map(|value| self.redact_context(value, key))
                    .collect(),
            ),
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(key, value)| (key.clone(), self.redact_context(value, Some(key))))
                    .collect(),
            ),
            _ => value.clone(),
        }
    }

    fn redact_text(&mut self, text: &str) -> String {
        // `PERSON_NAME` is intentionally not redacted server-side: the
        // current regex (`\b[A-Z][a-z]+ [A-Z][a-z]+\b`) matches any pair
        // of capitalized words and would mangle legitimate proper-noun
        // text the engine and Tier-3 judges need to evaluate. SDK-local
        // redaction keeps it because the customer opts in per request.
        let text = self.redact_with(email_regex(), "EMAIL", text);
        let text = self.redact_with(sin_regex(), "SIN", &text);
        self.redact_with(income_regex(), "INCOME_AMOUNT", &text)
    }

    fn redact_with(&mut self, regex: &Regex, entity_type: &str, text: &str) -> String {
        regex
            .replace_all(text, |captures: &regex::Captures<'_>| {
                self.token_for(entity_type, captures.get(0).map_or("", |m| m.as_str()))
            })
            .into_owned()
    }

    fn token_for(&mut self, entity_type: &str, raw: &str) -> String {
        let key = (entity_type.to_string(), raw.to_string());
        let token = if let Some(token) = self.tokens_by_raw.get(&key) {
            token.clone()
        } else {
            let next = self
                .next_by_type
                .entry(entity_type.to_string())
                .or_insert(1);
            let token = format!("[{entity_type}_{next}]");
            *next += 1;
            self.tokens_by_raw.insert(key.clone(), token.clone());
            token
        };
        *self
            .counts_by_token
            .entry((entity_type.to_string(), token.clone()))
            .or_insert(0) += 1;
        token
    }

    fn entities(self) -> Vec<RedactedEntity> {
        self.counts_by_token
            .into_iter()
            .map(|((entity_type, token), count)| RedactedEntity {
                entity_type,
                token,
                count,
            })
            .collect()
    }
}

fn is_passthrough_key(key: &str) -> bool {
    CONTEXT_PASSTHROUGH_KEYS.contains(&key)
}

fn email_regex() -> &'static Regex {
    static EMAIL: OnceLock<Regex> = OnceLock::new();
    EMAIL.get_or_init(|| Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap())
}

fn sin_regex() -> &'static Regex {
    static SIN: OnceLock<Regex> = OnceLock::new();
    SIN.get_or_init(|| Regex::new(r"\b\d{3}[- ]?\d{3}[- ]?\d{3}\b").unwrap())
}

fn income_regex() -> &'static Regex {
    static INCOME: OnceLock<Regex> = OnceLock::new();
    INCOME.get_or_init(|| Regex::new(r"\$\d{1,3}(?:,\d{3})*(?:\.\d{2})?\b").unwrap())
}
