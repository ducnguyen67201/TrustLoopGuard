# Integration & Interception — How TrustLoopGuard Hooks an Agent

Notes capturing how LLM tool calling actually works and where TrustLoopGuard
slots in. This is the question every customer asks ("I route to you and that's
it?"), so it lives here as onboarding/design reference. The event engine it
feeds is documented in [`docs/concept/event-engine.md`](../concept/event-engine.md).

Status: **explainer / draft.**

---

## The key truth: the LLM never runs anything. It only *asks.*

An LLM is pure text-in → text-out. It cannot read email, call an API, or run
code. "Tool calling" (a.k.a. function calling) is an **API feature**:

1. **You** declare the available tools in the request (names + argument schemas).
2. The model can reply with a structured output meaning *"I want you to run
   `send_email` with these args"* — a **request**, not an execution.
3. **Your code / the framework** receives that request, decides whether to run it,
   runs it, and sends the **result** back to the model.
4. The model continues with the result.

**The model controls *what to request*; your code controls *whether and how to
run it*. That gap is everything.**

## Concrete trace (email agent)

```text
─ Request 1 ─  your code → LLM API
   messages: [user: "summarize my latest email and send it to Bob"]
   tools:    [read_email(), send_email(to, body)]      ← you declared these

─ Response 1 ─  LLM → your code
   NOT text. A tool-call request:  read_email()          ← model is ASKING
   (the model did NOT read anything)
   ▶ YOUR CODE runs read_email() → "<email text>"        ← execution happens HERE

─ Request 2 ─  your code → LLM API
   messages: [..., tool result: "<email text>"]

─ Response 2 ─  LLM → your code
   tool-call request:  send_email(to="bob@…", body="summary…")
   ▶ YOUR CODE runs send_email(...) → actually sends      ← execution happens HERE

─ Response 3 ─  LLM → your code
   text: "Done, I sent it to Bob."
```

The model emitted two tool-call *requests*. Your code executed them. The model
touched nothing.

## The framework's role (LiveKit example)

LiveKit Agents / LangChain / OpenAI Agents SDK just **automate the loop above**.
You register a function:

```python
@function_tool
def send_email(to: str, body: str): ...
```

and the framework:
- sends the tool schemas to the LLM,
- receives the tool-call request,
- **calls your Python function** (it is the thing that executes — the model can't),
- feeds the result back to the LLM,
- repeats until a final answer,
- (plus STT/TTS for voice, in LiveKit's case).

The tool-calling *mechanism* is the LLM provider's feature (OpenAI/Anthropic). The
framework is the **orchestrator** that wires "model asked → run the function →
return result."

## Where TrustLoopGuard intercepts

Because execution lives in the orchestrator (LiveKit / your code), **not** in the
LLM, that is where we hook:

```text
orchestrator gets tool-call request: send_email(to="attacker@…")
        │
        ▼   ← TrustLoopGuard hook goes HERE (before the function runs)
   POST /v1/events  { kind: tool.call.proposed, action: send_email, ... }
        │
   block → orchestrator does NOT call send_email
   allow → orchestrator calls it as normal
   rewrite → safer version · escalate → human approval
```

Two distinct interception points:

- **LLM boundary** — a gateway proxying the provider API sees the prompt, the
  completion, and the model's tool-call *requests*. It sees what the model *says*.
- **Action boundary** — a hook inside the agent runtime sees the tool actually
  *executing*, plus memory writes, files, messages, and **provenance**. It sees
  what the agent *does*.

A gateway sees what the model *says*; an interceptor sees what the agent *does*.
Provenance ("where did this parameter value come from") lives only in the agent
runtime — so the full event model needs an action-boundary hook, not just an LLM
proxy.

## Integration tiers

| Tier | How it hooks | Sees | Enforcement | Customer effort | Provenance |
|------|-------------|------|-------------|-----------------|------------|
| **A — Gateway** (LLM proxy) | point `base_url` at us | LLM I/O + model-proposed tool calls | rewrite / strip the completion | ~zero (config) | low |
| **B — SDK / framework adapter** | wrap tool executors, memory writes, sends (or a LiveKit / LangChain / OpenAI-Agents adapter we ship) | actual proposed actions + app-supplied provenance | block / rewrite / escalate before execution | moderate (instrument once or use our adapter) | **high — the real product** |
| **C — MCP proxy** | sit between agent and its MCP tool servers | tool-call requests at the MCP boundary | block / rewrite the MCP call | low if they use MCP | medium-high |

"Auto-sends tool calls to us" happens only in Tier B *when we ship a framework
adapter* — it is automatic because we wrote the hook, not because routing LLM
traffic magically captures actions.

## Bottom line

Routing a LiveKit agent's LLM through our gateway is **not** "and that's it" for
the full product — it captures model I/O and proposed tool calls only. To guard
tool execution, memory, and files **with provenance**, ship a LiveKit
adapter / SDK hook at the tool-execution boundary; that is what emits
`tool.call.proposed` to us **before** execution.
