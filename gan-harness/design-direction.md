# Design direction — "Instrument"

A calm, high-signal control surface for the people who run AI guardrails. It keeps
TrustLoopGuard's technical identity (the orange brand, monospace as the language of
data) and removes what made the old "Terminal" theme feel like a re-skin: harsh
0-radius everywhere, monospace as the body font, flat undifferentiated neutrals, and
the total absence of loading / empty / error states.

## Principles

1. **Signal over decoration.** Saturated color always *means* something — verdicts
   (allow/rewrite/block/escalate), status, alerts. The base is a disciplined
   neutral: a warm "lab" in light, a true "control room" in dark. Orange is the
   brand action accent, used sparingly, never as wallpaper.
2. **Two-face typography.** `Inter` for all UI and prose (clarity); `IBM Plex Mono`
   retained for data, IDs, code, metrics, and verdict labels (precision). Monospace
   now carries *meaning* — it is no longer the default body face.
3. **Structured depth.** A real radius scale (crisp ~8px, not harsh 0, not soft
   pill) and layered surfaces (`background` < `card` < `raised`) with hairline
   borders. Depth comes from layering + borders; shadows stay a whisper.
4. **Rhythm, not uniform padding.** A spacing scale used with intent — generous page
   gutters, tighter density inside data tables.
5. **Designed states.** Every async surface has a skeleton, every empty list has a
   purposeful empty state with a primary action, every failure has a recoverable
   error state. No blank or frozen screens.
6. **Wayfinding.** One consistent page header (eyebrow + title + description +
   primary action) across every page, breadcrumbs on nested routes. The sidebar's
   Monitor / Configure grouping stays.
7. **Functional motion.** Transform/opacity only, used to clarify state changes
   (skeleton shimmer, row hover, panel transitions). Always honors
   `prefers-reduced-motion`.

## Tokens (implemented in `app/globals.css`)

| Token group | Light | Dark |
|---|---|---|
| background | `#fafaf9` warm paper | `#0b0b0d` near-black |
| card | `#ffffff` | `#141417` |
| raised (popover/secondary) | `#f5f5f4` | `#1c1c20` |
| foreground | `#1c1917` | `#f4f4f5` |
| muted-foreground | `#78716c` | `#a1a1aa` |
| border | `#e7e5e4` | `#26262b` |
| primary (brand action) | `#ff6900` | `#ff6900` |
| radius base | `0.5rem` | `0.5rem` |

Verdict tokens (`--color-allow/-rewrite/-block/-escalate`) and the 5-color chart
identity are preserved.

## What "from-scratch" means here

Foundation = new visual **DNA** (tokens, type system, primitives, shell, states).
Per-page loop = new **composition** (hierarchy, layout, bento where it helps, flow,
wayfinding). Same Rust-backed data, reimagined surface.
