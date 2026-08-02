# SDK Recipes

This directory is the source of truth for copyable Featherlane AI SDK setup
snippets.

Add or update recipe YAML files under `recipes/sdk/`, then run:

```bash
pnpm recipes:update
```

The update command scans `recipes/**/*.yaml` and rewrites every target block
declared by each recipe. CI runs:

```bash
pnpm recipes:check
```

That check fails when a README, demo, or docs snippet has drifted from the
canonical recipe.

Recipe target files must contain matching marker comments:

```md
<!-- BEGIN recipe:output-boundary-guard:typescript -->
<!-- END recipe:output-boundary-guard:typescript -->
```
