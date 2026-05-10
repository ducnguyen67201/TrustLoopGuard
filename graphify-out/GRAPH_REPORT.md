# Graph Report - TrustLoopGuard  (2026-05-10)

## Corpus Check
- 137 files · ~49,359 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1030 nodes · 1540 edges · 88 communities (77 shown, 11 thin omitted)
- Extraction: 91% EXTRACTED · 9% INFERRED · 0% AMBIGUOUS · INFERRED: 132 edges (avg confidence: 0.71)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `a79375b7`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- [[_COMMUNITY_Community 0|Community 0]]
- [[_COMMUNITY_Community 1|Community 1]]
- [[_COMMUNITY_Community 2|Community 2]]
- [[_COMMUNITY_Community 3|Community 3]]
- [[_COMMUNITY_Community 4|Community 4]]
- [[_COMMUNITY_Community 5|Community 5]]
- [[_COMMUNITY_Community 6|Community 6]]
- [[_COMMUNITY_Community 7|Community 7]]
- [[_COMMUNITY_Community 8|Community 8]]
- [[_COMMUNITY_Community 9|Community 9]]
- [[_COMMUNITY_Community 10|Community 10]]
- [[_COMMUNITY_Community 11|Community 11]]
- [[_COMMUNITY_Community 12|Community 12]]
- [[_COMMUNITY_Community 13|Community 13]]
- [[_COMMUNITY_Community 14|Community 14]]
- [[_COMMUNITY_Community 15|Community 15]]
- [[_COMMUNITY_Community 16|Community 16]]
- [[_COMMUNITY_Community 17|Community 17]]
- [[_COMMUNITY_Community 18|Community 18]]
- [[_COMMUNITY_Community 19|Community 19]]
- [[_COMMUNITY_Community 20|Community 20]]
- [[_COMMUNITY_Community 21|Community 21]]
- [[_COMMUNITY_Community 22|Community 22]]
- [[_COMMUNITY_Community 23|Community 23]]
- [[_COMMUNITY_Community 24|Community 24]]
- [[_COMMUNITY_Community 25|Community 25]]
- [[_COMMUNITY_Community 26|Community 26]]
- [[_COMMUNITY_Community 27|Community 27]]
- [[_COMMUNITY_Community 28|Community 28]]
- [[_COMMUNITY_Community 29|Community 29]]
- [[_COMMUNITY_Community 30|Community 30]]
- [[_COMMUNITY_Community 31|Community 31]]
- [[_COMMUNITY_Community 32|Community 32]]
- [[_COMMUNITY_Community 33|Community 33]]
- [[_COMMUNITY_Community 34|Community 34]]
- [[_COMMUNITY_Community 35|Community 35]]
- [[_COMMUNITY_Community 36|Community 36]]
- [[_COMMUNITY_Community 37|Community 37]]
- [[_COMMUNITY_Community 38|Community 38]]
- [[_COMMUNITY_Community 39|Community 39]]
- [[_COMMUNITY_Community 40|Community 40]]
- [[_COMMUNITY_Community 41|Community 41]]
- [[_COMMUNITY_Community 42|Community 42]]
- [[_COMMUNITY_Community 43|Community 43]]
- [[_COMMUNITY_Community 44|Community 44]]
- [[_COMMUNITY_Community 45|Community 45]]
- [[_COMMUNITY_Community 46|Community 46]]
- [[_COMMUNITY_Community 47|Community 47]]
- [[_COMMUNITY_Community 48|Community 48]]
- [[_COMMUNITY_Community 49|Community 49]]
- [[_COMMUNITY_Community 50|Community 50]]
- [[_COMMUNITY_Community 51|Community 51]]
- [[_COMMUNITY_Community 52|Community 52]]
- [[_COMMUNITY_Community 53|Community 53]]
- [[_COMMUNITY_Community 54|Community 54]]
- [[_COMMUNITY_Community 55|Community 55]]
- [[_COMMUNITY_Community 56|Community 56]]
- [[_COMMUNITY_Community 57|Community 57]]
- [[_COMMUNITY_Community 58|Community 58]]
- [[_COMMUNITY_Community 59|Community 59]]
- [[_COMMUNITY_Community 60|Community 60]]
- [[_COMMUNITY_Community 61|Community 61]]
- [[_COMMUNITY_Community 62|Community 62]]
- [[_COMMUNITY_Community 63|Community 63]]
- [[_COMMUNITY_Community 64|Community 64]]
- [[_COMMUNITY_Community 65|Community 65]]
- [[_COMMUNITY_Community 66|Community 66]]
- [[_COMMUNITY_Community 67|Community 67]]
- [[_COMMUNITY_Community 69|Community 69]]
- [[_COMMUNITY_Community 70|Community 70]]
- [[_COMMUNITY_Community 71|Community 71]]
- [[_COMMUNITY_Community 72|Community 72]]
- [[_COMMUNITY_Community 73|Community 73]]
- [[_COMMUNITY_Community 74|Community 74]]

