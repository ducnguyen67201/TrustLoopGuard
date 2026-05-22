# Diagrams

D2 source files for generated documentation diagrams.

Edit `.d2` files here, then render SVG assets into both documentation surfaces:

- `docs/concept/assets/` for repo Markdown docs.
- `apps/docs/public/diagrams/` for the docs website.

```bash
pnpm docs:diagrams
# or
make diagrams
```

The render command expects the D2 CLI on `PATH`.

On macOS:

```bash
brew install d2
```

Other official install methods are documented at <https://d2lang.com/tour/install/>.
