# Operator apply — approved slimming batch (2026-07-05, "approve all")

Every call and every lane re-reads the injected floor; these three cuts are permanent.

## 1. CLAUDE.md CORE split (−~2.5K/call)
The current .claude/CLAUDE.md content that is NOT in CLAUDE-CORE.md (Repowise long tool-guide +
trust protocol details, hotspot/biomarker tables, guided tour, architectural layers, full
self-improvement stages, ponytail details) is REFERENCE material agents read on demand.
```
cp .claude/CLAUDE.md docs/operating-model/claude-md-reference.md   # preserve full text as the reference doc
cp docs/operating-model/proposed-claude-md/CLAUDE-CORE.md .claude/CLAUDE.md
```

## 2. MEMORY.md cap — DONE (no action needed)
107 → ~30 lines; closed topics in memory/MEMORY-ATTIC.md. Already applied (memory dir is not a
protected zone).

## 3. Hook-nudge narrowing (−~100-200 tokens × every turn) — .claude/hooks is protected
Per docs/operating-model/proposed-hooks/route-request-nudge-dedup.md (existing proposal) +
the 07-04 audit lever #6: make the serious-gate + repeat-task nudges fire once per N turns per
topic instead of every prompt, and narrow the pre-edit-lessons docs/** trigger. Operator applies
the hook edits (or grants a one-time exception).

## 4. MCP/connector trim (−1-4K/lane)
The claude.ai connectors (Gmail/Notion/Calendar/Drive/Figma/Sentry/Common Room/Consensus/Harmonic/
Scholar/Synapse/Learning-Commons) + browser-use ride the deferred-tool list of every call from
user-level config — disable unused ones in the Claude Code / claude.ai connector settings (not
repo-controlled). Repo-side .mcp.json keeps repowise + codebase-memory + playwright-test (used).
