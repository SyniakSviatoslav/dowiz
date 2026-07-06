# Sovereign Core MVP — Autobuild PROGRESS (git-based resume cursor)

> The auto-continuation routine runs in an isolated cloud checkout and CANNOT see the local memory store —
> THIS file is the resume cursor. Each run: read this + `DECISIONS.md` + `GRAND-PLAN.md` +
> `STRUCTURE-UPGRADE.md` + `LEAD-REVIEW.md`, do the NEXT step, gate it, commit-if-green, then UPDATE the
> DONE / NEXT / BLOCKERS sections here and push.

## Branch: `feat/sovereign-core-phase-zero`
**Concurrent staged work — NOT ours, NEVER commit:** `rebuild/Cargo.lock`,
`rebuild/crates/api/src/routes/orders/*`, `rebuild/crates/domain/{Cargo.toml,src/codec*}`.

## DONE (2026-07-06)
- **STRUCTURE-UPGRADE Part B · B0 baseline (this run):** built `scripts/audit-token-router.mjs` — the
  deterministic $0 read-only auditor (B4's script, needed first for B0). Self-test red→green proven
  (a/e→exit1 + over-block guards for the JSON-blob heuristic + TaskCreate-exclusion). Ran it over 95
  transcripts → baseline appended as §10 to `docs/research/token-economy-comparison-2026-07-05.md`.
  **Headline: only ≈10% of 1027 dispatches carry an explicit `model:` (885 = 86% model-less; 43 fable
  across 5 sessions).** → drives the B1 rollout decision in NEXT/BLOCKERS. (b) 80K per-lane is NOT
  hook-visible (sub-agent sidechains never appear in the lead transcript — confirms B2's honest caveat).
- **Phase-0a token opt:** route-request −91% nudges (excl. `<task-notification>`) + `LEDGER-INDEX.md`
  (`93c2ef26`). Marginal hook tweaks (require-classification/red-line-gate scope) deferred.
- **Harness unlock+seal (`340a8c3a`):** `.claude/*` agent-editable; migrations/.env/db/contracts/.github/
  lockfile + human-only `.claude/state` stay protected; guard-bash inline-interpreter-write hole sealed.
- **Phase-0b safe hardening:** f64/f32 banned in core (`83ac471e`, red→green); cargo-deny Gate 3
  (`87cfa823`). Core float-free + supply-chain-gated. cargo-deny surfaced **RUSTSEC-2023-0071** (rsa
  "Marvin Attack", no patch) + yanked num-bigint — actionable at the auth/crypto work.
- **Plans committed** (`33caae75`): MANIFESTO/DECISIONS/ANALYSIS/GRAND-PLAN/LEAD-REVIEW/STRUCTURE-UPGRADE.

## NEXT SEQUENCE
1. **STRUCTURE-UPGRADE.md Part B · B1** — the dispatch gate. B0 (DONE) changed its shape: a blind hard-DENY
   on missing `model:` would block ~90% of the real dispatch pattern (incl. 106 legit Explore lanes +
   triad council). **DECISION NEEDED before B1 ships (see BLOCKERS): warn-then-ratchet vs grace-flagged
   hard-deny.** Recommended sequence once decided: land B1 as PostToolUse WARN first (logs `_hev` WARN +
   names the `model:`/`LANE-CLASS:` fix, ZERO block), let the stamp habit build in `_hev`, THEN promote the
   habitual-compliant checks to PreToolUse DENY. Still: ONE check at a time, each with a
   `guardrail-token-gates.mjs` armament case proving DENY + non-over-block (narrow-never-remove; #1 risk =
   #47 wholesale-unregister). `context-budget-guard.sh` (B2) already staged in `proposed-hooks/`.
1b. **B5 wiring (cheap, do alongside):** add `node scripts/audit-token-router.mjs --self-test` to the
   guardrail suite / weekly curation so the auditor itself can't rot; feed its exit-1 into THE EYE as a `bad`
   signal once B1 is armed.
2. **STRUCTURE-UPGRADE.md Part A** — incremental strangler moves (A0 `module.toml` manifests, A1 boundary
   gate, A2 `channel.rs` pilot, A4 route-freeze, A5 placement). A3 (orders split) BLOCKED on GRAND-PLAN 0b-5.
3. **Money boundary — GRAND-PLAN 0b:** extract `pricing.rs` (f64-boundary split; f64 haversine stays in
   shell) + corridors behind `decide`. Verify with a DECORRELATED, independent, hand-derived (NON-mirror)
   money oracle + byte-parity + sovereign gate green. Delegate the oracle to a FRESH opus worker
   (decorrelation — a same/rotted reviewer is how #56 shipped "certified green").
4. Then Envelope+events, proptests, shell-flip (REUSE the existing cutover shadow-diff, F1), CI+cargo-deny
   (`.github` operator-gated) → Phase 1 hub → Phase 2 MVP.

## GUARDRAILS (every run)
Enforce `rebuild/scripts/sovereign-gate.sh` + cargo tests. Commit+push ONLY when green, scoped, EXCLUDING the
concurrent work above. **STAGING-ONLY: never prod/secrets/.env/.github/prod-deploy/force-push.** STOP + record
in BLOCKERS below (do NOT guess or ship) on: unresolvable gate-red, byte-parity mismatch, money/RLS/auth
uncertainty, a harness edit risking recoverability, or a genuine product-vision fork. Skip an unresolved
recorded blocker. Keep context lean — delegate fiddly/risky work to fresh workers with an explicit `model:`.

## BLOCKERS (awaiting operator — soft: do not block other Part-B/GRAND-PLAN work)
- **B1 rollout mode (design fork surfaced by the B0 data).** B0 measured that ≈90% of real dispatches
  omit `model:` (886-ish incl. Explore + the triad council agents). A blind hard-DENY B1 would block the
  operator's own normal dispatch pattern from call one — an over-block against live workflow, which the
  autonomy mandate says to STOP + record rather than guess. Two safe paths:
  (A) **warn-then-ratchet** — B1 ships as a PostToolUse WARN that names the fix and logs `_hev`; promote to
      PreToolUse DENY per-check only once the log shows the stamp habit took (recommended; lowest blast radius).
  (B) **grace-flagged hard-deny** — ship DENY now behind a human-only expiring `.claude/state/token-gate-grace`
      that fails OPEN until a date, so the operator opts into teeth when ready.
  Auditor + baseline are already committed and useful regardless. Awaiting operator pick of A vs B.
