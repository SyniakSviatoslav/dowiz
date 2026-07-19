# Blueprint — Deterministic-Safety-First Swarm Circuit Breaker (Phase 1/2, buildable)

> Status: buildable engineering blueprint. This is the FIRST work item of the swarm-safety
> system per operator directive — plan + implement this before any other swarm component.
> Companions (research, do not duplicate): `SWARM-SAFETY-DETERMINISTIC-CIRCUIT-BREAKER-SYNTHESIS-2026-07-19.md`
> (Synthesis I) and `SWARM-SAFETY-SYNTHESIS-2-truthfulness-time-metric-2026-07-19.md` (Synthesis II).
> House honesty rule (kept from both): **GROUNDED** = cited or verified-in-tree; **PROPOSED** =
> designed but unmeasured; **ANALOGY** = reasonable, flagged. No claim is inflated past its evidence.

## 0. Placement summary (consistent with the existing kernel layout)

The breaker is a new subtree matching the `ports/agent/` precedent (subtree of cohesive `.rs`
files behind one `pub mod`), plus one core numerics module and one CLI bin matching
`kernel/src/bin/markov_attractor.rs`.

```
kernel/src/breaker/mod.rs        Breaker struct, tick(), public API, re-exports
kernel/src/breaker/signal.rs     SignalVector + the five extractors
kernel/src/breaker/state.rs      BreakerState enum + the transition table (§3)
kernel/src/breaker/replay.rs     GoldenStore, ReplayProbe, bitwise compare (truthfulness)
kernel/src/breaker/audit.rs      AuditRing — hash-chained fixed-capacity ring buffer
kernel/src/breaker/thresholds.rs Thresholds + fit_from_rates() (no magic constants)
kernel/src/breaker/testkit.rs    #[cfg(any(test, feature="breaker-testkit"))] Phase-3 harness
kernel/src/detreduce.rs          batch-invariant reductions + DeterminismLedger (HARD PREREQ, §4)
kernel/src/bin/breaker_replay.rs Phase-3 attack-corpus replay runner CLI
```

`pub mod breaker;` and `pub mod detreduce;` go in `lib.rs` next to `markov`, `noether`, `causal`.
Discipline preserved: **pure `std`, zero new dependencies** (the breaker's math is `f64`; digests
reuse `event_log::sha3_256`; serialization is hand-rolled JSON like `markov_attractor.rs`, so
`cargo tree -p dowiz-kernel -e no-dev | grep -c serde` stays `0`). No feature flag needed for the
default path; only `breaker-testkit` gates the Phase-3 driver so production binaries carry no
attack-injection symbols (same discipline as `chaos`). **[GROUNDED — layout verified against
`kernel/src/lib.rs`, `ports/agent/`, `bin/markov_attractor.rs`.]**

## 1. Truthfulness — the criterion the breaker is built around

🧭 **OPERATOR VISION.** The operator coined **truthfulness** as the replacement for the ML term
"faithfulness." Definition (verbatim intent): *byte-for-byte identical output given byte-for-byte
identical input and conditions, checked at two points in time; any divergence is a poisoning /
hallucination signal.* This is a determinism criterion, not a semantic-similarity score — which is
exactly why it is checkable in a deterministic kernel. It enters the breaker as one signal class:
**replay probes** in the Half-Open state (§3, §2.3). Synthesis II's search found no prior work
using byte-identical replay this way — the *inverse* of SelfCheckGPT's temp=1 variance-mining
assumption (Synthesis II §2), which expects and mines variance rather than forbidding it.
**[GROUNDED as a definition; detector efficacy is PROPOSED and gated on §4; novelty claim is
absence-of-evidence, not proof of absence.]**

## 2. Data schemas (Rust structs — the buildable form)

All fixed-width, `Copy` where possible, no heap on the hot path.

