use serde_json::json;
use tl_core::{PolicyDraft, PolicyMatchType};
use tl_llm::JsonSchema;
use tl_policy::policy_ast::WhenClause;
use tl_policy::{MatchClause, Matcher, Policy};

/// System instructions prepended to every policy-draft prompt. Kept here
/// rather than in a file so the OpenAPI surface fully describes the
/// behavior the server exposes.
pub(super) const POLICY_DRAFT_SYSTEM_PROMPT: &str = concat!(
    "You write Featherlane AI guardrail policies. Given a short natural-language ",
    "description, return a single policy draft as JSON matching the response schema.\n\n",
    "Rules:\n",
    "- `id` is kebab-case (lowercase letters, digits, hyphens only).\n",
    "- Prefer `match_type` = `literal` for specific phrases; `regex` for patterns; ",
    "`semantic` for meaning-based matches that must survive paraphrase or encoding ",
    "(the `match_value` is then a short natural-language description of what to catch).\n",
    "- Default `action` is `deny`. Use `transform` only when a clear safe replacement exists; ",
    "in that case set `rewrite` to the replacement text. Otherwise leave `rewrite` null.\n",
    "- Use `require_approval` only for explicit human authority; use `defer` for unresolved evidence.\n",
);

/// System instructions for `POST /v1/agents/{id}/guardrails:generate`.
/// The model receives the customer's agent system prompt and must emit a
/// **set** of guardrail drafts tailored to that agent — not a single one.
pub(super) const POLICY_SET_DRAFT_SYSTEM_PROMPT: &str = concat!(
    "You write Featherlane AI guardrail policy sets for a single agent.\n",
    "Given the customer's agent system prompt, derive 3–8 policies that protect ",
    "that specific agent from common failure modes. Return a JSON array matching ",
    "the response schema.\n\n",
    "Required coverage (at minimum, when applicable to the agent):\n",
    "- Customer-info / PII leakage (emails, phone numbers, addresses, payment data).\n",
    "- Scope discipline: refuse off-topic requests outside the agent's stated role.\n",
    "- Tone discipline: avoid forbidden tones implied by the prompt.\n",
    "- Hallucinated guarantees: no promises about refunds, SLAs, medical/legal ",
    "outcomes, prices, or anything not explicitly authorized by the prompt.\n",
    "- Role-specific risks the prompt implies (e.g. a baking agent: no medical/",
    "dietary safety claims; a finance agent: no investment advice; a support agent: ",
    "no unauthorized refund commitments).\n\n",
    "Rules for each policy in the array:\n",
    "- `id` is kebab-case (lowercase letters, digits, hyphens). Distinct across the array.\n",
    "- Prefer `match_type` = `literal` for specific phrases; `regex` for patterns; ",
    "`semantic` for meaning-based matches that must survive paraphrase or encoding ",
    "(the `match_value` is then a short natural-language description of what to catch).\n",
    "- Default `action` is `deny`. Use `transform` only when a clear safe replacement ",
    "exists; in that case set `rewrite` to the replacement text. Otherwise leave ",
    "`rewrite` null.\n",
    "- Use `require_approval` for explicit human authority and `defer` for unresolved evidence.\n",
    "- Do not emit near-duplicates: every entry should cover a distinct risk.\n",
);

/// Reusable policy-draft item schema. Shared by the single-draft endpoint
/// and the multi-draft array schema below so the two surfaces can't drift.
#[cfg(test)]
pub(super) fn policy_draft_item_schema() -> serde_json::Value {
    shared_policy_draft_item_schema()
}

fn shared_policy_draft_item_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "id", "description", "match_type", "match_value",
            "action", "severity", "rewrite",
        ],
        "properties": {
            "id": { "type": "string", "description": "kebab-case identifier" },
            "description": { "type": "string" },
            "match_type": { "type": "string", "enum": ["literal", "regex", "semantic"] },
            "match_value": { "type": "string" },
            "action": { "type": "string", "enum": ["deny", "transform", "require_approval", "defer"] },
            "severity": {
                "type": "string",
                "enum": ["low", "medium", "high", "critical"],
            },
            "rewrite": {
                "type": ["string", "null"],
                "description": "safe replacement when action is rewrite, else null",
            },
        },
    })
}

/// Strict JSON schema for the multi-policy draft endpoint. OpenAI's
/// strict mode requires a top-level object, so we wrap the array in
/// `{ "policies": [...] }` rather than returning a bare array.
pub(super) fn policy_set_draft_json_schema() -> JsonSchema {
    JsonSchema {
        name: "policy_set_draft".to_string(),
        schema: json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["policies"],
            "properties": {
                "policies": {
                    "type": "array",
                    "minItems": 3,
                    "maxItems": 8,
                    "items": shared_policy_draft_item_schema(),
                },
            },
        }),
    }
}

pub(super) fn policy_draft_json_schema() -> JsonSchema {
    JsonSchema {
        name: "policy_draft".to_string(),
        schema: shared_policy_draft_item_schema(),
    }
}

/// Pull the array out of `{ "policies": [...] }` (OpenAI strict-mode
/// requires the wrapper object) and decode each item into a typed draft.
pub(super) fn parse_policy_set(mut raw: serde_json::Value) -> Result<Vec<PolicyDraft>, String> {
    let arr = raw
        .get_mut("policies")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| "model response missing `policies` array".to_string())?;
    let mut drafts = Vec::with_capacity(arr.len());
    for (idx, mut item) in std::mem::take(arr).into_iter().enumerate() {
        // Strict-mode null -> absent so it lands as Option::None.
        if item.get("rewrite") == Some(&serde_json::Value::Null) {
            if let Some(obj) = item.as_object_mut() {
                obj.remove("rewrite");
            }
        }
        let draft: PolicyDraft = serde_json::from_value(item)
            .map_err(|e| format!("policies[{idx}] is not a valid policy draft: {e}"))?;
        drafts.push(draft);
    }
    Ok(drafts)
}

/// Convert an LLM-emitted draft into a stored `Policy` scoped to a
/// specific agent. `owner_agent_id` drives cascade delete; the
/// `when.agents` list makes the engine evaluate the policy only for
/// requests targeting that agent.
pub(super) fn policy_from_draft(draft: &PolicyDraft, agent_id: &str) -> Policy {
    let matcher = match draft.match_type {
        PolicyMatchType::Literal => Matcher::Literal(draft.match_value.clone()),
        PolicyMatchType::Regex => Matcher::Regex(draft.match_value.clone()),
        PolicyMatchType::Semantic => Matcher::Semantic(draft.match_value.clone()),
    };
    let action = draft.action;
    Policy {
        id: draft.id.clone(),
        description: Some(draft.description.clone()),
        when: WhenClause {
            channels: vec![],
            domains: vec![],
            agents: vec![agent_id.to_string()],
        },
        r#match: MatchClause::Single(matcher),
        action,
        rewrite: draft.rewrite.clone(),
        severity: draft.severity,
        owner_agent_id: Some(agent_id.to_string()),
    }
}
