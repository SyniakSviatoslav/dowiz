# BLUEPRINT — Item 9: `kernel/src/breaker/` — the deterministic fault-containment circuit breaker (THE PIVOT)

- **Date:** 2026-07-19 · **Tier:** 3 "THE PIVOT" (roadmap §D) · **Status:** BLUEPRINT (planning
  artifact, no code) — implementation-ready; expected to be picked up immediately after item 8.
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §D item 9
  (lines 368–371) + §G.9 proof; `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §2 (the
  structural-gap statement, lines 87–91), §9 item 9 (line 173), §10/P4 (line 192, the
  `Result<Permit, Tripped>` ruling), §19 "illegal states unrepresentable" (line 354);
  **`BLUEPRINT-TELEMETRY-SAFETY-2026-07-19.md` ("Blueprint A" — the full buildable breaker design,
  read in full)**; `docs/audits/hardening/CHECKLIST.md` (the 5-point standard the breaker must
  satisfy as a new algorithmic hot path). Ground truth for every code citation: `/root/dowiz/kernel`
  at HEAD this session.
- **Upstream (must exist first):** item 2's finding (`FileEventStore` wiring — DONE, defect filed),
  Tier-1's FDR (items 4+29 — DONE, `kernel/src/fdr/`). The breaker consumes both.
- **Downstream (blocked behind this — the reason it is the pivot):** item 11 (ARINC-653 scheduler
  code), item 12 (temporal-TMR pilot), item 21 (autonomic gain-scheduling), item 27 (response half),
  item 32 (§16 pilot control-law half). All five need a running breaker to trip into.

---

## 0. Scope / goal (one paragraph)

Build `kernel/src/breaker/` — the fault-containment layer the rest of the synthesis assumes exists
and currently does not (synthesis §2: "there is **no** `kernel/src/breaker/` directory and zero
references to one anywhere in the kernel"). The kernel today can *classify* faults — `CommitError`
is typed "must alarm" (`event_log.rs:274`), the Markov detector and `DriftClass` diagnose
instability — but **nothing receives the alarm and acts**: the FDR of §5 records, a breaker *acts*.
This item builds Blueprint A (`BLUEPRINT-TELEMETRY-SAFETY-2026-07-19.md`) under the §1.5/§10-P4
house standard: breaker states as types where "tripped-but-permitting" is **unconstructible**, one
decision function returning two typed poles (`Result<Permit, Tripped>`, never per-call-site
`if breaker.is_tripped()`), a small FSM reusing `order_machine.rs`'s golden-signature proof
machinery, `CommitError` alarms actually routed in, zero external dependencies. Blueprint A is the
authoritative design; this document is its **space-grade execution binding** — it (a) reconciles
Blueprint A's Phase-1/2 scope against the roadmap's item-9 proof line, (b) makes the §10/P4 typed-
permit shape concrete and load-bearing, (c) pins the 5-point hardening-checklist obligations, and
(d) draws the honest line between "build now" and "Phase-3 red-team / detreduce, deferred".

**Non-goals (explicitly out of item 9):** the `detreduce.rs` batch-invariant reduction layer
(Blueprint A §4 — its own large item, gated on a GPU-serving report seam that does not exist; the
truthfulness/replay-probe signal ships **disarmed**, `truthfulness_fail` masked to 0, until it
lands); the Phase-3 red-team corpus harness (Blueprint A §7 — specified there, built behind
`breaker-testkit`, not part of the item-9 proof); the autonomic graduated-control layer (item 21,
which lands *after* this and routes its most severe responses *through* this breaker).

---

## 1. Verified current state — grounded, not assumed

Re-verified this session against `/root/dowiz/kernel` at HEAD:

- **No breaker exists.** `kernel/src/breaker/` is absent; no `pub mod breaker` in `lib.rs`; no
  `Permit`/`Tripped`/`BreakerState` type anywhere in the kernel. Confirms synthesis §2 verbatim.
- **The alarm has a typed source and no receiver.** `event_log.rs:271–281` defines
  `pub enum CommitError { Rejected(DecideRejected), Store(StoreError) }` — the doc-comment on `:274`
  states the dangerous pole `Store` is "safe to retry / **must alarm**". It is produced by
  `commit_after_decide` (`event_log.rs:372`, `-> Result<(AppendOutcome, Option<T>), CommitError>`)
  and `commit_after_decide_drift_gate` (`event_log.rs:425`). Grep confirms **no consumer routes a
  `CommitError::Store` anywhere it "acts"** — it is returned to the caller and (per §10/P4's second
  finding) "an alarm with no consumer is a pole-collapse by omission." This breaker is that consumer.
- **The red-line classifier the breaker consumes already exists and is tested.**
  `ports/agent/scope.rs`: `Scope::touches_red_line()` (`:247`), backed by `Resource::is_red_line()`
  (`:96`) and `Action::is_red_line()` (`:176`), and the deny-by-default policy
  `RedLinePolicy::DenyByDefault` / `RedLinePolicy::check` (`:257`, `:267`), unit-tested at
  `scope.rs:344–376` (`red_line_classification`, `deny_by_default_rejects_red_line_allows_clean`,
  `allow_list_narrows_precisely`). The breaker's `red_line_class` field **derives from this** at
  `BreakerRecord` construction — it does not invent a red-line policy (Blueprint A §2.2, corrected).
- **The FSM proof machinery to reuse is in `order_machine.rs`.** `assert_transition` (`:139`) with
  its dual-representation `debug_assert!` cross-check (slice vs `FSM_ADJ` bitmask, `:150–156`);
  the const adjacency `FSM_ADJ` (`:208`); the aggregate structural signature `fsm_graph_report()`
  (`:476`) over `FsmGraphReport` (`:443`); the pinned golden fingerprint `FSM_GOLDEN_SIGNATURE`
  (`:513`) and its drift gate `verify_fsm_signature()` (`:543`) returning `FsmSignatureDrift`
  (`:526`); `spectral_radius()` (`:391`); `fold_transitions` (`:167`). This is the machinery
  Blueprint A §0 says to reuse ("golden signature, proven-DAG-or-proven-cycle-set").
- **The FDR the breaker writes into exists (Tier-1, DONE).** `kernel/src/fdr/`:
  `schema.rs:186–190` `pub enum Kind { …, Alarm, PostMortem }` — the breaker's trips are
  `Kind::Alarm` records; `mod.rs` carries the durable A/B segment ring (`FdrRing`, `mod.rs:344`) and
  the emit path. The audit ring in Blueprint A §2.4 **shares this buffer** (synthesis §5: "the
  logger *is* the flight recorder") rather than building a parallel ring — a P2 Correspondence
  requirement, not an option.
- **The conserved-quantity feeds exist.** `token_bucket.rs`: `try_acquire` (`:92`), `available`
  (`:115`), the over-grant invariant test `token_bucket_never_over_grants_under_refill`;
  `noether.rs:28` `step_preserves`; `event_log.rs` monotone actor-seq chain. These back the
  `constraint_violations` signal (Blueprint A §5.1) with no new primitive.
- **`proptest` is already a dev-dependency** (used for the P47 reconciliation invariant at
  `ports/payment.rs`), so the state-machine property tests need no new dependency (Blueprint A §8.3).
- **The `chaos` feature-gate pattern exists** (`chaos = []`, `#[cfg(any(test, feature = "chaos"))]`)
  — the model for `breaker-testkit` (OFF by default, Phase-3 injection symbols absent from
  production).

