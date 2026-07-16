# Contributing to TrustLoopGuard

Thanks for your interest in contributing. This document covers how to report bugs, propose changes, and get your code merged.

## Reporting bugs

Open a [GitHub issue](https://github.com/ducnguyen67201/TrustLoopGuard/issues) with:

- A minimal reproduction (the smallest `GuardEvent` or SDK call that triggers the problem)
- The SDK and version you're using
- Expected versus actual `Decision`

## Proposing changes

For small fixes (typos, docs, one-function changes) open a PR directly.

For larger changes — new SDK methods, engine behavior, wire type additions — open an issue first to align on the approach before writing code.

## Development setup

```bash
# Clone and enter the repo
git clone https://github.com/ducnguyen67201/TrustLoopGuard.git
cd TrustLoopGuard

# Install workspace dependencies and build the local TypeScript SDK
corepack enable
pnpm install

# Start the server
cargo run -p tl-server

# Run the dispute demo to verify your environment
pnpm --filter @trustloopguard/demo dispute:check
```

This repository setup is only for contributors and self-hosting. Customers
install the published SDK and do not clone the repository. See the
[README](README.md) for both paths.

## The three SDK-driven rules

Every change follows the rules in [`docs/SDK_DRIVEN.md`](docs/SDK_DRIVEN.md). The short version:

1. **Engine changes ship across all surfaces** — if you add a feature to the engine, it lands in `tl-core` types, and all three SDKs (Rust, Python, TypeScript) in the same PR.
2. **Demos use only public SDK surfaces** — no internal crate imports in `demo/`.
3. **Wire types live in `tl-core`** — do not duplicate request/response structs elsewhere.

## Pull request checklist

- [ ] `make ci-lint` passes locally
- [ ] `cargo test -p <changed-crate>` passes for any touched Rust crate
- [ ] New behavior has corresponding test coverage
- [ ] Wire type changes update `tl-core` and regenerate SDK types
- [ ] Docs in `docs/concept/` reflect any architectural change

## Commit style

```
<type>: <short description>
```

Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `perf`, `ci`

## License

By contributing, you agree that your contributions will be licensed under the [Apache License, Version 2.0](LICENSE).
