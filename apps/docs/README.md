# docs

The TrustLoopGuard documentation site, built with [Fumadocs](https://fumadocs.dev). Readable by both humans and LLM agents.

## Develop

From the repo root:

```sh
pnpm install
pnpm --filter docs dev
```

The site runs at <http://localhost:3001>.

To exercise the same-domain rewrite from the marketing site, run both in parallel:

```sh
pnpm --filter docs dev   # :3001
pnpm --filter web dev    # :3000 (proxies /docs/*, /llms.txt, etc. to :3001)
```

Then `http://localhost:3000/docs` reaches the docs through the rewrite chain.

## Content

Pages live under `content/docs/` as MDX. Sidebar order is controlled by `meta.json` files in each section. Add a new page by dropping `foo.mdx` into a section and adding `"foo"` to that section's `meta.json` `pages` array.

## HTTP API reference

`content/docs/reference/api/*.mdx` is generated from `docs/openapi.yaml`. Regenerate after editing the spec:

```sh
pnpm --filter docs gen:openapi
```

The OpenAPI YAML is itself generated from Rust types in `crates/tl-core` via `cargo run -p tl-codegen`. Single source of truth: Rust → openapi.yaml → MDX → site.

## Agent endpoints

Every page is reachable in three formats so LLM agents can consume the docs without scraping HTML:

| URL | Format |
|---|---|
| `/docs/<path>` | HTML page |
| `/docs/<path>.md` | Raw processed markdown |
| `/llms.txt` | Curated index per [llmstxt.org](https://llmstxt.org/) |
| `/llms-full.txt` | Entire docs concatenated as plain markdown |
| `/robots.txt` | Allow-all + Sitemap → `/llms.txt` |

The `<ViewOptionsPopover>` button on every docs page surfaces these to humans (Copy as Markdown, View on GitHub).

## Deploy

This app deploys cleanly to Vercel as its own project:

1. Create a new Vercel project pointing at this repo
2. Set **Root Directory** to `apps/docs`
3. Set environment variable `NEXT_PUBLIC_SITE_URL` to the public origin where the docs will be served (e.g. `https://trustloopguard.dev`, even if Vercel hosts at a different URL behind a rewrite)
4. Deploy

To make `https://trustloopguard.dev/docs/*` route to this app from the marketing site, set `DOCS_ORIGIN` in the `apps/web` Vercel project to this app's deployment URL (e.g. `https://trustloopguard-docs.vercel.app`). The rewrites in `apps/web/next.config.ts` handle the rest.

## Why a separate app

The Rust workspace and the marketing site (`apps/web`) are deliberately not coupled to docs. This app can be deployed independently, replaced, or removed without touching the rest of the repo.
