# Graph Report - TrustLoopGuard  (2026-07-10)

## Corpus Check
- 1371 files · ~758,856 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 14910 nodes · 30495 edges · 1849 communities (1206 shown, 643 thin omitted)
- Extraction: 95% EXTRACTED · 5% INFERRED · 0% AMBIGUOUS · INFERRED: 1644 edges (avg confidence: 0.7)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `b0aea692`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Client
- GuardEvent
- cn
- AnalyticsCatalogDimension
- fetchMock
- AsyncClient
- oauth.rs
- FinancialActionsContent.tsx
- dashboard-data.ts
- PolicyEditorDialog.tsx
- Integrating TrustLoopGuard
- code:block1 (POST /v1/check)
- proxyRustJson
- Field-by-field
- code:yaml (id: refund-guarantee)
- redteam_job_repo.rs
- ApiErrorCode
- redteam.rs
- settings_update.rs
- types.py
- PostgresStore
- Client
- ReviewQueueContent.tsx
- AgentListResponse
- RunSummary
- code:block1 (tl-cli      tl-server      tl-sdk-rust)
- 0. Start the server (all languages need this)
- code:block1 (Guard.check(draft, ctx) -> Decision)
- githubRepo
- DashboardAdminStoreError
- Result
- AnalyticsChartGrid.tsx
- param_auth.rs
- PostgresGatewayAdapter
- code:bash (curl -X PATCH \)
- llm_pricing.rs
- latest_review_outcomes
- SDK-Driven Development at TrustLoopGuard
- code:text (id: id must use lowercase letters, numbers, '-' or '_')
- code:python (import trustloopguard as trustloop)
- tests.rs
- create_knowledge_source
- label.rs
- BadgeProps
- auth.rs
- Ownership
- code:block1 (+-------------------+      CheckRequest       +-------------)
- Domain terms
- JwtSigner
- agent.rs
- tests.rs
- apiKeyHeaders
- agents.ts
- report.rs
- provider_record_to_wire
- ._run_with_retry
- run_summary
- EnvironmentStoreError
- properties
- code:text (policies/refund-promise.yaml)
- GuardEvent.ts
- rustApiForAuthorizedWorkspace
- scripts
- guard.ts
- tlClientForRequest
- _shared.ts
- workspace_id_from_headers
- models.rs
- RunDetailLiveView.tsx
- synthesis.rs
- properties
- ToolHandlers
- redteam-jobs.ts
- AgentRepo
- AuthConfig
- RunnerError
- Load test
- Default
- change_password
- pipeline_e2e.rs
- schema.rs
- attacks-panel.tsx
- RedteamState
- gateway.rs
- UserContext
- run-detail-live.ts
- share.rs
- checker_enforcement.rs
- mod.rs
- check.ts
- path
- Policy
- normalization.rs
- event_ingestion.rs
- RetryConfig
- AnalyticsStoreError
- redteam-report.ts
- tests.rs
- req
- payload
- create_api_key
- AppState
- AnalyticsDashboardWidget.ts
- RedteamReportShareRepo
- page.tsx
- Technical terms
- ManagementPages.tsx
- code:text (agent drafts risky output)
- dashboard_admin_repo.rs
- harden-job-card.tsx
- financial_authorization_service.rs
- family_parse.rs
- AgentProfile.ts
- BudgetAlertRepo
- analytics.rs
- KnowledgeSourceDocument.ts
- HnswFuzzyChecker
- policy_cli.rs
- tl-client.ts
- adapter.ts
- @auth/drizzle-adapter
- compilerOptions
- { GET, POST }
- Write Your First Policy
- plan-card.tsx
- FinancialActionRecord
- event_policy.rs
- server.ts
- MemoryBudgetAlertStore
- code:bash (npm view @trustloopguard/sdk version)
- api_error_response
- package.json
- PoliciesPageContent.tsx
- RedteamJobStoreError
- gateway_budget.rs
- index.ts
- seo-landing-page.tsx
- SAMPLES
- type
- Result
- MemoryFinancialStore
- properties
- type
- BudgetAlertStoreError
- Repository Agent Instructions
- Result
- MemoryRedteamJobStore
- TierResult
- tier_results
- code:block1 (agent proposes output → trustloop.check(...) → allow | block)
- v0 Design Decisions
- Runtime Refactor Jobs
- agents
- FinancialStoreError
- in_scope
- properties
- definitions
- Dashboard
- package.json
- forbidden
- route.ts
- definitions
- TriggeredPolicy
- Acceptance flow (Option A)
- proxy-helpers.ts
- financial.rs
- latest_review_outcomes
- drizzle-kit
- db:generate
- pull_request_template.md
- WorkspaceInvite.ts
- validation.rs
- agent-profile.schema.json
- default
- PolicyRepo
- workflow_requirements
- compilerOptions
- channels
- policy.schema.json
- lint-web-backend-only.sh
- Client
- code:text (┌─────────────────────────────────────┐)
- lint-no-internal-imports.sh
- FamilyPolicy
- budget_alerts.rs
- PolicyStoreError
- MemoryRunStore
- code:text (UI component)
- RedteamJobStore
- lint-storage-boundaries.sh
- Decision.ts
- lint-api-contracts.sh
- validation.rs
- dashboard.rs
- policy.rs
- redteam_runner.rs
- check-schema-drift.sh
- PostgresAnalyticsAdapter
- value_limit.rs
- metrics.rs
- RunState
- tool_metadata.rs
- EscalationRepo
- RouterConfig
- package.json
- entrypoint
- setup.ts
- code:text (Browser / SDK)
- engine.rs
- code:ts (const decision = await client.check({)
- README.md
- code:text (Customer app -> SDK -> /v1/check -> Decision -> customer han)
- MemoryRedteamPlanStore
- escalation.rs
- .update_run
- SemanticPolicyJudgeResult
- KnowledgeRepo
- MemoryAnalyticsStore
- writer.rs
- package.json
- RunRepo
- require_approved_user
- Result
- .prettierrc.json
- dependencies
- MokaCache
- validate_raw_policy
- lib.rs
- code:text (app -> /v1/gateway/<route_id>/openai -> TrustLoopGuard -> pr)
- button.tsx
- event
- tool.rs
- finalize_gateway_response
- label_policy.rs
- code:text (source of truth)
- trust-band.tsx
- code:text (Dashboard / customer integration)
- properties
- code:bash (npm install @trustloopguard/sdk)
- harden_job
- gateway.rs
- spawn_writer
- memory.rs
- traces.rs
- EnvironmentRepo
- load_str
- runs.rs
- RedteamPlanRepo
- knowledge.rs
- precommit-typecheck.sh
- definitions
- redteam-core.ts
- precommit-secretlint.sh
- handlers.rs
- api_keys.rs
- code:py (import trustloopguard as trustloop)
- plan.rs
- aggregate
- financial_error_response
- PostgresHumanReviewAdapter
- code:text (POST /v1/traces/{trace_id}/review-events)
- store.rs
- event_service.rs
- .create_event
- Web UI Conventions
- MemoryLlmUsageStore
- enforcement.rs
- properties
- trialIndex
- package.json
- kind
- llm_usage_repo.rs
- PostgresTraceAdapter
- backend-coverage.sh
- fresh_repo
- fresh_pool
- ignoreBinaries
- Code of Conduct
- PostgresLabelPolicyAdapter
- guard
- properties
- decision.schema.json
- code-block.tsx
- prepush-fast.sh
- GatewayRepo
- RedactedEntity
- parse_retry_after
- tool_metadata.rs
- RunStoreError
- render-diagrams.sh
- evaluate_financial_policies
- StorageError
- Validation
- GuardEvent Redaction Spec
- authorize_workspace_admin
- route.ts
- financial_actions.rs
- budget_alerts.rs
- build_postgres_layer
- financial_actions_integration.rs
- team.rs
- FinancialExecutionResult
- guard.rs
- .query
- enum
- Security Policy
- CheckerFindingEvidence
- RunnerDocumentTemplate
- TrustLoopGuard Hardening v2 — Attack-Grounded Policy Synthesis
- GatewayState
- components.json
- fresh_repo
- GatewayStoreError
- effective_checker_modes
- properties
- route.ts
- LlmClient
- tests.rs
- llm-docs.ts
- tests.rs
- read_filter
- sync-recipes.ts
- code:sh (pnpm demo:job)
- key.rs
- properties
- HnswIndex
- code:text (Browser analytics UI)
- OpenRouterClient
- resolved_event
- code:sh (pnpm demo:n8n:bridge)
- TokenBudget
- load_agent_str
- monitoring_integration.rs
- validation.rs
- mod.rs
- guardrails.rs
- financial_repo.rs
- handlers.ts
- Crates
- redteam-runner.schema.json
- test_events.py
- main.rs
- TierResult
- docs-auth.ts
- scripts
- code:sh (pnpm --filter @trustloopguard/example-typescript start \)
- human_review.rs
- .from_response
- put_llm_price
- Common Workflows
- main.rs
- agents.rs
- UserStoreError
- str
- properties
- ParamLimit
- PolicyValidateResponse
- ParamLimit
- RedteamDispatchRequest.ts
- Client
- code:sh (pip install -e sdks/python)
- forward_payment
- monitoring.tsx
- router
- run.rs
- nav-actions.tsx
- Event Engine
- code:sh (TL_SERVER_URL=http://127.0.0.1:8080 \)
- Policy YAML Reference
- LabelPolicyStoreError
- compilerOptions
- verify_candidate
- PostgresUserAdapter
- client.ts
- code:py (retry=RetryConfig(max_attempts=1, total_budget_s=0.25))
- workflow_analyzer.rs
- policy.rs
- seal_key_material
- null
- route.test.ts
- page.tsx
- build_app
- validation.rs
- check_pipeline.rs
- definitions
- MemoryToolMetadataStore
- Human Review Analytics Spec
- WorkflowRequirement
- HumanReviewAnalyticsResponse.ts
- normalize_payment_requirement
- compilerOptions
- devDependencies
- LlmRouter
- seed-demo.ts
- compilerOptions
- knowledge.rs
- api-keys.ts
- HardenCandidate.ts
- LlmPricingRepo
- lib.rs
- trustloopguard
- http.rs
- policy
- build_app_state
- request
- policy_authority.rs
- RunnerPlanRequest
- RunnerPlanResponse
- 4. Goal-Driven Execution
- SourceLabelPolicy
- BudgetAlertConfig.ts
- retry_integration.rs
- validate_create_action
- MemoryStore
- guard-event.schema.json
- MemoryHumanReviewStore
- CheckerFinding
- .analytics
- 1. Think Before Coding
- budget.rs
- Plugin contract
- RunnerReport
- Policy Cookbook
- LlmUsageEvent.ts
- ToolMetadataRepo
- TeamStoreError
- ConnectAgentStep.tsx
- Any
- index.mdx
- server.test.ts
- fresh_store
- Architecture
- Team & invites
- 2. Simplicity First
- MemorySettingsStore
- core.ts
- compilerOptions
- parse_body
- events_integration.rs
- .create_event
- .create_financial_policy
- fresh_repo
- tier.rs
- Red-Team Dispatch
- @trustloopguard/sdk
- theme-provider.tsx
- scripts
- Contributing to TrustLoopGuard
- code:bash (curl -X POST $TLG_URL/v1/check \)
- TeamStoreError
- semantic_policy_batch.md
- wire.rs
- gateway_routes
- .submit_event
- header_value
- marketing-event-link.tsx
- JsonSchema
- MemoryPolicyStore
- Authorization
- Gateway
- ProfileResolver
- feature_request.md
- view_from_record
- PolicyEditorDialog.test.tsx
- code:json ({)
- AuthUserState
- ApiError
- create_review_event
- runs_integration.rs
- ProvenanceMap
- fresh_repo
- dashboard-widgets.tsx
- ._send_json_model
- LiveKitSupportAgent
- code:sh (pnpm demo:chat)
- LiveKit agent guardrail demo
- Agent-hardening loop
- Red-Team Report Sharing
- RunnerHandle
- Red-Team Runner Contract v1
- Integration & Interception — How TrustLoopGuard Hooks an Agent
- compilerOptions
- guard-modes.mdx
- properties
- properties
- policy_ast.rs
- LabelBasisSet
- properties
- 3. Surgical Changes
- .generate_guardrails
- api_error_response
- code:sh (pnpm demo:chat:interactive)
- HandlerCtx
- null
- definitions
- enum
- params
- properties
- Analytics Dashboards
- enum
- devDependencies
- code:text (Customer / integrator runtime)
- run.sh
- code:text (1. [Step] -> verify: [check])
- code:bash (make quickstart)
- required
- .check
- properties
- code:block2 (CheckRequest)
- proxy_anthropic_messages
- .list_policies
- docs
- package.json
- Result
- WorkflowDefinition
- enum
- RunDetail.ts
- CheckerRun
- query_parts
- Financial Authorization
- Financial Authorization Contract TDD Evidence
- MemoryKnowledgeStore
- proxy.ts
- env.ts
- Environments
- TrustLoopGuard concepts
- Runs
- normalize_create_mandate_request
- budget_alert.rs
- RedteamJobSummary
- TraceSummary
- SignalEvidence
- Policies
- README.md
- hallucination.md
- semantic_policy.md
- route.test.ts
- insert_trace
- route.test.ts
- route.ts
- llm_usage.rs
- Web Dashboard And Authentication
- tool-metadata.schema.json
- seo.ts
- authority.md
- tone.md
- page.tsx
- Human Review Analytics
- generate-openapi-docs.mjs
- EntityVersionListResponse.ts
- WorkspaceEnvironmentListResponse.ts
- layout.tsx
- OpenAiClient
- next.config.mjs
- auth.rs
- default_settings
- KnowledgeStoreError
- AnalyticsFact
- source.config.ts
- generate_guardrails
- next.config.ts
- postcss.config.mjs
- next.config.ts
- postcss.config.mjs
- Glossary
- README.md
- AiEditRequest.ts
- AiEditResponse.ts
- AuthRequest.ts
- AuthResponse.ts
- ChangePasswordRequest.ts
- CreateWorkspaceEnvironmentRequest.ts
- CreateWorkspaceRequest.ts
- EntityVersionDetail.ts
- OAuthIdentityRequest.ts
- UpdateWorkspaceEnvironmentRequest.ts
- code:text (agent drafts output)
- code:text (SDK / agent runtime)
- code:sh (pnpm install)
- code:sh (make quickstart)
- code:bash (export TL_API_KEY=dev-admin)
- code:sh (cargo run -p example-rust -- "show me my password" "here it )
- code:bash (pnpm --filter web dev)
- code:ts (import OpenAI from 'openai';)
- code:sh (pnpm demo:chat)
- code:sh (cargo run -p tl-cli -- policy validate policies/refund-guara)
- code:ts (import OpenAI from 'openai';)
- code:text (POST /v1/gateway/{route_id}/openai/chat/completions)
- code:ts (import Anthropic from '@anthropic-ai/sdk';)
- code:text (POST /v1/gateway/{route_id}/anthropic/v1/messages)
- Dashboard setup
- code:ts (import { GuardMode, guard } from '@trustloopguard/sdk';)
- code:ts (const guardrail = guard({)
- code:python (async def regenerate_reply(feedback: trustloop.RegenerateFee)
- route.test.ts
- index.mdx
- proxy_provider_request
- auth-redirect.ts
- properties
- proxy_healthcare_agent.py
- HumanReviewStoreError
- RunListFilter
- SourceLabelEvidence
- code:sh (cargo run -p tl-cli -- policy validate policies/example.yaml)
- next-env.d.ts
- code:sh (pnpm install)
- code:sh (DOCS_PASSWORD=replace-with-a-secret)
- STEPS
- REASONS
- next-env.d.ts
- { POST }
- window_starts
- providers
- DocsPageProps
- ButtonProps
- onSelectedRowKeysChange
- user
- labelVariants
- SheetContentProps
- sheetVariants
- apiKeyColumns
- runColumns
- policyColumns
- payloadWithoutNote
- req
- fetchMock
- headers
- drizzle-orm
- postgres
- db:push
- client
- db
- GuardrailGenerateResponse
- GuardrailListResponse
- Policy
- PolicyDocument
- Response
- T
- AnalyticsCatalogMetric
- AnalyticsChartType
- AnalyticsDashboardView
- AnalyticsDashboardViewConfig
- AnalyticsDashboardViewListResponse
- AnalyticsDashboardWidget
- AnalyticsDimension
- AnalyticsFacet
- AnalyticsFacetCatalogResponse
- AnalyticsFilter
- AnalyticsMetric
- AnalyticsQueryPoint
- AnalyticsQueryRequest
- AnalyticsQueryResponse
- AnalyticsWidgetLayout
- ApiKeyBatchRevokeRequest
- ApiKeyBatchRevokeResponse
- ApiKeyListResponse
- Channel
- CreateAnalyticsDashboardViewRequest
- CreateApiKeyRequest
- CreateApiKeyResponse
- CreateHumanReviewEventRequest
- CreateKnowledgeSourceRequest
- CreateWorkspaceEnvironmentRequest
- DashboardApiKey
- DashboardKnowledgeSourceKind
- DataHandlingMode
- HumanReviewAnalyticsResponse
- HumanReviewAnalyticsSummary
- HumanReviewEvent
- HumanReviewEventListResponse
- HumanReviewGroupRow
- HumanReviewOutcomeCounts
- HumanReviewPolicyRow
- HumanReviewReasonRow
- HumanReviewWorkflowStepRow
- KnowledgeFileInput
- KnowledgeFileMetadata
- KnowledgeSourceDocument
- KnowledgeSourceFileResponse
- KnowledgeSourceListResponse
- KnowledgeSourceStatus
- RedactedEntity
- RedactionInfo
- RedactionMode
- RedactionStatus
- Formatter
- HumanReviewOutcome
- Value
- RunEventKind
- RunKind
- RunListResponse
- RunStatus
- Severity
- TlError
- TraceSummary
- TriggeredPolicy
- UpdateAnalyticsDashboardViewRequest
- UpdateWorkspaceEnvironmentRequest
- WorkspaceEnvironment
- WorkspaceEnvironmentListResponse
- WorkspaceSettings
- InviteLookupResponse
- MockRunner
- Arc
- CancellationToken
- HandlerCtx
- Policy
- TierOutput
- Vec
- MockClient
- StrictEnvironmentStore
- gateway.mdx
- CreateRunEventRequest
- CreateRunRequest
- Duration
- GuardrailGenerateResponse
- GuardrailListResponse
- HeaderMap
- RunDetail
- RunEventListResponse
- RunEventSummary
- T
- TraceListResponse
- UpdateRunRequest
- MemoryAgentStore
- AgentProfile
- ApiErrorCode
- HashMap
- Result
- RwLock
- Self
- StatusCode
- Vec
- AnalyticsUserId
- ApiErrorCode
- HashMap
- RwLock
- StatusCode
- Uuid
- ApiErrorCode
- StatusCode
- MemoryUserStore
- PasswordError
- ApiErrorCode
- HashMap
- Json
- Response
- RwLock
- Self
- State
- StatusCode
- MemoryApiKeyRecord
- MemoryApiKeyStore
- MemorySettingsStore
- ApiErrorCode
- Extension
- HeaderMap
- InternalServiceContext
- Json
- Response
- RwLock
- State
- StatusCode
- UserContext
- MemoryEnvironmentStore
- HashMap
- RwLock
- Self
- Vec
- WorkspaceEnvironment
- MemoryHumanReviewStore
- HashMap
- HeaderMap
- Item
- Iterator
- Json
- Path
- RwLock
- State
- Uri
- MemoryKnowledgeStore
- ApiErrorCode
- CreateKnowledgeSourceRequest
- HashMap
- HeaderMap
- Json
- KnowledgeSourceDocument
- KnowledgeSourceFileResponse
- Path
- Response
- Result
- RwLock
- Self
- State
- StatusCode
- LlmModelPrice.ts
- Vec
- ApiDoc
- AppState
- AuthConfig
- Instant
- Json
- LlmClient
- Next
- Request
- Response
- RunStoreError
- State
- StatusCode
- GuardrailState
- MemoryPolicyRecord
- MemoryPolicyStore
- ParsedPolicyBody
- Action
- AgentStore
- ApiErrorCode
- Bytes
- EntityVersionDetail
- EntityVersionListResponse
- HashMap
- HeaderMap
- JsonSchema
- Path
- Policy
- PolicyDocument
- PolicySummary
- PolicyValidationIssue
- Result
- RwLock
- Self
- State
- StatusCode
- Vec
- MemoryRunStore
- HashMap
- Item
- Iterator
- Json
- Path
- Response
- RunEventKind
- RwLock
- Self
- State
- TraceSummary
- Uri
- Vec
- InviteLookupRecord
- MemoryTeamState
- MemoryTeamStore
- ApiErrorCode
- Extension
- HeaderMap
- Json
- Path
- Response
- Result
- RwLock
- State
- StatusCode
- StorageError
- TeamRepo
- UserContext
- Uuid
- TeamRepoAdapter
- AnalyticsFact
- MetricAccumulator
- NewViewRecord
- AnalyticsDashboardView
- AnalyticsDashboardViewConfig
- AnalyticsWidgetLayout
- CreateAnalyticsDashboardViewRequest
- DateTime
- Item
- Iterator
- Option
- String
- UpdateAnalyticsDashboardViewRequest
- Utc
- Value
- Vec
- ViewRecord
- ApiKeyAuthRecord
- ApiKeyRecord
- NewApiKeyRecord
- DashboardApiKey
- Uuid
- Vec
- DateTime
- FailMode
- GatewayInputAction
- GatewayOutputAction
- GatewayProviderKind
- NewEnforcementProfile
- NewGatewayProviderConnection
- NewGatewayRoute
- ResponseMode
- RetentionMode
- Utc
- Vec
- GroupAccumulator
- PolicyAccumulator
- F
- HumanReviewAnalyticsFilter
- HumanReviewAnalyticsResponse
- HumanReviewOutcome
- T
- Value
- WorkflowAccumulator
- Vec
- ContainerAsync
- PostgresImage
- CreateRunEventRequest
- DateTime
- HashMap
- HumanReviewOutcome
- RunEventKind
- RunEventSummary
- TraceReviewLookupRow
- TraceSummary
- Utc
- Uuid
- Value
- RunStats
- InviteLookup
- InviteRow
- MemberRow
- DateTime
- MyWorkspace
- Option
- String
- Utc
- Uuid
- Vec
- fresh_repos
- WorkspaceRole
- UserNameRow
- str
- str
- code:ts (import { GuardMode, guard } from '@trustloopguard/sdk';)
- code:sh (pip install -e sdks/python)
- code:sh (TL_API_KEY=dev-admin \)
- code:json ({)
- Gateway proxy
- code:yaml (agent_id: my-bot)
- code:yaml (agent_id: string                 # required, non-empty)
- code:yaml (knowledge_sources:)
- code:yaml (tone:)
- code:yaml (# policies/no-guarantees.yaml)
- code:yaml (authority:)
- code:bash (# Register (creates or replaces by agent_id))
- code:text (POST /v1/identity/oauth-session)
- Workspace API keys (future)
- code:yaml (id: refund-promise)
- code:bash (cargo run -p tl-codegen           # write)
- `tl-storage` — decision log
- code:text (Customer app -> /v1/gateway/... -> input check -> provider -)
- code:text (baseURL = https://<server>/v1/gateway/<route_id>/openai)
- code:text (baseURL = https://<server>/v1/gateway/<route_id>/anthropic)
- code:block2 (fn check(draft: Draft, ctx: Context) -> Decision)
- code:block3 (Draft {)
- code:block4 (Context {)
- code:block5 (Decision {)
- code:block6 (fn push(chunk: String) -> StreamDecision)
- code:bash (pnpm docs:diagrams)
- code:text (Workspace -> Agent -> Run -> Run event -> Trace / Decision)
- code:text (POST   /v1/runs)
- code:bash (git tag -l 'sdk-v0.0.6')
- code:bash (git fetch origin main --tags)
- code:bash (git rev-parse sdk-v0.0.6)
- code:bash (gh run list --workflow "Publish SDK" --limit 5)
- code:bash (git tag -f sdk-v0.0.6 <commit>)
- code:text (+----------+         +----------+         +----------+)
- code:text (+---------------+       POST /v1/team/invites       +-------)
- Existing users
- Public lookup
- Why signup-with-token, not full JWT
- code:block2 (t=0       all three tiers start in parallel)
- code:rust (let cancel = CancellationToken::new();)
- code:block4 (1. Built-in defaults (we ship))
- code:block5 (Layer 4: Framework adapters     (Vercel AI SDK, LangChain, O)
- code:typescript (const reply = await tl.guard({)
- Data model
- Goals
- Non-goals
- Shared resources and generated contracts
- Web dashboard and authentication spec
- code:ts (interface DataTableColumn<T> {)
- code:bash (brew install d2)
- EntityVersionRecord
- Gateway Provider Management TDD Evidence
- RedteamAttackRecord.ts
- zod
- code:block1 (one-time, off the hot path)
- code:bash (pip install trustloopguard)
- code:python (import os)
- code:bash (curl -X POST $TLG_URL/v1/agents \)
- code:ts (onError: (_err, _draft) => "I'm having trouble right now — l)
- code:python (on_error=lambda _err, _draft: "I'm having trouble right now )
- code:yaml (# policies/agents/acme-support-v3.yaml)
- code:bash (curl -X POST https://your-trustloopguard/v1/agents \)
- code:python (import trustloopguard as trustloop)
- code:ts (import { GuardMode, guard } from "@trustloopguard/sdk";)
- code:python (async def regenerate_reply(feedback: trustloop.RegenerateFee)
- code:ts (const guardrail = guard({)
- code:bash (pnpm add @trustloopguard/sdk)
- code:ts (import { Client, guard } from "@trustloopguard/sdk";)
- code:bash (curl -X POST \)
- code:bash (curl -H "Authorization: Bearer $TL_API_KEY" \)
- code:bash (curl -X DELETE \)
- code:yaml (match:)
- code:yaml (when:)
- code:bash (cargo run -p tl-cli -- policy-lint docs/policies/examples/re)
- code:text (POST /v1/policies/validate)
- code:yaml (agent_id: baker-9000)
- code:bash (# Register the agent first.)
- code:text (universal built-ins)
- code:bash (cargo run -p tl-cli -- policy validate policies/refund-promi)
- code:yaml (id: refund-guarantee)
- code:bash (cargo run -p tl-cli -- policy validate policies/refund-guara)
- code:text (ok: policy `refund-guarantee` valid)
- code:text (local file -> tl-policy parser -> Policy)
- code:bash (cargo run -p tl-cli -- policy push policies/refund-guarantee)
- code:bash (cargo run -p tl-cli -- policy pull refund-guarantee \)
- code:yaml (id: refund-guarantee)
- code:text (match.regex: regex failed to compile)
- code:text (rewrite: rewrite is required when action is rewrite)
- code:yaml (action: rewrite)
- code:text (match.literal: literal matcher must not be empty)
- code:text (when.agents[0]: must not be empty)
- code:yaml (action: rewrite)
- code:yaml (id: Refund Guarantee)
- code:yaml (description: Prevents agents from guaranteeing refunds.)
- code:yaml (when:)
- code:yaml (match:)
- code:ts (const run = await client.startRun({)
- code:text (Customer app raw data)
- code:text (POST /v1/check raw or sanitized request)
- code:text (CheckRequest)
- code:text ([PERSON_NAME_1] has SIN [SIN_1] and income [INCOME_AMOUNT_1])
- code:text ([customer_info] has private data.)
- code:rust (pub struct CheckRequest {)
- code:rust (pub struct Decision {)
- code:json ({)
- code:json ({)
- code:sh (# Terminal 1)
- code:sh (./loadtest/run.sh allow -n 10000 -c 200)
- code:bash (pnpm recipes:update)
- code:bash (pnpm recipes:check)
- code:md (<!-- BEGIN recipe:output-boundary-guard:typescript -->)
- next
- float
- str
- bool
- float
- int
- str
- InviteLookupResponse
- bool
- float
- int
- Run a check and dispatch the appropriate callback. Returns the     string the ca
- float
- int
- str
- float
- code:ts (import { guard, GuardMode } from '@trustloopguard/sdk';)
- code:ts (import { guard } from '@trustloopguard/sdk';)
- code:ts (import { Client } from '@trustloopguard/sdk';)
- code:ts (import OpenAI from 'openai';)
- code:ts (import Anthropic from '@anthropic-ai/sdk';)
- client
- fetchSpy
- body
- client
- { client, fetchSpy }
- events
- fetchSpy
- guardrail
- init
- onAllow
- onBlock
- onError
- onEscalate
- onRevise
- regenerate
- seen
- client
- fetchSpy
- headers
- client
- fetchSpy
- c
- d
- client
- fetchSpy
- scenarios.core.ts
- TierRunner
- code:text (Customer / integrator runtime)
- code:text (1. [Step] -> verify: [check])
- code:text (source of truth)
- code:bash (# Clone and enter the repo)
- code:block2 (<type>: <short description>)
- 0a. Set up secrets (one-time, per machine)
- 0b. Start the server (all languages need this)
- 1. Rust
- 2. Python
- 3. TypeScript
- code:bash (brew install dopplerhq/cli/doppler   # or see docs.doppler.c)
- code:bash (pnpm demo:chat               # scripted live-chat scenarios)
- code:bash (pnpm test:backend)
- code:bash (pnpm coverage:backend)
- code:bash (cargo install cargo-llvm-cov)
- code:bash (make backend-test-db)
- code:bash (make server                          # = doppler run -- carg)
- code:bash (TL_LOG_FORMAT=pretty doppler run -- cargo run -p tl-server)
- code:bash (cargo run -p example-rust -- "show me my password" "here it )
- code:bash (pip install -e sdks/python)
- code:bash (pnpm install)
- code:block7 (verdict       : block)
- Repo philosophy
- TrustLoopGuard
- What is TrustLoopGuard?
- rules
- openapi.rs
- defaults.rs
- fresh_pool
- Red-team harden (policy synthesis)
- index.mdx
- Agent Breakaway Arena
- llm_pricing.rs
- latency_ms
- PolicyError
- index.mdx
- hash_password
- api_error
- required
- enum
- enum
- enum
- api_error_response
- enum
- team_repo.rs
- null
- enum
- definitions
- RunnerSessionEvent
- enum
- route.test.ts
- Workspace feature flags: TDD evidence
- enum
- EnforcementMode
- enum
- enum
- Principal
- LimitAction
- clsx
- @dnd-kit/core
- next
- next-auth
- react
- recharts
- sonner
- @tabler/icons-react
- tailwind-merge
- tw-animate-css

## God Nodes (most connected - your core abstractions)
1. `StorageError` - 360 edges
2. `cn()` - 182 edges
3. `Client` - 148 edges
4. `FinancialStoreError` - 141 edges
5. `AsyncClient` - 116 edges
6. `AppState` - 87 edges
7. `Policy` - 83 edges
8. `proxyRustJson()` - 82 edges
9. `Client` - 80 edges
10. `Domain terms` - 70 edges

## Surprising Connections (you probably didn't know these)
- `main()` --indirect_call--> `event()`  [INFERRED]
  demo/dispute/scenarios.ts → apps/mcp-server/src/handlers.test.ts
- `entrypoint()` --calls--> `RetryConfig`  [INFERRED]
  demo/livekit/guarded_healthcare_agent.py → sdks/python/src/trustloopguard/retry.py
- `createOutputGuard()` --indirect_call--> `decision()`  [INFERRED]
  sdks/typescript/src/guard.ts → apps/mcp-server/src/handlers.test.ts
- `DecisionHandler` --indirect_call--> `decision()`  [INFERRED]
  sdks/typescript/src/guard.ts → apps/mcp-server/src/handlers.test.ts
- `AttacksPanel()` --indirect_call--> `summary()`  [INFERRED]
  apps/web/app/attacks/_components/attacks-panel.tsx → apps/web/app/r/[token]/report-document.test.ts

## Import Cycles
- 2-file cycle: `crates/tl-server/src/redteam/mod.rs -> crates/tl-server/src/redteam/share.rs -> crates/tl-server/src/redteam/mod.rs`
- 2-file cycle: `crates/tl-server/src/policies.rs -> crates/tl-server/src/policies/authoring.rs -> crates/tl-server/src/policies.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/redteam_report_share_repo.rs -> crates/tl-storage/src/lib.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/trace_repo.rs -> crates/tl-storage/src/lib.rs`
- 2-file cycle: `crates/tl-storage/src/knowledge_repo.rs -> crates/tl-storage/src/lib.rs -> crates/tl-storage/src/knowledge_repo.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/redteam_job_repo.rs -> crates/tl-storage/src/lib.rs`
- 2-file cycle: `crates/tl-storage/src/gateway_repo.rs -> crates/tl-storage/src/lib.rs -> crates/tl-storage/src/gateway_repo.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/redteam_plan_repo.rs -> crates/tl-storage/src/lib.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/user_repo.rs -> crates/tl-storage/src/lib.rs`

## Communities (1849 total, 643 thin omitted)

### Community 1 - "Client"
Cohesion: 0.08
Nodes (38): AsyncBaseTransport, BaseTransport, Client, RetryConfig, Synchronous TrustLoopGuard client.      Args:         base_url: TrustLoopGuard s, action_body(), approval_body(), decision_receipt_body() (+30 more)

### Community 2 - "GuardEvent"
Cohesion: 0.11
Nodes (19): Action, EventKind, GuardEvent, Principal, Action, CheckerRun, EventKind, Option (+11 more)

### Community 3 - "cn"
Cohesion: 0.03
Nodes (107): AgentFilter(), AgentFilterProps, AppSidebar(), AppSidebarProps, data, getVisibleNavGroups(), workspace, NavGroup (+99 more)

### Community 6 - "AsyncClient"
Cohesion: 0.12
Nodes (58): AsyncClient, AsyncFinancialOperation, _AsyncRunContext, _AsyncRunEventContext, FinancialOperation, CreateRunEventRequest, CreateRunRequest, RunEventSummary (+50 more)

### Community 9 - "oauth.rs"
Cohesion: 0.08
Nodes (56): caps_per_retry_delay_at_max_delay(), honors_retry_after_when_longer_than_jittered(), ignores_retry_after_when_jitter_already_longer(), invalid(), jitter_fraction_clamps_to_unit_interval(), non_retriable_errors_stop_immediately(), rate_limited(), retries_unavailable_with_exponential_backoff() (+48 more)

### Community 10 - "FinancialActionsContent.tsx"
Cohesion: 0.05
Nodes (76): Badge(), badgeVariants, PageHeader(), PageHeaderProps, BadgeVariant, counterpartyLabel(), currentContextQuery(), FinancialStatusBadge() (+68 more)

### Community 11 - "dashboard-data.ts"
Cohesion: 0.03
Nodes (146): ChangePasswordCard(), AccountPage(), AgentsPage(), AnalyticsPage(), AnalyticsSearchParams, ApiKeysPage(), escapeHeaderValue(), GET() (+138 more)

### Community 12 - "PolicyEditorDialog.tsx"
Cohesion: 0.03
Nodes (129): POST(), requestSchema, withOwnerAgent(), compareLabel(), formatDate(), ReportShareCard(), JOB, mockState (+121 more)

### Community 13 - "Integrating TrustLoopGuard"
Cohesion: 0.12
Nodes (16): Async, Bear-trap checklist, Fail-open vs fail-closed, Financial actions and receipts, Guard modes, Integrating TrustLoopGuard, LLM/model route failures, MCP server (+8 more)

### Community 18 - "proxyRustJson"
Cohesion: 0.04
Nodes (46): POST(), GET(), POST(), POST(), RouteContext, POST(), RouteContext, GET() (+38 more)

### Community 19 - "Field-by-field"
Cohesion: 0.10
Nodes (21): 1. Putting banned vocabulary in `tone.forbidden`, 2. Listing categories instead of commitments in `authority.cannot_promise`, `agent_id`, Agent profile — field reference, `authority.can_promise`, `authority.cannot_promise`, `display_name`, `escalation_triggers` (+13 more)

### Community 22 - "redteam_job_repo.rs"
Cohesion: 0.12
Nodes (29): RedteamSessionEventRecord, attack_record(), event_record_text(), job_summary(), JobCounts, parse_event_created_at(), parse_status(), parse_uuid() (+21 more)

### Community 24 - "redteam.rs"
Cohesion: 0.13
Nodes (41): AttackVector, ComparedAttackStatus, CreateReportRequest, empty_json_object(), HardenCandidate, HardenCandidateOperation, HardenRejection, HardenRejectionReason (+33 more)

### Community 25 - "settings_update.rs"
Cohesion: 0.22
Nodes (23): app_with_owner(), environment_checker_modes_get_without_override_returns_all_inherit(), environment_checker_modes_round_trip(), get_request(), patch_settings_is_scoped_by_workspace_header(), patch_settings_rejects_invalid_mode_string(), patch_settings_rejects_non_numeric_retention_days(), patch_settings_rejects_unknown_default_action() (+15 more)

### Community 26 - "types.py"
Cohesion: 0.02
Nodes (228): BaseModel, AgentAuthority, AgenticPaymentDecision, AgenticPaymentMandateScope, AgenticPaymentReservation, AgenticPaymentReservationStatus, AgentListResponse, AgentProfile (+220 more)

### Community 27 - "PostgresStore"
Cohesion: 0.18
Nodes (14): connect(), migrate(), PostgresStore, repair_known_schema_drift(), DbConnection, DbPool, Debug, Decision (+6 more)

### Community 28 - "Client"
Cohesion: 0.07
Nodes (14): browserRunContext(), buildFinancialOperationRequest(), cleanFinancialOperationField(), Client, runContext(), RunContextStore, stringifyJson(), AgenticPaymentRecord (+6 more)

### Community 29 - "ReviewQueueContent.tsx"
Cohesion: 0.11
Nodes (22): ReviewActionDialog(), ReviewActionDialogProps, Verdict, Filter, FILTERS, isActionableVerdict(), OUTCOME_LABEL, reasonOf() (+14 more)

### Community 36 - "DashboardAdminStoreError"
Cohesion: 0.14
Nodes (18): DashboardAdminStoreError, NewApiKey, Option, String, Uuid, dashboard_admin_store_error(), PostgresDashboardAdminAdapter, Arc (+10 more)

### Community 37 - "Result"
Cohesion: 0.08
Nodes (63): action_from_record(), approval_from_record(), clean_operation(), clean_optional(), clean_required(), enum_from_text(), enum_text(), event_from_record() (+55 more)

### Community 38 - "AnalyticsChartGrid.tsx"
Cohesion: 0.05
Nodes (59): AnalyticsChartGrid(), AnalyticsChartGridProps, AnalyticsWidget(), applyGridOrder(), DEFAULT_LAYOUT, DEFAULT_VIEW, DIMENSION_LABELS, dimensionLabel() (+51 more)

### Community 39 - "param_auth.rs"
Cohesion: 0.09
Nodes (44): origin_str(), Origin, source(), allowed(), authority_param(), content_bearing_params_are_ignored(), content_param(), correct_source_yields_no_findings() (+36 more)

### Community 40 - "PostgresGatewayAdapter"
Cohesion: 0.15
Nodes (13): gateway_store_error(), PostgresGatewayAdapter, Arc, GatewayProviderConnection, GatewayRoute, GatewayRoutePatch, NewGatewayProviderConnection, NewGatewayRoute (+5 more)

### Community 42 - "llm_pricing.rs"
Cohesion: 0.06
Nodes (41): cost_minor(), cost_nanos(), default_table(), deployment_prefixes_suffix_match(), known_model_prices_exactly(), LlmPricingStoreError, LlmPricingTable, MemoryLlmPricingStore (+33 more)

### Community 43 - "latest_review_outcomes"
Cohesion: 0.15
Nodes (20): latest_review_outcomes(), parse_review_outcome(), DateTime, DbConnection, DbPool, Debug, Formatter, HashMap (+12 more)

### Community 44 - "SDK-Driven Development at TrustLoopGuard"
Cohesion: 0.11
Nodes (18): 1. Engine-only PRs aren't done, 2. No internal imports in `demo/`, 3. Cross-cutting concerns live in the SDK, once, Direct event submission, Gateway contract, How features are built (the loop), MCP adapter, Out of scope (+10 more)

### Community 47 - "tests.rs"
Cohesion: 0.06
Nodes (74): run_dispatch(), account_workflow_profile(), create_report_mints_share_for_complete_job(), create_report_rejects_incomplete_job(), create_report_rejects_self_comparison(), dispatch_message(), dispatch_rejects_invalid_target(), dispatch_req() (+66 more)

### Community 48 - "create_knowledge_source"
Cohesion: 0.17
Nodes (17): create_knowledge_source(), get_knowledge_source_file(), list_knowledge_sources(), CreateKnowledgeSourceRequest, HeaderMap, Json, Path, Response (+9 more)

### Community 50 - "label.rs"
Cohesion: 0.19
Nodes (17): Confidentiality, Integrity, LabelBasis, LabelBasisSet, LabelPolicyStatus, LabelResolution, Labels, Origin (+9 more)

### Community 52 - "auth.rs"
Cohesion: 0.10
Nodes (58): blank_principal_is_stored_as_unbound(), create_api_key_request_with_principal(), internal_bearer_can_revoke_workspace_keys(), internal_bearer_with_forwarded_user_can_issue_workspace_key_used_by_sdk_runtime(), internal_bearer_without_forwarded_user_cannot_issue_workspace_key(), key_bound_to_principal_carries_it_into_handler_context(), key_without_principal_keeps_context_principal_none(), local_dev_missing_forwarded_user_is_unauthorized() (+50 more)

### Community 53 - "Ownership"
Cohesion: 0.17
Nodes (11): 1. Lane ownership, 2. Critical-path crates, 3. Contracts (the only cross-lane surface), 4. Wire-format versioning, 5. Demo independence checkpoint, 6. Conflict resolution, 7. When to split further, Founder A — Engine + Plugin SDK (+3 more)

### Community 55 - "Domain terms"
Cohesion: 0.03
Nodes (70): Action vs Verdict, Agent, Agent profile, Agentic payment, Approval rule, Attack success rate, Authority-bearing parameter, Automated intervention (+62 more)

### Community 56 - "JwtSigner"
Cohesion: 0.18
Nodes (20): access_token_carries_workspace_and_type(), Claims, JwtError, JwtSigner, rejects_garbage(), rejects_wrong_secret(), round_trip_mints_and_verifies(), Arc (+12 more)

### Community 57 - "agent.rs"
Cohesion: 0.13
Nodes (19): AgentAuthority, AgentScope, AgentTone, AgentAuthority, AgentListResponse, AgentProfile, AgentScope, AgentTone (+11 more)

### Community 58 - "tests.rs"
Cohesion: 0.19
Nodes (14): allow_output(), default_runner_with_no_policies_yields_allow(), different_request_misses_cache(), empty_engine_allows(), MockRunner, req(), CancellationToken, second_identical_request_hits_cache() (+6 more)

### Community 60 - "agents.ts"
Cohesion: 0.09
Nodes (20): COMPLETE_DETAIL, mockState, QUEUED, SAVED_PLAN, agentListSchema, AgentProfile, agentProfileWireSchema, AgentSummary (+12 more)

### Community 61 - "report.rs"
Cohesion: 0.13
Nodes (33): ComparedAttackStatus, aggregate(), aggregates_exclude_clean_control_from_denominator(), blocked_and_clean_are_informational_with_no_evidence(), build_report(), categorize(), compared_attacks(), compared_status() (+25 more)

### Community 62 - "provider_record_to_wire"
Cohesion: 0.09
Nodes (24): parse_provider_kind(), provider_record_to_wire(), route_record_to_wire(), DateTime, GatewayProviderConnection, GatewayProviderKind, GatewayRoute, Result (+16 more)

### Community 63 - "._run_with_retry"
Cohesion: 0.06
Nodes (30): RunListResponse, FinancialActionDecisionReceipt, FinancialActionListResponse, FinancialApprovalRequestListResponse, FinancialMandateListResponse, FinancialOutcomeListResponse, FinancialPolicyListResponse, FinancialReceipt (+22 more)

### Community 64 - "run_summary"
Cohesion: 0.13
Nodes (20): event_summary(), p95(), Option, Result, RunEventSummary, RunSummary, Vec, run_summary() (+12 more)

### Community 65 - "EnvironmentStoreError"
Cohesion: 0.07
Nodes (48): create_environment(), delete_environment(), environment_id_from_headers(), EnvironmentState, EnvironmentStoreError, list_environments(), ensure_default(), MemoryEnvironmentStore (+40 more)

### Community 66 - "properties"
Cohesion: 0.08
Nodes (24): properties, required, type, anyOf, Action, type, default, default (+16 more)

### Community 68 - "GuardEvent.ts"
Cohesion: 0.06
Nodes (30): GuardToolCallOptions, Action, AllowedSource, ApprovalRule, Confidentiality, EventKind, Integrity, LabelBasis (+22 more)

### Community 69 - "rustApiForAuthorizedWorkspace"
Cohesion: 0.06
Nodes (28): GET(), PUT(), RouteContext, stringListSchema, AGENT, MockRustApiError, MockWorkspaceAccessError, rustMock (+20 more)

### Community 70 - "scripts"
Cohesion: 0.07
Nodes (29): scripts, build, codegen, codegen:check, coverage:backend, coverage:backend:lcov, coverage:frontend, dead-code:check (+21 more)

### Community 71 - "guard.ts"
Cohesion: 0.03
Nodes (81): agentProfile(), client(), decision(), policyDocument(), runSummary(), ClientOptions, CODE_TO_CLASS, codeFromHttpStatus() (+73 more)

### Community 72 - "tlClientForRequest"
Cohesion: 0.09
Nodes (25): POST(), RouteContext, POST(), RouteContext, POST(), requestSchema, DraftingClient, DraftResponse (+17 more)

### Community 73 - "_shared.ts"
Cohesion: 0.09
Nodes (31): GET(), RouteContext, POST(), RouteContext, GET(), GET(), DELETE(), PATCH() (+23 more)

### Community 76 - "workspace_id_from_headers"
Cohesion: 0.14
Nodes (33): batch_set_policy_enabled(), delete_policy(), get_policy(), list_policies(), parse_policy_family(), read_policy_family(), Bytes, HeaderMap (+25 more)

### Community 78 - "models.rs"
Cohesion: 0.13
Nodes (69): ApprovalRequestRecord, BudgetAlertConfigRecord, BudgetAlertFiringRecord, EscalationRecord, FinancialActionEventRecord, FinancialActionOutcomeRecord, FinancialActionRecord, FinancialLedgerEntryRecord (+61 more)

### Community 79 - "RunDetailLiveView.tsx"
Cohesion: 0.08
Nodes (46): BudgetDecisionCard(), buildGuardFlow(), buildRows(), DeliveryInterventionDetail(), DetailItem(), displayPolicy(), displayReason(), displayUserPrompt() (+38 more)

### Community 80 - "synthesis.rs"
Cohesion: 0.11
Nodes (42): action_candidate_backstop_matches_review_bypass_not_policy_questions(), Candidate, classifies_action_claim_from_reply_assertion(), classifies_configured_workflow_before_generic_action(), classifies_credential_from_reply_token(), classifies_pii_from_goal(), classifies_refund_workflow_before_generic_action(), classifies_system_prompt() (+34 more)

### Community 81 - "properties"
Cohesion: 0.11
Nodes (19): items, type, properties, required, type, items, type, ApprovalRule (+11 more)

### Community 82 - "ToolHandlers"
Cohesion: 0.13
Nodes (5): ToolHandlers, registerTrustLoopTools(), runEventRequest(), runRequest(), traceInput()

### Community 83 - "redteam-jobs.ts"
Cohesion: 0.07
Nodes (27): CreateReportInput, DispatchBody, DispatchInput, DocumentTemplateInput, DocumentTemplateWire, errorEnvelopeSchema, jobStatusSchema, ListJobsParams (+19 more)

### Community 84 - "AgentRepo"
Cohesion: 0.06
Nodes (46): AgentStoreError, MemoryAgentStore, AgentProfile, Arc, HashMap, Result, RwLock, Self (+38 more)

### Community 86 - "AuthConfig"
Cohesion: 0.14
Nodes (23): AuthConfig, EnvError, require_bearer(), require_internal_bearer(), Arc, Debug, Formatter, Into (+15 more)

### Community 88 - "RunnerError"
Cohesion: 0.08
Nodes (30): RedteamPlanner, RedteamRunner, RedteamRunnerClient, Client, Error, Into, Option, Result (+22 more)

### Community 89 - "Load test"
Cohesion: 0.29
Nodes (6): Load test, Prerequisites, Run, Scenarios, What's NOT here, What to look for

### Community 92 - "change_password"
Cohesion: 0.18
Nodes (19): AuthRequest, ChangePasswordRequest, change_password(), login(), Json, Response, signup(), change_password_same_as_current_is_400() (+11 more)

### Community 93 - "pipeline_e2e.rs"
Cohesion: 0.05
Nodes (83): combine_all_trusted_is_trusted(), combine_any_untrusted_is_untrusted(), combine_confidentiality_takes_max_rank(), combine_integrity_takes_min_rank(), combine_labels(), combine_unknown_conf_outranks_public_only(), combine_unknown_without_untrusted_is_unknown(), confidentiality_rank() (+75 more)

### Community 94 - "schema.rs"
Cohesion: 0.06
Nodes (48): ensure_oauth_user_exists(), ensure_user_exists(), generate_token(), invite_row_to_wire(), DbConnection, Result, String, Uuid (+40 more)

### Community 95 - "attacks-panel.tsx"
Cohesion: 0.05
Nodes (46): AttackButton(), AttackFlow(), AttackFlowProps, AttacksPanel(), AttackTranscript(), buildDocumentTemplate(), bytesToBase64(), ConsoleState (+38 more)

### Community 96 - "RedteamState"
Cohesion: 0.13
Nodes (38): resolve_environment_id(), HeaderMap, Response, Result, String, cancel_job(), create_report(), dispatch_job() (+30 more)

### Community 97 - "gateway.rs"
Cohesion: 0.23
Nodes (15): build_app(), create_common_gateway_config(), create_workspace_key(), gateway_owner_id(), json_request(), read_body(), read_text(), Body (+7 more)

### Community 99 - "UserContext"
Cohesion: 0.12
Nodes (42): AnalyticsState, analytics_user_id(), AnalyticsUserId, authorize_analytics_workspace(), forwarded_user_id(), require_workspace_member(), Arc, Extension (+34 more)

### Community 100 - "run-detail-live.ts"
Cohesion: 0.10
Nodes (36): budgetDecisionSchema, budgetWindowSchema, defaultEventLabel(), eventSnapshot(), guardrailUsageSchema, latestUserDisplayText(), objectSchema, providerUsageSchema (+28 more)

### Community 101 - "share.rs"
Cohesion: 0.12
Nodes (27): create_then_get_round_trips(), expired_share_reads_as_not_found(), generate_share_token(), is_expired(), MemoryRedteamReportShareStore, MemShare, new_share(), NewReportShare (+19 more)

### Community 103 - "checker_enforcement.rs"
Cohesion: 0.14
Nodes (45): all_none_override_inherits_workspace_modes(), app_with_approval_mode(), app_with_modes(), app_with_override(), approval_enforce_escalates_tool_requiring_approval(), approval_enforce_ignores_tools_without_approval_rules(), approval_escalation_enqueues_existing_worker_payload(), approval_shadow_keeps_decision_unchanged() (+37 more)

### Community 104 - "mod.rs"
Cohesion: 0.07
Nodes (71): checker_ctx(), client_submitted_checker_evidence_never_survives(), composer_applies_worst_finding_and_copies_evidence_fields(), composer_ignores_signals_for_verdict(), composer_keeps_decision_when_no_finding_carries_a_verdict(), composer_never_downgrades_the_seeded_verdict(), composer_upgrades_rewrite_seed_and_preserves_it_against_weaker_findings(), ctx_with_metadata() (+63 more)

### Community 105 - "check.ts"
Cohesion: 0.15
Nodes (27): assertProviderSuccess(), providerRequest(), restoreEnv(), testProviderAuthAndSimulation(), testStripeSafetyAndMapping(), handleHttpRequest(), handleProviderPayment(), providerApiKey() (+19 more)

### Community 106 - "path"
Cohesion: 0.12
Nodes (28): deadline_exceeded_yields_timeout(), malformed_inner_json_yields_parse_error(), non_2xx_yields_status_error(), ok_response(), openai_sends_bearer_auth_and_json_schema_body(), openrouter_adds_http_referer(), schema(), generate_404_maps_to_not_found() (+20 more)

### Community 107 - "Policy"
Cohesion: 0.12
Nodes (29): CheckRequest, CreateRunEventRequest, Default, RedactionInfo, absent_domain_defaults_to_customer_support(), agent_scope_matches(), channel_scope_matches(), domain_scope_matches() (+21 more)

### Community 108 - "normalization.rs"
Cohesion: 0.19
Nodes (21): seal_provider_key(), normalize_gateway_route(), normalize_gateway_route_patch(), normalize_optional_text(), normalize_optional_url(), normalize_provider_connection(), normalize_provider_connection_patch(), provider_kind_storage_text() (+13 more)

### Community 109 - "event_ingestion.rs"
Cohesion: 0.15
Nodes (38): app(), CannedLlmClient, CannedLlmResponse, direct_event_cannot_spoof_gateway_to_skip_run_stats(), direct_event_rejects_run_event_from_another_run(), direct_event_with_run_updates_run_stats(), full_evidence_flows_to_trace(), json_request() (+30 more)

### Community 110 - "RetryConfig"
Cohesion: 0.05
Nodes (71): Exception, RateLimited, code_from_http_status(), Decode, Forbidden, from_response(), Gone, Internal (+63 more)

### Community 111 - "AnalyticsStoreError"
Cohesion: 0.14
Nodes (21): AnalyticsStoreError, MemoryAnalyticsStore, AnalyticsDashboardView, AnalyticsFacetCatalogResponse, AnalyticsQueryRequest, AnalyticsQueryResponse, CreateAnalyticsDashboardViewRequest, HashMap (+13 more)

### Community 112 - "redteam-report.ts"
Cohesion: 0.08
Nodes (26): COLORS, COMPARISON_STATUS, ComparisonSection(), Finding(), formatDate(), outcomeStyle(), pct(), ReportDocument() (+18 more)

### Community 113 - "tests.rs"
Cohesion: 0.23
Nodes (19): authority_violation_blocks(), CannedClient, ctx_with(), empty_router_yields_skipped(), FixedResolver, hallucination_violation_blocks(), no_profile_yields_skipped(), pre_cancelled_token_short_circuits() (+11 more)

### Community 116 - "create_api_key"
Cohesion: 0.17
Nodes (29): ApiKeyBatchRevokeRequest, DashboardAdminState, batch_revoke_api_keys(), create_api_key(), generate_plaintext_key(), get_environment_checker_modes(), get_settings(), list_api_keys() (+21 more)

### Community 117 - "AppState"
Cohesion: 0.20
Nodes (27): agent_routes(), analytics_routes(), auth_identity_routes(), budget_alert_routes(), dashboard_admin_routes(), environment_routes(), financial_routes(), guardrail_routes() (+19 more)

### Community 118 - "AnalyticsDashboardWidget.ts"
Cohesion: 0.11
Nodes (18): AnalyticsCatalogDimension, AnalyticsCatalogMetric, AnalyticsChartType, AnalyticsDashboardView, AnalyticsDashboardViewConfig, AnalyticsDashboardViewListResponse, AnalyticsDashboardWidget, AnalyticsDimension (+10 more)

### Community 119 - "RedteamReportShareRepo"
Cohesion: 0.16
Nodes (16): NewShare, parse_uuid(), RedteamReportShareRepo, ReportShareRow, DateTime, DbConnection, DbPool, Debug (+8 more)

### Community 120 - "page.tsx"
Cohesion: 0.09
Nodes (14): AUTHORIZATION_CHECKS, ControlLoop(), OUTCOMES, Cta(), DECISION_FIELDS, Evidence(), Hero(), PROOF_POINTS (+6 more)

### Community 121 - "Technical terms"
Cohesion: 0.06
Nodes (35): Attack plan, Attack runner, Attack vector, Cache key, Cold path, Decision log, Embedded mode, Escalation worker (+27 more)

### Community 122 - "ManagementPages.tsx"
Cohesion: 0.05
Nodes (36): AppLayoutProps, DataTableColumn, columns, Row, rows, EmptyState(), EmptyStateProps, DashboardUsageData (+28 more)

### Community 124 - "dashboard_admin_repo.rs"
Cohesion: 0.15
Nodes (27): DashboardAdminRepo, environment_checker_modes_from_record(), EnvironmentCheckerModesRecord, EnvironmentCheckerModesWriteRecord, mode_to_db(), optional_mode_to_db(), parse_data_handling_mode(), parse_enforcement_mode() (+19 more)

### Community 125 - "harden-job-card.tsx"
Cohesion: 0.10
Nodes (26): coverageLabel(), draftPolicyFromSessions(), HardenJobCard(), HardenJobCardProps, messageOf(), newPolicyHref(), operationLabel(), rejectionSummary() (+18 more)

### Community 126 - "financial_authorization_service.rs"
Cohesion: 0.06
Nodes (71): executable_refund_request(), financial_policy(), mandate_request(), mandate_request_with_scope(), outcome(), payment_financial_policy(), payment_request(), refund_request() (+63 more)

### Community 127 - "family_parse.rs"
Cohesion: 0.11
Nodes (42): approval_requires_at_least_one_condition(), documented_family_examples_parse(), existing_content_examples_parse_via_load_any_str(), family(), family_id_uses_content_slug_rule(), family_less_yaml_parses_as_content_identical_to_load_str(), family_policies_round_trip_through_yaml_with_family_tag(), FamilyProbe (+34 more)

### Community 128 - "AgentProfile.ts"
Cohesion: 0.17
Nodes (10): UpsertAgentInput, AgentAuthority, AgentListResponse, AgentProfile, AgentScope, AgentTone, KnowledgeSource, KnowledgeSourceKind (+2 more)

### Community 129 - "BudgetAlertRepo"
Cohesion: 0.11
Nodes (29): BudgetAlertRepo, NewBudgetAlertConfigParams, NewBudgetAlertFiringParams, parse_config_id(), DateTime, DbConnection, DbPool, Debug (+21 more)

### Community 130 - "analytics.rs"
Cohesion: 0.24
Nodes (23): AnalyticsCatalogDimension, AnalyticsCatalogMetric, AnalyticsChartType, AnalyticsDashboardView, AnalyticsDashboardViewConfig, AnalyticsDashboardViewListResponse, AnalyticsDashboardWidget, AnalyticsDimension (+15 more)

### Community 131 - "KnowledgeSourceDocument.ts"
Cohesion: 0.22
Nodes (7): CreateKnowledgeSourceRequest, DashboardKnowledgeSourceKind, KnowledgeFileInput, KnowledgeSourceDocument, RFC-3339, KnowledgeSourceListResponse, KnowledgeSourceStatus

### Community 133 - "HnswFuzzyChecker"
Cohesion: 0.13
Nodes (20): BuildError, dedup_when_both_tiers_match_same_policy(), empty_policies_yields_no_hits(), HnswFuzzyChecker, levenshtein_catches_typo_bypass(), levenshtein_misses_unrelated_text(), Arc, HashMap (+12 more)

### Community 134 - "policy_cli.rs"
Cohesion: 0.19
Nodes (21): Command, find_header_end(), policy_pull_writes_source_yaml_to_file(), policy_push_posts_yaml_to_server(), policy_push_rejects_family_yaml_with_clear_error(), policy_validate_reports_valid_family_yaml(), policy_validate_reports_valid_yaml(), read_http_request() (+13 more)

### Community 135 - "tl-client.ts"
Cohesion: 0.08
Nodes (39): GET(), MyWorkspace, MyWorkspacesResponse, POST(), userFromSession(), requestSchema, POST(), ConsentForm() (+31 more)

### Community 136 - "adapter.ts"
Cohesion: 0.06
Nodes (58): ArenaAdapterChatRequest, ArenaAdapterChatResult, ArenaAdapterFinishReason, ArenaAdapterHandlers, ArenaAdapterPhase, ArenaAdapterProfile, ArenaAdapterServer, ArenaAdapterVerdict (+50 more)

### Community 138 - "compilerOptions"
Cohesion: 0.09
Nodes (21): compilerOptions, esModuleInterop, exactOptionalPropertyTypes, isolatedModules, module, moduleResolution, noEmit, noUnusedLocals (+13 more)

### Community 141 - "Write Your First Policy"
Cohesion: 0.06
Nodes (27): Copyable Policy Examples, Legal Advice Escalation, PII Block, Refund Guarantee Rewrite, Voice-Only Disclosure, CLI Workflow, Cloud Mode, Hybrid Mode (+19 more)

### Community 143 - "plan-card.tsx"
Cohesion: 0.15
Nodes (18): humanizeToken(), PlanStep(), PlanSummary(), PlanVectors(), SEVERITY_ORDER, SEVERITY_STYLE, SEVERITY_TIERS, severityOfTechnique() (+10 more)

### Community 144 - "FinancialActionRecord"
Cohesion: 0.07
Nodes (30): action(), apiAction(), buildRefundRequest(), createRefundMandate(), FinancialDemoClient, REFUND_SCENARIOS, RefundScenario, runRefundDemo() (+22 more)

### Community 145 - "event_policy.rs"
Cohesion: 0.12
Nodes (46): all_literal_miss_does_not_call_semantic_judge(), any_literal_match_does_not_call_semantic_judge(), apply_semantic_policy_result(), channel_name(), ClauseDecision, eval_ctx(), evaluate_event_policies(), evaluate_semantic_policies() (+38 more)

### Community 146 - "server.ts"
Cohesion: 0.10
Nodes (21): agentInput(), agentSchema, allowedSourceInput(), approval, approvalInput(), guardEvent, jsonObject, jsonValue (+13 more)

### Community 147 - "MemoryBudgetAlertStore"
Cohesion: 0.18
Nodes (16): config(), config_names_are_unique_within_each_spend_meter(), config_round_trip_and_name_conflict(), firing(), firing_dedup_is_per_config_principal_window(), MemoryBudgetAlertStore, BudgetAlertConfig, BudgetAlertFiring (+8 more)

### Community 149 - "api_error_response"
Cohesion: 0.23
Nodes (20): delete_label_policy(), get_label_policy(), invalid_origin_response(), LabelPolicyState, list_label_policies(), parse_origin(), Arc, HeaderMap (+12 more)

### Community 151 - "package.json"
Cohesion: 0.22
Nodes (8): description, engines, node, license, name, packageManager, private, version

### Community 152 - "PoliciesPageContent.tsx"
Cohesion: 0.02
Nodes (99): AlertDialog(), AlertDialogAction(), AlertDialogCancel(), AlertDialogContent(), AlertDialogDescription(), AlertDialogFooter(), AlertDialogHeader(), AlertDialogMedia() (+91 more)

### Community 153 - "RedteamJobStoreError"
Cohesion: 0.12
Nodes (24): RedteamJobStoreError, is_loopback_target(), RedteamDispatchRequest, Result, validate_attack_vectors(), validate_dispatch(), validate_document_template(), validate_template_fields() (+16 more)

### Community 154 - "gateway_budget.rs"
Cohesion: 0.20
Nodes (47): actions_meter_policy_does_not_gate_llm_calls(), admin_request(), at_cap_denies_without_calling_upstream(), build_app(), chat_request(), concurrent_requests_cannot_reserve_the_same_remaining_budget(), create_common_gateway_config(), create_extra_runtime_key() (+39 more)

### Community 155 - "index.ts"
Cohesion: 0.03
Nodes (89): ActiveRun, ActiveRunContext, FinancialOperationRunOptions, FinancialOperationSpec, ListTracesOptions, WithRunOptions, AgenticPaymentAuthorizationResponse, AgenticPaymentAuthorizeRequest (+81 more)

### Community 156 - "seo-landing-page.tsx"
Cohesion: 0.12
Nodes (21): metadata, Page, metadata, Page, metadata, Page, metadata, Page (+13 more)

### Community 158 - "type"
Cohesion: 0.22
Nodes (11): default, type, null, string, description, type, description, owner_agent_id (+3 more)

### Community 159 - "Result"
Cohesion: 0.12
Nodes (19): Client, Client, CreateRunEventRequest, CreateRunRequest, Decision, F, GuardEvent, Option (+11 more)

### Community 160 - "MemoryFinancialStore"
Cohesion: 0.09
Nodes (37): clean_optional(), clean_required(), key(), mandate_key(), MemoryAgenticPayments, MemoryAgenticPaymentSession, MemoryFinancialStore, MemoryLedgerEntry (+29 more)

### Community 161 - "properties"
Cohesion: 0.14
Nodes (14): type, $ref, type, properties, agent_id, authority, display_name, scope (+6 more)

### Community 162 - "type"
Cohesion: 0.13
Nodes (16): properties, type, default, items, type, default, items, type (+8 more)

### Community 163 - "BudgetAlertStoreError"
Cohesion: 0.20
Nodes (14): BudgetAlertStoreError, budget_alert_store_error(), config_from_stored(), conflict_aware_error(), firing_from_stored(), PostgresBudgetAlertAdapter, Arc, BudgetAlertConfig (+6 more)

### Community 164 - "Repository Agent Instructions"
Cohesion: 0.15
Nodes (12): Architecture: Rust Backend Is the Source of Truth, Coding Conventions, Docs Are the Single Source of Truth (`docs/concept`), General Coding Discipline, Goal-Driven Execution, Implementation Checklist, Page Integration Expectations, Repository Agent Instructions (+4 more)

### Community 165 - "Result"
Cohesion: 0.24
Nodes (13): any_policy_row_from_record(), policy_family_from_storage(), policy_from_json(), policy_from_storage(), policy_row_from_record(), PolicyRepo, Arc, Option (+5 more)

### Community 166 - "MemoryRedteamJobStore"
Cohesion: 0.13
Nodes (19): event_text(), MemoryRedteamJobStore, HashMap, JobCounts, JobStatus, Option, RedteamAttackRecord, RedteamAttackRecordFilter (+11 more)

### Community 167 - "TierResult"
Cohesion: 0.17
Nodes (12): TierResult, format, minimum, type, elapsed_ms, status, tier, $ref (+4 more)

### Community 168 - "tier_results"
Cohesion: 0.15
Nodes (13): $ref, reasons, tier_results, triggered_policies, default, items, type, default (+5 more)

### Community 170 - "v0 Design Decisions"
Cohesion: 0.07
Nodes (30): 10. Crate alignment, 11. Build order (v0), 12. Open questions (need answers before phase 1), 13. Things deliberately not in v0, 14. Confirmation checklist, 15. Event-centered runtime (locked), 16. Enforcement is an opt-in rollout (locked), 17. Labeling strategy: structure-first, fail-closed for authority (locked) (+22 more)

### Community 171 - "Runtime Refactor Jobs"
Cohesion: 0.07
Nodes (28): Continuation Readability Pass, Current Status, Final Acceptance Gates, Phase 0: Baseline Evidence, Phase 1: Server Shell Cleanup, Phase 2: Guard Service Extraction, Phase 3: App State Decomposition, Phase 4: Gateway Decomposition (+20 more)

### Community 172 - "agents"
Cohesion: 0.18
Nodes (12): default, items, type, WhenClause, default, items, type, type (+4 more)

### Community 173 - "FinancialStoreError"
Cohesion: 0.08
Nodes (31): FinancialStoreError, financial_store_error(), PostgresFinancialAdapter, AgenticPaymentReservation, ApprovalRequirement, Arc, CreateFinancialActionRequest, CreateFinancialMandateRequest (+23 more)

### Community 174 - "in_scope"
Cohesion: 0.18
Nodes (11): properties, type, AgentScope, default, items, type, default, items (+3 more)

### Community 175 - "properties"
Cohesion: 0.09
Nodes (32): type, type, type, type, type, array, null, string (+24 more)

### Community 176 - "definitions"
Cohesion: 0.40
Nodes (5): definitions, MatchClause, Matcher, anyOf, oneOf

### Community 178 - "package.json"
Cohesion: 0.05
Nodes (42): dependencies, fumadocs-core, fumadocs-mdx, fumadocs-openapi, fumadocs-ui, next, react, react-dom (+34 more)

### Community 179 - "forbidden"
Cohesion: 0.29
Nodes (7): properties, default, items, type, forbidden, target, type

### Community 180 - "route.ts"
Cohesion: 0.73
Nodes (3): GET(), authSignOutRedirectUrl(), isAuthSignOutGet()

### Community 181 - "definitions"
Cohesion: 0.15
Nodes (13): definitions, RedactionMode, Tier, TierStatus, enum, type, description, oneOf (+5 more)

### Community 182 - "TriggeredPolicy"
Cohesion: 0.15
Nodes (13): TriggeredPolicy, type, id, reason, id, reason, severity, type (+5 more)

### Community 185 - "proxy-helpers.ts"
Cohesion: 0.15
Nodes (18): GET(), POST(), DELETE(), PATCH(), GET(), POST(), PATCH(), GET() (+10 more)

### Community 190 - "financial.rs"
Cohesion: 0.10
Nodes (66): AgenticPaymentAuthorizationResponse, AgenticPaymentAuthorizeRequest, AgenticPaymentCommitRequest, AgenticPaymentDecision, AgenticPaymentMandateScope, AgenticPaymentRecord, AgenticPaymentReservation, AgenticPaymentReservationStatus (+58 more)

### Community 191 - "latest_review_outcomes"
Cohesion: 0.15
Nodes (14): latest_review_outcomes(), DateTime, DbConnection, HashMap, HumanReviewOutcome, Result, TraceReviewLookupRow, Utc (+6 more)

### Community 194 - "pull_request_template.md"
Cohesion: 0.25
Nodes (7): 🔁 Cross-cutting concerns, 👀 Reviewer prompt, 🧩 SDK-parity checklist, 📝 Summary, ✅ Test plan, 🧭 Type of change, 🎨 UI Changes

### Community 195 - "WorkspaceInvite.ts"
Cohesion: 0.14
Nodes (12): CreateInviteRequest, CreateInviteResponse, InviteListResponse, InviteStatus, MemberListResponse, MyWorkspace, MyWorkspacesResponse, RFC-3339 (+4 more)

### Community 196 - "validation.rs"
Cohesion: 0.19
Nodes (23): Box, document_family(), FamilyTag, is_yaml_content_type(), parse_document(), parse_policy(), parse_policy_body(), ParsedPolicyBody (+15 more)

### Community 198 - "agent-profile.schema.json"
Cohesion: 0.18
Nodes (10): description, agent_id, required, $schema, title, type, authority, display_name (+2 more)

### Community 199 - "default"
Cohesion: 0.33
Nodes (6): agents, channels, domains, when, allOf, default

### Community 200 - "PolicyRepo"
Cohesion: 0.10
Nodes (30): AnyPolicyRow, cache_key(), ensure_all_policies_exist(), ensure_policy_exists(), load_any_deployment_records(), PolicyRepo, Arc, DbConnection (+22 more)

### Community 201 - "workflow_requirements"
Cohesion: 0.20
Nodes (10): $ref, default, items, type, knowledge_sources, workflow_requirements, default, description (+2 more)

### Community 202 - "compilerOptions"
Cohesion: 0.09
Nodes (21): compilerOptions, allowSyntheticDefaultImports, esModuleInterop, exactOptionalPropertyTypes, forceConsistentCasingInFileNames, isolatedModules, lib, module (+13 more)

### Community 203 - "channels"
Cohesion: 0.40
Nodes (5): default, items, type, $ref, channels

### Community 204 - "policy.schema.json"
Cohesion: 0.25
Nodes (7): action, id, required, $schema, title, type, match

### Community 205 - "lint-web-backend-only.sh"
Cohesion: 0.60
Nodes (5): is_server_side(), scan_browser_only_rules(), scan_file(), scan_provider_sdks_anywhere(), lint-web-backend-only.sh script

### Community 206 - "Client"
Cohesion: 0.07
Nodes (35): Client, FinancialOperation, AgenticPaymentAuthorizationResponse, AgenticPaymentAuthorizeRequest, AgenticPaymentCommitRequest, AgenticPaymentRecord, AgenticPaymentRollbackRequest, CounterpartyRef (+27 more)

### Community 208 - "lint-no-internal-imports.sh"
Cohesion: 0.70
Nodes (4): scan_python(), scan_rust(), scan_typescript(), lint-no-internal-imports.sh script

### Community 210 - "FamilyPolicy"
Cohesion: 0.11
Nodes (31): AnyPolicy, ApprovalPolicy, ApprovalWhen, default_block_action(), default_escalate_action(), default_severity(), FamilyPolicy, FinancialPolicy (+23 more)

### Community 211 - "budget_alerts.rs"
Cohesion: 0.09
Nodes (31): BudgetAlertRuntime, crossed(), deliver_firing(), evaluate_spend_alerts(), firing_payload(), meter_from_str(), meter_label(), min_window_caps() (+23 more)

### Community 213 - "PolicyStoreError"
Cohesion: 0.08
Nodes (38): any_policy_document(), any_policy_summary(), normalize_policy_ids(), policy_action(), policy_document(), policy_summary(), Action, PolicyDocument (+30 more)

### Community 214 - "MemoryRunStore"
Cohesion: 0.15
Nodes (16): MemoryRunStore, p95_latency(), CreateRunEventRequest, CreateRunRequest, HashMap, Option, Result, RunEventSummary (+8 more)

### Community 216 - "RedteamJobStore"
Cohesion: 0.10
Nodes (35): RedteamJobStore, Send, Sync, DispatchConfig, DispatchJob, DispatchOutcome, drive(), is_cancelled() (+27 more)

### Community 220 - "Decision.ts"
Cohesion: 0.09
Nodes (19): CheckerFindingEvidence, CheckerRun, DataHandlingMode, EnforcementMode, EnvironmentCheckerModes, RFC-3339, RedactedEntity, RedactionInfo (+11 more)

### Community 222 - "validation.rs"
Cohesion: 0.14
Nodes (25): accepts_max_only_value_limit(), accepts_valid_value_limit(), metadata(), rejects_blank_allowed_source_id(), rejects_blank_approver_role(), rejects_blank_tool_name(), rejects_duplicate_param_paths(), rejects_empty_param_path() (+17 more)

### Community 223 - "dashboard.rs"
Cohesion: 0.18
Nodes (23): ApiKeyBatchRevokeRequest, ApiKeyBatchRevokeResponse, ApiKeyListResponse, CreateApiKeyRequest, CreateApiKeyResponse, CreateWorkspaceEnvironmentRequest, DashboardApiKey, DataHandlingMode (+15 more)

### Community 224 - "policy.rs"
Cohesion: 0.12
Nodes (30): AiEditRequest, AiEditResponse, default_policy_family(), EntityVersionDetail, EntityVersionListResponse, EntityVersionSummary, GuardrailGenerateResponse, GuardrailListResponse (+22 more)

### Community 225 - "redteam_runner.rs"
Cohesion: 0.13
Nodes (35): empty_json_object(), RedteamRunnerContract, HashMap, Option, String, Value, Vec, runner_attack_surface_is_default() (+27 more)

### Community 228 - "PostgresAnalyticsAdapter"
Cohesion: 0.15
Nodes (13): AnalyticsRepo, analytics_store_error(), PostgresAnalyticsAdapter, AnalyticsDashboardView, AnalyticsFacetCatalogResponse, AnalyticsQueryRequest, AnalyticsQueryResponse, Arc (+5 more)

### Community 229 - "value_limit.rs"
Cohesion: 0.17
Nodes (23): absent_param_is_skipped(), allows_amount_at_max_boundary(), allows_amount_at_min_boundary(), allows_amount_under_max(), blocks_when_amount_below_min(), blocks_when_amount_exceeds_max(), bound_finding(), escalates_when_value_is_not_an_integer() (+15 more)

### Community 230 - "metrics.rs"
Cohesion: 0.15
Nodes (23): AnalyticsChartType, AnalyticsDimension, AnalyticsFilter, AnalyticsMetric, BTreeSet, default_chart_type(), dimension_label(), fact_values() (+15 more)

### Community 231 - "RunState"
Cohesion: 0.16
Nodes (29): resolve_environment_id(), HeaderMap, Response, Result, String, create_run(), create_run_event(), get_run() (+21 more)

### Community 232 - "tool_metadata.rs"
Cohesion: 0.25
Nodes (25): app(), delete_then_get_returns_404(), disabled_tool_resolves_as_unregistered(), dotted_tool_name_routes_in_path(), duplicate_param_path_returns_422(), event_request(), event_trace_carries_resolved_metadata(), event_trace_carries_unregistered_resolution() (+17 more)

### Community 233 - "EscalationRepo"
Cohesion: 0.14
Nodes (16): EscalationRepo, EscalationRow, DateTime, DbConnection, DbPool, Debug, Duration, Formatter (+8 more)

### Community 234 - "RouterConfig"
Cohesion: 0.16
Nodes (15): BudgetConfig, ConfigError, empty_budgets_section_uses_default(), ProviderConfig, round_trips_sample_config(), RouteConfig, RouterConfig, AsRef (+7 more)

### Community 237 - "package.json"
Cohesion: 0.06
Nodes (32): bin, trustloopguard-mcp-server, dependencies, @modelcontextprotocol/sdk, @trustloopguard/sdk, zod, description, devDependencies (+24 more)

### Community 238 - "entrypoint"
Cohesion: 0.20
Nodes (11): blocked_reply(), entrypoint(), escalated_reply(), HealthcareAgent, log_guardrail(), Agent, Decision, JobContext (+3 more)

### Community 239 - "setup.ts"
Cohesion: 0.09
Nodes (28): guardedPayout(), headers(), main(), registerTool(), enforceModes(), headers(), main(), tools (+20 more)

### Community 242 - "engine.rs"
Cohesion: 0.12
Nodes (17): Engine, policy_cache_scope(), Arc, Decision, Self, String, Vec, OrchestrateConfig (+9 more)

### Community 244 - "README.md"
Cohesion: 0.07
Nodes (26): Agentic refund authorization, Bring your own agent, LiveKit, Money agent — guarded scenarios (flagship), NorthPay dispute, Stripe refund agent, TrustLoopGuard demos, Backend tests (+18 more)

### Community 246 - "MemoryRedteamPlanStore"
Cohesion: 0.11
Nodes (20): MemoryRedteamPlanStore, RedteamPlanStoreError, AttackVector, RedteamPlanResponse, Result, RwLock, Self, String (+12 more)

### Community 247 - "escalation.rs"
Cohesion: 0.12
Nodes (36): default_retry_policy_is_five_attempts(), deliver_one(), delivery_loop(), EscalationConfig, EscalationPayload, persist_pending(), RetryPolicy, Arc (+28 more)

### Community 248 - ".update_run"
Cohesion: 0.19
Nodes (8): RunStatus, RunSummary, UpdateRunRequest, Async variant of ``Client.update_run``., Async variant of ``Client.finish_run``., Create a run grouping for subsequent ``check`` calls., Update a run's status, metadata, or end timestamp., Mark a run completed, failed, or canceled.

### Community 249 - "SemanticPolicyJudgeResult"
Cohesion: 0.16
Nodes (17): BatchRecordingJudge, json_string_array(), LlmRouter, malformed_semantic_judge_json_returns_error(), RecordingJudge, AtomicUsize, Mutex, String (+9 more)

### Community 250 - "KnowledgeRepo"
Cohesion: 0.15
Nodes (18): KnowledgeFileRow, KnowledgeRepo, KnowledgeSourceRow, NewKnowledgeFile, NewKnowledgeSource, DateTime, DbConnection, DbPool (+10 more)

### Community 252 - "writer.rs"
Cohesion: 0.13
Nodes (23): build_trace_payload(), event(), flush(), DbPool, Decision, Default, Duration, GuardEvent (+15 more)

### Community 253 - "package.json"
Cohesion: 0.06
Nodes (35): dependencies, geist, next, react, react-dom, @t3-oss/env-nextjs, zod, devDependencies (+27 more)

### Community 254 - "RunRepo"
Cohesion: 0.14
Nodes (18): status_text(), CreateRunRequest, DbConnection, DbPool, Debug, Formatter, Option, Result (+10 more)

### Community 255 - "require_approved_user"
Cohesion: 0.19
Nodes (14): forwarded_user_id(), require_approved_user(), Option, Request, Response, Result, Uuid, api_error() (+6 more)

### Community 256 - "Result"
Cohesion: 0.19
Nodes (11): insert_policy_version(), PolicyRepo, DbConnection, Option, PolicyFamily, PolicyRow, Result, String (+3 more)

### Community 257 - ".prettierrc.json"
Cohesion: 0.17
Nodes (11): arrowParens, bracketSameLine, bracketSpacing, endOfLine, printWidth, quoteProps, semi, singleQuote (+3 more)

### Community 258 - "dependencies"
Cohesion: 0.09
Nodes (23): dependencies, class-variance-authority, @dnd-kit/sortable, @dnd-kit/utilities, lucide-react, @monaco-editor/react, radix-ui, @radix-ui/react-dialog (+15 more)

### Community 259 - "MokaCache"
Cohesion: 0.18
Nodes (14): disabled_cache_never_stores(), fake_decision(), miss_returns_none(), MokaCache, put_overwrites_existing_key(), put_then_get_returns_value(), Cache, Decision (+6 more)

### Community 260 - "validate_raw_policy"
Cohesion: 0.23
Nodes (13): create_path_accepts_family_policies(), family_policy_json_validates_through_endpoint_path(), family_policy_yaml_validates_through_endpoint_path(), invalid_family_policy_returns_structured_issues_and_id(), load_str_and_validate_endpoint_agree_on_valid_yaml(), malformed_yaml_returns_validation_issue(), HeaderMap, unknown_family_is_invalid_with_truncated_echo() (+5 more)

### Community 264 - "button.tsx"
Cohesion: 0.04
Nodes (69): ChangePasswordCardProps, AuthScreenProps, BrandRailProps, VERDICTS, buildRetryUrl(), createWorkspace(), firstParam(), readOptionalField() (+61 more)

### Community 265 - "event"
Cohesion: 0.09
Nodes (43): allows_trusted_public_flow_to_external_sink(), blocks_private_source_flowing_to_external_sink(), blocks_untrusted_controlled_high_impact_action(), emits_both_rules_when_both_violated(), escalates_dangling_provenance_source_ids(), escalates_missing_provenance_on_high_impact_action(), escalates_unattributed_provenance_paths(), escalates_unknown_trust_control_on_high_impact_action() (+35 more)

### Community 267 - "tool.rs"
Cohesion: 0.13
Nodes (23): AllowedSource, ApprovalRule, LimitAction, ParamLimit, ParamRole, ParamSpec, AllowedSource, ApprovalRule (+15 more)

### Community 268 - "finalize_gateway_response"
Cohesion: 0.20
Nodes (19): enforcement_headers(), finish_completed(), handle_output_enforcement(), output_blocked_response(), OutputEnforcement, Decision, Option, P (+11 more)

### Community 269 - "label_policy.rs"
Cohesion: 0.24
Nodes (23): app(), delete_then_get_returns_not_found(), disabled_policy_listed_but_not_resolved(), disabled_policy_not_applied_at_runtime(), event_path_decision_unchanged_with_label_policies_configured(), event_request(), invalid_origin_path_rejected(), json_request() (+15 more)

### Community 273 - "properties"
Cohesion: 0.17
Nodes (12): type, items, type, type, $ref, context_redacted, entities, input_redacted (+4 more)

### Community 276 - "harden_job"
Cohesion: 0.13
Nodes (29): candidate_source(), ClassGroup, harden_job(), is_control(), load_workflow_requirements(), match_has_semantic(), matcher_is_semantic(), policy_has_semantic_matcher() (+21 more)

### Community 277 - "gateway.rs"
Cohesion: 0.31
Nodes (13): CreateGatewayProviderConnectionRequest, CreateGatewayRouteRequest, GatewayCredentialStatus, GatewayProviderConnection, GatewayProviderConnectionListResponse, GatewayProviderKind, GatewayRoute, GatewayRouteListResponse (+5 more)

### Community 278 - "spawn_writer"
Cohesion: 0.29
Nodes (16): Sender, spawn_writer(), batch_size_triggers_flush(), caller_send_is_non_blocking_under_load(), event_evidence_round_trips_in_payload(), fake_decision(), fresh_pool(), graceful_shutdown_flushes_remaining() (+8 more)

### Community 279 - "memory.rs"
Cohesion: 0.19
Nodes (12): lock_error(), MemoryGatewayRoute, MemoryGatewayStore, MemoryProviderConnection, GatewayProviderConnection, GatewayRoute, RwLock, Self (+4 more)

### Community 280 - "traces.rs"
Cohesion: 0.08
Nodes (37): ChannelTraceStore, list_traces(), MemoryTraceStore, read_query_param(), Arc, DateTime, Decision, GuardEvent (+29 more)

### Community 281 - "EnvironmentRepo"
Cohesion: 0.18
Nodes (14): clear_default(), environment_to_wire(), EnvironmentRepo, CreateWorkspaceEnvironmentRequest, DbConnection, DbPool, Debug, Formatter (+6 more)

### Community 282 - "load_str"
Cohesion: 0.14
Nodes (27): matches_canonical_scope_fields(), skips_agent_scope_mismatch(), skips_domain_scope_mismatch(), accepts_canonical_scope_fields(), accepts_legacy_channel_scope_field(), content_family_tag_passes_load_str_directly(), documented_examples_parse(), format_issues() (+19 more)

### Community 283 - "runs.rs"
Cohesion: 0.29
Nodes (14): create_gateway_assistant_event(), create_gateway_evidence_event(), create_gateway_provider_failure_event(), create_gateway_run(), create_gateway_turn_event(), finish_gateway_run(), gateway_run_external_id(), CreateRunEventRequest (+6 more)

### Community 284 - "RedteamPlanRepo"
Cohesion: 0.17
Nodes (16): parse_uuid(), plan_response(), PlanBody, RedteamPlanRepo, AttackVector, DbConnection, DbPool, Debug (+8 more)

### Community 285 - "knowledge.rs"
Cohesion: 0.18
Nodes (15): knowledge_kind_text(), knowledge_row_to_document(), parse_knowledge_kind(), parse_knowledge_status(), PostgresKnowledgeAdapter, Arc, CreateKnowledgeSourceRequest, KnowledgeSourceDocument (+7 more)

### Community 287 - "definitions"
Cohesion: 0.10
Nodes (20): definitions, RunnerAttackSurface, RunnerRunMode, RunnerStatus, chat, description, enum, type (+12 more)

### Community 288 - "redteam-core.ts"
Cohesion: 0.13
Nodes (17): ALLOWED_AGENT_HOSTS, REDTEAM_PROFILES, RedteamCase, redteamCaseSchema, redteamLlmSchema, redteamOutcomeSchema, redteamProfileSchema, RedteamReport (+9 more)

### Community 291 - "handlers.rs"
Cohesion: 0.21
Nodes (30): api_error_response(), budget_alert_error_response(), BudgetAlertApiState, clean_optional(), create_budget_alert(), delete_budget_alert(), list_budget_alert_firings(), list_budget_alerts() (+22 more)

### Community 292 - "api_keys.rs"
Cohesion: 0.21
Nodes (20): ApiKeyListRow, api_key_row_to_wire(), ApiKeyAuthRecord, ApiKeyRecord, DashboardAdminRepo, ensure_all_keys_exist(), environment_slug(), load_api_key_rows() (+12 more)

### Community 295 - "plan.rs"
Cohesion: 0.15
Nodes (27): agent_disambiguator(), core_path(), core_vector(), delete_plan(), generate_static_policies(), id_slug(), list_plans(), plan_attack_vectors() (+19 more)

### Community 296 - "aggregate"
Cohesion: 0.22
Nodes (22): BlockSignal, Verdict, JudgeOutcomes, JudgeResult, LlmRouter, run_judges(), aggregate(), apply_authority_verdict() (+14 more)

### Community 297 - "financial_error_response"
Cohesion: 0.21
Nodes (39): FinancialState, approve_action(), authorize_agentic_payment(), commit_agentic_payment(), create_action(), create_mandate(), create_policy(), deny_action() (+31 more)

### Community 298 - "PostgresHumanReviewAdapter"
Cohesion: 0.12
Nodes (16): human_review_store_error(), PostgresHumanReviewAdapter, Arc, CreateHumanReviewEventRequest, HumanReviewAnalyticsFilter, HumanReviewAnalyticsResponse, HumanReviewEvent, Option (+8 more)

### Community 300 - "store.rs"
Cohesion: 0.21
Nodes (11): GatewayRoutePatch, NewGatewayProviderConnection, NewGatewayRoute, ProviderConnectionPatch, ProviderConnectionSecret, ResolvedGatewayRoute, GatewayProviderConnection, GatewayProviderKind (+3 more)

### Community 301 - "event_service.rs"
Cohesion: 0.10
Nodes (30): GuardEvent, HeaderMap, Json, Response, String, submit_event(), workspace_id_for_event(), event() (+22 more)

### Community 302 - ".create_event"
Cohesion: 0.09
Nodes (27): HumanReviewRepo, CreateHumanReviewEventRequest, DbConnection, DbPool, Debug, Formatter, HashMap, HumanReviewEvent (+19 more)

### Community 303 - "Web UI Conventions"
Cohesion: 0.07
Nodes (27): API, API, API, API, BatchActionBar, CopyBlock, Current adopters, Dashboard API Calls (+19 more)

### Community 304 - "MemoryLlmUsageStore"
Cohesion: 0.05
Nodes (71): list_llm_usage(), llm_usage_error_response(), LlmBudgetCapsNanos, LlmBudgetWindow, LlmBudgetWindowSnapshot, LlmUsageFilter, LlmUsageGroupBy, LlmUsageState (+63 more)

### Community 306 - "enforcement.rs"
Cohesion: 0.18
Nodes (10): CheckerFindingEvidence, CheckerFindingEvidence, CheckerRun, EnforcementMode, Option, Severity, String, Vec (+2 more)

### Community 307 - "properties"
Cohesion: 0.06
Nodes (48): type, type, type, default, type, default, type, default (+40 more)

### Community 308 - "trialIndex"
Cohesion: 0.40
Nodes (5): integer, trialIndex, default, format, type

### Community 309 - "package.json"
Cohesion: 0.08
Nodes (25): default, description, devDependencies, typescript, vitest, exports, files, import (+17 more)

### Community 310 - "kind"
Cohesion: 0.13
Nodes (17): properties, required, type, AllowedSource, Source, type, id, origin (+9 more)

### Community 311 - "llm_usage_repo.rs"
Cohesion: 0.10
Nodes (35): AsyncPgConnection, active_reservation_nanos_in_window(), LlmBudgetCapsNanos, LlmBudgetWindow, LlmBudgetWindowSnapshot, LlmUsageBucketRow, LlmUsageEventFilter, LlmUsageGroupBy (+27 more)

### Community 312 - "PostgresTraceAdapter"
Cohesion: 0.22
Nodes (10): PostgresTraceAdapter, Arc, Option, Result, Self, Sender, TraceSummary, Vec (+2 more)

### Community 314 - "fresh_repo"
Cohesion: 0.30
Nodes (14): disabled_row_still_readable_with_flag(), fresh_repo(), get_is_isolated_by_workspace(), insert_and_get_round_trips_typed_metadata(), list_returns_only_active_workspace_rows(), negative_cache_serves_repeated_misses(), ContainerAsync, PostgresImage (+6 more)

### Community 315 - "fresh_pool"
Cohesion: 0.36
Nodes (7): create_event_auto_sequence_is_concurrency_safe(), create_event_rejects_invalid_input(), create_list_and_update_run(), fresh_pool(), ContainerAsync, DbPool, PostgresImage

### Community 316 - "ignoreBinaries"
Cohesion: 0.08
Nodes (26): ignore, ignoreBinaries, ignoreDependencies, ignoreFiles, ignoreIssues, apps/docs/source.config.ts, apps/web/components/ui/**, sdks/typescript/src/generated/** (+18 more)

### Community 317 - "Code of Conduct"
Cohesion: 0.33
Nodes (5): Attribution, Code of Conduct, Enforcement, Our Pledge, Our Standards

### Community 318 - "PostgresLabelPolicyAdapter"
Cohesion: 0.19
Nodes (16): label_policy_store_error(), origin_key(), PostgresLabelPolicyAdapter, Arc, Option, Origin, PolicyRepo, Result (+8 more)

### Community 319 - "guard"
Cohesion: 0.05
Nodes (87): GuardModeInput, OnAllowAsync, OnAllowSync, OnBlockAsync, OnBlockSync, OnErrorAsync, OnErrorSync, OnEscalateAsync (+79 more)

### Community 320 - "properties"
Cohesion: 0.12
Nodes (16): description, type, description, type, goal, injectionPayload, sourcePath, targetOperation (+8 more)

### Community 321 - "decision.schema.json"
Cohesion: 0.22
Nodes (8): required, $schema, title, type, latency_ms, trace_id, triggered_policies, verdict

### Community 322 - "code-block.tsx"
Cohesion: 0.19
Nodes (12): CodeBlock(), CodeBlockProps, highlight(), KEYWORDS, LABELS, Lang, tokenize(), Mode (+4 more)

### Community 323 - "prepush-fast.sh"
Cohesion: 0.43
Nodes (5): add_package(), detect_base_ref(), ref_exists(), run(), prepush-fast.sh script

### Community 325 - "GatewayRepo"
Cohesion: 0.33
Nodes (5): GatewayRepo, DbConnection, DbPool, Result, Self

### Community 326 - "RedactedEntity"
Cohesion: 0.13
Nodes (15): format, minimum, type, RedactedEntity, type, count, entity_type, token (+7 more)

### Community 327 - "parse_retry_after"
Cohesion: 0.26
Nodes (10): B, Client, parse_retry_after(), Duration, F, HeaderMap, Option, Result (+2 more)

### Community 328 - "tool_metadata.rs"
Cohesion: 0.36
Nodes (13): delete_tool_metadata(), get_tool_metadata(), list_tool_metadata(), Arc, HeaderMap, Json, Path, Response (+5 more)

### Community 329 - "RunStoreError"
Cohesion: 0.20
Nodes (13): RunStoreError, PostgresRunAdapter, Arc, CreateRunEventRequest, CreateRunRequest, Result, RunEventSummary, RunSummary (+5 more)

### Community 331 - "evaluate_financial_policies"
Cohesion: 0.15
Nodes (29): action_verdict(), compose(), evaluate_financial_policies(), financial_windowed_verdict(), per_action_verdicts(), Action, FinancialAction, I (+21 more)

### Community 332 - "StorageError"
Cohesion: 0.07
Nodes (47): AnalyticsRepo, clear_default(), ensure_view_exists(), AnalyticsDashboardView, CreateAnalyticsDashboardViewRequest, DbConnection, Result, UpdateAnalyticsDashboardViewRequest (+39 more)

### Community 333 - "Validation"
Cohesion: 0.20
Nodes (11): memory_store_delete_then_get_not_found(), memory_store_list_sorted(), memory_store_round_trip(), profile(), AgentProfile, validate_accepts_small_workflow_definition(), validate_rejects_empty_agent_id(), validate_rejects_empty_in_scope() (+3 more)

### Community 334 - "GuardEvent Redaction Spec"
Cohesion: 0.10
Nodes (19): 1. SDK-local redaction, 2. Customer-environment redaction service, 3. Server-side redaction, Acceptance Criteria, Deployment Modes, Goals, GuardEvent Redaction Spec, Hosted Cloud Behavior (+11 more)

### Community 335 - "authorize_workspace_admin"
Cohesion: 0.31
Nodes (15): authorize_api_key_management(), authorize_workspace_admin(), forwarded_user_id(), require_admin_role(), Arc, Extension, HeaderMap, Option (+7 more)

### Community 337 - "route.ts"
Cohesion: 0.16
Nodes (11): cleanupAgent(), createAgentSchema, GET(), POST(), stringListSchema, AgentClient, AgentProfileWire, mockState (+3 more)

### Community 339 - "financial_actions.rs"
Cohesion: 0.22
Nodes (28): app(), app_for(), create_payment_connection(), financial_action_decision_receipt_explains_held_refund(), financial_action_decision_receipt_missing_action_returns_404(), financial_action_outcomes_record_and_list(), financial_actions_create_get_and_transition(), financial_actions_list_workspace_actions() (+20 more)

### Community 340 - "budget_alerts.rs"
Cohesion: 0.21
Nodes (28): absolute_threshold_fires_when_remaining_drops_to_value(), admin_request(), app_with_owner(), create_alert(), create_weekly_cap(), crud_round_trip_via_router(), delivery_tx(), disabled_config_stays_silent() (+20 more)

### Community 341 - "build_postgres_layer"
Cohesion: 0.05
Nodes (68): AgentStore, Send, Sync, AnalyticsStore, Send, Sync, Send, Sync (+60 more)

### Community 342 - "financial_actions_integration.rs"
Cohesion: 0.11
Nodes (35): action_body(), agentic_payment_helpers_cover_x402_lifecycle(), decision_receipt_body(), financial_action_helpers_encode_ids_and_parse_statuses(), financial_mandate_helpers_create_list_and_revoke(), financial_outcome_helpers_record_and_list(), financial_policy_body(), financial_policy_helpers_create_and_list_controls() (+27 more)

### Community 343 - "team.rs"
Cohesion: 0.20
Nodes (15): CreateInviteRequest, CreateInviteResponse, CreateWorkspaceRequest, InviteListResponse, InviteStatus, MemberListResponse, MyWorkspace, MyWorkspacesResponse (+7 more)

### Community 344 - "FinancialExecutionResult"
Cohesion: 0.10
Nodes (29): AgenticPaymentBudgetReservationRequest, FinancialExecutionError, FinancialExecutionResult, FinancialExecutor, PaymentHttpFinancialExecutor, provider_body(), recovery_status(), reversal_capability() (+21 more)

### Community 345 - "guard.rs"
Cohesion: 0.12
Nodes (18): Channel, check_request_omits_absent_session_id_on_serialize(), Decision, RedactedEntity, RedactionInfo, RedactionMode, RedactionStatus, Into (+10 more)

### Community 347 - ".query"
Cohesion: 0.15
Nodes (12): AnalyticsRepo, AnalyticsQueryRequest, Result, validate_query(), AnalyticsQueryRequest, AnalyticsQueryResponse, DbConnection, DbPool (+4 more)

### Community 348 - "enum"
Cohesion: 0.29
Nodes (7): description, enum, type, Channel, chat, email, voice

### Community 349 - "Security Policy"
Cohesion: 0.25
Nodes (7): Coordinated Disclosure, Reporting a Vulnerability, Scope, Security Policy, Supported Versions, What to expect, What to include

### Community 350 - "CheckerFindingEvidence"
Cohesion: 0.33
Nodes (6): description, required, type, CheckerFindingEvidence, reason, rule

### Community 351 - "RunnerDocumentTemplate"
Cohesion: 0.09
Nodes (23): type, type, RunnerDocumentTemplate, additionalProperties, type, type, default, type (+15 more)

### Community 352 - "TrustLoopGuard Hardening v2 — Attack-Grounded Policy Synthesis"
Cohesion: 0.11
Nodes (18): 1. Attack taxonomy → remediation substrate, 2. Synthesis pipeline, 3. Generalization (concrete → class), 4. Verify-before-recommend (loop closure), 5. LLM usage: synthesis-time vs runtime (two planes), Architecture, Background: how v1 hardening works, and why it can't generalize, Concept-doc / contract impact when this ships (+10 more)

### Community 353 - "GatewayState"
Cohesion: 0.11
Nodes (37): GatewayState, create_gateway_provider_connection(), delete_gateway_provider_connection(), list_gateway_provider_connections(), patch_gateway_provider_connection(), CreateGatewayProviderConnectionRequest, Extension, HeaderMap (+29 more)

### Community 354 - "components.json"
Cohesion: 0.11
Nodes (17): aliases, components, hooks, lib, ui, utils, iconLibrary, rsc (+9 more)

### Community 355 - "fresh_repo"
Cohesion: 0.29
Nodes (15): batch_set_enabled_is_atomic_for_missing_policy(), batch_set_enabled_updates_all_selected_policies(), fresh_repo(), list_enabled_filters_disabled_and_deleted(), missing_policy_returns_not_found(), ContainerAsync, PolicyRepo, PostgresImage (+7 more)

### Community 356 - "GatewayStoreError"
Cohesion: 0.23
Nodes (10): GatewayStoreError, MemoryGatewayStore, GatewayProviderConnection, GatewayRoute, GatewayRoutePatch, NewGatewayProviderConnection, NewGatewayRoute, ResolvedGatewayRoute (+2 more)

### Community 357 - "effective_checker_modes"
Cohesion: 0.19
Nodes (18): checker_run_evidence(), CheckerModes, CheckerRun, EnforcementMode, all_none_override_inherits_workspace_modes(), checker_modes(), effective_checker_modes(), no_override_inherits_workspace_modes() (+10 more)

### Community 358 - "properties"
Cohesion: 0.20
Nodes (10): $ref, type, $ref, properties, action, id, match, severity (+2 more)

### Community 359 - "route.ts"
Cohesion: 0.14
Nodes (14): attackVectorSchema, dispatchBodySchema, documentTemplateSchema, isBase64(), POST(), MockRustApiError, MockWorkspaceAccessError, proxyMock (+6 more)

### Community 360 - "LlmClient"
Cohesion: 0.29
Nodes (16): LlmClient, Send, Sync, build_budget(), build_provider(), build_providers(), build_routes(), ensure_provider_exists() (+8 more)

### Community 361 - "tests.rs"
Cohesion: 0.21
Nodes (15): missing_route_yields_http_error(), MockClient, no_fallback_propagates_primary_error(), over_budget_blocks_request_before_calling_provider(), primary_failure_falls_back_to_secondary(), primary_success_records_budget_and_skips_fallback(), Arc, AtomicUsize (+7 more)

### Community 362 - "llm-docs.ts"
Cohesion: 0.18
Nodes (20): GET(), RouteContext, GET(), GET(), candidateRelativePaths(), DOCS_ROOT, getRawDocBySlug(), isMarkdownFile() (+12 more)

### Community 363 - "tests.rs"
Cohesion: 0.27
Nodes (17): assert_human_review_schema_exists(), assert_human_review_schema_missing(), assert_legacy_orphan_trace_preserved(), assert_migration_was_recorded(), assert_relation_state(), drop_human_review_schema(), establish(), fresh_database_url() (+9 more)

### Community 364 - "read_filter"
Cohesion: 0.26
Nodes (12): parse_kind(), parse_status(), query_parts(), read_filter(), read_limit(), Item, Iterator, Option (+4 more)

### Community 365 - "sync-recipes.ts"
Cohesion: 0.20
Nodes (8): changed, escapeRegExp(), failures, Recipe, recipePaths, replaceBlock(), Snippet, Target

### Community 367 - "key.rs"
Cohesion: 0.24
Nodes (17): canonical_json(), context_object_key_order_does_not_affect_key(), different_domain_changes_key(), different_drafts_hash_differently(), for_check_request(), for_check_request_with_policy_scope(), identical_requests_hash_equal(), missing_domain_is_treated_as_default() (+9 more)

### Community 368 - "properties"
Cohesion: 0.08
Nodes (24): RunnerWorkflowPath, sinkCategory, sinkNode, sinkType, sourceCategory, sourceNode, sourceType, additionalProperties (+16 more)

### Community 369 - "HnswIndex"
Cohesion: 0.06
Nodes (36): cosine(), Embedder, EmbedError, FastEmbedder, fnv1a(), mock_embedder_is_deterministic(), mock_embedder_normalises_to_unit(), MockEmbedder (+28 more)

### Community 371 - "OpenRouterClient"
Cohesion: 0.32
Nodes (7): OpenRouterClient, Client, Duration, Into, Result, Self, String

### Community 372 - "resolved_event"
Cohesion: 0.19
Nodes (16): ApprovalChecker, empty_roles_fall_back_to_generic_remediation(), escalates_when_tool_requires_approval(), metadata(), no_approval_rule_emits_nothing(), not_required_emits_nothing(), registry_reason_wins_over_generated_remediation(), remediation() (+8 more)

### Community 374 - "TokenBudget"
Cohesion: 0.18
Nodes (14): BudgetExceeded, BudgetState, exceeding_default_limit_errors(), HashMap, Into, Mutex, Result, Self (+6 more)

### Community 375 - "load_agent_str"
Cohesion: 0.21
Nodes (15): load_agent_str(), AgentProfile, Result, loads_committed_fixture_acme_support_v3(), parses_full_featured_profile(), parses_minimal_profile(), parses_web_knowledge_source_metadata(), rejects_duplicate_knowledge_source_ids() (+7 more)

### Community 376 - "monitoring_integration.rs"
Cohesion: 0.25
Nodes (16): allow_decision(), caller_explicit_session_is_never_overwritten(), client_without_monitoring_sends_no_session_id(), event(), mock_post(), monitoring_client_tags_submitted_events_with_session(), one_shot_retry(), record_event_delivers_without_blocking() (+8 more)

### Community 377 - "validation.rs"
Cohesion: 0.23
Nodes (12): clean_optional(), CreateRunEventRequest, CreateRunRequest, Option, Result, String, UpdateRunRequest, Value (+4 more)

### Community 378 - "mod.rs"
Cohesion: 0.11
Nodes (20): JobCounts, PublicReportState, RedteamAttackRecordFilter, RedteamJobListFilter, Arc, Option, String, allows_up_to_max_then_blocks() (+12 more)

### Community 379 - "guardrails.rs"
Cohesion: 0.24
Nodes (15): build_app(), delete_agent_cascades_to_owned_policies(), generate_for_missing_agent_is_404(), generate_persists_each_draft_disabled_and_returns_them(), generate_without_system_prompt_is_422(), list_for_unknown_agent_returns_empty(), list_returns_policies_scoped_to_agent(), read_body() (+7 more)

### Community 380 - "financial_repo.rs"
Cohesion: 0.18
Nodes (21): approval_requests_are_tenant_scoped_and_newest_first(), create_action_is_idempotent_and_tenant_scoped(), fresh_pool(), list_actions_is_tenant_scoped_and_newest_first(), mandate_request(), mandates_create_list_and_revoke_are_tenant_scoped(), outcome(), outcomes_append_and_list_by_action_without_affecting_spend() (+13 more)

### Community 381 - "handlers.ts"
Cohesion: 0.24
Nodes (12): agentProfile(), createToolHandlers(), errorToolResult(), JsonObject, JsonPrimitive, jsonReplacer(), jsonToolResult(), JsonValue (+4 more)

### Community 382 - "Crates"
Cohesion: 0.12
Nodes (17): Adding a new crate, Crates, Current Boundary Decisions, Dependency graph, `tl-cache` — decision cache, `tl-cli` — operator command line, `tl-codegen` — derived-artifact generator, `tl-core` — the type backbone (+9 more)

### Community 383 - "redteam-runner.schema.json"
Cohesion: 0.09
Nodes (21): description, $ref, $ref, $ref, $ref, properties, dispatch, handle (+13 more)

### Community 384 - "test_events.py"
Cohesion: 0.36
Nodes (12): default_allow_decision(), GuardEvent, submit_event tests: typed round trip + error mapping, sync and async., run_event_summary(), run_summary(), send_email_event(), test_async_run_context_inherits_run_id(), test_async_submit_event_round_trip() (+4 more)

### Community 385 - "main.rs"
Cohesion: 0.26
Nodes (14): Args, main(), normalize_typescript(), patch_openapi_label_policy_upsert(), render_pydantic(), repo_root(), Option, Path (+6 more)

### Community 387 - "docs-auth.ts"
Cohesion: 0.22
Nodes (11): POST(), redirectTo(), POST(), redirectTo(), UnlockPage(), UnlockPageProps, createDocsAuthToken(), safeDocsRedirectPath() (+3 more)

### Community 388 - "scripts"
Cohesion: 0.05
Nodes (43): dependencies, openai, pdfjs-dist, @trustloopguard/sdk, yaml, description, devDependencies, tsx (+35 more)

### Community 390 - "human_review.rs"
Cohesion: 0.28
Nodes (15): CreateHumanReviewEventRequest, HumanReviewAnalyticsResponse, HumanReviewAnalyticsSummary, HumanReviewEvent, HumanReviewEventListResponse, HumanReviewGroupRow, HumanReviewOutcome, HumanReviewOutcomeCounts (+7 more)

### Community 391 - ".from_response"
Cohesion: 0.22
Nodes (12): body_with_unknown_code_falls_back_to_status(), carries_retry_after_for_rate_limit(), empty_body_500_synthesizes_internal_error(), falls_back_to_status_when_body_unrecognized(), parses_canonical_body_to_typed_variant(), ApiError, ApiErrorCode, Duration (+4 more)

### Community 392 - "put_llm_price"
Cohesion: 0.20
Nodes (22): api_error_response(), delete_llm_price(), list_llm_pricing(), LlmPricingState, precise_rate(), price_row(), put_llm_price(), ApiErrorCode (+14 more)

### Community 393 - "Common Workflows"
Cohesion: 0.33
Nodes (5): Add A New Runtime Capability, Available Guides, Build A Demo, Common Workflows, Guard An Agent Reply

### Community 394 - "main.rs"
Cohesion: 0.18
Nodes (19): generate_guardrails(), list_guardrails(), GuardrailGenerateResponse, GuardrailListResponse, Option, Result, String, run_agents() (+11 more)

### Community 395 - "agents.rs"
Cohesion: 0.32
Nodes (12): AgentState, delete_agent(), get_agent(), list_agents(), Arc, Bytes, HeaderMap, Option (+4 more)

### Community 396 - "UserStoreError"
Cohesion: 0.26
Nodes (9): MemoryUserStore, HashMap, Result, RwLock, Self, String, UserRecord, Uuid (+1 more)

### Community 399 - "properties"
Cohesion: 0.11
Nodes (18): $ref, RunnerDispatch, anyOf, $ref, type, attackSurface, documentTemplate, mode (+10 more)

### Community 400 - "ParamLimit"
Cohesion: 0.14
Nodes (14): ParamLimit, description, format, description, format, allOf, default, description (+6 more)

### Community 403 - "ParamLimit"
Cohesion: 0.14
Nodes (14): ParamLimit, description, format, description, format, allOf, default, description (+6 more)

### Community 404 - "RedteamDispatchRequest.ts"
Cohesion: 0.18
Nodes (9): AttackVector, RedteamAttackSurface, RedteamDispatchRequest, RedteamDocumentTemplate, RedteamPlanListResponse, RedteamPlanResponse, RFC-3339, RedteamRunMode (+1 more)

### Community 405 - "Client"
Cohesion: 0.18
Nodes (10): Client, ApiError, Into, Option, RetryConfig, Self, String, synthesize_api_error() (+2 more)

### Community 407 - "forward_payment"
Cohesion: 0.08
Nodes (32): AnthropicGatewayProvider, Client, GatewayProviderConnection, Result, String, Value, GatewayProvider, latest_user_message_content() (+24 more)

### Community 408 - "monitoring.tsx"
Cohesion: 0.10
Nodes (17): Ascii(), ASCII_ART, AsciiName, CountUp(), CountUpProps, Eyebrow(), LOOP, PROBLEMS (+9 more)

### Community 409 - "router"
Cohesion: 0.12
Nodes (29): build_policy_draft_llm(), router(), Arc, Option, memory_app_state(), analytics_catalog_query_and_saved_views_round_trip(), analytics_endpoints_are_protected_by_bearer_auth(), internal_bearer_analytics_requires_forwarded_workspace_member() (+21 more)

### Community 410 - "run.rs"
Cohesion: 0.28
Nodes (20): CreateRunEventRequest, CreateRunRequest, Option, String, TraceSummary, Value, Vec, RunBudgetWindowSnapshot (+12 more)

### Community 411 - "nav-actions.tsx"
Cohesion: 0.24
Nodes (7): formatStars(), GitHubStarLink(), NavActions(), NavActionsProps, Nav(), NAV_LINKS, getStarCount()

### Community 412 - "Event Engine"
Cohesion: 0.13
Nodes (15): Checkers And Enforcement Modes, Collection Points, Compatibility Rules, Contract Vocabulary, Current Runtime Flow, Direct ingestion, Event Engine, Gateway (low fidelity) (+7 more)

### Community 414 - "Policy YAML Reference"
Cohesion: 0.13
Nodes (15): `action`, `description`, `id`, `literal`, `match`, Matchers, Policy YAML Reference, `regex` (+7 more)

### Community 415 - "LabelPolicyStoreError"
Cohesion: 0.23
Nodes (12): LabelPolicyStoreError, MemoryLabelPolicyStore, origin_key(), HashMap, Origin, Result, RwLock, Self (+4 more)

### Community 416 - "compilerOptions"
Cohesion: 0.08
Nodes (25): compilerOptions, allowJs, exactOptionalPropertyTypes, incremental, jsx, lib, noEmit, paths (+17 more)

### Community 417 - "verify_candidate"
Cohesion: 0.17
Nodes (21): Send, Sync, SemanticPolicyJudge, candidate_that_false_blocks_a_control_does_not_pass(), candidate_that_misses_a_variant_does_not_pass(), fires(), KeywordJudge, output_event() (+13 more)

### Community 418 - "PostgresUserAdapter"
Cohesion: 0.22
Nodes (10): PostgresUserAdapter, Arc, Result, Self, UserRecord, Uuid, user_record_from_row(), user_store_create_error() (+2 more)

### Community 419 - "client.ts"
Cohesion: 0.43
Nodes (5): ClientEnv, createTrustLoopClient(), readClientOptions(), main(), createTrustLoopMcpServer()

### Community 421 - "workflow_analyzer.rs"
Cohesion: 0.22
Nodes (17): adjacency(), analyze(), classify(), finds_source_to_sink_path_through_neutral_node(), lookalike_node_names_do_not_create_phantom_paths(), no_path_when_source_does_not_reach_sink(), node_types(), NodeRole (+9 more)

### Community 422 - "policy.rs"
Cohesion: 0.38
Nodes (11): decode_policy_response(), load_policy_file(), pull_policy(), push_policy(), Option, PathBuf, PolicyDocument, Response (+3 more)

### Community 423 - "seal_key_material"
Cohesion: 0.21
Nodes (12): build_seal_key(), Option, Result, String, seal_key_config_requires_secret_without_explicit_dev_override(), seal_key_material(), unseal_provider_key(), env_filter() (+4 more)

### Community 424 - "null"
Cohesion: 0.08
Nodes (37): type, properties, type, type, type, integer, null, string (+29 more)

### Community 425 - "route.test.ts"
Cohesion: 0.32
Nodes (5): GET(), POST(), RouteContext, proxyMock, RouteContext

### Community 426 - "page.tsx"
Cohesion: 0.15
Nodes (8): { GET }, APIPage, MediaBody, scalarToYaml(), toYaml(), yamlMediaAdapter, openapi, source

### Community 427 - "build_app"
Cohesion: 0.29
Nodes (13): build_app(), delete_then_get_returns_404(), delete_unknown_yields_404(), list_returns_all_agents(), missing_agent_yields_404(), read_body(), Response, Router (+5 more)

### Community 429 - "validation.rs"
Cohesion: 0.23
Nodes (13): clean_reason_codes(), non_empty_string(), normalize_metadata(), parse_uuid(), CreateHumanReviewEventRequest, Option, Result, String (+5 more)

### Community 430 - "check_pipeline.rs"
Cohesion: 0.35
Nodes (11): bench_check_async_50_policies_4kb(), bench_check_async_cache_hit(), bench_check_async_empty_default(), bench_check_sync_empty(), bench_check_sync_empty_4kb(), bench_check_sync_policy_block_4kb(), fifty_policies(), large_req() (+3 more)

### Community 431 - "definitions"
Cohesion: 0.06
Nodes (38): enum, type, definitions, Confidentiality, Integrity, LabelBasis, LabelPolicyStatus, ParamRole (+30 more)

### Community 432 - "MemoryToolMetadataStore"
Cohesion: 0.10
Nodes (20): PostgresToolMetadataAdapter, Arc, Option, Result, Self, ToolMetadata, ToolMetadataEntry, Vec (+12 more)

### Community 433 - "Human Review Analytics Spec"
Cohesion: 0.14
Nodes (13): Acceptance Criteria, API Contract, Dashboard UX, Data Model, Definitions, Goals, Human Review Analytics Spec, Implementation Scope (+5 more)

### Community 434 - "WorkflowRequirement"
Cohesion: 0.13
Nodes (15): WorkflowRequirement, type, name, required_before, sensitive_steps, default, items, type (+7 more)

### Community 435 - "HumanReviewAnalyticsResponse.ts"
Cohesion: 0.21
Nodes (7): HumanReviewAnalyticsResponse, HumanReviewAnalyticsSummary, HumanReviewGroupRow, HumanReviewOutcomeCounts, HumanReviewPolicyRow, HumanReviewReasonRow, HumanReviewWorkflowStepRow

### Community 436 - "normalize_payment_requirement"
Cohesion: 0.36
Nodes (9): clean_required(), normalize_pay_to(), normalize_payment_requirement(), Result, String, X402NormalizedPaymentRequirement, X402SettlementProof, verify_settlement_proof() (+1 more)

### Community 437 - "compilerOptions"
Cohesion: 0.08
Nodes (23): compilerOptions, allowJs, incremental, jsx, lib, noEmit, paths, plugins (+15 more)

### Community 438 - "devDependencies"
Cohesion: 0.08
Nodes (25): devDependencies, jsdom, tailwindcss, @tailwindcss/postcss, @testing-library/jest-dom, @testing-library/react, @testing-library/user-event, @types/node (+17 more)

### Community 439 - "LlmRouter"
Cohesion: 0.18
Nodes (20): LlmOutput, ProviderTarget, AuditedLlmError, AuditedLlmOutput, error_code(), failed_audit(), JudgeKind, LlmCallAudit (+12 more)

### Community 440 - "seed-demo.ts"
Cohesion: 0.31
Nodes (12): createKnowledgeSource(), DemoAgentProfile, DemoKnowledgeSource, DemoToolMetadata, DemoTraceInput, enforceDemoGuardSettings(), main(), recordTrace() (+4 more)

### Community 441 - "compilerOptions"
Cohesion: 0.08
Nodes (23): compilerOptions, allowJs, incremental, jsx, lib, noEmit, paths, plugins (+15 more)

### Community 442 - "knowledge.rs"
Cohesion: 0.28
Nodes (12): CreateKnowledgeSourceRequest, DashboardKnowledgeSourceKind, KnowledgeFileInput, KnowledgeFileMetadata, KnowledgeSourceDocument, KnowledgeSourceFileResponse, KnowledgeSourceListResponse, KnowledgeSourceStatus (+4 more)

### Community 443 - "api-keys.ts"
Cohesion: 0.16
Nodes (9): apiKeyBatchRevokeResponseSchema, apiKeySchema, revokeApiKeys(), ApiKeyBatchRevokeRequest, ApiKeyBatchRevokeResponse, ApiKeyListResponse, CreateApiKeyResponse, DashboardApiKey (+1 more)

### Community 444 - "HardenCandidate.ts"
Cohesion: 0.24
Nodes (7): HardenCandidate, HardenCandidateOperation, HardenRejection, HardenRejectionReason, HardenResponse, RFC-3339, VerifyResult

### Community 445 - "LlmPricingRepo"
Cohesion: 0.17
Nodes (12): LlmPricingRepo, DbConnection, DbPool, Debug, Formatter, Option, Result, Self (+4 more)

### Community 446 - "lib.rs"
Cohesion: 0.26
Nodes (9): buffer_truncates_to_window(), continues_when_evaluator_allows(), interrupts_when_evaluator_flags_window(), F, Self, String, Verdict, StreamDecision (+1 more)

### Community 448 - "http.rs"
Cohesion: 0.27
Nodes (9): decode_typed_response(), resolve_api_key(), Option, Response, Result, String, T, server_url() (+1 more)

### Community 449 - "policy"
Cohesion: 0.21
Nodes (10): policy(), rejects_empty_override(), Confidentiality, Integrity, Option, Result, SourceLabelPolicy, String (+2 more)

### Community 450 - "build_app_state"
Cohesion: 0.12
Nodes (17): BuildOptions, Option, String, password_auth_enabled_from_env(), password_auth_enabled_from_values(), Option, build_app_state(), build_escalation_worker() (+9 more)

### Community 451 - "request"
Cohesion: 0.13
Nodes (19): build_app(), create_json_policy_canonicalizes_source_yaml(), create_then_get_policy_round_trips_source_yaml(), batch_disable_missing_policy_does_not_partially_update(), batch_disable_updates_multiple_policies(), delete_policy_makes_get_return_404(), disable_policy_updates_document_but_get_still_works(), disable_policy_with_malformed_json_returns_api_error() (+11 more)

### Community 453 - "policy_authority.rs"
Cohesion: 0.39
Nodes (7): gateway_and_events_share_the_same_policy_decision(), gateway_applies_policy_escalation_before_provider_call(), gateway_applies_policy_input_rewrite_without_a_rule_set(), gateway_applies_policy_output_rewrite_without_regeneration(), gateway_returns_bad_gateway_for_provider_failure(), Router, upsert_gateway_policy()

### Community 454 - "RunnerPlanRequest"
Cohesion: 0.12
Nodes (16): type, RunnerPlanRequest, default, type, agentDisplayName, paths, systemPrompt, workflowPresent (+8 more)

### Community 455 - "RunnerPlanResponse"
Cohesion: 0.11
Nodes (19): description, items, RunnerPlanResponse, default, items, type, $ref, items (+11 more)

### Community 457 - "SourceLabelPolicy"
Cohesion: 0.21
Nodes (10): Confidentiality, Integrity, Option, Origin, Trust, Vec, SourceLabelPolicy, SourceLabelPolicyEntry (+2 more)

### Community 458 - "BudgetAlertConfig.ts"
Cohesion: 0.27
Nodes (7): RFC-3339, RFC-3339, BudgetAlertThresholdType, BudgetAlertWindow, CreateBudgetAlertConfigRequest, SpendMeter, UpdateBudgetAlertConfigRequest

### Community 459 - "retry_integration.rs"
Cohesion: 0.36
Nodes (11): does_not_retry_401(), event(), fast_retry(), gives_up_after_max_attempts(), honors_retry_after_header(), ok_decision_body(), retries_503_until_success(), GuardEvent (+3 more)

### Community 460 - "validate_create_action"
Cohesion: 0.33
Nodes (8): clean_operation(), clean_required(), is_valid_transition(), CreateFinancialActionRequest, FinancialActionStatus, Result, String, validate_create_action()

### Community 461 - "MemoryStore"
Cohesion: 0.20
Nodes (11): DecisionStore, Send, Sync, MemoryStore, Arc, Decision, HashMap, Result (+3 more)

### Community 463 - "guard-event.schema.json"
Cohesion: 0.25
Nodes (7): action, kind, required, $schema, title, type, principal

### Community 464 - "MemoryHumanReviewStore"
Cohesion: 0.18
Nodes (14): empty_analytics(), key(), MemoryHumanReviewStore, CreateHumanReviewEventRequest, HashMap, HumanReviewAnalyticsFilter, HumanReviewAnalyticsResponse, HumanReviewEvent (+6 more)

### Community 465 - "CheckerFinding"
Cohesion: 0.22
Nodes (11): CheckerFinding, finding_with(), NoOpChecker, HashMap, Option, Result, String, ToolMetadata (+3 more)

### Community 466 - ".analytics"
Cohesion: 0.12
Nodes (27): count_outcome(), group_row(), GroupAccumulator, is_human_intervention(), payload_string(), percentage(), policy_ids(), PolicyAccumulator (+19 more)

### Community 468 - "budget.rs"
Cohesion: 0.13
Nodes (32): bounded_output_tokens(), budget_exceeded_response(), budget_request_error(), evaluate_llm_budget_alerts(), llm_budget_policy_matches(), LlmBudgetReservation, meter_llm_usage(), MeterLlmUsage (+24 more)

### Community 469 - "Plugin contract"
Cohesion: 0.17
Nodes (11): Adding a new language binding, `Context` — anything the customer wants logged but not evaluated, `Decision` — what TrustLoopGuard returns, `Draft` — what the agent wants to do, Plugin contract, Pseudocode, Required behaviors per host adapter, Required behaviors per language binding (+3 more)

### Community 470 - "RunnerReport"
Cohesion: 0.15
Nodes (13): RunnerReport, status, sessions, status, additionalProperties, description, properties, required (+5 more)

### Community 471 - "Policy Cookbook"
Cohesion: 0.17
Nodes (12): Apply A Rule To One Agent, Apply A Rule To Voice Only, Auto-Generate Guardrails From An Agent Prompt, Block PII Leakage, CLI, Deletion, Escalate Legal Advice, HTTP (+4 more)

### Community 472 - "LlmUsageEvent.ts"
Cohesion: 0.21
Nodes (7): RFC-3339, LlmUsageBucketsResponse, LlmUsageEvent, RFC-3339, LlmUsageKind, LlmUsageListResponse, LlmUsageResponse

### Community 473 - "ToolMetadataRepo"
Cohesion: 0.16
Nodes (19): cache_key(), deserialize_spec(), Arc, Cache, DbConnection, DbPool, Debug, Duration (+11 more)

### Community 475 - "ConnectAgentStep.tsx"
Cohesion: 0.07
Nodes (38): ConnectAgentStep(), FirstEventStatus(), FLOW_BEATS, NEXT_STEPS, onboardingContextQuery(), CREATED, CopyBlock(), useFirstTrace() (+30 more)

### Community 476 - "Any"
Cohesion: 0.11
Nodes (20): FactsT, InputT, _build_financial_operation_request(), _clean_financial_operation_field(), Any, CounterpartyRef, CreateFinancialActionRequest, EvidenceRef (+12 more)

### Community 477 - "index.mdx"
Cohesion: 0.33
Nodes (5): Run The Chat Demo, Start The Server, Try Gateway Mode, Try The Demo Surfaces, Write Your First Policy

### Community 478 - "server.test.ts"
Cohesion: 0.38
Nodes (5): ToolResult, handlers(), RegisteredTool, registerTools(), toolResult()

### Community 479 - "fresh_store"
Cohesion: 0.25
Nodes (13): new_trace_id(), duplicate_put_is_idempotent(), fake_decision(), fresh_store(), invalid_uuid_returns_internal_error(), migration_runs_clean_and_is_idempotent(), missing_trace_id_returns_not_found(), put_then_get_round_trips() (+5 more)

### Community 480 - "Architecture"
Cohesion: 0.18
Nodes (11): Architecture, Customer integration paths, Dashboard-owned surfaces, End-state to keep in mind, Event-centered check model, Latency budget (committed), Request lifecycle (HTTP path), Runtime data flow (+3 more)

### Community 481 - "Team & invites"
Cohesion: 0.18
Nodes (11): Acceptance flow, Authorization model, Endpoints, Enforcement, Invite lifecycle, Memory mode, Ownership, Roles (+3 more)

### Community 483 - "MemorySettingsStore"
Cohesion: 0.16
Nodes (17): memory_api_key_to_wire(), MemoryApiKeyRecord, MemoryApiKeyStore, MemorySettingsStore, normalize_ids(), DashboardApiKey, EnvironmentCheckerModes, HashMap (+9 more)

### Community 484 - "core.ts"
Cohesion: 0.05
Nodes (86): runRefundAgent(), shouldUseOpenAI(), main(), promptFromArgsOrStdin(), testHeldActionDoesNotExecute(), testOfflineAgentApprovesAndExecutesProposedRefund(), testOrderSearch(), testOverRefundStillSubmitsFinancialAction() (+78 more)

### Community 485 - "compilerOptions"
Cohesion: 0.12
Nodes (16): compilerOptions, declaration, lib, outDir, rootDir, types, exclude, extends (+8 more)

### Community 486 - "parse_body"
Cohesion: 0.17
Nodes (13): api_error_response(), ApiErrorCode, Response, StatusCode, String, is_yaml_content_type(), parse_body(), AgentProfile (+5 more)

### Community 487 - "events_integration.rs"
Cohesion: 0.38
Nodes (9): observe_only_decision(), one_shot_retry(), GuardEvent, RetryConfig, Value, run_scoped_client_attaches_run_and_event_ids(), send_email_event(), submit_event_maps_server_error() (+1 more)

### Community 489 - ".create_event"
Cohesion: 0.18
Nodes (16): CreateRunEventRequest, Result, RunEventSummary, Vec, RunRepo, non_empty_string(), normalize_metadata(), parse_run_id() (+8 more)

### Community 490 - ".create_financial_policy"
Cohesion: 0.24
Nodes (9): enforcing_action(), financial_policy_from_request(), financial_policy_record(), policy_action(), Action, CreateFinancialPolicyRequest, FinancialPolicyListResponse, FinancialPolicyRecord (+1 more)

### Community 491 - "fresh_repo"
Cohesion: 0.27
Nodes (10): api_key_principal_round_trips_create_list_verify(), batch_revoke_api_keys_is_workspace_scoped(), batch_revoke_api_keys_updates_status_and_auth_lookup(), checker_mode_check_constraint_rejects_invalid_values(), fresh_repo(), get_settings_round_trips_checker_enforcement_modes(), ContainerAsync, DashboardAdminRepo (+2 more)

### Community 492 - "tier.rs"
Cohesion: 0.43
Nodes (5): TriggeredPolicy, Vec, Tier, TierResult, TierStatus

### Community 493 - "Red-Team Dispatch"
Cohesion: 0.20
Nodes (10): API, Configuration, Hardening loop, Job lifecycle, Ownership boundary, Red-Team Dispatch, Request flow, Runner contract (+2 more)

### Community 494 - "@trustloopguard/sdk"
Cohesion: 0.20
Nodes (9): Custom handlers, Gateway mode, Guard modes, Installation, License, Low-level client, Quick start, Requirements (+1 more)

### Community 495 - "theme-provider.tsx"
Cohesion: 0.12
Nodes (19): ibmPlexMono, inter, metadata, RootLayoutProps, applyTheme(), disableTransitions(), getSystemTheme(), ResolvedTheme (+11 more)

### Community 496 - "scripts"
Cohesion: 0.22
Nodes (9): scripts, build, db:seed, dev, start, test, test:coverage, test:watch (+1 more)

### Community 497 - "Contributing to TrustLoopGuard"
Cohesion: 0.09
Nodes (19): Commit style, Contributing to TrustLoopGuard, Development setup, License, Proposing changes, Pull request checklist, Reporting bugs, The three SDK-driven rules (+11 more)

### Community 499 - "TeamStoreError"
Cohesion: 0.05
Nodes (61): AddMemberOutcome, create_invite(), create_my_workspace(), list_invites(), list_members(), list_my_workspaces(), revoke_invite(), Extension (+53 more)

### Community 500 - "semantic_policy_batch.md"
Cohesion: 0.40
Nodes (4): Candidate policies, Event, Instructions, Proposed output

### Community 501 - "wire.rs"
Cohesion: 0.29
Nodes (12): call_chat_completions(), malformed_inner_json_yields_parse_error(), missing_content_yields_missing_field(), missing_usage_defaults_to_zero(), parse_chat_response(), parses_well_formed_response(), RequestParts, Client (+4 more)

### Community 502 - "gateway_routes"
Cohesion: 0.50
Nodes (4): build_gateway_http_client(), gateway_routes(), Client, Router

### Community 503 - ".submit_event"
Cohesion: 0.31
Nodes (6): Client, Decision, GuardEvent, Option, Result, SdkError

### Community 504 - "header_value"
Cohesion: 0.25
Nodes (8): header_value(), log_http_response(), HeaderMap, Next, Option, Request, Response, String

### Community 505 - "marketing-event-link.tsx"
Cohesion: 0.26
Nodes (11): Footer(), getFooterEvent(), LINK_GROUPS, Status, MarketingEventLink(), MarketingEventLinkProps, mergeRel(), MarketingEventName (+3 more)

### Community 506 - "JsonSchema"
Cohesion: 0.11
Nodes (23): Duration, Result, JsonSchema, LlmError, Duration, String, Value, authority_template_substitutes_all_placeholders() (+15 more)

### Community 507 - "MemoryPolicyStore"
Cohesion: 0.36
Nodes (7): MemoryPolicyRecord, MemoryPolicyStore, Arc, HashMap, RwLock, Self, String

### Community 508 - "Authorization"
Cohesion: 0.22
Nodes (9): Authorization, OAuth users (Google / GitHub), See also, Three lanes, one middleware, `TL_API_KEY` — internal / web-to-Rust, User-session JWT — HS256, minted by Rust, What this model does *not* have, Why this shape (+1 more)

### Community 509 - "Gateway"
Cohesion: 0.18
Nodes (11): Budgets and metering, Configuration, Data handling, Gateway, Observability, Ownership, Policy verdicts, Response signals (+3 more)

### Community 510 - "ProfileResolver"
Cohesion: 0.16
Nodes (15): FuzzyChecker, FuzzyHit, NoOpFuzzyChecker, NoOpProfileResolver, ProfileResolver, Action, AgentProfile, Arc (+7 more)

### Community 511 - "feature_request.md"
Cohesion: 0.22
Nodes (8): Acceptance criteria, Additional context, Alternatives considered, Compatibility and migration, Problem, Proposed behavior, SDK/API surface, Summary

### Community 512 - "view_from_record"
Cohesion: 0.27
Nodes (9): NewViewRecord, AnalyticsDashboardView, DateTime, Result, String, Utc, Value, view_from_record() (+1 more)

### Community 513 - "PolicyEditorDialog.test.tsx"
Cohesion: 0.25
Nodes (6): generatePolicyDraft, getPolicy, NON_ROUNDTRIP_YAML, ROUNDTRIP_YAML, upsertPolicy, validatePolicy

### Community 515 - "AuthUserState"
Cohesion: 0.17
Nodes (12): AuthUserState, normalize_oauth_provider(), oauth_session(), Json, Response, Arc, Option, Result (+4 more)

### Community 516 - "ApiError"
Cohesion: 0.15
Nodes (10): ApiError, ApiErrorCode, ApiErrorCode, Display, Formatter, Result, Self, String (+2 more)

### Community 517 - "create_review_event"
Cohesion: 0.33
Nodes (11): create_review_event(), human_review_analytics(), list_review_events(), CreateHumanReviewEventRequest, HeaderMap, Json, Path, Response (+3 more)

### Community 518 - "runs_integration.rs"
Cohesion: 0.46
Nodes (7): event_body(), one_shot_retry(), RetryConfig, Value, run_body(), run_helpers_encode_ids_and_parse_typed_responses(), start_run_posts_typed_request_with_bearer_auth()

### Community 519 - "ProvenanceMap"
Cohesion: 0.36
Nodes (5): ProvenanceMap, BTreeMap, Into, String, Vec

### Community 520 - "fresh_repo"
Cohesion: 0.39
Nodes (7): fresh_repo(), insert_then_mark_failed(), insert_then_mark_sent(), list_stale_returns_only_old_pending(), record_attempt_increments_counter(), ContainerAsync, PostgresImage

### Community 521 - "dashboard-widgets.tsx"
Cohesion: 0.05
Nodes (40): BadgeVariant, countVerdicts(), DASHBOARD_WIDGET_KEYS, DASHBOARD_WIDGETS, DashboardWidget, decisionColumns, DecisionMixWidget(), DecisionRow (+32 more)

### Community 522 - "._send_json_model"
Cohesion: 0.08
Nodes (20): _merge_context(), AgenticPaymentAuthorizationResponse, AgenticPaymentAuthorizeRequest, AgenticPaymentCommitRequest, AgenticPaymentRecord, AgenticPaymentRollbackRequest, CreateFinancialMandateRequest, CreateFinancialPolicyRequest (+12 more)

### Community 523 - "LiveKitSupportAgent"
Cohesion: 0.29
Nodes (3): LiveKitSupportAgent, Decision, Smallest possible LiveKit-style TrustLoopGuard integration.  This is the shape w

### Community 525 - "LiveKit agent guardrail demo"
Cohesion: 0.25
Nodes (7): Files, LiveKit agent guardrail demo, Modes, Run gateway mode, Run SDK mode, SDK mode, Setup (isolated env)

### Community 526 - "Agent-hardening loop"
Cohesion: 0.25
Nodes (8): Agent-hardening loop, Attack-vector planner (`redteam:plan`), Ownership, Saved plans (per-agent library), Seeds reach the attacker, not generic templates, The loop, The workflow graph is the provenance graph, Two honest policy sources

### Community 527 - "Red-Team Report Sharing"
Cohesion: 0.25
Nodes (8): API, Configuration, Red-Team Report Sharing, Rendering, Share tokens, Storage, The report payload, Two surfaces

### Community 528 - "RunnerHandle"
Cohesion: 0.22
Nodes (9): RunnerHandle, type, jobId, additionalProperties, description, properties, required, type (+1 more)

### Community 529 - "Red-Team Runner Contract v1"
Cohesion: 0.25
Nodes (7): Event Fields, `GET /health`, `GET /redteam/jobs/{jobId}`, `POST /redteam/jobs`, Red-Team Runner Contract v1, Session Fields, Transport

### Community 530 - "Integration & Interception — How TrustLoopGuard Hooks an Agent"
Cohesion: 0.25
Nodes (7): Bottom line, Concrete trace (email agent), Integration & Interception — How TrustLoopGuard Hooks an Agent, Integration tiers, The framework's role (LiveKit example), The key truth: the LLM never runs anything. It only *asks.*, Where TrustLoopGuard intercepts

### Community 531 - "compilerOptions"
Cohesion: 0.17
Nodes (11): compilerOptions, declaration, lib, outDir, rootDir, extends, include, DOM (+3 more)

### Community 532 - "guard-modes.mdx"
Cohesion: 0.29
Nodes (6): Choosing A Mode, Modes, Rewrite, Rewrite Or Regenerate, Streaming output, Strict

### Community 533 - "properties"
Cohesion: 0.06
Nodes (38): $ref, $ref, description, items, type, default, LabelResolution, additionalProperties (+30 more)

### Community 534 - "properties"
Cohesion: 0.20
Nodes (10): anyOf, properties, approval, reversible, sandbox_hint, side_effect, tool, type (+2 more)

### Community 535 - "policy_ast.rs"
Cohesion: 0.20
Nodes (10): Action, default_severity(), MatchClause, Matcher, Channel, Matcher, Severity, String (+2 more)

### Community 536 - "LabelBasisSet"
Cohesion: 0.09
Nodes (23): allOf, default, $ref, LabelBasisSet, Labels, allOf, default, $ref (+15 more)

### Community 537 - "properties"
Cohesion: 0.17
Nodes (12): properties, required, type, items, type, ApprovalRule, type, required (+4 more)

### Community 539 - ".generate_guardrails"
Cohesion: 0.38
Nodes (5): Client, GuardrailGenerateResponse, GuardrailListResponse, Result, SdkError

### Community 540 - "api_error_response"
Cohesion: 0.43
Nodes (6): api_error_response(), log_api_error(), ApiErrorCode, Response, StatusCode, String

### Community 542 - "HandlerCtx"
Cohesion: 0.09
Nodes (35): HandlerCtx, Default, Self, aggregate(), DefaultTierRunner, Arc, CancellationToken, Decision (+27 more)

### Community 543 - "null"
Cohesion: 0.21
Nodes (13): properties, integer, null, string, type, type, type, $ref (+5 more)

### Community 544 - "definitions"
Cohesion: 0.17
Nodes (12): required, type, definitions, AllowedSource, Origin, ParamRole, authority_bearing, content_bearing (+4 more)

### Community 545 - "enum"
Cohesion: 0.05
Nodes (46): anyOf, enum, type, definitions, Confidentiality, Integrity, Origin, Trust (+38 more)

### Community 546 - "params"
Cohesion: 0.25
Nodes (8): items, type, $ref, default, items, type, allowed_sources, params

### Community 547 - "properties"
Cohesion: 0.15
Nodes (13): ParamSpec, path, role, anyOf, description, properties, required, type (+5 more)

### Community 548 - "Analytics Dashboards"
Cohesion: 0.29
Nodes (7): Access, Analytics Dashboards, Ownership, Queries, Saved Views, Template Variables, Widget Layout

### Community 549 - "enum"
Cohesion: 0.25
Nodes (8): Verdict, allow, block, escalate, rewrite, description, enum, type

### Community 550 - "devDependencies"
Cohesion: 0.13
Nodes (15): lefthook, devDependencies, knip, lefthook, prettier, secretlint, @secretlint/secretlint-rule-preset-recommend, tsx (+7 more)

### Community 556 - "required"
Cohesion: 0.11
Nodes (18): RunnerAttackSession, RunnerAttackVector, additionalProperties, description, required, type, additionalProperties, description (+10 more)

### Community 557 - ".check"
Cohesion: 0.18
Nodes (16): applies_on_memory_write_side_effect_even_for_other_kinds(), blocks_untrusted_memory_write(), escalates_unattributed_provenance_paths(), escalates_unverified_memory_write(), ignores_non_memory_events(), MemoryChecker, provenance(), GuardEvent (+8 more)

### Community 558 - "properties"
Cohesion: 0.12
Nodes (16): items, type, ParamSpec, path, role, anyOf, description, properties (+8 more)

### Community 560 - "proxy_anthropic_messages"
Cohesion: 0.38
Nodes (9): proxy_anthropic_messages(), proxy_openai_chat_completions(), Bytes, Extension, HeaderMap, Option, Path, Response (+1 more)

### Community 561 - ".list_policies"
Cohesion: 0.31
Nodes (7): Client, Option, PolicyDocument, PolicyFamily, PolicyListResponse, Result, SdkError

### Community 562 - "docs"
Cohesion: 0.33
Nodes (5): Content, Develop, docs, Password protection, Why a separate app

### Community 565 - "package.json"
Cohesion: 0.33
Nodes (5): license, name, private, type, version

### Community 567 - "Result"
Cohesion: 0.06
Nodes (79): AgenticPaymentDecision, financial_matches(), WorkspaceKeyContext, action_decision(), agentic_payment_authorization_reason(), agentic_payment_counterparty(), agentic_payment_decision(), agentic_payment_metadata() (+71 more)

### Community 569 - "WorkflowDefinition"
Cohesion: 0.17
Nodes (12): description, WorkflowDefinition, definition, source, description, type, description, properties (+4 more)

### Community 570 - "enum"
Cohesion: 0.29
Nodes (7): Severity, critical, high, low, medium, enum, type

### Community 571 - "RunDetail.ts"
Cohesion: 0.13
Nodes (12): CreateHumanReviewEventRequest, HumanReviewEvent, RFC-3339, HumanReviewEventListResponse, HumanReviewOutcome, RunBudgetWindowSnapshot, RunDetail, RunGuardrailUsage (+4 more)

### Community 572 - "CheckerRun"
Cohesion: 0.13
Nodes (15): type, description, properties, required, type, CheckerRun, items, type (+7 more)

### Community 573 - "query_parts"
Cohesion: 0.23
Nodes (11): query_parts(), read_filter(), read_limit(), HumanReviewAnalyticsFilter, Item, Iterator, Option, String (+3 more)

### Community 575 - "Financial Authorization"
Cohesion: 0.20
Nodes (10): Agentic x402 Payments, Contract, Durable Storage, Evidence And Eligibility, Financial Authorization, HTTP API, Outcome Data, Policies, Mandates, And Runtime Payments (+2 more)

### Community 577 - "Financial Authorization Contract TDD Evidence"
Cohesion: 0.29
Nodes (6): Completion Status, Financial Authorization Contract TDD Evidence, RED/GREEN Evidence, Test Specification, User Journeys, Validation Commands

### Community 578 - "MemoryKnowledgeStore"
Cohesion: 0.22
Nodes (9): MemoryKnowledgeStore, HashMap, KnowledgeSourceDocument, KnowledgeSourceFileResponse, Result, RwLock, Self, String (+1 more)

### Community 579 - "proxy.ts"
Cohesion: 0.43
Nodes (7): config, isAuthenticated(), isPublicPath(), proxy(), PUBLIC_PATH_PREFIXES, safeRedirect(), SESSION_COOKIE_NAMES

### Community 580 - "env.ts"
Cohesion: 0.07
Nodes (37): event(), AuthScreen(), FormError(), FormErrorProps, OrDivider(), Spinner(), SpinnerProps, CredentialsForm() (+29 more)

### Community 581 - "Environments"
Cohesion: 0.33
Nodes (6): API, Environments, Ownership, Policy Deployment, Relationship to Workspaces, Runtime Resolution

### Community 582 - "TrustLoopGuard concepts"
Cohesion: 0.33
Nodes (6): Diagram workflow, Reading order, TrustLoopGuard concepts, Visual map, What TrustLoopGuard is, When to update these docs

### Community 583 - "Runs"
Cohesion: 0.33
Nodes (6): Events, External ID, Lifecycle, Ownership, Relationship to traces, Runs

### Community 584 - "normalize_create_mandate_request"
Cohesion: 0.47
Nodes (5): AgenticPaymentMandateScope, normalize_create_mandate_request(), payment_scope_json(), CreateFinancialMandateRequest, validate_payment_mandate_scope()

### Community 586 - "budget_alert.rs"
Cohesion: 0.34
Nodes (13): BudgetAlertConfig, BudgetAlertConfigListResponse, BudgetAlertFiring, BudgetAlertFiringListResponse, BudgetAlertThresholdType, BudgetAlertWindow, CreateBudgetAlertConfigRequest, Option (+5 more)

### Community 587 - "RedteamJobSummary"
Cohesion: 0.14
Nodes (13): ReportShareCardProps, ComparedAttackStatus, JobStatus, RedteamComparedAttack, RedteamJobDetail, RedteamJobListResponse, RedteamJobSummary, RFC-3339 (+5 more)

### Community 588 - "TraceSummary"
Cohesion: 0.32
Nodes (7): HumanReviewOutcome, Option, String, Value, Vec, TraceListResponse, TraceSummary

### Community 589 - "SignalEvidence"
Cohesion: 0.15
Nodes (13): SignalEvidence, type, message, provider_id, severity, type, anyOf, description (+5 more)

### Community 590 - "Policies"
Cohesion: 0.40
Nodes (5): API, Environment Enablement, Policies, Registry, Runtime Boundaries

### Community 592 - "hallucination.md"
Cohesion: 0.40
Nodes (4): Agent profile, Conversation, Grounding documents, Task

### Community 593 - "semantic_policy.md"
Cohesion: 0.40
Nodes (4): Event, Instructions, Policy, Proposed output

### Community 594 - "route.test.ts"
Cohesion: 0.40
Nodes (3): POST(), RouteContext, proxyMock

### Community 595 - "insert_trace"
Cohesion: 0.29
Nodes (10): analytics_distinguishes_guardrail_and_human_interventions(), fresh_pool(), insert_trace(), review_events_are_append_only_and_latest_is_queryable(), ContainerAsync, DbPool, Option, PostgresImage (+2 more)

### Community 596 - "route.test.ts"
Cohesion: 0.40
Nodes (3): GET(), RouteContext, proxyMock

### Community 597 - "route.ts"
Cohesion: 0.60
Nodes (4): forwardToWebhook(), hits, isRateLimited(), POST()

### Community 598 - "llm_usage.rs"
Cohesion: 0.31
Nodes (10): LlmUsageBucket, LlmUsageBucketsResponse, LlmUsageEvent, LlmUsageKind, LlmUsageListResponse, LlmUsageResponse, Option, String (+2 more)

### Community 599 - "Web Dashboard And Authentication"
Cohesion: 0.40
Nodes (5): Acceptance Criteria, Authentication, Dashboard Data Boundary, Status, Web Dashboard And Authentication

### Community 600 - "tool-metadata.schema.json"
Cohesion: 0.25
Nodes (7): reversible, side_effect, tool, required, $schema, title, type

### Community 601 - "seo.ts"
Cohesion: 0.14
Nodes (19): inter, metadata, RootLayout(), RootLayoutProps, spaceGrotesk, robots(), HOME_LAST_MODIFIED, sitemap() (+11 more)

### Community 603 - "authority.md"
Cohesion: 0.50
Nodes (3): Agent authority profile, Conversation, Task

### Community 604 - "tone.md"
Cohesion: 0.50
Nodes (3): Agent tone profile, Conversation, Task

### Community 607 - "Human Review Analytics"
Cohesion: 0.50
Nodes (4): Analytics, Human Review Analytics, Ownership, Review Events

### Community 608 - "generate-openapi-docs.mjs"
Cohesion: 0.40
Nodes (3): generatedPages, meta, openapi

### Community 612 - "OpenAiClient"
Cohesion: 0.32
Nodes (7): OpenAiClient, Client, Duration, Into, Result, Self, String

### Community 615 - "auth.rs"
Cohesion: 0.48
Nodes (6): AuthRequest, AuthResponse, ChangePasswordRequest, OAuthIdentityRequest, Option, String

### Community 618 - "KnowledgeStoreError"
Cohesion: 0.38
Nodes (10): KnowledgeStoreError, CreateKnowledgeSourceRequest, String, decode_file_data(), CreateKnowledgeSourceRequest, Result, Vec, validate_create_request() (+2 more)

### Community 619 - "AnalyticsFact"
Cohesion: 0.35
Nodes (10): AnalyticsFact, AnalyticsRepo, payload_string(), policy_ids(), Option, Result, String, Value (+2 more)

### Community 622 - "generate_guardrails"
Cohesion: 0.13
Nodes (26): draft_policy(), Bytes, Response, parse_policy_set(), policy_draft_item_schema(), policy_draft_json_schema(), policy_from_draft(), policy_set_draft_json_schema() (+18 more)

### Community 665 - "route.test.ts"
Cohesion: 0.40
Nodes (3): PUT(), RouteContext, proxyMock

### Community 669 - "proxy_provider_request"
Cohesion: 0.10
Nodes (29): blocked_response(), check_gateway_content(), GatewayContentCheck, GatewayDecisionLog, log_gateway_decision(), Decision, Option, ResolvedGatewayRoute (+21 more)

### Community 670 - "auth-redirect.ts"
Cohesion: 0.53
Nodes (4): AuthRedirectConfig, isRustOrLocalOrigin(), safeAuthRedirect(), config

### Community 671 - "properties"
Cohesion: 0.18
Nodes (11): KnowledgeSource, type, allOf, default, properties, required, type, description (+3 more)

### Community 673 - "proxy_healthcare_agent.py"
Cohesion: 0.27
Nodes (8): entrypoint(), gateway_api_key(), gateway_openai_base_url(), HealthcareProxyAgent, livekit_run_external_id(), Agent, JobContext, LiveKit healthcare agent that routes its LLM through TrustLoopGuard gateway.  Th

### Community 674 - "HumanReviewStoreError"
Cohesion: 0.17
Nodes (12): HumanReviewAnalyticsFilter, HumanReviewStoreError, review_error_response(), Response, Arc, Option, String, normalize_metadata() (+4 more)

### Community 676 - "RunListFilter"
Cohesion: 0.40
Nodes (5): Option, RunKind, RunStatus, String, RunListFilter

### Community 677 - "SourceLabelEvidence"
Cohesion: 0.12
Nodes (17): $ref, confidentiality, integrity, trust, SourceLabelEvidence, allOf, default, $ref (+9 more)

### Community 697 - "window_starts"
Cohesion: 0.47
Nodes (6): window_starts(), monday_is_its_own_week_start(), month_rollover_resets_day_and_month_but_not_week(), DateTime, sunday_belongs_to_the_previous_monday(), utc()

### Community 899 - "StrictEnvironmentStore"
Cohesion: 0.40
Nodes (4): EnvironmentCheckerModes, Result, WorkspaceSettings, StrictEnvironmentStore

### Community 915 - "gateway.mdx"
Cohesion: 0.22
Nodes (8): Anthropic client, How policies apply, OpenAI-compatible client, Set up a route, Spend controls, Streaming, Troubleshooting, What you configure

### Community 1138 - "LlmModelPrice.ts"
Cohesion: 0.47
Nodes (3): LlmModelPrice, LlmPriceSource, LlmPricingListResponse

### Community 1581 - "fresh_repos"
Cohesion: 0.33
Nodes (6): create_workspace_seeds_enabled_starter_policies(), fresh_repos(), ContainerAsync, PolicyRepo, PostgresImage, TeamRepo

### Community 1651 - "EntityVersionRecord"
Cohesion: 0.47
Nodes (4): EntityVersionRecord, PolicyRepo, Result, Vec

### Community 1652 - "Gateway Provider Management TDD Evidence"
Cohesion: 0.33
Nodes (5): Gateway Provider Management TDD Evidence, RED/GREEN evidence, Test specification, User journeys, Validation

### Community 1653 - "RedteamAttackRecord.ts"
Cohesion: 0.50
Nodes (3): RedteamAttackRecord, RFC-3339, RedteamAttackRecordListResponse

### Community 1774 - "scenarios.core.ts"
Cohesion: 0.09
Nodes (27): executePayment(), PaymentRequest, PaymentResult, simulatedLedger, StripePaymentIntent, assertEnforced(), main(), makeDecision() (+19 more)

### Community 1803 - "defaults.rs"
Cohesion: 0.33
Nodes (5): default_views(), empty_catalog(), AnalyticsDashboardView, AnalyticsFacetCatalogResponse, Vec

### Community 1805 - "fresh_pool"
Cohesion: 0.40
Nodes (5): fresh_pool(), ContainerAsync, DbPool, PostgresImage, upsert_get_list_and_delete_round_trip()

### Community 1807 - "Red-team harden (policy synthesis)"
Cohesion: 0.29
Nodes (7): Inputs and outputs, Outcome model, Ownership, Reachable substrates, Red-team harden (policy synthesis), What it does, Where it sits

### Community 1809 - "index.mdx"
Cohesion: 0.40
Nodes (4): Core Ideas, Latency Model, Runtime Shape, Source Of Truth

### Community 1813 - "Agent Breakaway Arena"
Cohesion: 0.33
Nodes (6): Adapter Contract, Agent Breakaway Arena, Flow, Hardening Loop, Ownership Boundary, What The Agent Receives

### Community 1814 - "llm_pricing.rs"
Cohesion: 0.36
Nodes (7): LlmModelPrice, LlmPriceSource, LlmPricingListResponse, Option, String, Vec, UpsertLlmModelPriceRequest

### Community 1816 - "latency_ms"
Cohesion: 0.50
Nodes (4): format, minimum, type, latency_ms

### Community 1817 - "PolicyError"
Cohesion: 0.30
Nodes (14): is_private_host(), is_private_ip(), public_url_error(), AgentProfile, Result, String, validate(), validate_knowledge_sources() (+6 more)

### Community 1820 - "index.mdx"
Cohesion: 0.40
Nodes (4): CLI, HTTP API, Rust Crates, SDKs

### Community 1824 - "hash_password"
Cohesion: 0.39
Nodes (7): hash_password(), PasswordError, Result, String, verify_password(), hash_roundtrip_matches(), verify_rejects_wrong_password()

### Community 1826 - "api_error"
Cohesion: 0.39
Nodes (7): api_error(), invalid_credentials(), password_auth_disabled(), ApiErrorCode, Response, StatusCode, String

### Community 1827 - "required"
Cohesion: 0.17
Nodes (12): RedactionInfo, mode, status, required, type, required, context_redacted, elapsed_ms (+4 more)

### Community 1830 - "enum"
Cohesion: 0.14
Nodes (14): EventKind, enum, type, api.mutation.proposed, browser.action.proposed, database.mutation.proposed, external_message.proposed, file.action.proposed (+6 more)

### Community 1831 - "enum"
Cohesion: 0.12
Nodes (16): Origin, ToolMetadata, api, email, file, memory, reversible, side_effect (+8 more)

### Community 1833 - "enum"
Cohesion: 0.15
Nodes (13): SideEffectClass, api_mutation, db_mutation, external_communication, file_write, memory_write, network_call, none (+5 more)

### Community 1834 - "api_error_response"
Cohesion: 0.24
Nodes (10): ai_edit_policy(), Bytes, Response, api_error_response(), api_error_response_with_details(), ApiErrorCode, Response, StatusCode (+2 more)

### Community 1835 - "enum"
Cohesion: 0.18
Nodes (12): LimitAction, Verdict, allow, block, escalate, rewrite, description, enum (+4 more)

### Community 1836 - "team_repo.rs"
Cohesion: 0.20
Nodes (8): AddMemberOutcome, DbConnection, DbPool, Result, Self, WorkspaceInvite, WorkspaceMember, TeamRepo

### Community 1837 - "null"
Cohesion: 0.24
Nodes (11): type, null, string, system_prompt, target_url, url, description, type (+3 more)

### Community 1838 - "enum"
Cohesion: 0.15
Nodes (13): SideEffectClass, api_mutation, db_mutation, external_communication, file_write, memory_write, network_call, none (+5 more)

### Community 1839 - "definitions"
Cohesion: 0.20
Nodes (10): required, type, definitions, AgentTone, KnowledgeSourceKind, local, web, enum (+2 more)

### Community 1840 - "RunnerSessionEvent"
Cohesion: 0.22
Nodes (9): RunnerSessionEvent, kind, additionalProperties, description, required, type, actor, eventId (+1 more)

### Community 1841 - "enum"
Cohesion: 0.22
Nodes (9): api, email, file, memory, system, unknown, user, web (+1 more)

### Community 1844 - "route.test.ts"
Cohesion: 0.33
Nodes (4): GET(), RouteContext, proxyMock, RouteContext

### Community 1846 - "Workspace feature flags: TDD evidence"
Cohesion: 0.29
Nodes (6): Coverage and known gaps, Merge evidence, Source and user journeys, Task report, Test specification, Workspace feature flags: TDD evidence

### Community 1847 - "enum"
Cohesion: 0.29
Nodes (7): RedactionStatus, enum, type, applied, failed, not_requested, rejected_raw_sensitive_data

### Community 1848 - "EnforcementMode"
Cohesion: 0.29
Nodes (7): EnforcementMode, description, enum, type, enforce, off, shadow

### Community 1849 - "enum"
Cohesion: 0.29
Nodes (7): enum, type, Action, allow, block, escalate, rewrite

### Community 1850 - "enum"
Cohesion: 0.29
Nodes (7): Severity, critical, high, low, medium, enum, type

### Community 1852 - "Principal"
Cohesion: 0.33
Nodes (6): Principal, agent_id, required, type, environment_id, workspace_id

### Community 1853 - "LimitAction"
Cohesion: 0.33
Nodes (6): LimitAction, block, escalate, description, enum, type

## Knowledge Gaps
- **2969 isolated node(s):** `printWidth`, `tabWidth`, `useTabs`, `semi`, `singleQuote` (+2964 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **643 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AppState` connect `AppState` to `oauth.rs`, `traces.rs`, `router`, `runs.rs`, `proxy_provider_request`, `HandlerCtx`, `event_service.rs`, `JwtSigner`, `build_app_state`, `authorize_workspace_admin`, `financial_actions.rs`, `budget.rs`, `build_postgres_layer`, `budget_alerts.rs`, `RedteamJobStore`, `GatewayState`, `effective_checker_modes`, `mod.rs`, `event_ingestion.rs`, `engine.rs`, `gateway_routes`, `escalation.rs`?**
  _High betweenness centrality (0.052) - this node is a cross-community bridge._
- **Why does `StorageError` connect `StorageError` to `view_from_record`, `BudgetAlertRepo`, `Result`, `redteam_job_repo.rs`, `RedteamJobStoreError`, `EnvironmentRepo`, `PostgresStore`, `RedteamPlanRepo`, `PostgresUserAdapter`, `BudgetAlertStoreError`, `DashboardAdminStoreError`, `api_keys.rs`, `Result`, `Result`, `PostgresGatewayAdapter`, `PostgresHumanReviewAdapter`, `llm_pricing.rs`, `team_repo.rs`, `FinancialStoreError`, `.create_event`, `validation.rs`, `MemoryLlmUsageStore`, `MemoryToolMetadataStore`, `latest_review_outcomes`, `llm_usage_repo.rs`, `LlmPricingRepo`, `PostgresLabelPolicyAdapter`, `provider_record_to_wire`, `latest_review_outcomes`, `EnvironmentStoreError`, `run_summary`, `GatewayRepo`, `PolicyRepo`, `RunStoreError`, `MemoryStore`, `Validation`, `.analytics`, `AgentRepo`, `ToolMetadataRepo`, `.query`, `schema.rs`, `PostgresAnalyticsAdapter`, `share.rs`, `metrics.rs`, `EscalationRepo`, `.create_event`, `AnalyticsFact`, `TeamStoreError`, `EntityVersionRecord`, `writer.rs`, `MemoryRedteamPlanStore`, `RedteamReportShareRepo`, `KnowledgeRepo`, `dashboard_admin_repo.rs`, `RunRepo`?**
  _High betweenness centrality (0.047) - this node is a cross-community bridge._
- **Why does `Policy` connect `Policy` to `Result`, `HnswFuzzyChecker`, `event_policy.rs`, `harden_job`, `policy_ast.rs`, `load_str`, `HandlerCtx`, `verify_candidate`, `Result`, `plan.rs`, `check_pipeline.rs`, `tests.rs`, `build_app_state`, `validation.rs`, `PolicyRepo`, `synthesis.rs`, `FamilyPolicy`, `PolicyStoreError`, `build_postgres_layer`, `fresh_repo`, `event_ingestion.rs`, `generate_guardrails`, `engine.rs`, `MemoryPolicyStore`?**
  _High betweenness centrality (0.042) - this node is a cross-community bridge._
- **Are the 95 inferred relationships involving `Client` (e.g. with `Decode` and `SdkError`) actually correct?**
  _`Client` has 95 INFERRED edges - model-reasoned connections that need verification._
- **What connects `printWidth`, `tabWidth`, `useTabs` to the rest of the system?**
  _3044 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Client` be split into smaller, more focused modules?**
  _Cohesion score 0.07729468599033816 - nodes in this community are weakly interconnected._
- **Should `GuardEvent` be split into smaller, more focused modules?**
  _Cohesion score 0.10822510822510822 - nodes in this community are weakly interconnected._