## God Nodes (most connected - your core abstractions)
1. `SdkError` - 21 edges
2. `main()` - 19 edges
3. `Client` - 16 edges
4. `AsyncClient` - 16 edges
5. `run()` - 15 edges
6. `v0 Design Decisions` - 15 edges
7. `ApiErrorCode` - 14 edges
8. `ApiError` - 14 edges
9. `Crates` - 13 edges
10. `Technical terms` - 13 edges

## Surprising Connections (you probably didn't know these)
- `main()` --calls--> `build_request()`  [EXTRACTED]
  crates/tl-cli/src/main.rs → apps/example-rust/src/main.rs
- `main()` --calls--> `print_decision()`  [EXTRACTED]
  crates/tl-cli/src/main.rs → apps/example-rust/src/main.rs
- `main()` --calls--> `buildRequest()`  [EXTRACTED]
  crates/tl-cli/src/main.rs → apps/example-typescript/src/main.ts
- `main()` --calls--> `printDecision()`  [EXTRACTED]
  crates/tl-cli/src/main.rs → apps/example-typescript/src/main.ts
- `Client` --uses--> `CheckRequest`  [INFERRED]
  sdks/python/src/trustloopguard/client.py → sdks/python/src/trustloopguard/_generated/types.py

## Communities (88 total, 11 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.06
Nodes (29): ApiError, ApiErrorCode, Channel, CheckRequest, Client, ClientOptions, CODE_TO_CLASS, codeFromHttpStatus() (+21 more)

### Community 1 - "Community 1"
Cohesion: 0.08
Nodes (18): AgentRepo, migrate(), PostgresStore, verdict_text(), build_from_config_validates_referenced_providers(), JudgeKind, LlmOutput, LlmRouter (+10 more)

### Community 2 - "Community 2"
Cohesion: 0.11
Nodes (25): bench_check_async_50_policies_4kb(), bench_check_async_empty_default(), bench_check_sync_empty(), bench_universal_only_4kb(), fifty_policies(), large_req(), small_req(), FuzzyChecker (+17 more)

### Community 3 - "Community 3"
Cohesion: 0.08
Nodes (25): Home(), check(), getClient(), DecisionResponse, decisionResponseSchema, formSchema, FormValues, ParsedForm (+17 more)

### Community 4 - "Community 4"
Cohesion: 0.08
Nodes (18): BudgetConfig, ConfigError, empty_budgets_section_uses_default(), ProviderConfig, ProviderTarget, round_trips_sample_config(), RouteConfig, RouterConfig (+10 more)

### Community 5 - "Community 5"
Cohesion: 0.15
Nodes (21): dim_mismatch_yields_empty_query(), empty_index_returns_empty_query(), HnswIndex, identical_vector_scores_one(), IndexHit, mock_embedder_round_trip_through_index(), orthogonal_vector_below_threshold(), ranks_by_similarity_descending() (+13 more)

