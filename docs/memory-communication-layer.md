# DeliveryOS — Memory Communication Layer

> Unified retrieval design. One coherent interface across heterogeneous stores.
> No MCP server — conceptual router that agents follow.
> Updated: 2026-06-08

## Phase 2: Engine Alignment

Current engines evaluated against the requirements:

| Requirement | `graphify` (graphify-out/) | `mempalace` | `docs/` vault |
|---|---|---|---|
| **Semantic retrieval** | ✅ via query (AST graph) | ✅ `mempalace_search` | ❌ file read only |
| **Keyword retrieval** | ❌ (query only) | ❌ (semantic only) | ✅ grep/glob |
| **Temporal validity** | ❌ (no valid_from/valid_to) | ⚠️ `kg_add` supports it | ✅ file versioning via git |
| **Supersession** | ❌ | ✅ `kg_invalidate` | ✅ via git log |
| **Self-host simplicity** | ✅ (local binary) | ✅ (local SQLite) | ✅ (plain files) |
| **Deduplication** | ❌ | ✅ `check_duplicate` | ✅ manual via writes |
| **Bulk index speed** | ⚠️ (stale — needs full rebuild) | ✅ fast | N/A |

**Recommendation**: Keep current engines. Align `graphify` to the temporal pattern by:
- Running `graphify update .` after every session to refresh
- Using `mempalace kg_add` with `valid_from`/`valid_to` for all facts with expiry
- Using `mempalace check_duplicate` before adding any fact
- Using `mempalace kg_invalidate` for stale facts (not deletion)

**graphify alignment path** (no engine rewrite):
| Current | Aligned |
|---|---|
| AST-only, no temporal metadata | Post-update, enrich graph nodes with git timestamp |
| Stale (referenced old path) | `graphify update .` on session start |
| Query returns current graph only | Before query, always run `graphify update .` (cheap AST-only) |

---

## Phase 3: Communication Layer (No-MCP Design)

### Read-Path Router (Conceptual)

The router is not an MCP server — it's the **retrieval decision tree in MEMORY-MAP.md** that agents follow. Before any store query, classify:

```
1. CLASSIFY query type:
   - "What is / is it built?" → As-Built Summary
   - "How does X work?" → Architecture docs
   - "Where in code?" → graphify query + grep
   - "Rules / conventions?" → AGENTS.md + .agents/rules/
   - "What did we do?" → mempalace diary
   - "Roadmap / when?" → v4.5 Handoff
   - "Design / colors?" → DESIGN.md + theme skill
   - "API contract?" → contract-map (which perspective?)
   - "Security?" → vulnerabilities.md + As-Built §5
   - "Specialized task?" → Skill tool

2. FETCH from canonical store only (no redundant reads)

3. VERIFY freshness (check date in doc footer / git log)

4. RETURN minimal non-contradictory slice (just what was asked)
```

### Write-Path Protocol

After a verified fix/fact (passed problem-solving gate):

```
1. VERIFY: fact is confirmed (not hypothesis)
2. DEDUP: mempalace_check_duplicate before any write
3. CLASSIFY fact type:
   a. Transient session state → mempalace diary_write
   b. Durable code fix → update AGENTS.md §9, then CODE
   c. Durable non-code decision → update relevant docs/ file
   d. Relational entity fact → mempalace kg_add with valid_from
4. SCHEMA: each recorded fact must include:
   - Signature: what was done/changed
   - Context: why / what triggered it
   - Root cause: what was wrong
   - Resolution: what fixed it
   - Verification: how it was proven (e.g., "24/24 E2E pass", "specific curl")
   - Sources: files changed, file:line
   - Timestamp: YYYY-MM-DD
5. SUPERSEDE: if replacing old fact → mempalace kg_invalidate(old) with valid_to
6. PROMOTE: when fact stabilizes (3+ sessions without revision) → 
   move from mempalace to docs/ file; update MEMORY-MAP.md
```

### Thin Always-On Context

`CONTEXT-INDEX.md` + `MEMORY-MAP.md` are the only always-loaded documents 
(alongside `AGENTS.md` and `As-Built-Summary`). They tell the agent:
- What exists (CONTEXT-INDEX)
- Where each knowledge type lives (MEMORY-MAP)
- Which is canonical / which to read first (retrieval decision tree)

---

## Phase 4: Supersession + Dedup Engine

### Supersession Metadata Format

Each fact in mempalace KG follows this schema:

```json
{
  "subject": "entity_or_file",
  "predicate": "relationship_type",
  "object": "target_entity",
  "valid_from": "2026-06-01",
  "valid_to": null,           // null = still valid
  "supersedes": null,          // drawer_id or fact_id of older version
  "superseded_by": null,       // drawer_id or fact_id of newer version
  "source_file": "path/to/canonical-file.md",
  "confidence": "verified"     // "verified" | "observed" | "inferred"
}
```

### Supersession Chains

When a fact is superseded:
1. New fact written with `valid_from = today`
2. Old fact updated: `valid_to = today`, `superseded_by = new_fact_id`
3. New fact: `supersedes = old_fact_id`
4. `MEMORY-MAP.md` supersession chain table updated

### Dedup Rules

- Before write: always `mempalace_check_duplicate`
- If duplicate found and still valid: **skip**, don't rewrite
- If duplicate found but outdated: supersede (not delete)
- If duplicate found but newer: skip old, update chains

### Graph Freshness

- `graphify-out` is a **materialized view** of code structure at a point in time
- Must be refreshed after any code change via `graphify update .`
- If not refreshed, graph queries may return stale (wrong file paths, missing entities)
- Session start: check `graphify-out/manifest.json` mod time vs last git commit

### Implementation Check (No-MCP)

Without an MCP server, the router is a **human-agent protocol**:
- Agent reads MEMORY-MAP.md → determines canonical store
- Agent reads CONTEXT-INDEX.md → knows what exists
- Agent follows write protocol for new facts
- Agent checks supersession chains before treating any fact as current

This protocol is enforced by the retrieval decision tree being always-on in 
MEMORY-MAP.md. No infrastructure code needed.