### 2.1 Signal vector (`signal.rs`) — one per agent per window
```rust
pub struct SignalVector {
    pub window_seq: u64,          // monotone window index (gap ⇒ dropped window ⇒ audited)
    pub confidence_gap: f32,      // 1 − p(top token), aggregated from logprobs; NaN if unavailable
    pub ewma_drift: f32,          // d_t = ‖a_t − μ_{t-1}‖ (Hooke linear regime, §5)
    pub cusum: f32,               // S_t = max(0, S_{t-1} + (x_t − μ0 − k)); trip when S_t > h
    pub constraint_violations: u16, // writes rejected by the conserved-quantity + causal gate (§5)
    pub disagreement: f32,        // shadow-execution pair distance (0 = identical)
    pub truthfulness_fail: u8,    // replay-probe bitwise mismatches; MASKED to 0 while disarmed (§4)
    pub weights: SignalWeights,   // fitted, not hand-tuned (§5.3, thresholds.rs)
}
```
`trip_score(&self) -> f32` is a fixed weighted sum of the normalized components; **one primitive,
three anomaly classes** (poisoning / hallucination / injection) are distinguished only by which
components carry weight, never by a bespoke code path. **[GROUNDED design — matches Synthesis I §1.3.]**

### 2.2 Breaker state record (`state.rs`)
```rust
pub struct BreakerRecord {
    pub agent_id: [u8; 16],
    pub state: BreakerState,      // Closed | Open | HalfOpen | Killed
    pub entered_at_seq: u64,
    pub consecutive_trips: u16,   // the W counter driving Closed→Open
    pub kill_window_count: u16,   // W_kill counter driving Open→Killed
    pub cooldown_ticks: u32,      // hysteresis timer; doubles on HalfOpen failure
    pub cooldown_cap: u32,        // the doubling cap
    pub probes_remaining: u8,     // N canary probes left in HalfOpen
    pub red_line_class: bool,     // action touches money/auth/RLS/migrations-equivalent
    pub human_gate_required: bool, // set on red-line Killed (§3); breaker never clears this itself
    pub last_score: f32,
    pub thresholds: ThresholdId,  // reference to the fitted threshold set (§5.3)
}
```
`red_line_class` is **not** a policy this blueprint invents: derive it from the already-built,
already-tested classifier at `kernel/src/ports/agent/scope.rs` — `Scope::touches_red_line()`,
backed by `Resource::is_red_line()`/`Action::is_red_line()`, unit-tested at `scope.rs:344-349`
(`red_line_classification`, `deny_by_default_rejects_red_line_allows_clean`). The breaker consumes
this classification at `BreakerRecord` creation time; it does not define red-line membership.
**[GROUNDED — verified in-tree, corrects an undefined-assumption gap in the original draft.]**

### 2.3 Golden replay-probe store (`replay.rs`)
```rust
pub struct GoldenPair {
    pub key: [u8; 32],            // sha3(input_bytes ‖ condition_bytes)
    pub input_digest: [u8; 32],
    pub output_digest: [u8; 32],  // the bitwise match target
    pub captured_at_seq: u64,
    pub invariant_verified: bool, // was detreduce verified when this pair was captured (§4)
}
```
A probe **passes iff** `sha3(fresh_output) == golden.output_digest` — bitwise, no tolerance. Pairs
captured while `invariant_verified == false` are quarantined and never used to gate a close.

### 2.4 Audit ring buffer (`audit.rs`) — hash-chained, tamper-evident
```rust
pub struct AuditEvent {
    pub seq: u64,                 // monotone; a gap is tamper/loss evidence
    pub prev_hash: [u8; 32],      // hash-chain (Synthesis I project #1 precedent)
    pub ts_millis: u64,
    pub agent_id: [u8; 16],
    pub kind: AuditKind,          // Signal|Transition|ProbeResult|Disarm|Kill|RedLineGate
    pub signal: SignalVector,
    pub state_from: BreakerState,
    pub state_to: BreakerState,
    pub self_hash: [u8; 32],      // sha3(prev_hash ‖ seq ‖ body)
}
```
Fixed-capacity ring (default 8192 entries, const). A drain sink (`tools/telemetry/lib.sh`
`tg_deliver` / `log_event`) must consume each entry **before** it is overwritten — logs ship
**unfiltered regardless of the trip's eventual verdict** (Synthesis I §1.4). The `prev_hash`/`seq`
chain makes deletion or reorder detectable. **[GROUNDED pattern — reuses `event_log.rs` sha3
hash-chaining; drain reuses the verified telemetry primitives.]**

## 3. State machine (transition table, not prose)

`W`, `W_kill`, `N`, `θ_open`, `θ_kill`, cooldown base/cap are all fitted (§5.3), never literals.

