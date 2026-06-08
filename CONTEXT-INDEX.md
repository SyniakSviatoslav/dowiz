# DeliveryOS — Context Index

> Always-on. Thin (~0.5 screen). Shows what exists and where to go for what.
> Updated: 2026-06-08

## Stores Overview

| Store | What's In It | How To Read |
|---|---|---|
| **`docs/`** (67 files) | Canonical: architecture (v3.1), roadmap (v4.5), build-plan (v4.4), As-Built (v1), design (DESIGN.md), security (vulnerabilities.md), AI-governance, phase-exit audits, contract-maps ×3 | Direct read |
| **Root `*.md`** | `AGENTS.md` (rules, router, memory), `MEMORY-MAP.md` (this), `CONTEXT-INDEX.md` (this), `CONVENTIONS.md` (code style — binary) | Always-on |
| **`.agents/rules/`** (4 files) | Always-on agent behavioral rules (design-system, graphify, research-first, token-saving) | Always-on |
| **`.agents/skills/`** (5 skills) | Specialized workflows (component-builder, deliveryos-theme, deliveryos-ui, screen-builder) | Skill tool on demand |
| **`graphify-out/`** | AST knowledge graph (code entity relationships) | `graphify query "..."` (currently stale) |
| **`mempalace`** | Session diary, transient decisions | `mempalace_search` / `mempalace_diary_read` |
| **`DeliveryOS-As-Built-Summary-v1.md`** | **START HERE** — code reality, security posture, stack, must-fix | Direct read (always loaded) |

## Quick Lookup

| You need... | Go to... |
|---|---|
| Architecture / infra decision | `docs/DeliveryOS-Architecture-Update-v3.1.md` |
| Build plan / scope / schema seams | `docs/DeliveryOS-Service-Build-Plan-v4_4.md` |
| Roadmap / launch | `docs/DeliveryOS-Context-Handoff-v4_5.md` |
| Code reality / is it built | `DeliveryOS-As-Built-Summary-v1.md` (trumps planning docs) |
| Security / vulnerabilities | `docs/audit/vulnerabilities.md` (full) + As-Built §5 (re-verified status) |
| Phase gate / exit audit | `docs/audit/phase4-exit.md` or `docs/audit/phase5-exit.md` |
| Design system | `docs/design/DESIGN.md` + `.agents/skills/deliveryos-theme/` |
| API contract (FE) | `docs/integration/contract-map.md` |
| API contract (Zod) | `docs/contract-map.md` |
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
