# OpenHands pilot — out-of-tree agent-plane candidate

**STATUS: SCAFFOLDED — DO NOT USE. PENDING CONFLICT-RESOLUTION.** Out-of-band. Not wired, not in CI, not
a dependency. Registered as a *candidate*, dark.

## What it is
[All-Hands-AI/OpenHands](https://github.com/All-Hands-AI/OpenHands) (MIT, ex-OpenDevin) — an autonomous
software-engineering agent platform (reads/writes code, runs commands, browses, opens PRs) that runs its
action-execution in a sandboxed Docker runtime.

## 🔶 Conflict / coherence eval (the "don't conflict, utilize" rule)
This repo's **agent plane is already Claude Code** + a heavy in-house harness (council, loops,
librarian/critics, doubt-escalation, TOOLING-REGISTRY subagents). OpenHands is a *competing* orchestration
harness — adopting it wholesale would duplicate the harness and split the loop system. **Not an
always-on second brain.**
- **Coherence decision (deferred):** if piloted at all, only as an **out-of-band, sandboxed executor for a
  narrowly-scoped batch task** (e.g. a mechanical migration sweep) benchmarked against a Claude-Code
  workflow — adopt only if it decisively wins that one job. Default expectation: **REJECT for overlap.**

## Boundary (dev/ops plane only — never product)
- OUT OF TREE. `openhands` is **FORBIDDEN-DEP** — never a product dependency/import.
- Its Docker runtime stays isolated from the dowiz management plane; run against a scratch checkout /
  staging, never prod data. Snapshot-restore plan required.
- No dowiz DB / RLS / tenant secret in its env: `node scripts/skyvern-pilot/no-credential-attest.mjs <env>`.
- BYO-LLM through the existing OpenRouter seam (`.env` `OPENROUTER_API_KEY`); telemetry off; egress
  allowlisted. Any code it produces re-enters the normal gate (lint→typecheck→build→proof), never
  auto-merged.
