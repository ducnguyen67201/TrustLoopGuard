# Graph Report - TrustLoopGuard  (2026-05-10)

## Corpus Check
- 45 files · ~14,345 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 268 nodes · 292 edges · 29 communities (22 shown, 7 thin omitted)
- Extraction: 97% EXTRACTED · 3% INFERRED · 0% AMBIGUOUS · INFERRED: 9 edges (avg confidence: 0.67)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `28c9ab16`
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

## God Nodes (most connected - your core abstractions)
1. `Crates` - 13 edges
2. `Technical terms` - 13 edges
3. `Domain terms` - 12 edges
4. `Client` - 10 edges
5. `AsyncClient` - 10 edges
6. `main()` - 10 edges
7. `Ownership` - 9 edges
8. `Architecture` - 9 edges
9. `Plugin contract` - 8 edges
10. `getServerUrl()` - 5 edges

## Surprising Connections (you probably didn't know these)
- `main()` --calls--> `router()`  [INFERRED]
  crates/tl-cli/src/main.rs → crates/tl-server/src/lib.rs
- `Client` --uses--> `CheckRequest`  [INFERRED]
  sdks/python/src/trustloopguard/client.py → sdks/python/src/trustloopguard/_generated/types.py
- `Client` --uses--> `Decision`  [INFERRED]
  sdks/python/src/trustloopguard/client.py → sdks/python/src/trustloopguard/_generated/types.py
- `AsyncClient` --uses--> `CheckRequest`  [INFERRED]
  sdks/python/src/trustloopguard/client.py → sdks/python/src/trustloopguard/_generated/types.py
- `AsyncClient` --uses--> `Decision`  [INFERRED]
  sdks/python/src/trustloopguard/client.py → sdks/python/src/trustloopguard/_generated/types.py

## Communities (29 total, 7 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.09
Nodes (24): Home(), check(), getClient(), DecisionResponse, decisionResponseSchema, formSchema, FormValues, ParsedForm (+16 more)

### Community 1 - "Community 1"
Cohesion: 0.11
Nodes (13): BaseModel, Enum, Channel, CheckRequest, Decision, Severity, TriggeredPolicy, Verdict (+5 more)

### Community 2 - "Community 2"
Cohesion: 0.11
Nodes (17): Adding a new language binding, code:block1 (Guard.check(draft, ctx) -> Decision), code:block2 (fn check(draft: Draft, ctx: Context) -> Decision), code:block3 (Draft {), code:block4 (Context {), code:block5 (Decision {), code:block6 (fn push(chunk: String) -> StreamDecision), `Context` — anything the customer wants logged but not evaluated (+9 more)

### Community 3 - "Community 3"
Cohesion: 0.16
Nodes (8): Channel, CheckRequest, Decision, Severity, TriggeredPolicy, Verdict, Client, ClientOptions

### Community 4 - "Community 4"
Cohesion: 0.12
Nodes (16): Adding a new crate, code:block1 (tl-cli         tl-server         tl-sdk-rust), code:yaml (id: refund-promise), code:bash (cargo run -p tl-codegen           # write), Crates, Dependency graph, `tl-cli` — operator command line, `tl-codegen` — derived-artifact generator (+8 more)

### Community 5 - "Community 5"
Cohesion: 0.13
Nodes (14): Action vs Verdict, Agent, Channel, CheckRequest, Decision, Domain terms, Glossary, Matcher (+6 more)

### Community 6 - "Community 6"
Cohesion: 0.21
Nodes (10): Args, Cli, Cmd, main(), render_pydantic(), repo_root(), write_or_check(), load_str() (+2 more)

### Community 7 - "Community 7"
Cohesion: 0.15
Nodes (13): Cold path, Decision log, Embedded mode, Fail-open vs fail-closed, Hosted mode, Hot path, Latency budget, LLM judge (+5 more)

### Community 8 - "Community 8"
Cohesion: 0.17
Nodes (11): 1. Lane ownership, 2. Critical-path crates, 3. Contracts (the only cross-lane surface), 4. Wire-format versioning, 5. Demo independence checkpoint, 6. Conflict resolution, 7. When to split further, Founder A — Engine + Plugin SDK (+3 more)

### Community 9 - "Community 9"
Cohesion: 0.17
Nodes (11): Architecture, code:block1 (+-------------------+      CheckRequest       +-------------), code:block2 (CheckRequest), End-state to keep in mind, Latency budget (committed), Layered model: input to verdict, Request lifecycle (HTTP path), The shape of one call (+3 more)

### Community 10 - "Community 10"
Cohesion: 0.2
Nodes (8): allow_helper_sets_verdict(), Channel, CheckRequest, Decision, Severity, TlError, TriggeredPolicy, Verdict

### Community 11 - "Community 11"
Cohesion: 0.18
Nodes (4): ApiDoc, AppState, router(), MemoryStore

### Community 12 - "Community 12"
Cohesion: 0.33
Nodes (4): matcher_hits(), policy_matches(), empty_engine_allows(), Engine

### Community 13 - "Community 13"
Cohesion: 0.29
Nodes (5): Action, MatchClause, Matcher, Policy, WhenClause

### Community 14 - "Community 14"
Cohesion: 0.38
Nodes (3): buffer_truncates_to_window(), StreamDecision, StreamingChecker

### Community 15 - "Community 15"
Cohesion: 0.29
Nodes (5): 1. Think Before Coding, 2. Simplicity First, 3. Surgical Changes, 4. Goal-Driven Execution, code:block1 (1. [Step] → verify: [check])

### Community 17 - "Community 17"
Cohesion: 0.33
Nodes (5): code:block1 (agent proposes output → trustloop.check(...) → allow | block), Reading order, TrustLoopGuard concepts, What TrustLoopGuard is, When to update these docs

### Community 20 - "Community 20"
Cohesion: 0.67
Nodes (3): diff(), replay_against(), ReplayDiff

## Knowledge Gaps
- **113 isolated node(s):** `ClientOptions`, `HTTP client for TrustLoopGuard. Mirrors the `Guard.check(draft, ctx)` plugin con`, `Synchronous TrustLoopGuard client.      Args:         base_url: TrustLoopGuard s`, `Async TrustLoopGuard client. Same surface as ``Client`` but awaitable.`, `TrustLoopGuard Python SDK.  Public surface:     Client          — HTTP client im` (+108 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **7 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `main()` connect `Community 6` to `Community 11`, `Community 12`?**
  _High betweenness centrality (0.012) - this node is a cross-community bridge._
- **Why does `router()` connect `Community 11` to `Community 6`?**
  _High betweenness centrality (0.007) - this node is a cross-community bridge._
- **Why does `Technical terms` connect `Community 7` to `Community 5`?**
  _High betweenness centrality (0.007) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `Client` (e.g. with `CheckRequest` and `Decision`) actually correct?**
  _`Client` has 2 INFERRED edges - model-reasoned connections that need verification._
- **Are the 2 inferred relationships involving `AsyncClient` (e.g. with `CheckRequest` and `Decision`) actually correct?**
  _`AsyncClient` has 2 INFERRED edges - model-reasoned connections that need verification._
- **What connects `ClientOptions`, `HTTP client for TrustLoopGuard. Mirrors the `Guard.check(draft, ctx)` plugin con`, `Synchronous TrustLoopGuard client.      Args:         base_url: TrustLoopGuard s` to the rest of the system?**
  _113 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.09 - nodes in this community are weakly interconnected._