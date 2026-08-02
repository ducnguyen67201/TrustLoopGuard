# Security Policy

Featherlane AI is a runtime guardrail for AI agents, so we take security
reports seriously and aim to respond quickly. Thank you for helping keep
Featherlane AI and its users safe.

## Supported Versions

Featherlane AI is pre-1.0 and under active development. Security fixes land on
the latest release and the `main` branch only — there are no long-term-support
or backport guarantees during the `0.x` series. Always upgrade to the latest
published version before reporting an issue.

| Component | Channel | Supported |
| --------- | ------- | --------- |
| Rust service (`crates/tl-server`) | `main` / latest release | :white_check_mark: |
| TypeScript SDK (`@featherlane-ai/sdk`) | latest npm release | :white_check_mark: |
| Python SDK (`featherlane-ai`) | latest PyPI release | :white_check_mark: |
| Older `0.x` releases | — | :x: |

## Reporting a Vulnerability

**Please do not open a public issue, pull request, or discussion for security
problems.** Public disclosure before a fix is available puts users at risk.

Report privately through GitHub:

1. Go to the **Security** tab of this repository.
2. Click **Report a vulnerability** (GitHub Private Vulnerability Reporting).
3. Describe the issue with enough detail for us to reproduce it (see below).

This keeps the report confidential between you and the maintainers until a fix
ships.

### What to include

- Affected component and version (server crate, SDK, or web proxy).
- A clear description of the vulnerability and its impact.
- Step-by-step reproduction, a proof-of-concept, or a failing request if you
  have one.
- Any suggested remediation, if known.

### What to expect

- **Acknowledgement:** within 3 business days.
- **Triage and severity assessment:** within 7 business days, with an initial
  assessment of whether the report is accepted.
- **Status updates:** at least every 7 days while the report is open.
- **Fix and disclosure:** we aim to release a fix and a coordinated advisory
  within 90 days of triage. We will credit you in the advisory unless you ask
  to remain anonymous.

If a report is declined, we will explain why so you can follow up if you
disagree with the assessment.

## Scope

In scope:

- The Rust API server and runtime guard logic (`crates/tl-server`,
  `crates/tl-engine`, `crates/tl-storage`, `crates/tl-core`).
- The official SDKs (`sdks/typescript`, `sdks/python`, `crates/tl-sdk-rust`).
- The dashboard proxy routes under `apps/web/app/api`.

Out of scope:

- Findings that require a compromised host, physical access, or a
  already-privileged account.
- Denial of service from unrealistic request volumes against local/demo
  deployments.
- Vulnerabilities in third-party dependencies that are already publicly known
  and tracked upstream (report those to the upstream project; we patch via
  dependency updates).
- Social engineering, spam, or issues only reproducible in outdated releases.

## Coordinated Disclosure

We follow coordinated disclosure: we ask that you give us a reasonable window
to ship a fix before any public disclosure, and we will work with you on
timing. We will publish a GitHub Security Advisory once a fix is available.