| From      | Guard (evaluated every `tick`)                                             | To            | Side effects |
|-----------|---------------------------------------------------------------------------|---------------|--------------|
| Closed    | `trip_score > θ_open` for `W` **consecutive** windows                      | Open          | quarantine → read-only; snapshot golden set; `cooldown_ticks = base`; audit `Transition` |
| Closed    | `trip_score ≤ θ_open`                                                      | Closed        | `consecutive_trips = 0`; audit `Signal` |
| Open      | `cooldown_ticks` elapsed                                                   | HalfOpen      | load `N` replay probes; `probes_remaining = N` |
| Open      | `trip_score > θ_kill` for `W_kill` windows **AND** `!red_line_class`       | Killed        | `human_gate_required = false`; irreversible for **this instance**; ship full audit ring |
| Open      | `trip_score > θ_kill` for `W_kill` windows **AND** `red_line_class`        | Killed        | `human_gate_required = true`; irreversible for this instance; never self-resumes |
| HalfOpen  | all `N` probes bitwise-match **AND** `trip_score ≤ θ_open`                 | Closed        | reset counters; `cooldown_ticks = base` |
| HalfOpen  | any probe mismatch **OR** `trip_score > θ_open`                            | Open          | `cooldown_ticks = min(cooldown_ticks*2, cap)`; `consecutive_trips += 1` |
| HalfOpen  | `consecutive_trips ≥ W_kill` (same threshold as Open→Killed, reused — not a separate constant) | Killed | as Open→Killed rows above, honoring `red_line_class` |
| Killed    | `red_line_class` (money/auth/RLS/migrations)                              | Killed        | **terminal**; resume only via explicit human gate |
| Killed    | `!red_line_class` **AND** operator reset event                            | Closed        | manual only; **provisions a fresh `BreakerRecord` for a newly issued agent instance** (this record's compromised instance is never itself resumed — see note below); audited `Transition` |

**Note on "irreversible for this instance" vs. the Killed→Closed row above:** these do not
contradict. The `Killed` state itself never flips back to `Closed` in place; the manual-reset
transition provisions a **new** `BreakerRecord` (fresh `entered_at_seq`, zeroed counters) for a
freshly issued agent instance, matching Synthesis I §1.4's "Steward reissues a fresh capability
token to a new instance." A naive implementation that resurrects the *same* compromised record by
flipping its `state` field back to `Closed` would violate the invariant this table exists to
enforce — this is called out explicitly because it is the mistake a fresh implementation is most
likely to make. **[CORRECTED — the original table's `Killed(gated)` variant and undefined
`kill_threshold` were inconsistent with the four-variant `BreakerState` enum in §0/§2.2; both are
resolved above by reusing existing fields (`human_gate_required`, `W_kill`) rather than adding new
ones.]**

The **only** place the breaker does not self-close **in place** is the red-line-classed `Killed`
state (Synthesis I §1.4). Hysteresis (cooldown doubling, capped) prevents flapping. **[GROUNDED design.]**

## 4. HARD PREREQUISITE — batch-invariant inference, in-kernel (`detreduce.rs`)

**This is work item #1 and gates every truthfulness claim.** LLM nondeterminism at `temp=0` is
caused by the serving engine's dynamic/continuous batching changing reduction-kernel summation
order under floating-point non-associativity (**not** GPU atomics), plus MoE router-logit drift.
Thinking Machines Lab's `batch_invariant_ops` (Sept 2025) fixes it — RMSNorm/matmul/attention made
invariant to batch composition, measured **1000/1000 bitwise-identical** completions vs **80/1000
unique** at baseline, at ~1.6–2.1x unoptimized / ~34% overhead with SGLang CUDA-graph integration.
Without it, ~92% of runs diverge from serving noise alone with zero poisoning present, so the
truthfulness signal's false-positive floor is ~1.0 and it is worthless. **[GROUNDED — TML Sept
2025; Synthesis II §2.]**

