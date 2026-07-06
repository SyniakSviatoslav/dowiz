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

## PHASE 0b STATUS — 0b-1 + 0b-2 DONE (2026-07-06)

- **0b-2 event vocabulary + `Envelope { seq, at, cause }` (GRAND-PLAN 0b-2) — ✅ DONE (this commit).**
  Grew the kernel alphabet from the lone `StatusChanged` to the money/binding facts the live lifecycle
  already produces (matching `policy::TransitionEffects`): `Event::Priced { subtotal, delivery_fee,
  tax_total, total }` (all `Lek`), `RefundObligated { amount: Lek }`, `BindingTerminalized`. Added
  `OrderTotals` (4 integer `Lek`), `CommandHash(String)` (serde-transparent), and
  `Envelope { seq: u64, at: Ts, cause: CommandHash, event: Event }`. `OrderState` grew a money snapshot
  (`totals: Option<OrderTotals>`), `refund_due: Option<Lek>`, and `binding_terminalized: bool`;
  `genesis()` stays `const` (None/None/false). `fold` is now the **accumulating** fold — exhaustive
  `match *event` (no `_` arm), each arm carries `..*state` and writes only its own field, SET-not-sum on
  money so the fold stays TOTAL (no fallible arithmetic). Added `replay_envelopes`.
  **KEY DESIGN CALLS (plan-vs-reality reconciliations):** (1) `CommandHash` is an OPAQUE core newtype
  SUPPLIED by the shell (like `Ts`), NOT computed in-core — the plan's `codec/request_hash.rs`
  placement pre-dated the discovery that `build_request_hash` lives in the `api` shell
  (`routes/orders/request_hash.rs`), which the core cannot reach; the core owns only the type it carries.
  (2) `decide` is UNCHANGED (still emits only `StatusChanged`) — EMITTING the new facts = corridor
  wiring = 0b-3, deliberately deferred (scope discipline). The new aggregate is inert on the live path
  (the api shell consumes only `kernel::{pricing,idempotency,policy}`, never `OrderState`/`fold`/`Event`
  — invariant-guardian confirmed). (3) `at` stays on the frozen `StatusChanged`; the newer facts carry
  no independent time — the `Envelope` records log-time.
  **Gate (D5):** exhaustive-match compile gate = **rustc E0004** (no `_` arm → a new variant without a
  fold arm is a hard COMPILE ERROR — the strongest gate, the compiler itself). RED proof re-verified:
  injecting `Event::DummyRedProof` → `error[E0004]: non-exhaustive patterns … not covered`. BELT against
  the one E0004 dodge (silencing it with `_ => *state`): a deterministic source self-test
  `fold_stays_exhaustive_no_wildcard_arm` (reads fold's own source, whitespace-insensitive, fails on any
  `_=>` arm; RED proof: inject `_=>` → test FAILED). **clippy `wildcard_enum_match_arm` was evaluated
  as the belt and REJECTED as a false-green** — clippy 1.96 fires it only crate-wide on an owned
  direct-binding match, NEVER on fold's `&Event`/deref match, so a fn-level `#[deny]` or gate lint would
  have LOOKED green while guarding nothing (documented in `fold`'s doc-comment). Hard-Truth replay
  extended (mixed-log reconstructs status+money+binding) + canonical-bytes round-trip property tests for
  the grown alphabet AND for `Envelope` logs (cause included). **Verified:** 86 core lib + 6 hard_truth
  + 9 kernel_hard_truth tests green; `sovereign-gate.sh` Gate 1 (wasm32) + Gate 2 (disallowed-types,
  float-free) green; `cargo check -p api` clean (no shell regression); invariant-guardian read (opus,
  decorrelated) = PASS/high/no-flags. Gate 3 (cargo-deny) fails ONLY on the pre-existing
  RUSTSEC-2023-0071 (rsa Marvin Attack) + yanked num-bigint in the api web-push/jwt chain — NOT this
  change (zero deps added, Cargo.lock untouched); tracked for the auth/crypto work. Files:
  `rebuild/crates/domain/src/{kernel.rs,lib.rs}`, `tests/kernel_hard_truth.rs`. Commit: this change.

## PHASE 0b STATUS — 0b-1 DONE (2026-07-06, fresh session)
- **0b-1 money boundary (GRAND-PLAN 0b-1) — ✅ DONE.** `pricing.rs` (884 lines) extracted from the
  `api` shell into `rebuild/crates/domain/src/kernel/pricing.rs` — integer-only by construction
  (core `disallowed-types` clippy gate bans f64/f32 outright). A thin shell adapter survives at the
  old path (`routes/orders/pricing.rs`) as the single float chokepoint: owns `distance_km`
  (Haversine) + converts `tax_rate→rate_micro` and `distance_km→distance_m` (whole meters) before
  calling the core; `pg.rs`/`shifts.rs` keep their existing f64 signatures unchanged.
  **Guard split** (Breaker-caught gap in the design review): the old f64 short-circuit
  (`subtotal==0 || tax_rate<=0.0 || !is_finite → Ok(0)`) is split by domain, not dropped — core
  catches `subtotal==0||rate_micro<=0` (protects every future caller), shell catches
  `!tax_rate.is_finite()` before conversion (±Infinity maps to a positive `i64::MAX` the core guard
  can't see) — together reproduce the old `Ok(0)` for every exotic input. `compute_order_pricing`'s
  `HashMap`/`HashSet` (the core's first would-be entropy source) → `BTreeMap`/`BTreeSet`.
  `PricingError.code` moves from a shell `&str` to `domain::ErrorCode`, deleting pg.rs's redundant
  `pricing_code` string mirror. **Verified:** a decorrelated oracle (fresh opus, blind to the Rust
  code) hand-derived 28 expected values from the Node reference alone — all match
  (`oracle-vectors.md`). 80 core tests + 56 api tests green; `sovereign-gate.sh` green (wasm32 +
  disallowed-types); RED→GREEN clippy proof independently re-verified (inject f64 → Gate 2 fails →
  revert → green); `invariant-guardian` read: PASS, no flags. Design process: Triadic Council ran
  first (proposal+breaker-findings+counsel-opinion+resolution, HARD EXIT reached, GO) — operator then
  removed the council requirement for this step going forward (kept + built on the existing
  artifacts); the actual code implementation proceeded on the decorrelated oracle + byte-parity +
  invariant-guardian read + sovereign-gate, no further council. Design docs:
  `docs/design/sovereign-core-money-boundary-0b1/{proposal,resolution,oracle-vectors,ADR}.md`.
  Commit: `c10814ab`.

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
fd444fbc→b6666bc6, ledgers #86–88). 0b-1 (money boundary) = COMPLETE (commit `c10814ab` — see PHASE 0b
STATUS above). 0b-2 (event vocabulary + Envelope) = COMPLETE (this commit — see PHASE 0b STATUS
above).** The next UNCOMPLETED step is #2 (0b-3: compose corridors behind `decide`).

1. **0b-2 (event vocabulary + Envelope) — ✅ DONE** (this commit; see PHASE 0b STATUS above). The
   concurrent `request_hash` workstream was already landed+clean (`codec.rs` @ `d5f9deb3`), so no
   codec collision; `CommandHash` ended up an opaque core newtype (shell-supplied), not a codec fn.

2. **▶ NEXT — 0b-3: Compose the corridors behind the single `decide` door (GRAND-PLAN 0b-3). FRESH
   SESSION (F5 mandatory).** This is the WIRE step 0b-2 deliberately left undone: make `decide` EMIT
   the new facts. Fold the pure corridors that today live beside `decide` — `policy::{actor-gate,
   transition_effects (terminalize→BindingTerminalized, refund_due→RefundObligated), cc1_strand_guard,
   needs_honest_dispatch}` + `kernel::pricing` (→ `Priced`) + `kernel::idempotency` — INTO the single
   `decide` door, so one command yields the full event set (`StatusChanged` + the money/binding facts)
   under one Envelope. **Gate (D5):** the existing Hard-Truth replay/determinism/round-trip suite must
   stay green AND extend to assert `decide` now emits the composed facts on the relevant edges (e.g.
   Cancel of a priced order → `[StatusChanged, BindingTerminalized, RefundObligated]`); byte-parity vs
   the Node reference for any newly-emitted money numbers; sovereign gate green. **Red-line:** money
   types — invariant-guardian read, NO council (removed 2026-07-06). STOP-and-record on any byte-parity
   mismatch. **NOTE:** check the concurrent `request_hash`/orders workstream state before touching
   `codec.rs`/`routes/orders/*`; those remain the "NOT ours, never commit" set (top of this file).
2. Then 0b-3 (corridors behind `decide`), 0b-4 (Hard Truth L1–L2), 0b-5 (shell-flip, REUSE the
   existing cutover shadow-diff, F1), 0b-6 (CI+cargo-deny, `.github` operator-gated) → Phase 1 hub →
   Phase 2 MVP.
3. Deferred (non-blocking, pick up opportunistically): RATCHET the B1 model-check warn→deny once
   `audit-token-router` shows model-less <10%; B1 LANE-CLASS/router-stamp checks after the stamp convention
   is written into AGENTS.md; KNOWLEDGE-AS-CIRCUITS `require_together` entry once a committed
   agent-dispatching script exists to guard; wire `run-armaments.sh` into THE EYE once THE EYE is applied.
   A3 (orders module split) stays BLOCKED on GRAND-PLAN 0b-5 + 1.3 — do NOT start early. R1 (sub-meter
   delivery-tier divergence, 0b-1 defer-flag) — revisit the moment any tier-author UI spec opens; land
   a `≤3-dp` `max_distance_km` validation/CHECK as that spec's DoD (see
   `docs/design/sovereign-core-money-boundary-0b1/resolution.md` L1). Also non-blocking:
   `pnpm lint:gates` is broken in this environment (`ERR_MODULE_NOT_FOUND: @eslint/js` — declared as
   `eslint: ^9.10.0` in `package.json` but the installed `eslint.config.js` imports `@eslint/js`,
   which isn't a declared dependency at all) — hit independently by 3 separate agents this run; does
   NOT block commits (pre-commit only lints staged JS/TS, none were staged), only spams the
   `post-edit-gates.sh` PostToolUse hook. A JS-tooling fix, out of scope for this Rust-core arc.

## GUARDRAILS (every run)
Enforce `rebuild/scripts/sovereign-gate.sh` + cargo tests. Commit+push ONLY when green, scoped, EXCLUDING the
concurrent work above. **STAGING-ONLY: never prod/secrets/.env/.github/prod-deploy/force-push.** STOP + record
in BLOCKERS below (do NOT guess or ship) on: unresolvable gate-red, byte-parity mismatch, money/RLS/auth
uncertainty, a harness edit risking recoverability, or a genuine product-vision fork. Skip an unresolved
recorded blocker. Keep context lean — delegate fiddly/risky work to fresh workers with an explicit `model:`.

## BLOCKERS (awaiting operator)
- _none currently._ (B1 rollout fork RESOLVED 2026-07-06: operator chose **A · warn-then-ratchet**; B1
  warn-gate shipped + live-verified this run. Ratchet to deny is data-gated on the `_hev` habit trend.)
