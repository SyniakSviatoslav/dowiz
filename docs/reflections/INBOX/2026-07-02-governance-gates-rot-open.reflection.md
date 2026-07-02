---
CONTEXT:   P0 of the 2026-07-02 meta-loop audit (human-approved): serious-gate + red-line gate
           were de-facto open since 06-21/06-23, the Bash lane had no gate at all, and
           verify:all --ci was wired into no workflow. Enacted: TTL'd clearances, guard-bash
           registered, gate-armament guardrail, CI wiring via proposed-ci-test-gates/APPLY.md.
DECISIONS: Per-line `slug|expiry-epoch` clearance (legacy bare lines never clear) instead of
           slug↔path matching (over-engineering); 60-min window for redline-confirmed; Bash
           guard rebuilt with staging-deploy explicitly ALLOWED (the old verbatim guard blocked
           it — the real reason it was unregistered in 43a018c1); armament proven by hermetic
           hook simulation (19 cases), not by registration checks alone.
WHERE:     .claude/logs/classification.log: last DENY 2026-06-21, then 400+ blind "ALLOW cleared"
           incl. plisio.ts (money), auth tests, a migration. guard-bash absent from settings.json
           since 43a018c1. loops/runs/metrics.jsonl: 7 rows ever. docs/lessons frozen at 4.
WHY:       The harness taxonomy had two categories — advisory signals and deterministic
           authority — but no third: AUTHORITY-BEARING STATE. The gates were deterministic
           code, yet their release condition was a mutable state file with no expiry, whose
           cleanup depended on a discipline step ("truncate after ship") that, like every
           discipline-triggered step in the system (librarian, retro, finalize), stopped being
           performed. Nothing measured armament — only registration — so "the hook runs" was
           mistaken for "the hook denies", and 400 blind ALLOWs read as silence-equals-health.
           Secondary root: when a gate over-blocked (guard-bash vs staging deploys), it was
           removed wholesale instead of precision-fixed — an over-broad gate converts to NO
           gate under delivery pressure.
CONFIDENCE: high
NEXT-TIME: Any gate released by a state file needs (a) an expiry in the state itself, (b) an
           armament test that simulates DENY, not just registration, and (c) a log line per
           decision so blind-open shows up in data. When a gate over-blocks, narrow the rule,
           never unregister the gate. Candidate follow-up: serious-gate exempts docs/*.md
           (precedent: post-edit-gates 06-29 docs exemption) — every ledger append currently
           needs a clearance line, and recurring friction is how gates get bypassed again.
LINK:      docs/regressions/REGRESSION-LEDGER.md #47 ; scripts/guardrail-gate-armament.mjs ;
           memory meta-loop-audit-2026-07-02
---