---

## 2. The load-bearing shape — §10/P4 made concrete (read before the plan)

Blueprint A specifies the *state machine and schemas* fully. What this document adds, because it is
where a fresh implementation will silently break the invariant, is the **typed-permit call-site
discipline** — §10/P4's exact ruling (line 192):

> "The §2 breaker satisfies P4 only if it is **one decision function returning two typed poles** —
> e.g. `Result<Permit, Tripped>` where `Permit` is a value gated operations require by signature —
> never per-call-site `if breaker.is_tripped()` checks."

Two independent unrepresentability properties, both required:

1. **State side (Blueprint A §2.2 / §3):** "tripped-but-permitting" is unconstructible. A `Killed`
   record cannot hold a live `Permit`; the transition table (§3) is the only constructor path.
2. **Call-site side (this document, §10/P4):** an operation that must be gated **takes a `&Permit`
   by signature**, so a call site *cannot forget or invert the check* — omitting the permit is a
   compile error, not a latent bug. `is_tripped()` accessors, if they exist at all, are for
   telemetry/tests only and can never gate a mutation.

Concretely — the breaker's public gate is **one** function:

```
// breaker/mod.rs (shape, not code-to-ship)
pub struct Permit<'b> { _breaker: &'b Breaker, agent: AgentId }   // no public constructor
pub struct Tripped  { pub state: BreakerState, pub cause: TripCause }  // the typed reject pole

impl Breaker {
    /// The ONE gate. Returns a Permit only from Closed/HalfOpen(probe-admitted); Tripped otherwise.
    pub fn admit(&self, agent: AgentId, action_class: RedLineClass) -> Result<Permit<'_>, Tripped>;
}
```

