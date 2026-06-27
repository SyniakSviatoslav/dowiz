# autoupgrade — run memory

## 2026-06-27 · run #1 (REPORT-ONLY, §8 step 2)

First run on the v3-FINAL harness. MAP grounded in real telemetry (codeburn + fs scan), CLASSIFY fail-safe, NO apply (oracle not yet built).

**Candidates (4):**
- Class A (auto-eligible once oracle proven, 3):
  - `ghost-mcp:claude_design` — codeburn: 0/20 tools used. Action: `claude mcp remove 'claude_design'`. Reversible.
  - `ghost-mcp:claude_ai_Notion` — codeburn: 0/18 tools used across 462 sessions (~$46/mo cached-prefix). Action: `claude mcp remove 'claude_ai_Notion'`. Reversible.
  - `config-bloat:CLAUDE.md` — large file re-read into every prefix. Action: distill → linked docs. Reversible (med blast).
- Class B (propose-only, human/DB-owner, 1):
  - `staged-security:SECURITY-DEFINER-search-path.migration.ts` — firm boundary (security/schema) → NEVER autonomously applied.

**Verdict:** firm boundary held (security migration → B; dev-loop/MCP/config → A). Report-only — nothing applied.

**Classifier regression fixed this run:** the ghost-MCP prune briefly mis-classified as B because the free-text regex matched "session" in "loaded every session" (auth-session false-positive). Fix: broad boundary list matches the structured `area`; tight list matches free text. Regression test added.

**Carry forward → run #2:** none of the Class-A actions are applied until the machine oracle (§2: green + RLS/auth/secret assertions + benchmark-replay ≥5% speedup + recorded revert) and atomic rollback are built and proven (§8 steps 3–4). `applyCandidate()` throws by design until then.
