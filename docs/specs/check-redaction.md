# GuardEvent Redaction Spec

## Purpose

Allow customers with hard data-residency or privacy constraints to use TrustLoopGuard without sending raw sensitive values such as SINs, tax forms, personal identifiers, or financial records to the hosted TrustLoopGuard service.

The key product rule is:

> The guardrail pipeline may evaluate redacted content, typed entity labels, workflow metadata, and policy context. Raw sensitive values must stay inside the customer's environment unless the customer explicitly chooses a private deployment that permits raw processing.

## Goals

- Add a redaction step before the guardrail engine evaluates `GuardEvent`.
- Preserve enough structure for policy checks to remain useful after redaction.
- Support typed placeholders such as `[SIN_1]`, `[PERSON_NAME_1]`, and `[INCOME_AMOUNT_1]`.
- Support customers who need raw data to remain in Azure or another trusted environment.
- Ensure traces, cache keys, LLM judges, analytics, and logs use sanitized content when redaction is enabled.
- Make redaction behavior explicit in the SDK and API response metadata.

## Non-goals

- Redaction is not a guarantee that all sensitive data is removed perfectly.
- Hosted TrustLoopGuard cannot claim "raw data never leaves customer infrastructure" if redaction only happens after the request reaches hosted `tl-server`.
- TrustLoopGuard does not validate tax correctness, SIN ownership, identity matching, or document authenticity in hosted mode.
- This spec does not add human review analytics. See `human-review-analytics.md`.

## Deployment Modes

There are three valid redaction placements.

### 1. SDK-local redaction

The SDK redacts before sending `POST /v1/events`.

```text
Customer app raw data
  -> TrustLoopGuard SDK redactor
  -> sanitized GuardEvent
  -> hosted /v1/events
  -> engine
```

This is the default mode for customers who cannot send raw data to TrustLoopGuard.

### 2. Customer-environment redaction service

A sidecar or private service runs in the customer's environment, such as TaxBuddy's Azure deployment.

```text
Customer app raw data
  -> customer-hosted redaction service
  -> sanitized GuardEvent
  -> hosted /v1/events
  -> engine
```

This is useful when customers want a managed service boundary inside their own cloud rather than embedding redaction in application code.

### 3. Server-side redaction

Hosted or private `tl-server` redacts before calling the engine.

```text
POST /v1/events raw or sanitized GuardEvent
  -> tl-server redaction stage
  -> sanitized GuardEvent
  -> engine
```

This is defense-in-depth for hosted TrustLoopGuard and useful for private deployments. It does not satisfy customers whose hard rule is that raw sensitive values must not leave their infrastructure.

## Pipeline Design

When redaction is enabled, the request lifecycle becomes:

```text
GuardEvent
  -> authenticate and resolve workspace
  -> validate run_id and run_event_id
  -> create inline run_event if provided
  -> redact action parameters, source excerpts, provenance-bearing text, and configured context fields
  -> load enabled policies
  -> run event pipeline and engine on sanitized GuardEvent
  -> persist sanitized trace metadata
  -> return Decision with redaction metadata
```

The engine should not need to know where redaction happened. It receives a normal `GuardEvent` whose sensitive values have already been replaced.

Cache keys must be computed from the sanitized request. Trace persistence must not write raw values. LLM judges must receive sanitized input and output only.

## Placeholder Format

Use typed, numbered placeholders instead of vague masks.

Good:

```text
[PERSON_NAME_1] has SIN [SIN_1] and income [INCOME_AMOUNT_1].
```

Bad:

```text
[customer_info] has private data.
```

Initial entity types:

- `PERSON_NAME`
- `SIN`
- `DATE_OF_BIRTH`
- `ADDRESS`
- `PHONE_NUMBER`
- `EMAIL`
- `INCOME_AMOUNT`
- `EMPLOYER_NAME`
- `TAX_FORM_ID`
- `BANK_ACCOUNT`
- `GOVERNMENT_ID`
- `CUSTOM`

Numbering is scoped to one check request. The raw-to-token map stays local to the redactor and must not be sent to hosted TrustLoopGuard.

## Wire Contract

Prefer additive fields in `tl-core` so old clients remain valid.

Suggested additions:

```rust
pub struct CheckRequest {
    // existing fields...
    pub redaction: Option<RedactionInfo>,
}

pub struct RedactionInfo {
    pub mode: RedactionMode,
    pub status: RedactionStatus,
    pub entities: Vec<RedactedEntity>,
    pub input_redacted: bool,
    pub proposed_output_redacted: bool,
    pub context_redacted: bool,
}

pub enum RedactionMode {
    SdkLocal,
    CustomerService,
    Server,
}

pub enum RedactionStatus {
    NotRequested,
    Applied,
    Failed,
    RejectedRawSensitiveData,
}

pub struct RedactedEntity {
    pub entity_type: String,
    pub token: String,
    pub count: u32,
}
```

