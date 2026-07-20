# Personalized marketing demos

Personalized marketing demos are private-link concept pages prepared for a named company from public material. Normal outbound pages use a company-neutral workflow category at `/demo/{workflow-category}`, such as `/demo/cloud-storage-security`; company names and domains do not belong in this one-level route. Fixed `healthcare` and `procurement` runtimes also support categorized presentation profiles at `/demo/{category}/{company-slug}`.

## Ownership boundary

The private marketing Postgres database owns the page profile. `apps/marketing` reads the profile on the server and renders it; the browser never receives a database credential. The profile is campaign presentation data only:

- category, company, and scenario labels;
- public-source workflow, risk, and rule copy;
- three illustrative outcomes: permit, require approval, and deny;
- display colors, provenance sources, disclaimer, activation state, and expiry.

Provenance sources remain in the server-side profile and outbound review packet. Prospect-facing pages do not render those URLs as navigation links.

The profile does not define or execute a TrustLoopGuard policy. Runtime policies, authorization decisions, traces, agents, and API keys remain owned by the Rust service and `crates/tl-storage`. A profile names a trusted `scenario_id`; the server maps that identifier to a fixed, reviewed Rust-owned runtime scenario. Company fields provide bounded model context but never add, remove, select, or rewrite runtime policies.

The trusted category-to-scenario mappings are:

- `healthcare` → `healthcare-scheduling-v1` for the fixed scheduling demo, or
  `internal-agent-tool-action-v1` for a reviewed contextual workflow;
- `procurement` → `procurement-submit-po-v1`.
- `generic` → `internal-agent-tool-action-v1`.

All generic profiles share one Rust workspace named `Contextual Demo`, its default environment, one `contextual-demo-agent` principal, and one agent-bound runtime key. Profile creation does not create a workspace, environment, API key, agent, or policy. Setup provisions the shared runtime once and safely reuses it on later runs. Because workspace creation seeds generic policies, contextual setup explicitly disables those starters and keeps the dedicated workspace on the reviewed five-policy pack.

## Read path

The dynamic marketing routes read `outbound_demo_profiles` with the server-only `OUTBOUND_DEMO_DATABASE_URL`. Saving a valid row makes its private-link page available immediately; `status`, `live_verified`, and `expires_at` remain workflow and audit metadata rather than visibility gates. A generic page is available only when all of these checks pass:

1. the workflow category is a lowercase kebab-case slug that is not the normalized company name or domain;
2. exactly one `generic` row exists for that slug;
3. the stored JSON passes the strict public demo profile schema;
4. the canonical URL in the profile is exactly `/demo/{workflow-category}`;
5. the company name is present for the text-only company treatment.
6. `scenario_id` is the reviewed generic scenario `internal-agent-tool-action-v1`.

Categorized healthcare pages require either the fixed scheduling scenario or the reviewed contextual scenario. Procurement pages require their fixed category-to-scenario mapping. Both use the canonical `/demo/{category}/{company-slug}` URL.

Missing configuration, database errors, unknown companies, or invalid profile JSON return the standard not-found response. Personalized pages are marked `noindex, nofollow` and canonicalize to their generic category page.

## Write path

The outbound workflow researches the company, creates a profile from public facts, verifies the page, and upserts the profile before drafting an email that links to it. The outbound skill owns table initialization and writes. The marketing app opens read-only database sessions and selects only the `profile` JSON for an eligible slug.

The shared database contract uses `(category, slug)` as the unique lookup key and stores category, company/scenario labels, activation status, live-verification state, the public profile JSON, its SHA-256 hash, revision, optional expiry, activation time, and audit timestamps. The writer applies the schema idempotently before an upsert; the reader never creates or changes records. An upsert may update a route only for the same company, so choosing an existing workflow category cannot replace another prospect's page.

Only public-facing scenario content belongs in this table. Recipient email addresses, personal contact data, CRM notes, credentials, and private research stay out of profile JSON. Updates increment the row revision and retain a SHA-256 hash so the outbound workflow can confirm that the exact verified profile is the active database version before drafting an email.

## Page behavior

Every page is explicitly labeled as a public-source concept that is not connected to the named company or its systems. Personalized pages show the company name as text only; company logos, favicons, and other brand images are not stored or rendered. Research links are never rendered.

Generic pages are live, synthetic, multi-turn chats. The browser sends only a session id, the current message, and bounded conversation history to the same-origin contextual route. The route loads the public profile on the server, rejects browser-supplied profile or policy fields, and injects bounded profile context into one OpenAI response call. The current message is checked by Rust before OpenAI runs, and the model draft is checked by Rust before it reaches the browser. An input deny or approval decision skips OpenAI. A rejected output draft is never exposed.

The right-side monitor reads the enabled shared-pack policy inventory and matched findings from Rust. Profile path copy supplies the three suggested chat prompts, but it is not a policy definition and does not predetermine the result. If the profile store, policy registry, Rust check, model, or response contract is unavailable, the request fails closed. `/demo/healthcare` and `/demo/procurement` remain the public generic entry points for their fixed runtimes.
