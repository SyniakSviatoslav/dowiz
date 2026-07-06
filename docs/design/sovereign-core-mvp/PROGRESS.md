# Sovereign Core MVP — Autobuild PROGRESS (git-based resume cursor)

> The auto-continuation routine runs in an isolated cloud checkout and CANNOT see the local memory store —
> THIS file is the resume cursor. Each run: read this + `DECISIONS.md` + `GRAND-PLAN.md` +
> `STRUCTURE-UPGRADE.md` + `LEAD-REVIEW.md`, do the NEXT step, gate it, commit-if-green, then UPDATE the
> DONE / NEXT / BLOCKERS sections here and push.

## Branch: `feat/sovereign-core-phase-zero`
**Concurrent staged work — NOT ours, NEVER commit:** `rebuild/Cargo.lock`,
`rebuild/crates/api/src/routes/orders/*`, `rebuild/crates/domain/{Cargo.toml,src/codec*}`.

## PART B STATUS — substantially COMPLETE (2026-07-06)
B0 baseline · B1 model-check (warn, live) · B1 fable-check (deny, live) · B2 context-budget-guard (live) ·
B3 distill-nudge (warn, live) · B4 auditor+self-test · B5 armament-runner+pins — all shipped + live-verified.
**Exit gate:** token-gates green ✓ · hook-matchers green (8 gates) ✓ · `_hev` shows real deny/warn traffic ✓ ·
full B0-vs-post-B1 delta = pending session accumulation (the ratchet trigger). **Deferred (not blocking):**
LANE-CLASS/router-stamp checks (need the stamp convention written into AGENTS.md first); KNOWLEDGE-AS-CIRCUITS
entry (YAGNI — no committed agent-dispatching script exists to guard yet); THE EYE wiring (EYE still proposed).
**→ Next phase = PART A** (modular strangler moves) — a different, Rust-crate-touching workstream; start it in a
FRESH session (F5 anti-context-rot).

## PART A STATUS — IN PROGRESS (2026-07-06, fresh session)
- **A0 manifest contract + A1 boundary gate — ✅ DONE.** `rebuild/crates/{domain,api}/module.toml` stamped;
  `scripts/module-integrity.mjs` (pure core + hermetic `--self-test`) enforces schema + `depends`==`cargo
  metadata` + core ban-list + hub-module cross-import + contract-existence. RED→GREEN proven live (api
  `depends=[]` → red → restore → green); self-test caught 2 real parser bugs. Wired: `--self-test` in
  `run-armaments.sh` (pre-commit 1.4d) + real-tree in `.husky/pre-commit` 1.4e (cargo-guarded) + CI step in
  `proposed-sovereign-core-ci/APPLY.md`. Schema = STRUCTURE-UPGRADE.md §MODULE-CONTRACT. Ledger #86.
- **A2 pilot module — ✅ DONE.** `channel.rs` → `modules/channel_attribution/mod.rs` (git mv, byte-identical)
  + 8-line `pub use` shim at the old path (single call site untouched) + hub-module manifest. `mod modules;`
  in main.rs. Proofs: empty byte-diff, 9 moved tests pass, `cargo check -p api` + module-integrity green,
  RED proof (blanked shim → E0425 at the call site → restore). Ledger #87. Landing zone for GRAND-PLAN 1.1/1.5.
- **A4 legacy-freeze — ✅ DONE.** `scripts/guardrail-legacy-freeze.mjs` + `legacy-api-baseline.json`
  (count=237/68 files) reds on any `apps/api` route-registration INCREASE; hermetic `--self-test`; RED
  proof (237→238 → red → remove). Wired: run-armaments (self-test) + pre-commit 1.4f (real). Ledger #88.
- **A5 placement rule — ✅ DONE.** Binding "Module placement (A5)" convention added to GRAND-PLAN
  §Conventions — new hub features land in `crates/api/src/modules/<name>/` with a manifest, events-only,
  enforced by module-integrity. Paper move; gate = A1's.