### Community 6 - "Community 6"
Cohesion: 0.1
Nodes (14): BuildError, dedup_when_both_tiers_match_same_policy(), empty_policies_yields_no_hits(), HnswFuzzyChecker, levenshtein_catches_typo_bypass(), levenshtein_misses_unrelated_text(), literal_policy(), semantic_match_on_paraphrase() (+6 more)

### Community 7 - "Community 7"
Cohesion: 0.06
Nodes (30): 10. Crate alignment, 11. Build order (v0), 12. Open questions (need answers before phase 1), 13. Things deliberately not in v0, 14. Confirmation checklist, 1. Product boundary (locked), 2. The wedge (locked), 3. The check contract (locked) (+22 more)

### Community 8 - "Community 8"
Cohesion: 0.15
Nodes (27): aggregate(), authority_violation_blocks(), bulleted(), cancelled(), CannedClient, ctx_with(), empty_router_yields_skipped(), extract_docs() (+19 more)

### Community 9 - "Community 9"
Cohesion: 0.07
Nodes (27): Action vs Verdict, Agent, Channel, CheckRequest, Cold path, Decision, Decision log, Domain terms (+19 more)

### Community 10 - "Community 10"
Cohesion: 0.13
Nodes (22): Exception, ApiError, Forbidden, Gone, Internal, Invalid, NotFound, Typed errors for the TrustLoopGuard Python SDK.  Mirrors `tl-sdk-rust`'s `SdkErr (+14 more)

### Community 11 - "Community 11"
Cohesion: 0.13
Nodes (19): load_agent_str(), loads_committed_fixture_acme_support_v3(), parses_full_featured_profile(), parses_minimal_profile(), rejects_malformed_yaml(), rejects_missing_agent_id(), rejects_missing_in_scope(), validate() (+11 more)

### Community 12 - "Community 12"
Cohesion: 0.12
Nodes (17): allow_helper_sets_verdict(), ApiError, Channel, CheckRequest, Decision, new_trace_id(), Severity, TlError (+9 more)

### Community 13 - "Community 13"
Cohesion: 0.12
Nodes (10): cc_candidate(), detect(), email(), first_valid_credit_card(), ids(), ipv4(), luhn_valid(), returns_multiple_hits() (+2 more)

### Community 14 - "Community 14"
Cohesion: 0.13
Nodes (18): canonical_json(), context_object_key_order_does_not_affect_key(), different_domain_changes_key(), different_drafts_hash_differently(), for_check_request(), identical_requests_hash_equal(), missing_domain_is_treated_as_default(), nested_objects_canonicalise_recursively() (+10 more)

### Community 15 - "Community 15"
Cohesion: 0.11
Nodes (10): ApiErrorCode, buffer_truncates_to_window(), Client, parse_retry_after(), SdkError, StreamDecision, StreamingChecker, synthesize_400_is_not_retriable() (+2 more)

### Community 16 - "Community 16"
Cohesion: 0.15
Nodes (15): ApiDoc, AppState, check(), router(), MemoryStore, build_app(), check_request(), correct_bearer_returns_200() (+7 more)

### Community 17 - "Community 17"
Cohesion: 0.18
Nodes (10): cosine(), Embedder, EmbedError, FastEmbedder, fnv1a(), mock_embedder_is_deterministic(), mock_embedder_normalises_to_unit(), MockEmbedder (+2 more)

