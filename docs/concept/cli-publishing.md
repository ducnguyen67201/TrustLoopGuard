# CLI publishing

This doc owns the release process for `@trustloopguard/cli`. The automated npm release lives in
`.github/workflows/publish-cli-npm.yml` and runs only for tags matching `cli-v*`.

## Release contract

- `apps/cli/package.json` and `apps/cli/src/version.ts` carry the same version.
- A release tag uses `cli-vX.Y.Z`; its suffix must match the package version exactly.
- The tagged commit must be an ancestor of `origin/main`.
- The workflow installs locked dependencies, typechecks, builds, runs tests, and executes the
  packed-artifact smoke test before publishing with npm provenance.
- `pnpm --filter @trustloopguard/cli test:package` extracts the exact tarball, verifies the bin and
  copied runtime modules, and runs install/status/uninstall against isolated user directories.
- The package contains `dist/**`, `README.md`, and `LICENSE`, not TypeScript sources or tests.
- npm versions are immutable. Increment the package and runtime version before requesting another
  release.

The workflow is release machinery, not permission to publish. Pushing the tag and approving any
protected GitHub environment remain explicit operator actions.

## Before tagging

```bash
npm view @trustloopguard/cli@X.Y.Z version
git tag -l 'cli-vX.Y.Z'
git ls-remote --tags origin 'cli-vX.Y.Z'
pnpm --filter @trustloopguard/cli typecheck
pnpm --filter @trustloopguard/cli test
pnpm --filter @trustloopguard/cli test:package
```

The npm query should return `E404` for a new version, and the tag queries should be empty.

## Publish and verify

After review, tag the intended commit and push only that tag:

```bash
git fetch origin main --tags
git tag cli-vX.Y.Z origin/main
git push origin cli-vX.Y.Z
```

Watch the `Publish CLI` workflow and confirm the resulting package:

```bash
gh run list --workflow "Publish CLI" --limit 5
gh run watch <run-id> --exit-status
npm view @trustloopguard/cli version
```

Never move a tag after npm has accepted that version.
