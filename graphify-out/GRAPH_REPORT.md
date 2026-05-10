# Graph Report - TrustLoopGuard  (2026-05-10)

## Corpus Check
- 112 files · ~37,473 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 743 nodes · 1060 edges · 68 communities (59 shown, 9 thin omitted)
- Extraction: 93% EXTRACTED · 7% INFERRED · 0% AMBIGUOUS · INFERRED: 71 edges (avg confidence: 0.78)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `effa5ce5`
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
- [[_COMMUNITY_Community 49|Community 49]]
- [[_COMMUNITY_Community 50|Community 50]]
- [[_COMMUNITY_Community 51|Community 51]]
- [[_COMMUNITY_Community 52|Community 52]]
- [[_COMMUNITY_Community 53|Community 53]]

## God Nodes (most connected - your core abstractions)
1. `run()` - 15 edges
2. `v0 Design Decisions` - 15 edges
3. `main()` - 13 edges
4. `Crates` - 13 edges
5. `Technical terms` - 13 edges
6. `Domain terms` - 12 edges
7. `Client` - 10 edges
8. `AsyncClient` - 10 edges
9. `load_agent_str()` - 10 edges
10. `LlmRouter` - 10 edges

## Surprising Connections (you probably didn't know these)
- `main()` --calls--> `load_str()`  [INFERRED]
  crates/tl-cli/src/main.rs → crates/tl-policy/src/policy_parse.rs
- `mock_embedder_round_trip_through_index()` --calls--> `word_bag_embed()`  [INFERRED]
  crates/tl-fuzzy/src/index.rs → crates/tl-fuzzy/src/embedder.rs
- `fifty_policies()` --calls--> `load_str()`  [INFERRED]
  crates/tl-engine/benches/check_pipeline.rs → crates/tl-policy/src/policy_parse.rs
- `tier2_fuzzy_hit_blocks_through_real_orchestrator()` --calls--> `load_str()`  [INFERRED]
  crates/tl-engine/src/lib.rs → crates/tl-policy/src/policy_parse.rs
- `semantic_policy()` --calls--> `load_str()`  [INFERRED]
  crates/tl-engine/src/fuzzy.rs → crates/tl-policy/src/policy_parse.rs

## Communities (68 total, 9 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.09
Nodes (19): Client, SdkError, migrate(), PostgresStore, verdict_text(), build_from_config_validates_referenced_providers(), JudgeKind, LlmOutput (+11 more)

### Community 1 - "Community 1"
Cohesion: 0.08
Nodes (25): Home(), check(), getClient(), DecisionResponse, decisionResponseSchema, formSchema, FormValues, ParsedForm (+17 more)

### Community 2 - "Community 2"
Cohesion: 0.08
Nodes (18): BudgetConfig, ConfigError, empty_budgets_section_uses_default(), ProviderConfig, ProviderTarget, round_trips_sample_config(), RouteConfig, RouterConfig (+10 more)

### Community 3 - "Community 3"
Cohesion: 0.15
Nodes (20): bench_check_async_50_policies_4kb(), bench_check_async_empty_default(), bench_check_sync_empty(), bench_universal_only_4kb(), fifty_policies(), large_req(), small_req(), HandlerCtx (+12 more)

### Community 4 - "Community 4"
Cohesion: 0.1
Nodes (14): BuildError, dedup_when_both_tiers_match_same_policy(), empty_policies_yields_no_hits(), HnswFuzzyChecker, levenshtein_catches_typo_bypass(), levenshtein_misses_unrelated_text(), literal_policy(), semantic_match_on_paraphrase() (+6 more)

### Community 5 - "Community 5"
Cohesion: 0.06
Nodes (30): 10. Crate alignment, 11. Build order (v0), 12. Open questions (need answers before phase 1), 13. Things deliberately not in v0, 14. Confirmation checklist, 1. Product boundary (locked), 2. The wedge (locked), 3. The check contract (locked) (+22 more)

### Community 6 - "Community 6"
Cohesion: 0.09
Nodes (20): load_agent_str(), loads_committed_fixture_acme_support_v3(), parses_full_featured_profile(), parses_minimal_profile(), rejects_malformed_yaml(), rejects_missing_agent_id(), rejects_missing_in_scope(), validate() (+12 more)

### Community 7 - "Community 7"
Cohesion: 0.15
Nodes (27): aggregate(), authority_violation_blocks(), bulleted(), cancelled(), CannedClient, ctx_with(), empty_router_yields_skipped(), extract_docs() (+19 more)

### Community 8 - "Community 8"
Cohesion: 0.1
Nodes (14): BaseModel, Enum, Channel, CheckRequest, Decision, Severity, TierResult, TriggeredPolicy (+6 more)