`Permit` has **no public constructor** (like `order_machine`'s constructor-forced `Pending` pole,
§1.5) — the only way to obtain one is `admit()` returning `Ok`. A gated operation's signature is
`fn do_gated(p: &Permit<'_>, …)`. A caller in the `Open`/`Killed` state has no `Permit` to pass, so
the operation is uncallable. This is the difference between "the breaker is checked" (a boolean the
next call site may forget) and "the breaker is unforgeable" (a type the borrow checker enforces).

> **Design note — lifetime vs. token id.** Two encodings of "unforgeable permit" are viable: a
> borrow-scoped `Permit<'b>` (shown; simplest, permit cannot outlive the breaker borrow) or an
> opaque `PermitId` minted per-admission and consumed once (allows the permit to cross an await/
> queue boundary at the cost of a consumption ledger). The kernel is synchronous on the decision
> path (no tokio on the core, `token_bucket.rs:9`), so the borrow-scoped form is the ponytail
> default. Pick the id form **only if** a real caller needs the permit to outlive the admit borrow;
> flagged as a §7 open design choice, not an open *operator* question.

---

## 3. Implementation plan — exact files, types, functions

Layout matches Blueprint A §0 (the `ports/agent/` subtree precedent), minus the two deferred
non-goals (`detreduce.rs`, the Phase-3 corpus wiring). `pub mod breaker;` goes in `lib.rs` next to
`markov`, `noether`, `causal`. Pure `std`, zero new dependencies (breaker math is `f64`; digests
reuse `event_log::sha3_256`; serialization is hand-rolled JSON like `markov_attractor.rs`, so the
zero-dep gate stays GREEN by construction — `cargo tree -e no-dev` unchanged).

| # | File | Contents (exact) |
|---|---|---|
| 1 | `kernel/src/breaker/signal.rs` | `SignalVector` (Blueprint A §2.1) — `window_seq`, `confidence_gap`, `ewma_drift`, `cusum`, `constraint_violations`, `disagreement`, `truthfulness_fail` (**hard-masked to 0** until detreduce lands, §0 non-goal), `weights`. `trip_score(&self) -> f32` = fixed weighted sum (one primitive; three anomaly classes distinguished only by which weights are nonzero). `SignalWeights` fitted, not literal (§thresholds). |
| 2 | `kernel/src/breaker/state.rs` | `pub enum BreakerState { Closed, Open, HalfOpen, Killed }` (4 variants, **no `Ord`** — a "severity ranking" must be unrepresentable, mirroring the routing-enum discipline). `BreakerRecord` (Blueprint A §2.2). The transition table (§3 of Blueprint A) as a single `fn step(rec, sig, thresholds) -> BreakerRecord` — the **one** guard-evaluating function, no per-call-site logic. `TripCause` enum (typed reason: `ScoreExceeded`, `CommitStoreFault`, `ConservedQuantityBreach`, `WindowGap`, `ProbeMismatch`, `VoteMismatch`†). |
| 3 | `kernel/src/breaker/graph.rs` | `BREAKER_ADJ` const adjacency over the 4 states; `breaker_graph_report()` reusing the `order_machine.rs` lens family; `BREAKER_GOLDEN_SIGNATURE` + `verify_breaker_signature()`. **The breaker graph is intentionally CYCLIC** (Closed→Open→HalfOpen→Closed) — unlike the acyclic order FSM its golden signature pins a *nonzero* cyclomatic number and a *proven cycle set*, which is exactly synthesis §2's "proven-DAG-**or**-proven-cycle-set" branch. |
| 4 | `kernel/src/breaker/mod.rs` | `Breaker` struct; the `admit() -> Result<Permit, Tripped>` gate (§2); `Permit` (no public constructor); `tick(sig)` driving `state::step` and emitting one FDR record per transition; `on_commit_error(&CommitError)` — **the alarm receiver** (§4 below). Re-exports. |
| 5 | `kernel/src/breaker/audit.rs` | Hash-chained tamper-evident record (Blueprint A §2.4) — but **writing into the existing `fdr` ring** (§1 finding; not a second ring). `prev_hash`/`seq` chain via `event_log::sha3_256`; `Kind::Alarm` for trips; backpressure (stall, never drop — Blueprint A §9). |
| 6 | `kernel/src/breaker/thresholds.rs` | `Thresholds`, `ThresholdId`, `fit_from_rates() -> Result<…>` (Blueprint A §5.3). **No numeric-literal θ in `state.rs`** — enforced by a grep-style structural test (precedent: `kernel/tests/no_card_data.rs`). A `Breaker` is unconstructible without a valid fitted `ThresholdId` (fail at bootstrap, not at tick). |
| 7 | `kernel/src/breaker/replay.rs` | `GoldenPair` store + probe (Blueprint A §2.3), **armed only behind detreduce's `verified_invariant()`** — since detreduce is a non-goal, this ships with the probe pool empty and `arm_truthfulness` a no-op that writes `AuditKind::Disarm`. Present so the Half-Open probe seam exists; inert until the item-9 follow-on wires detreduce. |
| 8 | `kernel/src/breaker/testkit.rs` | `#[cfg(any(test, feature = "breaker-testkit"))]` Phase-3 harness *interface* (`drive`/`inject`/`snapshot`/`audit_drain`, Blueprint A §7) — the corpus wiring is deferred, but the seam is specified now so it is not retrofitted. |
| — | `kernel/Cargo.toml` | Add `breaker-testkit = []` (chaos-pattern). No runtime dep. |
| — | `docs/audits/hardening/HOT-PATHS.tsv` | New `@ZONE kernel/src/breaker/` rows registering the breaker as an algorithmic hot path (§4) — required before the code can merge (the item-6 gate fails an unregistered hot-path diff). |

