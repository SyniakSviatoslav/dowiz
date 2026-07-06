# Sovereign Core MVP — Autobuild PROGRESS (git-based resume cursor)

> The auto-continuation routine runs in an isolated cloud checkout and CANNOT see the local memory store —
> THIS file is the resume cursor. Each run: read this + `DECISIONS.md` + `GRAND-PLAN.md` +
> `STRUCTURE-UPGRADE.md` + `LEAD-REVIEW.md`, do the NEXT step, gate it, commit-if-green, then UPDATE the
> DONE / NEXT / BLOCKERS sections here and push.

## Branch: `feat/sovereign-core-phase-zero`
**Concurrent staged work — NOT ours, NEVER commit:** `rebuild/Cargo.lock`,
`rebuild/crates/api/src/routes/orders/*`, `rebuild/crates/domain/{Cargo.toml,src/codec*}`.

## DONE (2026-07-06)
- **Phase-0a token opt:** route-request −91% nudges (excl. `<task-notification>`) + `LEDGER-INDEX.md`
  (`93c2ef26`). Marginal hook tweaks (require-classification/red-line-gate scope) deferred.
- **Harness unlock+seal (`340a8c3a`):** `.claude/*` agent-editable; migrations/.env/db/contracts/.github/
  lockfile + human-only `.claude/state` stay protected; guard-bash inline-interpreter-write hole sealed.
- **Phase-0b safe hardening:** f64/f32 banned in core (`83ac471e`, red→green); cargo-deny Gate 3
  (`87cfa823`). Core float-free + supply-chain-gated. cargo-deny surfaced **RUSTSEC-2023-0071** (rsa
  "Marvin Attack", no patch) + yanked num-bigint — actionable at the auth/crypto work.
- **Plans committed** (`33caae75`): MANIFESTO/DECISIONS/ANALYSIS/GRAND-PLAN/LEAD-REVIEW/STRUCTURE-UPGRADE.

## NEXT SEQUENCE
1. **STRUCTURE-UPGRADE.md Part B** (mandatory token-reduction) — apply ONE gate-check at a time, each with an
   armament test proving DENY + non-over-block (narrow-never-remove; B1 incremental — its #1 risk is #47
   wholesale-unregister). `context-budget-guard.sh` already exists in `docs/operating-model/proposed-hooks/`.
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
- _none currently._