🧭 **OPERATOR VISION — batch-invariant work lives inside the Rust kernel.** Operator directive
(this blueprint's binding constraint): the batch-invariant *computation* AND
its *measurement/verification* live natively in the Rust kernel — alongside `order_machine.rs`,
`markov_attractor.rs`, `simd.rs`, `spectral.rs` — **not** a Python wrapper or a bolt-on service.
Module boundary:

- **Reference reductions.** `detreduce::DeterministicReducer` provides `rmsnorm`, `matmul_rowsum`,
  and `attention_reduce` with a **fixed reduction tree / fixed split-size** independent of the batch
  dimension (the CPU-side analog of `batch_invariant_ops`; same discipline as the existing
  fixed-order numerics in `spectral.rs`/`householder.rs`). This is the authority on what the
  *correct* batch-invariant sum is.
- **Metric authority.** `detreduce::DeterminismLedger` ingests `(batch_composition_id,
  completion_digest)` where `completion_digest = event_log::sha3_256(bytes)`, and computes
  `unique_completion_count` and `divergence_rate` over a run of `N`. `verified_invariant() -> bool`
  is `divergence_rate == 0.0` over the last `N = 1000`.
- **Interface to the breaker.** `Breaker::arm_truthfulness(&DeterminismLedger)`. While
  `verified_invariant()` is false, `SignalVector::truthfulness_fail` is **masked to 0** (the replay
  signal is disarmed, fail-open on that one component) and an `AuditKind::Disarm` row is written.
  Replay probes may only gate a Half-Open→Closed transition once the ledger is verified.
- **The external edge (honest boundary).** The kernel owns the reference reductions + verifier +
  metric. Wiring the *production GPU serving replica* (SGLang deterministic mode) to report
  per-completion digests into the ledger is the integration seam. **[In-kernel reference + ledger:
  buildable now. GPU-serving report path: PROPOSED — external, one seam.]**

## 5. Physics-derived design rules (concrete sections, not decoration)

### 5.1 Kepler — check the conserved invariant, don't re-simulate
**[CORRECTED — live-code check 2026-07-19: `TokenBucket` has no `release` method.]** The type only
grants-or-refuses (`try_acquire`) and reports (`available`); the conserved quantity is the
over-grant ceiling `try_acquire` enforces, verified by the in-tree test
`token_bucket_never_over_grants_under_refill` (`token_bucket.rs`). Enumerated conserved quantities,
each a `noether::step_preserves`-style check (confirmed present, `noether.rs:28`) whose violation
`+=` into `constraint_violations`:

| Conserved quantity | Where it lives | Violation ⇒ |
|--------------------|----------------|-------------|
| TokenBucket permits (over-grant ceiling) | `token_bucket.rs` (invariant already GREEN) | rate/budget breach |
| Message in-count == out-count per agent turn | breaker signal collector | dropped/injected message |
| Event-log actor_seq strictly monotone | `event_log.rs` (content-addressed chain) | replay/tamper |
| Budget totals (i64, exact) net conserved | `budget.rs` / `money.rs` | value leak |

These feed the **constraint-violation-count** signal directly. **[GROUNDED — `token_bucket.rs`
over-grant test and `event_log.rs` monotone chain are in-tree and passing; the message/budget
counters are PROPOSED extensions of the same pattern.]**

### 5.2 Hooke — linear first, escalate to nonlinear only past the radius of validity
Near a healthy baseline the dynamics are linear to first order, so the cheap linear-regime signals
(**EWMA drift + CUSUM**) are the principled first line of defense. The categorical/nonlinear
machinery (**replay probes**, external oracle checks) is the escalation, triggered only once drift
exceeds the linear regime's radius `r`. `r` sets the escalation threshold. **[GROUNDED principle;
`r` is PROPOSED — unmeasured.]**

### 5.3 Time as a Lyapunov potential (operator origination, corrected)
🧭 **OPERATOR VISION — time as the loop-refinement metric.** The operator proposed **time** as the convergence metric. Corrected: it is **not** a Banach
contraction metric (fails symmetry on artifact-space). It **is** a Foster-Lyapunov potential —
`V(state) = expected residual time to the accepting/green state`; the drift condition
`E[V(next) − V(now)] < −ε` implies geometric ergodicity. This already runs in-tree:
`order_machine.rs` (topology: `has_cycle`, cyclomatic `μ = |E|−|V|+c`, `topological_order`, spectral
radius `ρ`; `ρ≈0 ⟺ acyclic ⟺ topo-order-exists`) + `markov.rs`/`bin/markov_attractor.rs`
(chronology: entropy-rate, escape-mass, Foster-Lyapunov drift, SLEM + period; verdicts
HEALTHY/LIMIT_CYCLE/STRANGE_ATTRACTOR, `budget(slem, tol)` as the spectrum-derived retry cap).
**SLEM is the empirical contraction ratio `q`** for the swarm's refine-loops. Honest caveat kept:
this is rigorous only inside the Markov-chain projection of the real artifact dynamics — aliasing
(distinct artifact states → same outcome token) means `escape_mass → 1` is *supporting evidence*
for convergence, not proof. **[GROUNDED — code in-tree; the `q̂ = t_{n+1}/t_n` per-loop estimator
is PROPOSED, no `q̂` measured yet.]** The breaker consumes `markov::analyze_detailed` as a loop-health
input to the disagreement/drift components.

