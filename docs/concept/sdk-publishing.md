# SDK publishing

This doc owns the release process for the TypeScript SDK package
`@trustloopguard/sdk`.

The source of truth for the automated npm release is
`.github/workflows/publish-sdk-npm.yml`. The workflow runs only when a tag that
matches `sdk-v*` is pushed.

## Release contract

- The git tag is the release request. Use `sdk-vX.Y.Z`.
- The tag version is the npm package version. The workflow strips `sdk-v` and
  runs `npm version X.Y.Z --no-git-tag-version` inside `sdks/typescript`.
- npm versions are immutable. If `@trustloopguard/sdk@X.Y.Z` already exists,
  the publish step will fail with `E403`.
- The `sdk-publish` environment is an approval gate. A reviewer must approve
  the pending deployment before npm publish runs.
- The npm token lives in the `NPM_TOKEN` GitHub Actions secret.
- `package.json` uses one export map for local and published consumers. Runtime
  imports resolve to `dist/index.js`; declarations resolve to `dist/index.d.ts`.
- `pnpm --filter @trustloopguard/sdk test:package` packs the exact npm artifact,
  imports it in Node, compiles a TypeScript consumer, verifies `guardAgent`, and
  rejects source or generated runtime files that should not ship.
- Relative ESM imports in emitted JavaScript include `.js` extensions so the
  packed artifact loads in Node without a custom resolver.

## Before tagging

Confirm the version you want to publish is not already on npm:

```bash
npm view @trustloopguard/sdk version
npm view @trustloopguard/sdk@X.Y.Z version
```

The second command should return `E404` for a new release version.

Confirm the tag does not already exist locally or remotely:

```bash
git tag -l 'sdk-vX.Y.Z'
git ls-remote --tags origin 'sdk-vX.Y.Z'
```

Verify the SDK and the exact packed artifact:

```bash
pnpm --filter @trustloopguard/sdk typecheck
pnpm --filter @trustloopguard/sdk test
pnpm --filter @trustloopguard/sdk test:package
```

Choose a target commit that contains the SDK code you want to publish and the
current tag-based publish workflow. Do not tag a commit whose commit message
contains a GitHub skip token such as `[skip ci]`; GitHub can skip the tag
workflow too. If the latest `main` commit is a generated `[skip ci]` commit,
tag the nearest suitable parent that contains the intended SDK source, or make
a normal release commit and tag that.

## Publish

Create and push the release tag:

```bash
git fetch origin main --tags
git tag sdk-vX.Y.Z origin/main
git push origin sdk-vX.Y.Z
```

If a local pre-push hook fails because of unrelated uncommitted work, do not
fix or stage unrelated files just to publish. Verify the tag points at the
intended remote commit, then push only the tag with hooks skipped:

```bash
git rev-parse sdk-vX.Y.Z
git push --no-verify origin sdk-vX.Y.Z
```

Approve the pending deployment in GitHub Actions:

1. Open the `Publish SDK` workflow run for the tag.
2. Click the pending `sdk-publish` deployment review.
3. Approve and deploy.

The workflow then installs dependencies, typechecks, builds, tests, validates
the packed artifact, and publishes to npm with provenance.

## Verify

Watch the run until completion:

```bash
gh run list --workflow "Publish SDK" --limit 5
gh run watch <run-id> --exit-status
```

Confirm npm shows the new version:

```bash
npm view @trustloopguard/sdk version
```

## Common failures

`E403 You cannot publish over the previously published versions`

The version already exists on npm. Pick the next semver version, create a new
`sdk-vX.Y.Z` tag, and publish that. Do not rerun the same version.

No workflow run appears after pushing the tag

Check whether the tagged commit message contains `[skip ci]` or another skip
token. Move the tag to a suitable non-skipped commit or create a normal release
commit, then force-push the tag update:

```bash
git tag -f sdk-vX.Y.Z <commit>
git push --force origin refs/tags/sdk-vX.Y.Z
```

Use this only before a successful publish. After npm accepts a version, never
move the published version tag to different source.
