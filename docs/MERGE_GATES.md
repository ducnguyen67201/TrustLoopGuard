# Merge gates

The four CI workflows below are **required status checks** on the `main`
branch. They enforce the SDK-driven discipline mechanically — see
[`SDK_DRIVEN.md`](SDK_DRIVEN.md) for the philosophy.

| Workflow                  | Job name                                    |
|---------------------------|---------------------------------------------|
| `codegen-check.yml`       | `Generated artifacts in sync with Rust types` |
| `sdk-build.yml`           | `Rust SDK`, `TypeScript SDK`, `Python SDK (3.10|3.11|3.12)` |
| `quickstart.yml`          | `README quickstart end-to-end`              |
| `lint-sdk-boundary.yml`   | `apps/ + demo/ import only published SDK surface` |

A maintainer enables these on the GitHub side; the workflow files in
this repo only define the checks themselves.

## Enabling required status checks (one-time, GitHub Settings)

> Anyone with **Maintain** or **Admin** rights on the repo can do this.
> No code change is needed.

1. Open <https://github.com/duc/TrustLoopGuard/settings/branches>.
2. Click **Add branch ruleset** (or edit the existing rule for `main`).
3. Under **Branch name pattern**, enter `main`.
4. Tick **Require a pull request before merging**.
5. Tick **Require status checks to pass before merging**.
6. Tick **Require branches to be up to date before merging** (this
   forces a rebase before merge so the gates run against the merge
   tip, not a stale PR head).
7. In the search box under **Status checks that are required**, add:
   - `Generated artifacts in sync with Rust types`
   - `Rust SDK`
   - `TypeScript SDK`
   - `Python SDK (3.10)`
   - `Python SDK (3.11)`
   - `Python SDK (3.12)`
   - `README quickstart end-to-end`
   - `apps/ + demo/ import only published SDK surface`
8. Save.

After saving, every PR sees these eight checks as required-and-blocking.

## Bypass policy

Don't. The discipline only works because we refuse to merge half-shipped
work. If a gate is broken (false positive, infrastructure flake), fix
the gate — don't bypass it. A "just this once" admin override on `main`
costs more than the half-day to repair the workflow.

The only legitimate bypass is a hot revert of code that's already on
`main` and breaking production. In that case, ship the revert, then
follow up with a normal PR that brings the gates green again.

## Updating the gate set

When a new gate is added to the SDK-driven stack:

1. Land the workflow file in a normal PR.
2. Wait until the workflow has run green on at least one PR (so the
   check name appears in GitHub's autocomplete).
3. Open the branch ruleset and add the new check name to the required
   list.
4. Update this doc and `SDK_DRIVEN.md` so the catalog stays accurate.

When a gate is retired:

1. Delete the workflow file in a normal PR.
2. Open the branch ruleset and remove the obsolete check name.
3. Update this doc.

## What the gates *don't* enforce

These workflows can't tell you whether the SDK surface is *good* — only
whether the surface is *consistent* (all three SDKs in sync) and
*self-sufficient* (the example apps don't cheat). Reviewer judgment is
still required for:

- API ergonomics ("is this the call site a stranger would write?")
- Naming and discoverability
- Documentation quality beyond the README quickstart
- Performance characteristics under load

The gates raise the floor. The reviewer raises the ceiling.
