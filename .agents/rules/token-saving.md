---
trigger: always_on
description: Token-saving protocol — mandatory for all agents on every session.
---

## Token-saving protocol (always-on)

Every agent MUST follow this sequence before doing real work:

### 1. Orient first (30s, not 5min)
- Read `AGENTS.md` on wake-up. It contains the repo map, skill router, and memory protocol.
- Do NOT re-explore the codebase from scratch if AGENTS.md already has the answer.

### 2. Query before grep
- For codebase/architecture questions: run `graphify query "<question>"` FIRST.
- Use `graphify path "A" "B"` for relationships, `graphify explain "X"` for concepts.
- Only fall back to raw `grep` / `glob` if graphify does not surface enough.
- After code changes: run `graphify update .` (AST-only, zero API cost).

### 3. Search memory before answering
- Before answering any factual question about past work/decisions: `mempalace_search` (wing: `dowiz`).
- For entities (people, projects, places): `mempalace_kg_query`.
- Never guess — verify. Wrong is worse than slow.

### 4. Use the skill router
- Match the task to the correct skill via the table in AGENTS.md §3.
- Load the skill via the `skill` tool before starting work.
- This avoids reinventing patterns already captured in skill files.

### 5. Preserve context across sessions
- After each session of real work: `mempalace_diary_write` (agent: `opencode`, wing: `dowiz`).
- File durable facts: `mempalace_add_drawer` (wing: `dowiz`, room from `mempalace.yaml`).
- When facts change: `mempalace_kg_invalidate` old, `mempalace_kg_add` new.

### Why this matters
- AGENTS.md orient = ~2K tokens. Re-exploring codebase = ~50K+ tokens.
- graphify query = ~2K tokens. Reading full files = ~10-50K tokens.
- mempalace search = ~1K tokens. Re-discovering known facts = ~5-20K tokens.
- Net savings per session: 30-70K tokens (60-80% reduction).
