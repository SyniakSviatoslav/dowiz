# Sovereign Core MVP — Autobuild PROGRESS (git-based resume cursor)

> The auto-continuation routine runs in an isolated cloud checkout and CANNOT see the local memory store —
> THIS file is the resume cursor. Each run: read this + `DECISIONS.md` + `GRAND-PLAN.md` +
> `STRUCTURE-UPGRADE.md` + `LEAD-REVIEW.md`, do the NEXT step, gate it, commit-if-green, then UPDATE the
> DONE / NEXT / BLOCKERS sections here and push.

## Branch: `feat/sovereign-core-phase-zero`
**Concurrent staged work — NOT ours, NEVER commit:** `rebuild/Cargo.lock`,
`rebuild/crates/api/src/routes/orders/*`, `rebuild/crates/domain/{Cargo.toml,src/codec*}`.

## DONE (2026-07-06)
- **STRUCTURE-UPGRADE Part B · B1 warn-gate (this run; operator picked rollout A = warn-then-ratchet):**
  new `.claude/hooks/agent-dispatch-gate.sh` — PreToolUse `Agent|Task` gate, WARN mode (never blocks),
  logs `_hev` + nudges on the one check B0 proved matters most (missing `model:`, 86%). Deny path armed
  (`TOKEN_GATE_MODE=deny`) so the ratchet is a config flip. Armament `scripts/guardrail-token-gates.mjs`
  (11 cases, red→green — caught a real tab-vs-newline empty-field parse bug). Registered in
  `settings.json`; pinned in `guardrail-hook-matchers.mjs` (#47 anti-unregister). **LIVE-VERIFIED:** a
  real model-less Explore dispatch fired the hook → `_hev` warn, non-blocking (agent completed).
- **Fixed a pre-existing stale governance test** (`guardrail-gate-armament.mjs`): the `340a8c3a` unlock
  narrowed guard-bash to make `.claude/hooks/` agent-editable but left the "sed into .claude/hooks blocked"
  assertion red (gate-armament isn't in pre-commit, so it went unnoticed). Now asserts the unlocked reality
  + keeps a still-protected-zone (migrations) sed-block case. red→green.
- **STRUCTURE-UPGRADE Part B · B0 baseline (prev run):** built `scripts/audit-token-router.mjs` — the
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
1. **B1 fable-check — DONE 2026-07-06** (operator "use cheaper models instead of fable"): Check 2 in
   `agent-dispatch-gate.sh` DENIES `model: fable` unless a non-expired human-only
   `.claude/state/fable-override` line exists (fail-closed; guard-bash OVERRIDES + protect-paths pin it
   human-only). Armament +6 fable cases; LIVE-VERIFIED (a real fable dispatch was refused). Model-routing
   memory v3.1 = arc Fable-override rescinded. **Next B1 increment:** `LANE-CLASS`/router-stamp checks —
   ONLY after the stamp convention is written into AGENTS.md (else pure nudge-spam), each with an armament case.
2. **RATCHET model-check warn→deny** once `_hev` shows the `model:` habit took (measure with
   `scripts/audit-token-router.mjs` trend going down): flip the check's default `MODE` warn→deny (the
   ratchet point is commented in the hook). Re-run the armament (deny path already proven).
3. **B2:** `cp docs/operating-model/proposed-hooks/context-budget-guard.sh .claude/hooks/` + register under
   `UserPromptSubmit` with `CONTEXT_WINDOW=1000000 CONTEXT_BUDGET_PCT=30` (fires at 300K). Re-arm its fixtures.
4. **B3** distill-nudge (PostToolUse Bash, WARN) → **B4** wire `audit-token-router` into weekly curation +
   THE EYE (`--self-test` in the guardrail suite so the auditor can't rot; exit-1 = a `bad` EYE signal) →
   **B5** ratchets (pre-commit registration pins, KNOWLEDGE-AS-CIRCUITS entry, ledger).
5. Then **PART A** (incremental modular strangler moves) after the Part B exit gate. All BEFORE money.
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

## BLOCKERS (awaiting operator)
- _none currently._ (B1 rollout fork RESOLVED 2026-07-06: operator chose **A · warn-then-ratchet**; B1
  warn-gate shipped + live-verified this run. Ratchet to deny is data-gated on the `_hev` habit trend.)