### 5.4 External deterministic oracle (VAL-style)
Fixed iteration-budget loop with **categorical pass/fail-per-precondition** validation (PDDL-INSTRUCT,
MIT CSAIL, arXiv:2509.13351 — a categorical validator, **not** a scalar-count convergence proof).
This validates the mechanism behind the `constraint_violations` signal: a candidate write is checked
against a `causal.rs` predicate gate (do-operator / back-door reasoning) and rejected categorically.
**[GROUNDED precedent; the causal-gate binding is PROPOSED wiring.]**

## 6. Acceptance criteria (measured numbers — reuse Synthesis I/II falsifiable steps)

| Component | Passing measurement | Marker |
|-----------|--------------------|--------|
| `detreduce` verifier | 1000/1000 bitwise-identical vs 80/1000 baseline; `divergence_rate == 0.0`; overhead ≤ 2.1x unopt | target GROUNDED (TML); **local run PROPOSED** |
| Replay probe FP rate | false-positive rate == `divergence_rate` under zero-poisoning == 0, *after* detreduce verified | PROPOSED (gated on above) |
| SLEM as `q` | on the labeled loop corpus, SLEM→1 beats a coin flip at separating loops from healthy (Synthesis II step 7, zero new compute) | PROPOSED, cheap |
| Conserved-quantity gate | `token_bucket` over-grant + `noether::step_preserves` tests GREEN; message/budget counters land with RED→GREEN tests | GROUNDED (existing) + PROPOSED (new counters) |
| Audit ring | injected deletion/reorder detected via `seq` gap or `prev_hash` break in a unit test | GROUNDED pattern |
| State machine | property test over the §3 table; cooldown doubling capped; red-line `Killed` never auto-resumes | buildable now |
| Thresholds | `fit_from_rates()` consumes a labeled ROC and emits θ at the target operating point; a grep test asserts **no numeric literal** θ in `state.rs` | buildable now |

No new unmeasured scalar claims are introduced; every number above traces to Synthesis I/II or an
in-tree test.

## 7. Phase-3 red-team harness interface (specify now, don't retrofit)

`breaker::testkit::Harness` (behind `#[cfg(any(test, feature="breaker-testkit"))]`):
```rust
impl Harness {
    fn drive(&mut self, s: SignalVector) -> BreakerState;   // step the breaker one window
    fn inject(&mut self, case: &AttackCase);                // apply an attack corpus row
    fn snapshot(&self) -> BreakerRecord;                    // current state for assertions
    fn audit_drain(&mut self) -> Vec<AuditEvent>;           // hash-chained log for the assertion
}
```
CLI driver `kernel/src/bin/breaker_replay.rs` reads a corpus JSONL (hand-rolled parse, serde-free)
exported from each tool and asserts the expected trip. **Threat model anchored to OWASP Top 10 for
Agentic Applications (Dec 2025) ASI06: Memory & Context Poisoning.**

| Red-team tool | Validates which breaker signal |
|---------------|-------------------------------|
| promptfoo **Memory Poisoning** plugin | replay probes + EWMA drift |
| garak **Agent-breaker** probe (v0.15+) | constraint-violation count + disagreement |
| **PyRIT** (XPIA / crescendo / multi-turn) | CUSUM (slow multi-turn drift accumulation) |
| **AgentDojo** (97 tasks / 629 paired security tests) | constraint-violation gate (injected tool-call rejected) |
| **InjecAgent** (1,054 cases) | constraint-violation gate (prompt-injection tool misuse) |

The harness needs only `drive`/`inject`/`snapshot`/`audit_drain`; because the breaker is one
primitive over a swappable signal vector, no red-team tool requires a breaker redesign — each maps
to a component weight. **[GROUNDED — all five tools and ASI06 are real and current per Synthesis
II §7.]**