### Community 9 - "Community 9"
Cohesion: 0.07
Nodes (27): Action vs Verdict, Agent, Channel, CheckRequest, Cold path, Decision, Decision log, Domain terms (+19 more)

### Community 10 - "Community 10"
Cohesion: 0.12
Nodes (10): cc_candidate(), detect(), email(), first_valid_credit_card(), ids(), ipv4(), luhn_valid(), returns_multiple_hits() (+2 more)

### Community 11 - "Community 11"
Cohesion: 0.13
Nodes (18): canonical_json(), context_object_key_order_does_not_affect_key(), different_domain_changes_key(), different_drafts_hash_differently(), for_check_request(), identical_requests_hash_equal(), missing_domain_is_treated_as_default(), nested_objects_canonicalise_recursively() (+10 more)

### Community 12 - "Community 12"
Cohesion: 0.13
Nodes (11): Channel, CheckRequest, Decision, Severity, Tier, TierResult, TierStatus, TriggeredPolicy (+3 more)

### Community 13 - "Community 13"
Cohesion: 0.15
Nodes (16): allow_helper_sets_verdict(), Channel, CheckRequest, Decision, new_trace_id(), Severity, TlError, TriggeredPolicy (+8 more)

### Community 14 - "Community 14"
Cohesion: 0.18
Nodes (10): cosine(), Embedder, EmbedError, FastEmbedder, fnv1a(), mock_embedder_is_deterministic(), mock_embedder_normalises_to_unit(), MockEmbedder (+2 more)

