# CHANGE-MANIFEST

CLASSIFICATION: build   # one of: spike | build | audit | challenge  (§1 — routes the governance mode)

FINDING-id: p1-p3-meta-loop-revival-2026-07-02
Intent: fix — enact P1/P2/P3 of the meta-loop audit (human-approved): forcing functions for the
advisory arm, harness self-measurement, loop-telemetry repair, ledger/registry integrity, broader
default permissions.

Touched files (unprotected, committed directly):
- scripts/agent-health-pass.mjs (new) — harness measures itself → docs/governance/agent-health-<date>.md
- scripts/guardrail-ledger-integrity.mjs (new) — unique ledger #N refs, red→green vs HEAD (dups 7/9/10/11)
- scripts/loops-registry-sync.mjs (new) — registry.md = SoT → runs/registry.json (router saw 2/16 loops)
- scripts/guardrail-gate-armament.mjs — V2 version-gated cases (event-log, docs exemption, lesson-inject)
- scripts/verify-all.ts — +ledger-integrity +registry-sync --check (ci:true)
- tools/loop-harness/src/breaker.ts — finite default budget ($75) + time cap (4h); Infinity never bound
- .gitignore — track loops/runs/{metrics,routing}.jsonl + registry.json (history was VM-local)
- loops/registry.md — honest cert flags (3 lost reports), +test-hardening/offer-builder rows, sync note
- loops/runs/metrics.jsonl — deduped double-counted test-hardening row
- docs/lessons/** + docs/reflections/** — librarian curation run (first since 06-23): INBOX drained,
  3 lessons promoted, 1 refreshed, 4 INDEX rows
- docs/regressions/REGRESSION-LEDGER.md — dup rows suffixed (7b/9b/10b/11b) + row #48
- docs/governance/agent-health-2026-07-02.md — first health report

Staged for OPERATOR apply (protect-paths; scratchpad staged-p1/apply-p1.sh):
- 7 hook v2s (harness-events.jsonl telemetry; Stop-gate: reflection pulse + placeholder block;
  serious-gate docs/* exemption; loop-detector escalation log + stale-counter purge)
- settings.json (broader defaults: MultiEdit/PowerShell/ToolSearch/Workflow/Monitor/figma MCP)
- phantom-ref fixes (converge-loop, loop-architect) + finalize wiring into 12 loop commands

Proof: gate-armament V2 24/24 vs staged hooks; pnpm verify:all --ci ALL PASSED (16 gates);
breaker tests 0 fail; ledger guardrail red (HEAD) → green (current).

# Reminder (§5): a well-proven FAIL / MISSING / BLOCKED is a SUCCESSFUL run, equal to PASS.
