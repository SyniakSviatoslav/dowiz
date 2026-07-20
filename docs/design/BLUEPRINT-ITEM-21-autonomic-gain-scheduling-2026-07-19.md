# BLUEPRINT — Item 21: Autonomic gain-scheduling module (deterministic bounded control, NOT ML)

- **Date:** 2026-07-19 · **Tier:** 4 (roadmap §E) · **Status:** BLUEPRINT (planning artifact, no code)
  — **gated strictly after item 9's breaker** (roadmap §E item 21, synthesis §16(c): "Sequencing is
  strict: this lands **after** §9.9's breaker exists").
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §E item 21
  (line 377); `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §16 (lines 279–285, the
  deterministic-autonomic-layer design), §9 addendum item 21 (line 294), §10/P3 (Vibration/CFL bound),
  §19 (Foster-Lyapunov = the stability-bound shape); live source `kernel/src/markov.rs` (Verdict),
  `kernel/src/spectral.rs` (DriftClass/classify_drift), `kernel/src/token_bucket.rs` (the pilot
  constant), `kernel/src/fdr/` (FDR events).
- **Relationship to item 9:** the autonomic layer is the **continuous, graduated** tuning layer; the
  breaker is the **binary emergency net**. They compose — "the autonomic layer's most severe responses
  should very plausibly route *through* the breaker rather than acting unilaterally at the extreme
  end" (synthesis §16(c)). Requires a running breaker to route into.

---

## 1. Scope / goal (one paragraph)

Build a small **deterministic bounded-adaptive-control** module — gain-scheduling in the classical
control-theory sense, **explicitly not machine learning** (synthesis §16(b): learned weights are the
non-reproducible cause→effect mapping §10/P6 outlaws, before the dependency question even arises).
The kernel's *sensing* is excellent and its *action* is absent (synthesis §16(a): grep for
`self_tun`/`self_optim`/`autonomous`/`self_heal` returns zero hits — genuinely absent). This item
closes that: a module that (1) subscribes to the already-classified drift outputs (`markov::Verdict`,
`spectral::DriftClass`), (2) holds a small, explicit, **auditable table** of {classified-state →
bounded adjustment} laws — one per tunable constant, (3) applies adjustments only through types that
make out-of-bound values **unconstructible**, (4) writes every adjustment to the Tier-1 FDR as a
first-class event, and (5) is covered by the item-6 hardening checklist from day one. Three properties
are non-negotiable (synthesis §16(b)): every adjustment stays within a **pre-proven stability bound**
(CFL-style, the same bound §10/P3 requires before any rate reaches an integrator) and a
bound-leaving adjustment is *inexpressible* not merely avoided; every adjustment is FDR-logged; every
control law is a checkable equation, never learned. The **pilot** is one constant: `token_bucket`'s
refill rate (synthesis §9 item 21).

---

## 2. Verified current state — grounded

- **The classification inputs exist and are typed.** `markov.rs:42` `pub enum Verdict { Healthy,
  LimitCycle, StrangeAttractor }` (Copy+Eq, fieldless), produced by `analyze_detailed`
  (`markov.rs:110`). `spectral.rs:674` `pub enum DriftClass { Damped, Resonant, Unstable }` (Copy+Eq),
  produced by `classify_drift` (`spectral.rs:704`), fail-closed to `Unstable` on any non-finite input.
  These are the "already-classified drift state" the control laws map from — the autonomic layer adds
  no new classifier (P2 Correspondence: one classification mechanism, not two).
- **The pilot tunable — and the exact gap that must be filled.** `token_bucket.rs`: `TokenBucket`
  (`:36`) with `refill_rate: f64` (`:38`) as a **private, immutable field with NO setter** — verified
  this session (grep for `set_refill`/`set_rate`/`with_rate`/`&mut self.*refill` → **zero matches**).
  So the autonomic layer **cannot adjust the refill rate today** — there is no API. This is the
  load-bearing finding: item 21 must add a **bounded rate-adjustment API** to `TokenBucket`, and it is
  the exact place the "out-of-bound is unconstructible" property lives (§3.2). The over-grant invariant
  test `token_bucket_never_over_grants_under_refill` must survive any new setter.
- **The FDR event target exists** (Tier-1 DONE — `fdr/schema.rs`, the emit path in `fdr/mod.rs`). Each
  adjustment is a first-class FDR event; a self-tuning system that does not log its own tuning is
  unauditable (synthesis §16(b)(ii)).
- **The breaker (the route for severe responses) does not exist yet** — item 9 builds it. The
  autonomic layer's extreme-end responses route *through* the breaker's `admit`/trip path, not
  unilaterally (synthesis §16(c)).
- **The stability-bound math is already in the kernel's own vocabulary.** `markov.rs` computes
  Foster-Lyapunov drift (`potential`, the SLEM contraction ratio); synthesis §19: "a control law that
  must keep a system's potential decreasing toward stability *is* a Lyapunov-function argument" —
  grounded in `markov.rs`, not sourced from Batch 5's empty sections (synthesis §8 boundary respected).
  `slem` gives the empirical contraction ratio the bound is stated against.

---

## 3. Implementation plan — exact files, types, functions

Placement is an **open design choice** per synthesis §16(c): a new `kernel/src/autonomic.rs`, or
folded into the breaker's own module. **Recommendation: a separate `kernel/src/autonomic.rs`** — the
breaker is the emergency net and the autonomic layer is the graduated controller; keeping them
separate keeps each single-purpose (they compose, they do not replace each other — §16(c)), and lets
the autonomic layer be built/tested against a *stable* breaker interface rather than co-evolving.

1. **`kernel/src/autonomic.rs`** — the module. Contents:
   - `struct BoundedRate` — a newtype over `f64` with a **private constructor** that clamps to a
     pre-proven `[min, max]` stability interval; **there is no way to construct an out-of-bound
     `BoundedRate`** (the §16(b)(i) "inexpressible, not merely avoided" property). Every adjustment
     produces a `BoundedRate`, never a raw `f64`.
   - `const LAW_TABLE: [(DriftClass, Verdict, Adjustment); N]` — the small, explicit
     {classified-state → bounded adjustment} table, **one row per tunable** (pilot: the refill rate).
     Each `Adjustment` is a checkable equation (e.g. `Damped → rate *= 1.0` no-op, `Resonant → rate *=
     0.9` back off, `Unstable → rate *= 0.5` + route-to-breaker) — never a learned weight. **No numeric
     literal outside the const table** (structural test, `no_card_data.rs` precedent).
   - `fn schedule(class: DriftClass, verdict: Verdict, current: BoundedRate) -> (BoundedRate,
     FdrAdjustment)` — the pure control law: table lookup → bounded adjustment → the FDR event to emit.
     Pure (P6 determinism: a replayed classification sequence reproduces the identical adjustment
     sequence).
   - The **route-through-breaker** seam: an adjustment classified at the extreme end (`Unstable` +
     `StrangeAttractor`) does not apply unilaterally — it emits a `TripCause`-adjacent signal into the
     item-9 breaker's `tick` path instead of writing the rate directly (§16(c)).
2. **`kernel/src/token_bucket.rs`** — add the bounded setter:
   `fn set_refill_rate(&self, r: BoundedRate)` — takes a `BoundedRate` **by type** (a raw `f64`
   cannot be passed), holding the mutex to keep the coupled `(tokens, last_refill)` over-grant
   invariant intact. This is the single edit to shipped `token_bucket.rs`; the over-grant test must
   stay GREEN across it (and gains a new case: an autonomic rate change never over-grants).
3. **`docs/audits/hardening/HOT-PATHS.tsv`** — a `@ZONE kernel/src/autonomic.rs` row (control-law math
   is an algorithmic hot path) + a bump on the `token_bucket` row for the new setter.
4. **`lib.rs`** — `pub mod autonomic;` next to `markov`, `breaker`.

Pure `std`, zero new dependency; `cargo tree -e no-dev` unchanged.

---

## 4. Tests / proofs — 5-point hardening applicability

Control-law adjustment is "precisely the algorithmic hot path that needs an oracle/differential test"
(synthesis §16(c)). The 5-point standard:

- **Item 1 (oracle):** **YES — exhaustive.** The law table is `{DriftClass=3} × {Verdict=3}` = 9
  input combinations — enumerable. A `#[test]` sweeps all 9, asserting `schedule` produces the exact
  table-defined `BoundedRate` and FDR event. **Headline property test (synthesis §9 item 21): no
  sequence of adjustments, under any classification sequence, can push a rate outside its
  proven-stable bound** — a proptest over random `(DriftClass, Verdict)` sequences asserting the
  result is always a valid `BoundedRate` (which is *structurally* guaranteed by the newtype, so the
  proptest is a belt-and-suspenders confirmation of the type-level property).
