# CookBook agent integrations: TDD evidence

## Source

The user journeys were derived during this implementation. No external plan
file was used.

## User journeys

- As a LiveKit developer, I can decorate one voice agent so blocked or
  transformed text is enforced before TTS receives it.
- As an OpenAI Agents SDK or Mastra developer, I can construct a normal
  framework agent and protect its local tools and final returned text.
- As a CookBook reader, I can type-check examples against the current framework
  packages without calling model or TrustLoopGuard services.

## Task report

| Behavior | RED evidence | GREEN evidence |
| --- | --- | --- |
| LiveKit pre-TTS enforcement | `pnpm --filter @trustloopguard/sdk test -- test/guard.test.ts` ran the new tests and failed because `guardLiveKitAgent` did not exist | The SDK suite passed 121 tests after implementation |
| Framework examples | `pnpm test` in CookBook failed because the three imported example modules did not exist | CookBook passed 3 constructor tests |
| Current framework compatibility | CookBook `pnpm typecheck` found the LiveKit 1.5.5 close-emitter and `Room.getSid()` contract mismatches | CookBook `pnpm typecheck` passed after adapting those boundaries |

## Test specification

| # | What is guaranteed | Test or command | Type | Result |
| --- | --- | --- | --- | --- |
| 1 | Blocked LiveKit text never reaches the original TTS node | `sdks/typescript/test/guard.test.ts` | Unit | PASS |
| 2 | Transformed LiveKit text replaces the unsafe draft before TTS | `sdks/typescript/test/guard.test.ts` | Unit | PASS |
| 3 | OpenAI Agents, Mastra, and LiveKit examples construct without network calls | `CookBook/tests/typescript_examples.test.ts` | Component | PASS |
| 4 | Examples match current public TypeScript contracts | `pnpm typecheck` in CookBook | Compile | PASS |
| 5 | The SDK remains type-safe and builds distributable output | `pnpm --filter @trustloopguard/sdk typecheck` and `build` | Compile | PASS |

## Coverage and known gaps

The repository has no configured TypeScript SDK coverage command, so no numeric
coverage percentage was recorded. The new LiveKit branch has direct deny and
transform coverage; the existing suite covers permit, transport failure,
local-tool authorization, and LiveKit session lifecycle behavior.

Direct realtime speech-to-speech audio bypasses LiveKit's text TTS node and is
intentionally not claimed as protected by this adapter.

## Merge evidence

- RED checkpoint: `23ec006f test: add LiveKit pre-TTS guard coverage`
- GREEN checkpoint: `ffdd7942 feat(sdk): guard LiveKit speech before TTS`
- LiveKit compatibility follow-up: `cfa6703f fix(sdk): accept current LiveKit session emitters`
