# docs

The TrustLoopGuard documentation site, built with [Fumadocs](https://fumadocs.dev).

## Develop

From the repo root:

```sh
pnpm install
pnpm --filter docs dev
```

The site runs at <http://localhost:3001>.

## Password protection

Set this env var to require a password before viewing docs:

```sh
DOCS_PASSWORD=replace-with-a-secret
```

When the value is unset, the docs app stays public. This keeps local development
frictionless while allowing private beta deployments to be gated by environment
configuration. The browser may still show a username field in its auth prompt;
any username is accepted, and only the password is checked.

## Content

Pages live under `content/docs/` as MDX. Sidebar order is controlled by
`meta.json` files in each section. Add a new page by dropping `foo.mdx` into a
section and adding `"foo"` to that section's `meta.json` `pages` array.

## Why a separate app

The Rust workspace and the marketing site (`apps/web`) are deliberately not
coupled to docs. This app can be deployed independently to any static host or
Vercel project.
