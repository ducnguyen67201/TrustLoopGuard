# Personalized marketing demos

Personalized marketing demos are private-link concept pages prepared for a named company from public material. They let an outbound recipient see a relevant TrustLoopGuard control boundary at `/demo/{company-slug}` without creating a new route or deployment for every company.

## Ownership boundary

The private marketing Postgres database owns the page profile. `apps/marketing` reads the profile on the server and renders it; the browser never receives a database credential. The profile is campaign presentation data only:

- company and scenario labels;
- public-source workflow, risk, and rule copy;
- three illustrative outcomes: permit, require approval, and deny;
- display colors, source links, disclaimer, activation state, and expiry.

The profile does not define or execute a TrustLoopGuard policy. Runtime policies, authorization decisions, traces, agents, and API keys remain owned by the Rust service and `crates/tl-storage`. A profile may name a trusted `scenario_id`, but that identifier selects the presentation scenario only.

## Read path

The dynamic marketing route reads `outbound_demo_profiles` with the server-only `OUTBOUND_DEMO_DATABASE_URL`. A page is available only when all of these checks pass:

1. the path segment is a lowercase kebab-case slug;
2. the row is `active` and `live_verified`;
3. its optional expiry is still in the future;
4. the stored JSON passes the strict public demo profile schema;
5. the canonical URL in the profile matches the requested company slug.

Missing configuration, database errors, unknown companies, drafts, expired rows, or invalid profile JSON all return the standard not-found response. Personalized pages are marked `noindex, nofollow` and canonicalize to the generic `/demo` page.

## Write path

The outbound workflow researches the company, creates a profile from public facts, verifies the page, and upserts the profile before drafting an email that links to it. The outbound skill owns table initialization and writes. The marketing app opens read-only database sessions and selects only the `profile` JSON for an eligible slug.

The shared database contract uses `slug` as the unique lookup key and stores company/scenario labels, activation status, live-verification state, the public profile JSON, its SHA-256 hash, revision, optional expiry, activation time, and audit timestamps. The writer applies the schema idempotently before an upsert; the reader never creates or changes records.

Only public-facing scenario content belongs in this table. Recipient email addresses, personal contact data, CRM notes, credentials, and private research stay out of profile JSON. Updates increment the row revision and retain a SHA-256 hash so the outbound workflow can confirm that the exact verified profile is the active database version before drafting an email.

## Page behavior

The page is explicitly labeled as a public-source concept that is not connected to the named company or its systems. Selecting an outcome changes the illustrated proposal, evidence, and decision locally. It does not call the Rust decision API and must not be described as a live policy result. The existing `/demo` refund experience remains the live product demonstration.