### Community 18 - "Community 18"
Cohesion: 0.11
Nodes (17): Adding a new language binding, code:block1 (Guard.check(draft, ctx) -> Decision), code:block2 (fn check(draft: Draft, ctx: Context) -> Decision), code:block3 (Draft {), code:block4 (Context {), code:block5 (Decision {), code:block6 (fn push(chunk: String) -> StreamDecision), `Context` — anything the customer wants logged but not evaluated (+9 more)

### Community 19 - "Community 19"
Cohesion: 0.12
Nodes (16): 0. Start the server (all languages need this), 1. Rust, 2. Python, 3. TypeScript, code:bash (cargo run -p tl-server), code:bash (cargo run -p example-rust -- "show me my password" "here it ), code:bash (pip install -e sdks/python), code:bash (pnpm install) (+8 more)

### Community 20 - "Community 20"
Cohesion: 0.12
Nodes (16): Adding a new crate, code:block1 (tl-cli         tl-server         tl-sdk-rust), code:yaml (id: refund-promise), code:bash (cargo run -p tl-codegen           # write), Crates, Dependency graph, `tl-cli` — operator command line, `tl-codegen` — derived-artifact generator (+8 more)

### Community 21 - "Community 21"
Cohesion: 0.2
Nodes (13): _invalid(), _rate_limited(), Retry-policy tests for the Python SDK.  Mirrors `crates/tl-sdk-rust/src/retry.rs, test_caps_per_retry_delay_at_max_delay(), test_honors_retry_after_when_longer_than_jittered(), test_ignores_retry_after_when_jitter_already_longer(), test_jitter_fraction_clamps_to_unit_interval(), test_non_retriable_errors_stop_immediately() (+5 more)

### Community 22 - "Community 22"
Cohesion: 0.27
Nodes (7): disabled_cache_never_stores(), fake_decision(), miss_returns_none(), MokaCache, put_overwrites_existing_key(), put_then_get_returns_value(), ttl_expires_old_entries()

### Community 23 - "Community 23"
Cohesion: 0.29
Nodes (11): caps_per_retry_delay_at_max_delay(), honors_retry_after_when_longer_than_jittered(), ignores_retry_after_when_jitter_already_longer(), jitter_fraction_clamps_to_unit_interval(), non_retriable_errors_stop_immediately(), rate_limited(), retries_unavailable_with_exponential_backoff(), shrinks_last_delay_to_remaining_budget() (+3 more)

### Community 24 - "Community 24"
Cohesion: 0.23
Nodes (7): Decision, Severity, Tier, TierResult, TierStatus, TriggeredPolicy, Verdict

### Community 25 - "Community 25"
Cohesion: 0.26
Nodes (8): BudgetExceeded, BudgetState, exceeding_default_limit_errors(), tenant_limit_overrides_default(), TokenBudget, unknown_tenant_uses_default_limit(), used_returns_running_total(), zero_limit_means_unlimited()

### Community 26 - "Community 26"
Cohesion: 0.14
Nodes (13): 1. Engine-only PRs aren't done, 2. No internal imports in `apps/` or `demo/`, 3. The README quickstart works on a clean machine, 4. Cross-cutting concerns live in the SDK, once, How features are built (the loop), Out of scope, Required CI gates, Reviewer checklist (+5 more)

### Community 27 - "Community 27"
Cohesion: 0.26
Nodes (10): matcher_hits(), policy_matches(), benign_request_no_block(), block_signal_from_action(), pii_in_output_blocks(), prompt_injection_in_input_escalates(), req_with(), run() (+2 more)

### Community 28 - "Community 28"
Cohesion: 0.24
Nodes (3): AsyncClient, Async TrustLoopGuard client. Same surface as ``Client`` but awaitable., Async TrustLoopGuard client. Same surface as ``Client`` but awaitable.

### Community 29 - "Community 29"
Cohesion: 0.41
Nodes (11): capacity_zero_disables_cache(), delete_is_idempotent_on_missing(), delete_makes_subsequent_get_not_found(), fresh_repo(), list_returns_only_active_agents(), missing_agent_returns_not_found(), sample_profile(), second_get_uses_cache() (+3 more)

### Community 30 - "Community 30"
Cohesion: 0.23
Nodes (6): AuthConfig, EnvError, from_env_rejects_missing(), require_bearer(), subtle_eq(), unauthorized()

### Community 31 - "Community 31"
Cohesion: 0.17
Nodes (11): 1. Lane ownership, 2. Critical-path crates, 3. Contracts (the only cross-lane surface), 4. Wire-format versioning, 5. Demo independence checkpoint, 6. Conflict resolution, 7. When to split further, Founder A — Engine + Plugin SDK (+3 more)

### Community 32 - "Community 32"
Cohesion: 0.17
Nodes (11): Architecture, code:block1 (+-------------------+      CheckRequest       +-------------), code:block2 (CheckRequest), End-state to keep in mind, Latency budget (committed), Layered model: input to verdict, Request lifecycle (HTTP path), The shape of one call (+3 more)

### Community 33 - "Community 33"
Cohesion: 0.29
Nodes (10): BaseModel, Enum, ApiErrorCode, Channel, CheckRequest, Decision, Severity, TierResult (+2 more)

### Community 34 - "Community 34"
Cohesion: 0.29
Nodes (6): body_with_unknown_code_falls_back_to_status(), carries_retry_after_for_rate_limit(), empty_body_500_synthesizes_internal_error(), falls_back_to_status_when_body_unrecognized(), parses_canonical_body_to_typed_variant(), SdkError

### Community 35 - "Community 35"
Cohesion: 0.29
Nodes (7): detect(), detects_classic_injection(), detects_dan_mode(), detects_role_override(), distinct_patterns_each_fire_once(), ids(), matcher()

### Community 36 - "Community 36"
Cohesion: 0.29
Nodes (5): AgentAuthority, AgentProfile, AgentScope, AgentTone, KnowledgeSource

### Community 37 - "Community 37"
Cohesion: 0.22
Nodes (4): Client, HTTP client for TrustLoopGuard. Mirrors the `Guard.check(draft, ctx)` plugin con, Synchronous TrustLoopGuard client.      Args:         base_url: TrustLoopGuard s, Synchronous TrustLoopGuard client.      Args:         base_url: TrustLoopGuard s

### Community 40 - "Community 40"
Cohesion: 0.22
Nodes (9): test_status_to_code_table_matches_rust(), code_from_http_status(), from_response(), RateLimited, 429 — rate limited. Honor `retry_after` when set., Build a typed SdkError from a raw HTTP response.      Mirrors `SdkError::from_re, Map a raw HTTP status to the canonical error code.      Used as a fallback when, Build an ApiError from a raw HTTP response when the body isn't ours. (+1 more)

### Community 41 - "Community 41"
Cohesion: 0.58
Nodes (8): does_not_retry_401(), fast_retry(), gives_up_after_max_attempts(), honors_retry_after_header(), ok_decision_body(), req(), retries_503_until_success(), sends_bearer_auth_header()

### Community 42 - "Community 42"
Cohesion: 0.25
Nodes (3): c, cfg, d

### Community 43 - "Community 43"
Cohesion: 0.25
Nodes (4): Decode, Pre-server failure (network, DNS, timeout). No ApiError from the     server, so, Server returned 2xx but the body didn't parse as Decision., Transport

### Community 44 - "Community 44"
Cohesion: 0.25
Nodes (5): AgentAuthority, AgentProfile, AgentScope, AgentTone, KnowledgeSource

### Community 45 - "Community 45"
Cohesion: 0.54
Nodes (7): deadline_exceeded_yields_timeout(), malformed_inner_json_yields_parse_error(), non_2xx_yields_status_error(), ok_response(), openai_sends_bearer_auth_and_json_schema_body(), openrouter_adds_http_referer(), schema()

### Community 46 - "Community 46"
Cohesion: 0.25
Nodes (7): code:bash (cargo run -p tl-server), code:bash (cargo run -p example-rust -- "show me my password" "here it ), code:block3 (verdict       : Block), Environment, example-rust, Run it, Why this exists

### Community 47 - "Community 47"
Cohesion: 0.29
Nodes (5): Action, MatchClause, Matcher, Policy, WhenClause

### Community 48 - "Community 48"
Cohesion: 0.48
Nodes (6): authority_template_substitutes_all_placeholders(), build(), hallucination_template_substitutes_all_placeholders(), schema(), schemas_have_required_fields(), tone_template_substitutes_all_placeholders()

### Community 49 - "Community 49"
Cohesion: 0.29
Nodes (5): 1. Think Before Coding, 2. Simplicity First, 3. Surgical Changes, 4. Goal-Driven Execution, code:block1 (1. [Step] → verify: [check])

### Community 50 - "Community 50"
Cohesion: 0.33
Nodes (4): Retry policy for the TrustLoopGuard Python SDK.  Mirrors `tl-sdk-rust`'s `RetryC, Retry policy. Defaults match `tl-sdk-rust`'s `RetryConfig::default`., Compute the delay before the next attempt, or ``None`` to stop.          Mirrors, RetryConfig

### Community 51 - "Community 51"
Cohesion: 0.33
Nodes (5): code:block1 (agent proposes output → trustloop.check(...) → allow | block), Reading order, TrustLoopGuard concepts, What TrustLoopGuard is, When to update these docs

### Community 52 - "Community 52"
Cohesion: 0.33
Nodes (5): code:sh (pnpm install), Content, Develop, docs, Why a separate app

### Community 53 - "Community 53"
Cohesion: 0.5
Nodes (3): metadata, RootLayout(), RootLayoutProps

### Community 54 - "Community 54"
Cohesion: 0.4
Nodes (3): Tier, TierResult, TierStatus

### Community 55 - "Community 55"
Cohesion: 0.4
Nodes (4): JsonSchema, LlmClient, LlmError, LlmOutput

### Community 56 - "Community 56"
Cohesion: 0.4
Nodes (4): code:bash (# Terminal 1: start the server), Environment, example-typescript, Run it

### Community 57 - "Community 57"
Cohesion: 0.4
Nodes (4): code:bash (# Terminal 1: start the server), Environment, example-python, Run it

### Community 58 - "Community 58"
Cohesion: 0.4
Nodes (4): Agent profile, Conversation, Grounding documents, Task

### Community 60 - "Community 60"
Cohesion: 0.83
Nodes (3): build_request(), main(), print_decision()

### Community 61 - "Community 61"
Cohesion: 0.83
Nodes (3): block_signal_from_hit(), cancelled(), run()

### Community 62 - "Community 62"
Cohesion: 0.83
Nodes (3): openai_round_trip(), openrouter_round_trip(), trivial_schema()

### Community 63 - "Community 63"
Cohesion: 0.67
Nodes (3): diff(), replay_against(), ReplayDiff

### Community 64 - "Community 64"
Cohesion: 0.5
Nodes (3): Agent authority profile, Conversation, Task

### Community 65 - "Community 65"
Cohesion: 0.5
Nodes (3): Agent tone profile, Conversation, Task

## Knowledge Gaps
- **252 isolated node(s):** `cfg`, `c`, `d`, `STATUS_TO_CODE`, `DEFAULT_RETRIABLE` (+247 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **11 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `main()` connect `Community 11` to `Community 1`, `Community 2`, `Community 4`, `Community 6`, `Community 16`?**
  _High betweenness centrality (0.029) - this node is a cross-community bridge._
- **Why does `load_str()` connect `Community 6` to `Community 1`, `Community 2`, `Community 11`?**
  _High betweenness centrality (0.021) - this node is a cross-community bridge._
- **Why does `run_judges()` connect `Community 8` to `Community 1`?**
  _High betweenness centrality (0.018) - this node is a cross-community bridge._
- **Are the 5 inferred relationships involving `SdkError` (e.g. with `Client` and `AsyncClient`) actually correct?**
  _`SdkError` has 5 INFERRED edges - model-reasoned connections that need verification._
- **Are the 6 inferred relationships involving `main()` (e.g. with `.ok()` and `.parse()`) actually correct?**
  _`main()` has 6 INFERRED edges - model-reasoned connections that need verification._
- **Are the 6 inferred relationships involving `Client` (e.g. with `CheckRequest` and `Decision`) actually correct?**
  _`Client` has 6 INFERRED edges - model-reasoned connections that need verification._
- **Are the 6 inferred relationships involving `AsyncClient` (e.g. with `CheckRequest` and `Decision`) actually correct?**
  _`AsyncClient` has 6 INFERRED edges - model-reasoned connections that need verification._