- **Item 3 (debug-differential):** `debug_assert!` that every `schedule` output rate is within
  `[min, max]` (cross-checking the newtype's clamp against an independent bound check) per call.
- **Item 5 (formal / falsifiability):** **a deliberately bound-violating law fails to construct or
  fails CI** (synthesis §9 item 21) — a test that attempts to add a `LAW_TABLE` row producing an
  out-of-bound rate must either not compile (the `BoundedRate` constructor rejects it) or be caught by
  the exhaustive oracle. This is the item-21 analog of the planted-fault self-test: an unsafe law is
  demonstrably rejected. Native-exhaustive suffices (9-combo space); no Kani needed.
- **Item 2 (dudect):** **N/A** — control-law arithmetic branches on public drift class, not secrets.
  Record `N/A(no-secret-compare)`.
- **Item 4 (asm):** **N/A** — no branch-free constant-time path.

---

## 5. Acceptance criteria (falsifiable) — synthesis §9 item 21

1. A **property test shows no classification sequence can drive the rate outside its pre-proven stable
   bound** (structurally, via `BoundedRate`, + confirmed by proptest).
2. **A deliberately bound-violating law fails to construct or fails CI** (the falsifiability proof).
3. **A replayed classification sequence reproduces the identical adjustment sequence** (P6 determinism)
   — same `(DriftClass, Verdict)` stream in → byte-identical `BoundedRate`/FDR sequence out.
4. **Each adjustment is present in the FDR** as a first-class event (a run with N adjustments produces
   N FDR records; a self-tuning event with no FDR entry is a defect).
5. **The pilot constant is `token_bucket` refill rate**, adjusted only through the `BoundedRate`-typed
   setter; the over-grant invariant test stays GREEN.
6. **Extreme responses route through the breaker** (item 9), not unilaterally (`Unstable +
   StrangeAttractor` → breaker path, demonstrated).
7. Zero new dependency; `cargo tree` unchanged; `HOT-PATHS.tsv` registered.

---

## 6. Dependency gates

- **Gated strictly after item 9** (roadmap §E item 21, synthesis §16(c)) — the extreme-response route
  is the breaker's `tick`/trip path; without a breaker the "route through, don't act unilaterally"
  property is unrealizable. **Hard gate, explicitly stated in the source, never resequenced** (roadmap
  §top: "item 21 strictly after item 9 (§16(c))" is one of the two verbatim-preserved dependencies).
- **Consumes (already exist):** `markov::Verdict`, `spectral::DriftClass`, the Tier-1 FDR.
- **Overlaps item 11:** the bounded rate-reconfiguration API this item adds to `TokenBucket` is also
  what a Phase-1 ARINC-653 scheduler (item 11) needs to reconfigure slice budgets — build it once
  here, reuse there (noted so it is not duplicated).
- **Feeds item 27 (response half):** item 27's PMU-informed autonomic responses route through *this*
  bounded-control-law path (synthesis §18(c), item 27 response is "after item 21").

---

## 7. Open questions (operator ruling)

1. **Placement: `autonomic.rs` vs folded-into-breaker (synthesis §16(c), explicitly "an open design
   choice, PROPOSED, not decided").** Recommendation is a separate module (§3 reasoning); this is an
   **executor** judgment, not an operator gate — flagged because the source explicitly left it open,
   but it does not need a human ruling.
2. **The pre-proven stability bound `[min, max]` for the refill rate.** The bound is a CFL-style
   stability limit (synthesis §10/P3) that must be *proven* before it reaches the integrator — its
   exact numeric interval depends on the token bucket's role in the specific rate-limited path and
   should be derived (Lyapunov/SLEM argument) and pinned as a named constant with a proof, not
   guessed. Whether that derivation is in-scope for item 21 or a prerequisite research task is a
   sequencing call — recommendation: derive it as part of item 21 (it is the load-bearing constant),
   flagged so the executor budgets for the derivation rather than picking a literal. Not an operator
   gate, but a real work item that must not be skipped.
3. **Which tunables beyond the refill-rate pilot.** Synthesis §16(b) also names retry/backoff cadence.
   The pilot is deliberately one constant; expanding to more is a post-pilot scope call. Named.