## 8. Build order (Phase 1 plan → Phase 2 implement)

1. `detreduce.rs` reference reductions + `DeterminismLedger`; verify 1000/1000 locally. **Nothing
   downstream is trustworthy until this is GREEN.** *Why first, precisely:* per the "measure
   inside the core" operator directive (§4), `DeterminismLedger` is the metric authority the
   `truthfulness_fail` component of `SignalVector` depends on. It is **not** a dependency of the
   other four signal components (`confidence_gap`, `ewma_drift`, `cusum`,
   `constraint_violations`, `disagreement`) — those are independently measurable and do not need
   detreduce's *verified result* to be built or tested. Step 1 is ordered first because it gates
   the *replay-probe/truthfulness* path specifically (§4's actual claim), not because the whole
   signal layer is blocked on it — a second builder could start step 2's non-replay components in
   parallel once `detreduce.rs`'s public types exist to compile `SignalVector.truthfulness_fail`
   against, without waiting on the 1000/1000 verification run itself.
2. `signal.rs` + the conserved-quantity feeds (§5.1) reusing `token_bucket`/`event_log`. Depends
   on step 1 only for the masking *type*, not its logic (see above).
3. `state.rs` transition table + `thresholds.rs` `fit_from_rates()` (property-tested — `proptest`
   is already a dev-dependency, used for the P47 reconciliation invariant at
   `ports/payment.rs`/`ports/payment_provider.rs`; no new dependency needed). Ordered after
   `signal.rs` because the transition guards consume `SignalVector::trip_score()`.
4. `audit.rs` hash-chained ring + `tools/telemetry` drain wiring. Depends on `state.rs`'s
   `BreakerState`/`AuditKind` enums existing (`AuditEvent` embeds both).
5. `replay.rs` golden store; arm truthfulness only behind `DeterminismLedger::verified_invariant()`.
   This is the step that actually consumes step 1's *result*, not just its types — the first
   point where "nothing downstream is trustworthy until detreduce is GREEN" becomes literally
   true, since this component reads `verified_invariant()` rather than merely linking against the
   ledger's types.
6. Add the `breaker-testkit` feature to `kernel/Cargo.toml` (follows the existing `chaos` feature's
   pattern verified in-tree: `chaos = []`, gated `#[cfg(any(test, feature = "chaos"))]`, OFF by
   default — `§0` assumes this Cargo.toml edit but does not itself specify it), then `testkit.rs`
   + `bin/breaker_replay.rs`; wire the five corpora (Phase 3).

Each step lands with an inline RED→GREEN falsifiable test (repo convention), stages on write
(untracked-file safety rule), and touches exactly one hot file per turn.

## 9. Failure-mode behavior (per component — explicit, not left to guesswork)

No new anomaly types beyond §2.1's five components are introduced below; each failure routes
through an existing mechanism (a signal field, an existing `AuditKind`, an existing state
transition, or a `Result` at construction time).

| Component | Failure condition | Required behavior |
|-----------|-------------------|--------------------|
| `DeterminismLedger` | Cold start — zero completions ingested yet | `verified_invariant()` returns `false` (undefined `divergence_rate` is never treated as `0.0`). This is the *same* `AuditKind::Disarm` path §4 already defines — not a new state. |
| `detreduce` reductions | Reference reduction computation panics/NaNs | Not swallowed — this is a kernel bug, fails loudly in tests/CI like any other kernel invariant (no `catch_unwind`). |
| `GoldenStore` / replay probe | Store unreachable, or a probe read errors, during HalfOpen | Treated as a probe **mismatch** — fail-closed, same discipline as `TokenBucket::try_acquire`'s "never a partial grant, never a silent downgrade." Drives the existing HalfOpen→Open row (cooldown doubles); reuses the existing `AuditKind::ProbeResult` entry, not a new kind. |
| `AuditRing` | Drain sink falls behind; ring about to overwrite an undrained entry (fixed 8192 capacity) | Backpressure: `tick()` stalls rather than silently drop an audit entry. Losing an entry defeats the ring's whole purpose (tamper-evidence), so this is a deliberate departure from a generic ring buffer's drop-oldest default. |
| `thresholds::fit_from_rates()` | Degenerate/empty labeled ROC (no red-team data yet) | Returns `Result::Err`, never a NaN/zero threshold. A `Breaker` cannot be constructed without a valid fitted `ThresholdId` — fail at construction/bootstrap time, not at tick time. |
| `causal.rs` gate check | Errors (e.g. malformed causal graph) feeding `constraint_violations` | An errored check counts as a violation (`constraint_violations += 1`), never silently ignored — fail-closed, consistent with every other row here. |
| `SignalVector.window_seq` | Gap (a window was dropped) | Per §2.1's own comment ("gap ⇒ dropped window ⇒ audited"): write an audit entry and count the gap toward `consecutive_trips` rather than silently resetting — a dropped window is itself anomaly-adjacent (input starvation against the breaker is a plausible attack). |

**[PROPOSED — none of these rows are measured; they are the explicit, falsifiable-by-unit-test
behaviors the components above must satisfy. Each maps onto an existing struct field/enum/state
transition already specified in this document.]**

## 10. Unit-testing story (per component, not the Phase-3 red-team harness)

The Phase-3 harness (§7) validates end-to-end trip behavior against real attack corpora; it is not
a substitute for unit-level tests of each component's own contract. Repo convention (inline
RED→GREEN, no test framework beyond `std`/`proptest`) applies throughout:

