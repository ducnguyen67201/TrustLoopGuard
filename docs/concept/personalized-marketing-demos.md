# Personalized marketing demos

Personalized marketing demos are private-link concept pages prepared for a named company from public material. They let an outbound recipient see a relevant TrustLoopGuard control boundary without creating a new route or UI for every company. Generic outbound concepts use `/demo/{company-slug}`. Fixed runtime concepts use `/demo/{category}/{company-slug}` with the `healthcare` or `procurement` renderer.

## Ownership boundary

The private marketing Postgres database owns the page profile. `apps/marketing` reads the profile on the server and renders it; the browser never receives a database credential. The profile is campaign presentation data only:

- category, company, and scenario labels;
- public-source workflow, risk, and rule copy;
- three illustrative outcomes: permit, require approval, and deny;
- display colors, source links, disclaimer, activation state, and expiry.

The profile does not define or execute a TrustLoopGuard policy. Runtime policies, authorization decisions, traces, agents, and API keys remain owned by the Rust service and `crates/tl-storage`. A profile names a trusted `scenario_id`; a categorized live demo may use that identifier to select a fixed, reviewed Rust-owned runtime scenario. Company fields never add, remove, or rewrite runtime policies.

The trusted category-to-scenario mappings are:

- `generic` → a lowercase kebab-case scenario identifier grounded in the saved workflow;
- `healthcare` → `healthcare-scheduling-v1`;
- `procurement` → `procurement-submit-po-v1`.

## Read path

The dynamic marketing route reads `outbound_demo_profiles` with the server-only `OUTBOUND_DEMO_DATABASE_URL`. Saving a valid row makes its private-link page available immediately; `status`, `live_verified`, and `expires_at` remain workflow and audit metadata rather than visibility gates. A page is available only when all of these checks pass:

1. the category is supported and the company path segment is a lowercase kebab-case slug;
2. a categorized lookup has one row for its category and slug, or a one-level lookup has exactly one row for the slug;
3. the stored JSON passes the strict public demo profile schema;
4. the canonical URL in the profile matches either its generic one-level route or its categorized route;
5. a categorized page recognizes the profile's fixed trusted `scenario_id`.

Missing configuration, database errors, unknown companies, duplicate one-level slugs, or invalid profile JSON return the standard not-found response. Personalized pages are marked `noindex, nofollow` and canonicalize to the URL stored in the validated profile.

## Write path

The outbound workflow researches the company, creates a profile from public facts, verifies the page, and upserts the profile before drafting an email that links to it. The outbound skill owns table initialization and writes. The marketing app opens read-only database sessions and selects only the `profile` JSON for an eligible slug.

The shared database contract uses `(category, slug)` as the unique lookup key and stores category, company/scenario labels, activation status, live-verification state, the public profile JSON, its SHA-256 hash, revision, optional expiry, activation time, and audit timestamps. This allows one company slug to have both healthcare and procurement demos. The writer applies the schema idempotently before an upsert; the reader never creates or changes records.

Only public-facing scenario content belongs in this table. Recipient email addresses, personal contact data, CRM notes, credentials, and private research stay out of profile JSON. Updates increment the row revision and retain a SHA-256 hash so the outbound workflow can confirm that the exact verified profile is the active database version before drafting an email.

## Page behavior

Every page is explicitly labeled as a public-source concept that is not connected to the named company or its systems. Generic concept renderers change illustrative proposal, evidence, and decision data locally and must not be described as live policy results. Categorized live renderers may reuse an existing fixed runtime, such as the healthcare scheduling or procurement agent, but company presentation data never changes that runtime's policy set. `/demo/{company-slug}` renders a generic saved profile with the shared concept UI. `/demo/healthcare` and `/demo/procurement` remain the categorized entry points, while their `/{company-slug}` routes apply a valid saved profile to the corresponding fixed runtime.
