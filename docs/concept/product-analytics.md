# Product analytics

Product analytics measure how people use the Featherlane AI marketing site and dashboard. They are separate from customer guardrail analytics: PostHog never owns policies, traces, decisions, runs, settings, or any other runtime product state.

## Ownership and flow

```text
apps/marketing ── browser events ──┐
                                   ├──> one PostHog project
apps/web ──────── browser events ──┘

customer SDKs and Rust hot path ──X──> PostHog
```

Both Next.js apps initialize `posthog-js` from `instrumentation-client.ts`. Initialization is disabled when `NEXT_PUBLIC_POSTHOG_PROJECT_TOKEN` is absent, so local forks and CI do not require analytics configuration. Events carry an `app_surface` property (`marketing` or `dashboard`) so the shared project can be filtered without splitting the customer journey across projects.

No PostHog code belongs in the Rust event engine or customer SDKs. Product analytics must never add work to the guardrail hot path.

## Event contract

PostHog automatically records page views and supported browser interactions in both apps.

The marketing app also mirrors its existing typed GTM funnel events into PostHog:

| Event | Meaning |
|---|---|
| `landing_cta_click` | A visitor used a primary landing-page call to action. |
| `install_sdk_click` | A visitor opened an SDK installation path. |
| `book_meeting_click` | A visitor opened the meeting-booking flow. |
| `docs_click` | A visitor opened product documentation. |
| `github_click` | A visitor opened the GitHub repository. |
| `waitlist_submit` | A visitor submitted the waitlist form. |

Custom marketing events may include `page`, `location`, and `label`. Add a new event to the typed `MarketingEventName` union before emitting it; do not introduce ad hoc spellings at individual call sites.

The dashboard identifies authenticated people with the stable Auth.js/Rust user ID. `name` and `email` are person properties. Signing out resets the browser identity before the Auth.js session ends so a subsequent account does not inherit the previous account's events.

Do not send policy bodies, guard events, trace payloads, API keys, provider credentials, or other customer runtime content to PostHog.

## Configuration

Each deployed Next.js app requires:

```dotenv
NEXT_PUBLIC_POSTHOG_PROJECT_TOKEN=phc_replaceWithProjectToken
NEXT_PUBLIC_POSTHOG_HOST=https://us.i.posthog.com
```

The project token is intentionally public and client-side. A PostHog personal API key is privileged and must never use a `NEXT_PUBLIC_` variable.

## Dashboard recipe

Keep the initial PostHog dashboard small:

1. Marketing conversion funnel: `$pageview` → `landing_cta_click` → dashboard-domain `$pageview`.
2. Intent trend: the six typed marketing events, broken down by `page` and `location`.
3. Product adoption: unique identified users and `$pageview`, filtered to `app_surface = dashboard` and broken down by path.

Dashboard definitions live in PostHog. This repository owns the event names and properties that make those definitions stable.
