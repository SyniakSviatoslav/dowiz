# dowiz — MEMORY-MAP

> One fact, one canonical place. Updated: 2026-07-22
> The project is now a single-file PWA (vanilla JS, CSS, HTML) with a Rust → WASM kernel.
> All DeliveryOS-era planning docs (v3.1, v4.x, As-Built) are historical — the repo has
> been rewritten from Node/TS/Fastify to a pure frontend + kernel architecture.

## Canonical Store Ownership

Each knowledge type has exactly **one** canonical store.

| Knowledge Type | Canonical Store | Notes |
|---|---|---|
| **App logic & UI** | `web/src/app.js` | 1000+ lines, 3 roles, cart, orders, Three.js BG |
| **Kernel (Rust → WASM)** | `kernel/src/` | Spectral, geo, FSM, event sourcing (859 tests) |
| **Design tokens & CSS** | `web/src/styles/` | `tokens.css` (vars), `base.css` (layout), `animations.css` (motion) |
| **Architecture decisions** | `docs/design/ARCHITECTURE.md` | Current architecture doc |
| **Roadmap & plans** | `docs/design/CORE-ROADMAP-2026-07-17/` | Comprehensive 19-phase roadmap |
| **Decision log** | `DECISIONS.md` | 410 lines of trade-off records |
| **Agent rules & doctrine** | `AGENTS.md` | 335 lines, always-on |
| **UX audit** | This session's full audit | 25 friction points, P0-P3 prioritization |
| **Web telemetry** | `web/src/lib/telemetry/oracle.mjs` | Local-only interaction oracle |
| **Route inventory** | `docs/audit/inventory.md` | Direct read | File:line precise; may be stale (2026-06-04) |
| **API contract (FE integration)** | `docs/integration/contract-map.md` (788 lines) | Direct read most comprehensive | Most complete of 3 contract maps |
| **API contract (Zod schemas)** | `docs/contract-map.md` (223 lines) | Direct read | Backend schema perspective |
| **API contract (FE concise)** | `docs/frontend/contract-map.md` (57 lines) | Direct read | Summary of E |
| **Code conventions** | `CONVENTIONS.md` | Direct read (binary) | Single source |
| **Agent behavioral rules** | `AGENTS.md` | Always-on in agent context | Supersedes ad-hoc instructions |
| **Always-on agent rules** | `.agents/rules/` (4 files) | Always-on; loaded per session | design-system.md, graphify.md, research-first.md, token-saving.md |
| **Specialized workflows** | `.agents/skills/` (5 skills) | Load on demand via Skill tool | component-builder, deliveryos-theme, deliveryos-ui, screen-builder |
| **Session diary** | `AGENTS.md` (§9 Known broken) | Always-on |