† `VoteMismatch` is item 12's temporal-TMR trip cause; the enum reserves it now (zero code cost) so
item 12 adds a variant, not a new mechanism (roadmap §E item 12: `VoteOutcome` non-unanimous → trip).

**The alarm-routing wire (the roadmap's own proof clause "`CommitError` alarms actually route to
it"):** the receiver is `Breaker::on_commit_error`. It is called from the commit path — the narrow,
reviewable edit is at the `CommitError::Store` construction sites in `event_log.rs`
(`commit_after_decide` `:395` `map_err(CommitError::Store)`, and `commit_after_decide_drift_gate`).
The breaker consumes the `Store` pole as a `TripCause::CommitStoreFault` signal (a durable-loss
event is a first-class trip input); the `Rejected` pole is a *correct* Law rejection and is **not**
an alarm (routing it would be the pole-blur `event_log.rs:271–274` explicitly forbids). This wiring
is the single behavioral change to shipped code and gets its own RED→GREEN test (a faulty store →
`CommitError::Store` → breaker observes a `CommitStoreFault` signal; a `decide`-rejected event →
`CommitError::Rejected` → breaker observes **nothing**).

---

## 4. Tests / proofs — the 5-point hardening standard applied (`CHECKLIST.md`)

The breaker is a **new algorithmic hot path** ("scheduler math, GCRA arithmetic, graph algorithms
all qualify" — CHECKLIST §"designation rule"), so it must register in `HOT-PATHS.tsv` and satisfy
the four checklist items, each with the honest applicability ruling:

| # | Item | Applies? | Design |
|---|---|---|---|
| 1 | **Oracle (exhaustive where enumerable)** | **YES — exhaustive.** | The transition table is `{4 states} × {guard predicates}` — enumerable exactly like the 12-state FSM. A `#[test]` sweeps every `(BreakerState, guard-truth-assignment)` pair and asserts the produced `to`-state matches the Blueprint A §3 table row-for-row (no `(from, guard) → to` diverges). This is the FSM idiom (`order_machine.rs` `for i in 0..12`), not sampling. Plus the graph golden-signature pin (`verify_breaker_signature()`) — a structural change to the breaker graph goes RED. |
| 2 | **dudect (secret-dependent timing)** | **N/A — no secret-dependent branch.** Record `N/A(no-secret-compare)`. The breaker's decisions branch on `trip_score`/counters (public safety state), not on secrets. The one hash surface — the audit-ring `prev_hash` chain — reuses `event_log::sha3_256`, already covered by its own row; the breaker adds no new constant-time-required comparator. If item 12's vote or a future signature-gated admission adds a secret compare, *that* addition triggers the dudect row (manifest note: "add a dudect row if a secret-dependent comparator is introduced"). |
| 3 | **debug_assert differential cross-check** | **YES.** Dual-representation, exactly the `assert_transition` pattern (`order_machine.rs:150–156`): `state::step` computes the next state from the guard table; a `debug_assert_eq!` recomputes it from an independent second encoding (`BREAKER_ADJ` reachability) and asserts agreement per tick. Compiled out of release; continuous verification at zero production cost. |
| 4 | **asm / binary spot-check on compiler bump** | **N/A — no branch-free constant-time path.** Record `N/A(no-branchfree-path)`. The breaker has no `arXiv:2410.13489`-class branch-free crypto path; item 14's toolchain-bump gate governs the crypto surfaces, not this. |
| 5 | **Kani / formal (arithmetic edge + panic-freedom)** | **YES — narrow, and there is a real bug class here.** Two obligations: (a) **cooldown-doubling overflow** — `cooldown_ticks = min(cooldown_ticks * 2, cap)` where `cooldown_ticks: u32` **overflows before the `min` clamps** (`*2` on a large `u32` wraps in release / panics in debug); the fix is `checked_mul(2).map_or(cap, \|v\| v.min(cap))` and the proof is that no reachable `cooldown_ticks` value overflows — native-exhaustive over `u32` (minutes-scale, the `reduce32` release-row precedent from item 7) **or** a `#[cfg(kani)]` harness `proof_cooldown_doubling_no_overflow`; (b) **`trip_score` weighted-sum** panic/NaN-freedom under the fitted weights. Executor picks native-exhaustive (preferred, zero new tooling — item 7's ruling) unless a Kani harness reads cleaner; either satisfies checklist item 4's arithmetic half. |

**Additional safety-property tests (Blueprint A §10, non-negotiable, RED-first):**
- `state.rs`: a **red-line-classed `Killed` record never transitions except via the explicit manual-
  reset row** (RED-first against a deliberately-wrong "flip state back to Closed" impl — Blueprint A
  §3's called-out most-likely mistake).
- `replay.rs`: a `GoldenPair` with `invariant_verified == false` is **excluded from the probe pool**
  (RED-first against a "gate on any pair" impl) — the one place the test is a safety property, not a
  data-flow check.
- `mod.rs`: the **typed-permit unforgeability** — a compile-fail test (`trybuild`-style *or* a
  documented "there is no public `Permit::new`" structural assertion) proving a gated op cannot be
  called without an `Ok(Permit)`. This is the §10/P4 call-site half made a test.
- `audit.rs`: hash-chain break detection (`prev_hash` mismatch / `seq` gap caught) + ring-capacity
  wraparound asserting `tick()` **stalls** rather than silently drops (Blueprint A §9 backpressure).

---

## 5. Acceptance criteria (falsifiable — what "done" looks like)

Straight from the roadmap §G.9 proof line, made concrete:

1. **`kernel/src/breaker/` exists** and `pub mod breaker;` is in `lib.rs`; the default build compiles
   it (pure-std, no feature gate on the core path); `cargo tree -e no-dev` is **unchanged** (zero-dep
   gate GREEN).
2. **FSM golden-signature-pinned.** `verify_breaker_signature()` GREEN against
   `BREAKER_GOLDEN_SIGNATURE`; a deliberately-mutated `BREAKER_ADJ` (add/remove one edge) makes it
   RED (the drift gate has a proven RED path, mirroring `verify_fsm_signature`).
3. **Adversarial trip tests green.** The transition-table property test passes; the four safety-
   property tests (§4) pass RED-first-then-GREEN; the cooldown-overflow proof (native-exhaustive or
   Kani) is GREEN and demonstrably RED on the un-clamped `*2`.
4. **`CommitError` alarms actually route to it.** The RED→GREEN wire test: a faulty store yields
   `CommitError::Store` → the breaker records a `TripCause::CommitStoreFault` FDR `Kind::Alarm`; a
   `decide`-rejected event yields `CommitError::Rejected` → the breaker records **nothing** (pole not
   blurred). Grep shows the `Store` pole has exactly one receiver (the breaker), no second consumer.
5. **Typed-permit unforgeability.** No public `Permit` constructor exists; a gated operation is
   uncallable without an `Ok(Permit)` from `admit()` (structural/compile-fail test).
6. **HOT-PATHS.tsv registered** with the four per-item verdicts of §4 (`oracle`, `N/A(no-secret-
   compare)`, `debug-differential`, `N/A(no-branchfree-path)`, `overflow-proof`); the item-6
   `hardening-gate` re-executes the breaker's oracle test with a `min_tests` floor (a deleted oracle
   goes RED).
7. **The two deferrals are honestly ledgered, not silently dropped:** `truthfulness_fail` masked-to-0
   with an `AuditKind::Disarm` path present; the Phase-3 testkit seam present but corpus-unwired;
   both named in the manifest gap column as `DEFERRED(detreduce)` / `DEFERRED(phase-3)`.

---

## 6. Dependency gates

- **Must land first:** item 2's finding (DONE — the durable-store defect is filed; the breaker
  consumes `CommitError` regardless of whether a durable store is wired, because the alarm is a
  *type*, not a runtime store); Tier-1 FDR (DONE — `kernel/src/fdr/` with `Kind::Alarm`); the item-6
  hardening gate (DONE — the breaker must register hot-path rows to merge). Best entered after item
  2's finding and Tier-1's FDR (roadmap §D item 9) — both satisfied.
- **Does NOT depend on:** item 8 (GCRA swap) — independent, though item 8 lands just before per the
  dispatch sequence; item 10 (TLA+) runs in parallel (same-tier, no structural link).
- **Blocks (the pivot):** items 11, 12, 21, 27(response), 32(control-law) all require a running
  breaker to trip into. This is why item 9 gets built next and gets this depth.

---

## 7. Open questions (operator ruling genuinely needed — flagged, not invented)

1. **Permit encoding: borrow-scoped vs. minted-id (§2 design note).** Resolvable by the executor from
   the call-site shape (borrow-scoped is the ponytail default for a synchronous decision path);
   flagged as an *engineering* choice, **not** requiring an operator ruling unless a caller needs the
   permit to cross an async/queue boundary. → Executor decides; no operator gate.
2. **Manual-reset authority for a `Killed` red-line record (Blueprint A §3, last table row).** The
   red-line `Killed` state "resumes only via explicit human gate" and "never self-resumes." *Who*
   constitutes the human gate in the dowiz deployment — an operator-signed capability
   (`ports/agent/cap.rs`), a specific FDR-witnessed event, or an out-of-band console — is a **policy
   decision only the operator can rule on**. The breaker builds the *mechanism* (a `Killed(red_line)`
   record cannot transition except through one explicit `manual_reset(proof)` entry point that
   provisions a fresh `BreakerRecord` for a newly-issued instance — never resurrecting the compromised
   one); the *proof* type it demands is the open ruling. **Flagged for human decision; do not invent
   an answer.** Until ruled, the mechanism takes an opaque `ManualResetProof` that only a test
   constructor can build, so production cannot self-reset a red-line kill.
3. **Detreduce sequencing (non-goal boundary).** The truthfulness/replay-probe signal is disarmed in
   item 9. Whether the follow-on `detreduce.rs` + GPU-serving-report seam is scheduled as "item 9b" or
   a separate roadmap item is an operator sequencing call — noted so the disarmed signal is not
   mistaken for a permanent scope cut.

---

## 8. Scope verdict

Item 9 delivers the fault-containment layer the entire synthesis presupposes: a pure-std, zero-dep,
golden-signature-pinned 4-state breaker whose gate returns `Result<Permit, Tripped>` with an
unforgeable permit (state-side *and* call-site-side unrepresentability of "tripped-but-permitting"),
the `CommitError::Store` alarm finally routed to a consumer, the audit trail sharing the Tier-1 FDR
ring, and the two large sub-systems it does **not** own (detreduce, Phase-3 corpus) honestly
deferred with their seams present. It registers as a hot path under the item-6 checklist with an
honest per-item applicability ruling (oracle + debug-differential + overflow-proof YES; dudect +
asm-check N/A-with-reason). Building it unblocks items 11, 12, 21, 27, and 32 — which is precisely
why it is the pivot.
