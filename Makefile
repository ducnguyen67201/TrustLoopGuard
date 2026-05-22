# TrustLoopGuard top-level Makefile.
#
# Goal: every contributor gets the same commands. CI runs the same recipes
# locally. If a step is hard to memorize, it goes here.
#
# Targets are grouped by surface area:
#   codegen     wire-format types and generated SDK artifacts
#   sdk-*       per-language SDK build/test
#   example-*   per-language example app run (added in PR 7/8)
#   quickstart  end-to-end stranger-on-clean-machine test (added in PR 9)
#   ci-*        what CI runs (use these locally to debug CI failures)

SHELL := /usr/bin/env bash
.SHELLFLAGS := -euo pipefail -c
.DEFAULT_GOAL := help
TL_LOG_FORMAT ?= pretty

# -----------------------------------------------------------------------------
# Help

.PHONY: help
help: ## Show this help
	@awk 'BEGIN {FS = ":.*## "; printf "\nUsage: make \033[36m<target>\033[0m\n\nTargets:\n"} \
		/^[a-zA-Z_-]+:.*?## / { printf "  \033[36m%-22s\033[0m %s\n", $$1, $$2 } \
		/^##@ / { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) }' $(MAKEFILE_LIST)

# -----------------------------------------------------------------------------
##@ Codegen — Rust types are the source of truth

.PHONY: diagrams
diagrams: ## Render D2 diagram sources into docs/concept/assets/*.svg
	bash scripts/render-diagrams.sh

.PHONY: codegen
codegen: ## Regenerate OpenAPI, JSON Schemas, TS types, and Pydantic models
	cargo run -p tl-codegen

.PHONY: codegen-check
codegen-check: ## Fail if any generated artifact drifted from tl-core (CI mode)
	cargo run -p tl-codegen -- --check

.PHONY: verify-contract
verify-contract: codegen-check ## Verify Rust API contract matches generated docs and SDK types

# -----------------------------------------------------------------------------
##@ SDK build & test

.PHONY: sdk-rust
sdk-rust: ## Build + test the Rust SDK
	cargo build -p tl-sdk-rust
	cargo test -p tl-sdk-rust

.PHONY: sdk-python
sdk-python: ## Install + test the Python SDK
	cd sdks/python && pip install -e ".[dev]" && pytest -v

.PHONY: sdk-typescript
sdk-typescript: ## Build + typecheck the TypeScript SDK
	cd sdks/typescript && npm install && npm run typecheck && npm run build

.PHONY: sdk-all
sdk-all: sdk-rust sdk-python sdk-typescript ## Build + test all three SDKs

# -----------------------------------------------------------------------------
##@ Dev — local processes wrapped in `doppler run`

.PHONY: dev-db
dev-db: ## Run only Postgres for host-based dev (server/web run outside Docker)
	docker compose up -d db

.PHONY: server
server: ## Run tl-server with secrets from Doppler (trustloopguard/dev)
	TL_LOG_FORMAT=$(TL_LOG_FORMAT) doppler run -- cargo run -p tl-server

.PHONY: server-watch
server-watch: ## Run tl-server with hot reload on .rs file changes
	TL_LOG_FORMAT=$(TL_LOG_FORMAT) doppler run -- cargo watch \
		-i 'target/*' -i 'apps/*' -i 'sdks/*' -i 'docs/*' \
		-x 'run -p tl-server'

.PHONY: web
web: ## Run the Next.js web app with secrets from Doppler
	cd apps/web && pnpm dev

.PHONY: db
db: ## Bring up just Postgres (other services stay off)
	docker compose up -d db

.PHONY: dev
dev: db ## Full stack: Postgres + tl-server (hot reload) + web (hot reload). Ctrl-C kills everything.
	@echo ""
	@echo "  ╭─ tl-server   http://localhost:8080  (hot reload)"
	@echo "  ├─ web         http://localhost:3000  (hot reload)"
	@echo "  ╰─ postgres    docker:db"
	@echo ""
	@lsof -ti:8080 | xargs kill -9 2>/dev/null || true
	@trap 'echo; echo "stopping…"; kill 0' EXIT INT TERM; \
		(TL_LOG_FORMAT=$(TL_LOG_FORMAT) doppler run -- cargo watch \
			-i 'target/*' -i 'apps/*' -i 'sdks/*' -i 'docs/*' \
			-x 'run -p tl-server' 2>&1 | sed 's/^/[server] /') & \
		(cd apps/web && pnpm dev 2>&1 | sed 's/^/[web]    /') & \
		wait

.PHONY: cli
cli: ## Run the tl CLI with secrets from Doppler (pass args via ARGS=)
	doppler run -- cargo run -p tl-cli -- $(ARGS)

.PHONY: agent-demo
agent-demo: ## Run the scripted chat demo against local tl-server
	pnpm demo:chat

# -----------------------------------------------------------------------------
##@ Quickstart — the README, run literally (added in PR 9)

.PHONY: quickstart
quickstart: ## Run the README quickstart end-to-end against a local tl-server
	@if [[ -x scripts/quickstart.sh ]]; then \
		bash scripts/quickstart.sh; \
	else \
		echo "scripts/quickstart.sh not present yet — wired in PR 9"; \
		exit 1; \
	fi

# -----------------------------------------------------------------------------
##@ Lint (added in PR 11)

.PHONY: lint-no-internal-imports
lint-no-internal-imports: ## Fail if apps/example-* or demo/ import internal crates
	@if [[ -x scripts/lint-no-internal-imports.sh ]]; then \
		bash scripts/lint-no-internal-imports.sh; \
	else \
		echo "scripts/lint-no-internal-imports.sh not present yet — wired in PR 11"; \
		exit 1; \
	fi

.PHONY: lint-api-contracts
lint-api-contracts: ## Fail if public API DTOs are defined in tl-server
	@if [[ -x scripts/lint-api-contracts.sh ]]; then \
		bash scripts/lint-api-contracts.sh; \
	else \
		echo "scripts/lint-api-contracts.sh not present"; \
		exit 1; \
	fi

.PHONY: lint-web-backend-only
lint-web-backend-only: ## Fail if apps/web browser code calls tl-server / external APIs directly
	@if [[ -x scripts/lint-web-backend-only.sh ]]; then \
		bash scripts/lint-web-backend-only.sh; \
	else \
		echo "scripts/lint-web-backend-only.sh not present"; \
		exit 1; \
	fi

.PHONY: check-schema-drift
check-schema-drift: ## Diff crates/tl-storage/src/schema.rs against the live database
	@bash scripts/check-schema-drift.sh

# -----------------------------------------------------------------------------
##@ CI mirrors — run the exact recipes CI runs

.PHONY: ci-codegen
ci-codegen: codegen-check ## What .github/workflows/codegen-check.yml runs

.PHONY: ci-sdk-build
ci-sdk-build: sdk-all ## What .github/workflows/sdk-build.yml runs

.PHONY: ci-quickstart
ci-quickstart: quickstart ## What .github/workflows/quickstart.yml runs (PR 10)

.PHONY: ci-lint
ci-lint: lint-no-internal-imports lint-api-contracts lint-web-backend-only ## What the lint workflow runs

.PHONY: ci
ci: ci-codegen ci-lint ci-sdk-build ci-quickstart ## Run every CI gate locally

# -----------------------------------------------------------------------------
##@ Misc

.PHONY: clean
clean: ## Remove build artifacts (does not touch generated wire types)
	cargo clean
	rm -rf sdks/typescript/dist sdks/typescript/node_modules
	rm -rf sdks/python/dist sdks/python/build sdks/python/*.egg-info