### Community 15 - "Community 15"
Cohesion: 0.11
Nodes (17): Adding a new language binding, code:block1 (Guard.check(draft, ctx) -> Decision), code:block2 (fn check(draft: Draft, ctx: Context) -> Decision), code:block3 (Draft {), code:block4 (Context {), code:block5 (Decision {), code:block6 (fn push(chunk: String) -> StreamDecision), `Context` — anything the customer wants logged but not evaluated (+9 more)

### Community 16 - "Community 16"
Cohesion: 0.12
Nodes (16): Adding a new crate, code:block1 (tl-cli         tl-server         tl-sdk-rust), code:yaml (id: refund-promise), code:bash (cargo run -p tl-codegen           # write), Crates, Dependency graph, `tl-cli` — operator command line, `tl-codegen` — derived-artifact generator (+8 more)

### Community 17 - "Community 17"
Cohesion: 0.33
Nodes (9): dim_mismatch_yields_empty_query(), empty_index_returns_empty_query(), HnswIndex, identical_vector_scores_one(), IndexHit, mock_embedder_round_trip_through_index(), orthogonal_vector_below_threshold(), ranks_by_similarity_descending() (+1 more)

### Community 18 - "Community 18"
Cohesion: 0.27
Nodes (7): disabled_cache_never_stores(), fake_decision(), miss_returns_none(), MokaCache, put_overwrites_existing_key(), put_then_get_returns_value(), ttl_expires_old_entries()

### Community 19 - "Community 19"
Cohesion: 0.26
Nodes (8): BudgetExceeded, BudgetState, exceeding_default_limit_errors(), tenant_limit_overrides_default(), TokenBudget, unknown_tenant_uses_default_limit(), used_returns_running_total(), zero_limit_means_unlimited()

### Community 20 - "Community 20"
Cohesion: 0.14
Nodes (13): 1. Engine-only PRs aren't done, 2. No internal imports in `apps/` or `demo/`, 3. The README quickstart works on a clean machine, 4. Cross-cutting concerns live in the SDK, once, How features are built (the loop), Out of scope, Required CI gates, Reviewer checklist (+5 more)

### Community 21 - "Community 21"
Cohesion: 0.26
Nodes (10): matcher_hits(), policy_matches(), benign_request_no_block(), block_signal_from_action(), pii_in_output_blocks(), prompt_injection_in_input_escalates(), req_with(), run() (+2 more)

### Community 22 - "Community 22"
Cohesion: 0.17
Nodes (11): 1. Lane ownership, 2. Critical-path crates, 3. Contracts (the only cross-lane surface), 4. Wire-format versioning, 5. Demo independence checkpoint, 6. Conflict resolution, 7. When to split further, Founder A — Engine + Plugin SDK (+3 more)

### Community 23 - "Community 23"
Cohesion: 0.17
Nodes (11): Architecture, code:block1 (+-------------------+      CheckRequest       +-------------), code:block2 (CheckRequest), End-state to keep in mind, Latency budget (committed), Layered model: input to verdict, Request lifecycle (HTTP path), The shape of one call (+3 more)

### Community 24 - "Community 24"
Cohesion: 0.29
Nodes (7): detect(), detects_classic_injection(), detects_dan_mode(), detects_role_override(), distinct_patterns_each_fire_once(), ids(), matcher()

### Community 25 - "Community 25"
Cohesion: 0.29
Nodes (5): AgentAuthority, AgentProfile, AgentScope, AgentTone, KnowledgeSource

### Community 27 - "Community 27"
Cohesion: 0.25
Nodes (5): AgentAuthority, AgentProfile, AgentScope, AgentTone, KnowledgeSource

### Community 28 - "Community 28"
Cohesion: 0.25
Nodes (5): FuzzyChecker, FuzzyHit, NoOpFuzzyChecker, NoOpProfileResolver, ProfileResolver

### Community 29 - "Community 29"
Cohesion: 0.54
Nodes (7): deadline_exceeded_yields_timeout(), malformed_inner_json_yields_parse_error(), non_2xx_yields_status_error(), ok_response(), openai_sends_bearer_auth_and_json_schema_body(), openrouter_adds_http_referer(), schema()

### Community 30 - "Community 30"
Cohesion: 0.29
Nodes (5): Action, MatchClause, Matcher, Policy, WhenClause

### Community 31 - "Community 31"
Cohesion: 0.48
Nodes (6): authority_template_substitutes_all_placeholders(), build(), hallucination_template_substitutes_all_placeholders(), schema(), schemas_have_required_fields(), tone_template_substitutes_all_placeholders()

### Community 32 - "Community 32"
Cohesion: 0.38
Nodes (3): buffer_truncates_to_window(), StreamDecision, StreamingChecker

### Community 33 - "Community 33"
Cohesion: 0.29
Nodes (5): 1. Think Before Coding, 2. Simplicity First, 3. Surgical Changes, 4. Goal-Driven Execution, code:block1 (1. [Step] → verify: [check])

### Community 34 - "Community 34"
Cohesion: 0.33
Nodes (5): code:block1 (agent proposes output → trustloop.check(...) → allow | block), Reading order, TrustLoopGuard concepts, What TrustLoopGuard is, When to update these docs

### Community 35 - "Community 35"
Cohesion: 0.33
Nodes (5): code:sh (pnpm install), Content, Develop, docs, Why a separate app

### Community 36 - "Community 36"
Cohesion: 0.5
Nodes (3): metadata, RootLayout(), RootLayoutProps

### Community 37 - "Community 37"
Cohesion: 0.4
Nodes (3): Tier, TierResult, TierStatus

### Community 38 - "Community 38"
Cohesion: 0.4
Nodes (4): JsonSchema, LlmClient, LlmError, LlmOutput

### Community 39 - "Community 39"
Cohesion: 0.4
Nodes (4): Agent profile, Conversation, Grounding documents, Task

### Community 41 - "Community 41"
Cohesion: 0.83
Nodes (3): block_signal_from_hit(), cancelled(), run()

### Community 42 - "Community 42"
Cohesion: 0.83
Nodes (3): openai_round_trip(), openrouter_round_trip(), trivial_schema()

### Community 43 - "Community 43"
Cohesion: 0.67
Nodes (3): diff(), replay_against(), ReplayDiff

### Community 44 - "Community 44"
Cohesion: 0.5
Nodes (3): Agent authority profile, Conversation, Task

### Community 45 - "Community 45"
Cohesion: 0.5
Nodes (3): Agent tone profile, Conversation, Task

## Knowledge Gaps
- **201 isolated node(s):** `ClientOptions`, `AgentProfile`, `HTTP client for TrustLoopGuard. Mirrors the `Guard.check(draft, ctx)` plugin con`, `Synchronous TrustLoopGuard client.      Args:         base_url: TrustLoopGuard s`, `Async TrustLoopGuard client. Same surface as ``Client`` but awaitable.` (+196 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **9 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `main()` connect `Community 6` to `Community 0`, `Community 2`, `Community 3`, `Community 4`?**
  _High betweenness centrality (0.047) - this node is a cross-community bridge._
- **Why does `load_str()` connect `Community 4` to `Community 0`, `Community 3`, `Community 6`?**
  _High betweenness centrality (0.034) - this node is a cross-community bridge._
- **Why does `fake_decision()` connect `Community 18` to `Community 13`?**
  _High betweenness centrality (0.023) - this node is a cross-community bridge._
- **Are the 6 inferred relationships involving `main()` (e.g. with `.parse()` and `.ok()`) actually correct?**
  _`main()` has 6 INFERRED edges - model-reasoned connections that need verification._
- **What connects `ClientOptions`, `AgentProfile`, `HTTP client for TrustLoopGuard. Mirrors the `Guard.check(draft, ctx)` plugin con` to the rest of the system?**
  _201 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.09 - nodes in this community are weakly interconnected._
- **Should `Community 1` be split into smaller, more focused modules?**
  _Cohesion score 0.08 - nodes in this community are weakly interconnected._