- **`detreduce.rs`**: property test (proptest) that `rmsnorm`/`matmul_rowsum`/`attention_reduce`
  digests are invariant to a randomly permuted batch-composition split of the same inputs. Unit
  tests for `DeterminismLedger::verified_invariant()`: empty ledger → `false`; 1000 identical
  digests → `true`; exactly 1 divergent digest among 1000 → `false` with the exact
  `divergence_rate` fraction asserted.
- **`signal.rs`**: unit test `trip_score()` against hand-computed weighted sums for fixed
  `SignalWeights` and known component values (exact `f32` expected output); unit test that a
  `window_seq` gap is flagged per §9's row.
- **`state.rs`**: property test (proptest) driving random guard-satisfying event sequences through
  the §3 table, asserting no `(from, guard) → to` pair diverges from the table; explicit unit
  tests for the cooldown-doubling cap boundary and for "a red-line `Killed` record never
  transitions except via the explicit manual-reset row."
- **`replay.rs`**: unit test that a bitwise mismatch drives Open, not Closed; unit test that a
  `GoldenPair` with `invariant_verified == false` is excluded from the probe pool — this is the
  one component where the test is a safety property (never gates a close), not just a data-flow
  check, so it should be written RED-first against a deliberately wrong "gate on any pair"
  implementation.
- **`audit.rs`**: unit test hash-chain break detection (already in §6's acceptance criteria: a
  `prev_hash` mismatch or `seq` gap must be caught); add a ring-capacity wraparound test enforcing
  §9's backpressure behavior specifically (assert `tick()` stalls rather than silently drops).
- **`thresholds.rs`**: unit test `fit_from_rates()` against a synthetic ROC with an analytically
  known optimal operating point. The "no numeric literal θ in `state.rs`" acceptance criterion
  (§6) has a direct in-tree precedent for how to write this kind of structural scan test:
  `kernel/tests/no_card_data.rs` (a whole-tree grep-style CI-teeth test with comment/string-literal
  stripping to avoid false positives on doc-comment prose).

## 11. Terms & assumptions a fresh builder needs (undefined elsewhere in this document)

- **"window"** = one `tick()` call / one agent-turn's signal sample, **not** a wall-clock
  interval. `cooldown_ticks` is counted in ticks, not seconds — Synthesis I's prose ("cooldown
  timer T elapses") describes the same concept before it was mapped onto this kernel's
  discrete-event tick clock rather than a real-time timer. **[PROPOSED clarification — not stated
  explicitly in either synthesis document.]**
- **`red_line_class`** — see §2.2: derive from `ports::agent::scope::Scope::touches_red_line()`
  (already built, already tested), do not invent a new classifier.
- **`ThresholdId`** — opaque handle defined in `thresholds.rs` (not shown in this document's
  struct excerpts); resolves to a fitted `Thresholds` value via `fit_from_rates()`. A fresh reader
  should not expect to find its definition in `state.rs` or `signal.rs`.
- **`SignalWeights`** — likewise defined in `thresholds.rs` per §5.3's cross-reference; fitted,
  never hand-tuned literals (same discipline as θ).
