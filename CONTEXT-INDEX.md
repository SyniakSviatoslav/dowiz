# dowiz — Context Index

> Always-on. Thin (~0.5 screen). Shows what exists and where to go for what.
> Updated: 2026-07-22

## Structure

| Path | What It Is | How To Read |
|---|---|---|
| **`web/src/app.js`** | Single-file UI (962→1010+ lines): routing, state, cart, checkout, order flow, Three.js background, keyboard shortcuts | Main entry. All UI logic lives here |
| **`web/src/lib/kernel/kernel_client.mjs`** | WASM bridge to Rust kernel: geo (haversine, ETA), FSM transitions, event sourcing, spectral analysis | Imported by app via kernel bridge |
| **`web/src/styles/`** | `tokens.css` (design tokens), `base.css` (layout + components), `animations.css` (springs, fades, morphs) | 3 CSS files, loaded together |
| **`web/src/lib/compose/`** | SDF neural-field canvas layer: scene composition, rendering, journey tracker | Background visual layer |
| **`web/src/lib/telemetry/oracle.mjs`** | Local interaction oracle: User Timing marks, Web Vitals observer, perf summary (localStorage only) | New (2026-07-22) |
| **`kernel/`** | Rust wasm kernel: spectral/geo/FSM math, event sourcing, security model, 859+116 tests | `cargo test` |
| **`docs/`** | ~400 markdown files. Key ones listed below | See ROADMAP.md for the curated subset |
| **`CLAUDE.md`** | Agent instructions: build, test, lint commands | Always-on |

## Key Entry Points

| You need... | Go to... |
|---|---|
| App overview, goals, build commands | `README.md` |
| Agent rules & operating doctrine | `AGENTS.md` (335 lines, always-on) |
| Architecture decisions (recorded) | `docs/design/ARCHITECTURE.md` |
| Roadmap & phase plans | `docs/design/CORE-ROADMAP-2026-07-17/` |
| Design rationale & trade-off logs | `DECISIONS.md` (410 lines) |
| Latest audit / full assessment | This session's audit (reverse-engineering, UX 25-point, telemetry gap) |
| UX flow (customer → owner → courier) | `web/src/app.js` — 3 roles, order life cycle, cart, checkout |
| Performance benchmarks | `agent-loop/` (criterion), `web/` (User Timing marks via oracle) |
| Security / compliance | `docs/audit/vulnerabilities.md` |
| Route inventory (file:line) | `docs/audit/inventory.md` |
| AI governance | `docs/ai-governance.md` |
| Connection budget | `docs/connection-budget.md` |
| Agent rules / behavior | `AGENTS.md` + `.agents/rules/` |
| Code conventions | `CONVENTIONS.md` (binary) |
| Code entity relationships | `graphify query "..."` (stale — needs `graphify update .`) |
| Session history / recent fixes | `mempalace diary_read` + AGENTS.md §9 |
| Known broken | AGENTS.md §9 table |

## Retrieval Priority

1. **As-Built Summary** — always loaded first; start here for anything about code reality
2. **AGENTS.md** — always loaded; contains skill router, rules, known-broken
3. **MEMORY-MAP.md** (this) — on first query only; tells you where to go
4. **CONTEXT-INDEX.md** (this) — always-on thin index; reference for what exists

## Freshness

- As-Built Summary: re-verified 2026-06-07
- AGENTS.md: continuous
- inventory.md: 2026-06-04 (may be stale)
- graphify-out: stale (built for old path `Documents\delivery\`)
- All other docs: varying dates; write dates in footers

## Write Protocol

To record a verified fact (after problem-solving gate):
1. Write to `mempalace` diary (transient, session-scoped)
2. If durable: update AGENTS.md §9 (known-broken) or the relevant `docs/` file
3. If relational: `graphify update .` + `graphify query` to refresh knowledge graph
4. If superseding: update MEMORY-MAP.md supersession chain
