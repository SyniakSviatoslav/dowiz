# Harness H1 — proposed CLAUDE.md lean-skeleton edits (for manual apply)

`.claude/CLAUDE.md` is under `.claude/` → the `protect-paths` hook hard-blocks agent edits
(governance zone, like `ci.yml`). This file is the ready-to-apply proposal; an operator
applies it manually. Goal: CLAUDE.md = a **map** (invariant · project command · pointer ·
policy), not a copy of the corpus. Keep every discipline/proof/ship/repowise-usage rule.

## A. TRIM — replace the auto-generated index block with one pointer

The "## IMPORTANT: Codebase Intelligence Instructions" sub-block that lists **Entry Points,
Tech Stack, Architectural Layers table, Guided Tour, Hotspots, Code health biomarkers** is
(a) stale-prone (snapshot at "Last indexed 2026-06-14") and (b) duplicated by the LIVE
repowise index + `v3.1`. Replace that block with:

```md
### Codebase intelligence
- Live structure / hotspots / dependency graph: query the **repowise MCP** (get_context,
  get_symbol, get_risk, get_dead_code) — do NOT trust a static snapshot here.
- Infrastructure & architecture decisions: `docs/DeliveryOS-Architecture-Update-v3.1.md`.
- Code reality ("is X built"): `DeliveryOS-As-Built-Summary-v1.md`.
- Canonical store map: `MEMORY-MAP.md`.
```
Keep the **Repowise MCP tool-usage table + Trust protocol + Output Distillation** sections
as-is (those are policy/how-to, not duplicated data).

## B. ADD — Model policy

```md
## Model policy
Pick the model by task class at the start of an étape (`/model` in Claude Code):
- **Opus** — contract design, adversarial exit-audits / gates, security reasoning.
- **Sonnet** — build-prompt grinding, refactors, test authoring.
- **Cheapest (Haiku)** — purely mechanical edits (rename, format, mass find-replace).
Same principle as the OpenRouter bridge's model rotation: match capability to the work,
don't pay Opus rates for a rename.
```

## C. ADD — MCP policy (conditional)

```md
## MCP policy (conditional)
MCP tool defs cost context budget on every call — load servers only when the étape needs them.
- **repowise** — enable for real retrieval / large-repo navigation; skip for small pinpoint edits.
- **playwright-test / browser-use** — enable only for live UI testing / browsing étapes.
Toggle via `.mcp.json` (comment out a server) or per-run flags. Default lean: no MCP for
trivial work.
```

## D. ADD — Memory policy (folds in H4)

```md
## Memory policy
Project memory = the markdown corpus (`v3.1`/`v4.x` + handoffs) as the single source of
truth; authority lives outside process memory. **Mem0 / mempalace = DEFERRED** (not on the
critical path; adds a node + store + LLM calls vs the minimalism axiom). Code-retrieval =
repowise. **RAGFlow not installed** (one RAG layer only). See `MEMORY-MAP.md`.
```

## Net effect
CLAUDE.md drops a ~40-line stale auto-index block (→ 4-line pointer) and gains 3 short
policy sections. All discipline/proof/ship/repowise-usage rules preserved. Apply by editing
`.claude/CLAUDE.md` directly (operator — protected path).
