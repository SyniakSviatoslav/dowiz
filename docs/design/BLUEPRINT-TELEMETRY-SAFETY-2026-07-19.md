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
**replay probes** in the Half-Open state (§3, §2.3). **[GROUNDED as a definition; detector efficacy
is PROPOSED and gated on §4.]**

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
    pub last_score: f32,
    pub thresholds: ThresholdId,  // reference to the fitted threshold set (§5.3)
}
```

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
| Open      | `trip_score > θ_kill` for `W_kill` windows **AND** `!red_line_class`       | Killed        | irreversible for this instance; ship full audit ring |
| Open      | `trip_score > θ_kill` for `W_kill` windows **AND** `red_line_class`        | Killed(gated) | as above **+** set `human_gate_required`; never self-resumes |
| HalfOpen  | all `N` probes bitwise-match **AND** `trip_score ≤ θ_open`                 | Closed        | reset counters; `cooldown_ticks = base` |
| HalfOpen  | any probe mismatch **OR** `trip_score > θ_open`                            | Open          | `cooldown_ticks = min(cooldown_ticks*2, cap)`; `consecutive_trips += 1` |
| HalfOpen  | `consecutive_trips ≥ kill_threshold`                                       | Killed        | as Open→Killed, honoring `red_line_class` |
| Killed    | `red_line_class` (money/auth/RLS/migrations)                              | Killed        | **terminal**; resume only via explicit human gate |
| Killed    | `!red_line_class` **AND** operator reset event                            | Closed        | manual only; audited `Transition` |

The **only** place the breaker does not self-close is the red-line-classed `Killed` state
(Synthesis I §1.4). Hysteresis (cooldown doubling, capped) prevents flapping. **[GROUNDED design.]**

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

**Operator directive (this blueprint's binding constraint):** the batch-invariant *computation* AND
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
`TokenBucket::release`/over-grant already does this for permits. Enumerated conserved quantities,
each a `noether::step_preserves`-style check whose violation `+=` into `constraint_violations`:

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
The operator proposed **time** as the convergence metric. Corrected: it is **not** a Banach
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
   downstream is trustworthy until this is GREEN.**
2. `signal.rs` + the conserved-quantity feeds (§5.1) reusing `token_bucket`/`event_log`.
3. `state.rs` transition table + `thresholds.rs` `fit_from_rates()` (property-tested).
4. `audit.rs` hash-chained ring + `tools/telemetry` drain wiring.
5. `replay.rs` golden store; arm truthfulness only behind `DeterminismLedger::verified_invariant()`.
6. `testkit.rs` + `bin/breaker_replay.rs`; wire the five corpora (Phase 3).

Each step lands with an inline RED→GREEN falsifiable test (repo convention), stages on write
(untracked-file safety rule), and touches exactly one hot file per turn.
