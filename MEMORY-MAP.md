# DeliveryOS — MEMORY-MAP

> One fact, one canonical place. Updated: 2026-06-08
> Source docs: `DeliveryOS-As-Built-Summary-v1.md` (authoritative for code reality),
> `docs/DeliveryOS-Context-Handoff-v4_5.md` (authoritative for roadmap),
> `docs/DeliveryOS-Architecture-Update-v3.1.md` (authoritative for infrastructure).

## Canonical Store Ownership

Each knowledge type has exactly **one** canonical store. All other stores link back.

| Knowledge Type | Canonical Store | Retrieval Strategy | Supersession Notes |
|---|---|---|---|
| **Architecture decisions** | `docs/DeliveryOS-Architecture-Update-v3.1.md` | Direct read | v3.1 supersedes v2/v3 on infra (§2) |
| **Service scope & schema seams** | `docs/DeliveryOS-Service-Build-Plan-v4_4.md` | Direct read | v4.4 supersedes v4.1-v4.3 on scope (§1) |
| **Roadmap & launch execution** | `docs/DeliveryOS-Context-Handoff-v4_5.md` | Direct read | v4.5 supersedes v4.4 on roadmap (§1) |
| **Code reality (what is built)** | `DeliveryOS-As-Built-Summary-v1.md` | Direct read | Supersedes all planning docs on factual "is it built" questions |
| **Security vulnerabilities** | `docs/audit/vulnerabilities.md` | Direct read | Parent doc; As-Built §5 re-verifies |
| **Phase exit gates** | `docs/audit/phase4-exit.md`, `docs/audit/phase5-exit.md` | Direct read | phase5-exit supersedes phase4-exit on overall GO/NO-GO |
| **Design system** | `docs/design/DESIGN.md` | Direct read | Skills derive from this, not replace it |
| **AI governance** | `docs/ai-governance.md` | Direct read | Formal policy; v4.4 §3 has business context only |
| **Connection budget** | `docs/connection-budget.md` | Direct read | Single source |
| **Route inventory** | `docs/audit/inventory.md` | Direct read | File:line precise; may be stale (2026-06-04) |
| **API contract (FE integration)** | `docs/integration/contract-map.md` (788 lines) | Direct read most comprehensive | Most complete of 3 contract maps |
| **API contract (Zod schemas)** | `docs/contract-map.md` (223 lines) | Direct read | Backend schema perspective |
| **API contract (FE concise)** | `docs/frontend/contract-map.md` (57 lines) | Direct read | Summary of E |
| **Code conventions** | `CONVENTIONS.md` | Direct read (binary) | Single source |
| **Agent behavioral rules** | `AGENTS.md` | Always-on in agent context | Supersedes ad-hoc instructions |
| **Always-on agent rules** | `.agents/rules/` (4 files) | Always-on; loaded per session | design-system.md, graphify.md, research-first.md, token-saving.md |
| **Specialized workflows** | `.agents/skills/` (5 skills) | Load on demand via Skill tool | component-builder, deliveryos-theme, deliveryos-ui, screen-builder |
| **Entity relationships** | `graphify-out/` | `graphify query` / `graphify path` | Knowledge graph; currently stale (relates to old path) |
| **Session diary + transient** | `mempalace` (open code diary) | `mempalace_search` / `mempalace_diary_read` | Transit layer; not canonical for any persistent fact |
| **Recent fixes & decisions** | `AGENTS.md` (§9 Known broken) + `mempalace` diary | `mempalace_search` then verify in code | Transit → promoted to permanent doc once verified |

## Retrieval Decision Tree

```
Agent query
    │
    ├─ "What is built / is X implemented?"
    │   → As-Built Summary (§1-4)
    │
    ├─ "How does this architecture work?"
    │   → v3.1 Architecture (or As-Built §2 for quick stack)
    │
    ├─ "What's the roadmap / when will X ship?"
    │   → v4.5 Handoff
    │
    ├─ "What's the API endpoint / contract?"
    │   → integration/contract-map.md (FE perspective)
    │   → docs/contract-map.md (Zod perspective)
    │
    ├─ "Is this secure / any vulnerabilities?"
    │   → vulnerabilities.md (full analysis)
    │   → As-Built §5 (re-verified status)
    │
    ├─ "Where is X in code / which file?"
    │   → graphify query (knowledge graph)
    │   → inventory.md (file:line index)
    │
    ├─ "What conventions / rules apply?"
    │   → AGENTS.md (§6-8 for design/anti-fake/anonymizer)
    │   → .agents/rules/ (always-on rules)
    │   → CONVENTIONS.md (code style)
    │
    ├─ "Design: colors, components, tokens?"
    │   → DESIGN.md
    │   → deliveryos-theme skill for implementation
    │
    ├─ "What did we do last session / fix recently?"
    │   → mempalace diary
    │   → AGENTS.md §9 (known broken)
    │
    └─ "Specialized task (build component, test, ship)?"
        → .agents/skills/ (load via Skill tool)
```

## Supersession Chain

```
v3.1 Architecture ─── supersedes ─── v2/v3 on infra
    │
    ▼
v4.4 Build Plan ─── supersedes ─── v4.1-v4.3 on scope
    │
    ▼
v4.5 Context Handoff ─── supersedes ─── v4.4 on roadmap/execution
    │
    ├── (separate track)
    │
    ▼
As-Built Summary (v1) ─── supersedes ─── ALL planning docs on "is it built" questions
    │
    ├── supersedes inventory.md where dates differ (inventory: 2026-06-04, As-Built: 2026-06-07)
    │
    └── supersedes vulnerabilities.md on fix status (re-verification more current)
```

## Phase 0: Conflict & Overlap Log

| # | Conflict | Docs Involved | Resolution |
|---|---|---|---|
| C1 | Build status: Phases 2-5 built (A) vs "prompts ready" (C) | A, C | **A wins** — evidence-based. C's roadmap table is outdated intent |
| C2 | JWT algorithm: RS256 (O) vs HS256 (code) | O, A, L | **HS256 wins** — code reality. O rule 9 is aspirational |
| C3 | Phase 4 RLS bug: critical (M) vs not tracked (A, L) | M, A, L | **M wins** — A and L must be updated to include phase4-exit FINDING-1 |
| C4 | Theme/notification auth: fixed (A) vs no auth (K) | A, K | **A wins** — K is stale (Jun 4 vs Jun 7) |
| C5 | Endpoint path conventions: 3 different patterns | E, F, G, K | **No resolution** — known design debt. Routes use varied conventions |
| C6 | Supabase: Pro (B) vs Free (C) | B, C | **C wins** — newer, reflects actual Free deployment |
| C7 | Polling interval: ~5s (B) vs unspecified (J) | B, J | **B wins** — architectural intent |
| C8 | R2 lifecycle: FAIL (N) vs not tracked (A) | N, A | **N wins** — A must be updated |
| C9 | 0 cookies: rule (O) vs unverified (code) | O, A | **O wins** — rule stands; code audit needed |
| C10 | Worker pool: 4 (B) vs 3 (J) | B, J | **J wins** — explicit budget |

**Duplications logged:** 6 areas (AI governance ×2, contract maps ×3, security ×2, exit findings ×2, design rules ×2, stateful debt ×3). Each has canonical owner assigned above.