Do not include raw values, raw offsets, hashes of raw values, or reversible token maps in the hosted request. If local rehydration is ever needed, it belongs in the customer process only.

`Decision` can include a redaction summary so the caller knows what happened:

```rust
pub struct Decision {
    // existing fields...
    pub redaction: Option<RedactionInfo>,
}
```

If adding `Decision.redaction` is too wide for the first slice, return redaction metadata through a response extension only after all SDKs can expose it consistently.

## Workspace Settings

Add a workspace-level data handling mode owned by Rust:

- `raw_allowed` - current behavior.
- `redacted_only` - hosted server rejects requests that appear to contain configured sensitive data without redaction metadata.
- `no_body_retention` - server may process sanitized content but must not persist request/response bodies.
- `private_deployment` - raw processing may be allowed because the server runs inside the customer's approved environment.

For TaxBuddy-style customers, the recommended setting is `redacted_only` plus SDK-local or customer-service redaction.

## Redaction Scope

Redact:

- `CheckRequest.input`
- `CheckRequest.proposed_output`
- selected `CheckRequest.context` string fields
- `run_event.input_summary`
- `run_event.output_summary`

Do not redact:

- `agent_id`
- `workspace_id`
- `run_id`
- `run_event_id`
- policy ids
- non-sensitive workflow metadata such as `workflow_step`, `document_type`, `confidence_bucket`, or `pii_types`

The redactor should fail closed when a configured required field cannot be safely sanitized.

## Hosted Cloud Behavior

For hosted TrustLoopGuard:

- SDK-local redaction is the primary privacy path.
- Server-side redaction is defense-in-depth only.
- If workspace mode is `redacted_only`, `tl-server` should reject obvious raw sensitive data in `input`, `proposed_output`, or configured context fields when `CheckRequest.redaction` is absent or says `None`.
- Logs must include `trace_id`, workspace, authorization effect, policy ids, and redaction status only.
- Logs must not include raw or sanitized bodies unless the workspace explicitly enables body logging.

## Local-Only Checks

Some checks require raw values and cannot be done by hosted TrustLoopGuard after redaction:

- SIN format or ownership validation.
- Name, address, and SIN matching across forms.
- Income amount matching against the uploaded T4.
- Bank-account or government-id consistency.
- Document authenticity checks.

These checks should run in one of these places:

- customer application code
- SDK-local helper
- customer-hosted redaction service
- embedded `tl-engine`
- private Azure deployment

Hosted TrustLoopGuard should receive the result as metadata, not the raw value:

```json
{
  "context": {
    "workflow_step": "document_extraction",
    "document_type": "T4",
    "local_validations": {
      "sin_format": "passed",
      "name_matches_profile": "failed",
      "income_matches_document": "passed"
    },
    "pii_types": ["SIN", "PERSON_NAME", "INCOME_AMOUNT"]
  }
}
```

## SDK Experience

Target TypeScript shape:

```ts
const decision = await client.check({
  agent_id: "tax-document-agent",
  channel: "chat",
  input,
  proposed_output,
  context: {
    workflow_step: "document_extraction",
    document_type: "T4",
  },
}, {
  redaction: {
    mode: "sdk_local",
    entities: ["SIN", "PERSON_NAME", "DATE_OF_BIRTH", "INCOME_AMOUNT"],
  },
});
```

Equivalent Python and Rust SDK ergonomics should exist before the feature is considered shipped.

SDK behavior:

- Build a new sanitized request without mutating the caller's object.
- Keep the token map local.
- Send only sanitized content and redaction metadata.
- Surface redaction failures as typed SDK errors.
- Let callers configure fail-open or fail-closed behavior. Regulated workflows should default to fail-closed.

## Acceptance Criteria

- SDK-local redaction replaces configured sensitive values before network egress.
- Server-side redaction runs before engine evaluation when enabled.
- The engine, cache, trace writer, escalation worker, LLM judges, and analytics receive sanitized content.
- Workspace mode `redacted_only` rejects unredacted obvious sensitive data.
- Redaction metadata is available to policies and analytics without exposing raw values.
- The SDKs expose the feature consistently across TypeScript, Python, and Rust.
- Tests prove the caller's original request object is not mutated.
- Tests prove raw sensitive samples are absent from serialized outbound SDK requests, server logs where testable, and persisted trace payloads.

## Verification

- `cargo fmt`
- `cargo test -p tl-core`
- `cargo test -p tl-engine`
- `cargo test -p tl-server`
- `cargo test -p tl-storage` if persistence/settings change
- `make verify-contract`
- `make sdk-all`
- targeted TypeScript and Python SDK tests for redaction behavior

## Worktree Boundary

This workstream owns redaction placement, wire metadata, SDK redaction helpers, workspace data-handling modes, and server pre-engine sanitization. It should not add human review outcome charts, review event storage, or reviewer workflow UI.

If this workstream and the human review analytics workstream both regenerate `docs/openapi.yaml` or SDK generated files, merge one branch first and rerun codegen in the second branch.
