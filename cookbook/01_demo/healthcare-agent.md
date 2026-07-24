# Healthcare Scheduling Agent

## What It Teaches

Protect a synthetic scheduling assistant at two different boundaries:

1. Check the visitor message in Rust before calling the model.
2. Check the model draft in Rust before returning it to the browser.

The Marketing UI displays policy summaries and findings read from Rust. Policy
templates in the demo directory are provisioning input, not runtime state.

## Fastest Check

```bash
pnpm test:healthcare-demo
```

The suite covers agent behavior, workspace provisioning, the Marketing route,
and deployment boundaries without calling OpenAI.

## Run It

```bash
pnpm --filter @trustloopguard/demo healthcare-agent:setup
pnpm marketing:dev
```

Setup requires the internal management credential and an approved
`TL_ADMIN_USER_ID`. Put the one-time `TL_HEALTHCARE_DEMO_API_KEY` output plus
`OPENAI_API_KEY` in the Marketing server environment. Open
`http://localhost:3002/demo/healthcare`.

## Expected Proof

- Scheduling input reaches one model call and its output is checked.
- Emergency and medication requests are denied before the model call.
- Requests about another patient receive a privacy-safe refusal.
- The demo never claims to integrate an EHR or provide diagnosis or treatment.

## Read The Code

- [`agent.ts`](../../demo/healthcare-agent/agent.ts) owns input and output checks.
- [`hosted.ts`](../../demo/healthcare-agent/hosted.ts) owns the public runtime.
- [`policy-templates.ts`](../../demo/healthcare-agent/policy-templates.ts)
  defines bootstrap policy input.
- [`demo/README.md`](../../demo/README.md#healthcare-scheduling-agent) owns the
  detailed safety and environment contract.