- **🟢 PART A EXIT (2026-07-06):** A0·A1·A2·A4·A5 all DONE + gated (3 commits: fd444fbc A0/A1, f1e647fb A2,
  + A4/A5). A3 (orders split) stays BLOCKED on GRAND-PLAN 0b-5 + 1.3 (do NOT start early — #1 failure root).
- **NEXT PHASE = the red-line MONEY BOUNDARY (GRAND-PLAN 0b-1), in a FRESH session (F5):** extract
  `pricing.rs` (884-line) into the core with the f64-boundary split (pure integer money → core; f64
  haversine STAYS in the shell), then corridors behind the single `decide` door. Verify with a
  DECORRELATED, independent, hand-derived (NON-mirror) money oracle + byte-parity + sovereign gate green
  — delegate the oracle to a FRESH opus worker (a same/rotted reviewer is how #56 shipped "certified
  green"). This is red-line: invariant-guardian read (NO council — removed 2026-07-06). STOP-and-record if any byte-parity mismatch.

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
**Part B (token-enforcement) = COMPLETE. Part A (modular topology) = COMPLETE (A0·A1·A2·A4·A5, commits
fd444fbc→b6666bc6, ledgers #86–88 — see the PART A STATUS block above).** The next UNCOMPLETED step is #1.

1. **▶ NEXT — Money boundary (GRAND-PLAN 0b-1), the red-line crown jewel. FRESH SESSION (F5 mandatory).**
   Extract `rebuild/crates/api/src/routes/orders/pricing.rs` (884 lines) into the core (`crates/domain`)
   with the f64-boundary split: the pure INTEGER money fns (`apply_tax`, `compute_line_total`,
   `compose_total`, `charged_tax`, `compute_order_pricing`) move to core; the f64 seam STAYS in the shell —
   `tax_rate: f64` is converted to `rate_micro` (i64 micro-units) at the shell edge BEFORE crossing into
   core, and `distance_km` (Haversine) + geo pins (`FeeLocation.lat/lng`, tier `max_distance_km`) stay in
   the shell (fee-from-distance selection is i64 and can move once distance is resolved shell-side). Then
   corridors behind the single `decide` door (0b-3/0b-5). **VERIFY (non-negotiable):** a DECORRELATED,
   independent, hand-derived, NON-mirror money oracle — delegate to a FRESH `model: opus` sub-agent that
   re-derives expected values from the Node source + spec, NOT from the Rust code under test (a same/rotted
   reviewer is how #56 shipped "certified green") — plus BYTE-PARITY against the existing hand-derived test
   vectors (`order_total_composition_byte_parity_vs_hand_derived_vectors` etc. already in pricing.rs) + the
   f64-ban clippy gate + `bash rebuild/scripts/sovereign-gate.sh` green. **⛔ NO COUNCIL (removed 2026-07-06):**
   do NOT convene any Triadic/Architect-Breaker-Counsel council — proceed DIRECTLY on the decorrelated
   oracle + byte-parity + invariant-guardian read + gates. The design already exists (ADR + proposal +
   resolution, commits fc0c4272→7584f035, council concluded GO) — build on it. **STOP + record in BLOCKERS on
   ANY byte-parity mismatch or money/RLS/auth uncertainty — do NOT guess or ship.** Keep the concurrent
   request_hash work excluded from every commit.
2. Then Envelope+events, proptests, shell-flip (REUSE the existing cutover shadow-diff, F1), CI+cargo-deny
   (`.github` operator-gated) → Phase 1 hub → Phase 2 MVP.
3. Deferred (non-blocking, pick up opportunistically): RATCHET the B1 model-check warn→deny once
   `audit-token-router` shows model-less <10%; B1 LANE-CLASS/router-stamp checks after the stamp convention
   is written into AGENTS.md; KNOWLEDGE-AS-CIRCUITS `require_together` entry once a committed
   agent-dispatching script exists to guard; wire `run-armaments.sh` into THE EYE once THE EYE is applied.
   A3 (orders module split) stays BLOCKED on GRAND-PLAN 0b-5 + 1.3 — do NOT start early.

## GUARDRAILS (every run)
Enforce `rebuild/scripts/sovereign-gate.sh` + cargo tests. Commit+push ONLY when green, scoped, EXCLUDING the
concurrent work above. **STAGING-ONLY: never prod/secrets/.env/.github/prod-deploy/force-push.** STOP + record
in BLOCKERS below (do NOT guess or ship) on: unresolvable gate-red, byte-parity mismatch, money/RLS/auth
uncertainty, a harness edit risking recoverability, or a genuine product-vision fork. Skip an unresolved
recorded blocker. Keep context lean — delegate fiddly/risky work to fresh workers with an explicit `model:`.

## BLOCKERS (awaiting operator)
- _none currently._ (B1 rollout fork RESOLVED 2026-07-06: operator chose **A · warn-then-ratchet**; B1
  warn-gate shipped + live-verified this run. Ratchet to deny is data-gated on the `_hev` habit trend.)
