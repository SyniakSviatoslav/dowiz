# BLUEPRINT — Predictive RESOURCES telemetry + anomaly stability signals (2026-07-20)

- **Date:** 2026-07-20 · **Component:** ECOSYSTEM-OPS (telemetry) / CORE (math reuse) · **Status:**
  BLUEPRINT v1 (planning artifact, no code). Operator-requested (2026-07-20, verbatim scope):
  real-time prediction of the RESOURCES telemetry series, reusing the kernel's existing
  spectral/graph-Laplacian math and the already-wired `llm-adapters` crate, with spikes/deviations
  surfaced as real stability-warning signals — explicitly reusing `tools/loop-signals/`'s proven
  detector shape (entropy / escape-mass / Foster-Lyapunov / SLEM over a Markov chain) rather than a
  parallel detection mechanism.
- **Sources read this session (live tree, this pass):** `tools/telemetry/topics/src/main.rs`
  (`build_resources_report`, `append_resources_summary`, `fetch_host_gauges`, `abs_pair`,
  `latency_summary`/`reconstruct_durations` — the existing "replay the log fresh each run" pattern);
  `tools/telemetry/hetzner-exporter/src/lib.rs` (the `/health` gauge source: `disk_pct`/`load1`/
  `mem_pct`/`mem_used_mb`/`mem_total_mb`/`disk_used_gb`/`disk_total_gb`); `kernel/src/kalman.rs`
  (`KalmanFilter::scalar`, `update`, `last_surprise`); `kernel/src/markov.rs`
  (`analyze_detailed`, `potential`, `is_escape`, `budget`); `kernel/src/spectral.rs`
  (`DriftClass`, `classify_drift`, `graph_energy_report`); `kernel/src/span_metrics/{obs,breach}.rs`
  (the existing `alert.jsonl`/`JsonlWriter` convention — a DIFFERENT subsystem, see §4.4);
  `kernel/src/fdr/json.rs` (`JsonWriter`, the kernel's serde-free JSON emission discipline);
  `llm-adapters/src/{lib,compose}.rs` (`Harness::chat`, `ChatRequest`, synchronous, no tokio);
  live sample read from `/root/ops/topics/resources-summary.jsonl` (5 rows, confirms the exact
  on-disk schema below).

---

## 1. Problem statement

Commit `d997fc104` landed a real `topics resources` RESOURCES pulse: every invocation (hourly via
cron, per the commit's own description) appends one JSON record to
`/root/ops/topics/resources-summary.jsonl`. Verified live sample (this session):

```json
{"co2e_g_per_h":1.73136320802005,"est_watts":5.278546365914787,"host":"dowiz-dev","kind":"resources_summary",
 "latency":{"jitter_stddev_us":0.0,"n":2000,"p50_le_us":0,"p99_le_us":0,"span":"place_order","span_kinds":1},
 "orders_observed":2000,"ts":1784581365,"util_frac":0.020050125313283207}
```

This is now a **real historical time series** — but purely a log. Nobody reads it back. Three
concrete gaps, verified by reading the source, not assumed:

1. **No prediction.** Every number is reported as-observed; there is no "what did we expect this
   sample to be" and therefore no "how surprising was it."
2. **Schema is incomplete for the operator's own stated scope.** `build_resources_report()`
   (`tools/telemetry/topics/src/main.rs:774-912`) computes `mem_pct`/`disk_pct` (via
   `fetch_host_gauges()`) and `mem_used_mb`/`mem_total_mb`/`disk_used_gb`/`disk_total_gb`/net
   rx-tx bytes (via `abs_pair`, lines 802/806, and the `net_line` block) for the **text** report,
   but `append_resources_summary`'s `serde_json::json!{...}` record (lines 916-932) only persists
   `util_frac` (a *separate* 1-second `/proc/stat` tick-delta CPU measurement, not `load1`),
   `est_watts`, `co2e_g_per_h`, `orders_observed`, and `latency`. **`mem_pct`, `disk_pct`, `load1`,
   and net throughput are computed every run and then discarded** — there is no historical series
   for them at all today, despite being named explicitly in the operator's ask ("CPU/mem/disk
   utilization"). This must be fixed before any of those three can be predicted.
3. **No anomaly signal.** A spike in `p99_le_us` or `est_watts` today is invisible unless a human
   reads the raw log by eye. There is no stability-warning surface at all for RESOURCES data,
   unlike the tool-outcome loop (`tools/loop-signals/` + `kernel::markov`), which already has one.

**What this blueprint is NOT:** it is not a general-purpose time-series ML system, not a new
external forecasting dependency, and not a duplicate of `kernel::markov`'s detector — see §3.3 for
why the SAME function is reused, generalized, not reimplemented.

## 2. Why the existing primitives are the right fit (grounding, not aspiration)

Three kernel primitives already exist and are already tested, independently of this blueprint:

- **`kernel::kalman::KalmanFilter`** (`kernel/src/kalman.rs:149-284`) — a full predict/update Kalman
  filter. `KalmanFilter::scalar(x0, p0, f, h, q, r)` (line 188) is the exact 1-D convenience
  constructor for a single scalar series. Every `update()` call caches `last_innovation()` and
  `last_surprise()` (`‖y‖/√tr(S)`, a dimensionless, already-implemented "how unexpected was this
  sample" scalar — line 275's own doc: "the deterministic novelty scalar"). This is **the**
  prediction primitive the operator asked to reuse; no new estimator needs writing.
- **`kernel::markov::analyze_detailed`** (`kernel/src/markov.rs:110-280`) — the exact detector
  `tools/loop-signals/` already proves in production: entropy rate, escape mass, Foster-Lyapunov
  drift, SLEM/spectral-gap/mixing-time, verdict ∈ {Healthy, LimitCycle, StrangeAttractor}. It
  operates on **any** `&[&str]` token stream — the alphabet is derived from the input at
  `markov.rs:136-139` (`alpha.sort_unstable(); alpha.dedup();`), not hardcoded. What IS hardcoded
  are the two small policy closures `potential()` (line 30) and `is_escape()` (line 37), which
  pattern-match the specific strings `"run_ok"`/`"run_fail"`/`"edit_fail"` — the loop-signals
  alphabet. §3.3 below generalizes exactly that seam (and only that seam) so a second alphabet
  (resource regimes) can drive the identical algorithm without string-aliasing a CPU metric onto a
  tool-outcome name.
- **`kernel::spectral::{classify_drift, graph_energy_report, DriftClass}`** (`spectral.rs:698-830`)
  — graph-Laplacian/adjacency spectral-radius machinery, already used by `autonomic.rs`'s
  gain-scheduling loop to classify a signal as `Damped`/`Resonant`/`Unstable`. §3.4 reuses this for
  a SECOND, complementary signal: not "is one metric flapping" (the Markov lens) but "are multiple
  metrics moving together as one coupled failure mode" (the graph lens) — genuinely using
  graph-Laplacian math for a distinct question, rather than forcing one primitive to answer both.

## 3. Design

### 3.1 Schema extension (prerequisite — `tools/telemetry/topics/src/main.rs`)

Persist what is already computed. Extend the `serde_json::json!{...}` record in
`build_resources_report()` (currently lines 916-932) with the fields already sitting in local
variables at that point in the function:

```rust
let record = serde_json::json!({
    "kind": "resources_summary",
    "ts": ts,
    "host": host,
    "util_frac": util.map(|(u, _)| u),          // unchanged (1s tick-delta CPU)
    "load1_norm": load_val,                       // NEW — hetzner-exporter's load1/nproc
    "mem_pct": gauges.as_ref().and_then(|j| j.get("mem_pct").and_then(|x| x.as_f64())),      // NEW
    "disk_pct": gauges.as_ref().and_then(|j| j.get("disk_pct").and_then(|x| x.as_f64())),    // NEW
    "net_rx_bytes": gauges.as_ref().and_then(|j| j.get("net_rx_bytes").and_then(|x| x.as_f64())), // NEW
    "net_tx_bytes": gauges.as_ref().and_then(|j| j.get("net_tx_bytes").and_then(|x| x.as_f64())), // NEW
    "est_watts": est.map(|(_, w, _)| w),
    "co2e_g_per_h": est.map(|(_, _, g)| g),
    "orders_observed": orders,
    "latency": lat.as_ref().map(|l| serde_json::json!({ /* unchanged */ })),
});
```

(Exact `gauges` key names for the net fields must match whatever `fetch_host_gauges()` /
`hetzner-exporter`'s `/health` JSON actually names them — verify at implementation time via
`curl 127.0.0.1:9091/health`; not re-verified byte-for-byte here since the exporter's JSON keys are
a live runtime contract, not a compile-time one, and this blueprint's own citations already trace
the code paths that produce them.) **Backward compatible**: old rows lack the new keys; every
reader must treat a missing key as `None`/absent — never a fabricated `0.0` (matches this file's
own existing convention, e.g. `abs_pair`'s `None` on a missing key, `jitter_stddev_us: Option<f64>`
on `n<2`).

### 3.2 New kernel module — `kernel/src/resource_forecast.rs` (pure math, no I/O, deterministic)

```rust
/// One tracked RESOURCES metric. Matches the schema fields above 1:1 (§3.1).
pub const METRICS: &[&str] = &[
    "p50_le_us", "p99_le_us", "jitter_stddev_us", "est_watts", "co2e_g_per_h",
    "util_frac", "load1_norm", "mem_pct", "disk_pct", "net_rx_bytes", "net_tx_bytes",
];

/// Per-metric regime, derived from the Kalman innovation `last_surprise()` against
/// fixed, named thresholds (a z-score-like reading — `last_surprise` is `‖y‖/√tr(S)`,
/// a standardized residual under the filter's Gaussian assumption). `Missing` is a
/// FIRST-CLASS state, never coerced to `Calm`/0 — matches the "absence tracked
/// explicitly" convention already used throughout this file's cited sources
/// (`metrics.rs`'s `gpu: Option<GpuSample>`, `abs_pair`'s `None` propagation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceState { Calm, Elevated, Spike, Missing }

/// Named, documented thresholds (mirrors `autonomic.rs`'s bounded-constant style —
/// a checkable equation, not a magic number). `SURPRISE_ELEVATED`/`SURPRISE_SPIKE` are
/// calibrated against a synthetic Gaussian-noise fixture in the acceptance tests
/// (§5) to land near the conventional 1.5σ/3σ tail rates; RE-CALIBRATE against real
/// production history once ~2 weeks of hourly samples exist (tracked as a named
/// follow-up, not a blocker — see §6).
pub const SURPRISE_ELEVATED: f64 = 1.5;
pub const SURPRISE_SPIKE: f64 = 3.0;

/// One metric's scalar predictor: a steady-state random-walk Kalman filter
/// (F=1, H=1 — "the value tomorrow is the value today plus drift noise", the
/// same model class `kalman.rs`'s own doc calls the generalization of `geo::ema_next`).
pub struct MetricPredictor {
    filter: crate::kalman::KalmanFilter,
    warm: bool, // fewer than 2 real samples ⇒ prediction not yet meaningful
}

pub struct Observation {
    pub predicted: f64,
    pub actual: Option<f64>,
    pub surprise: f64,
    pub state: ResourceState,
}

impl MetricPredictor {
    pub fn new() -> Self { /* KalmanFilter::scalar(0.0, 1e6, 1.0, 1.0, q, r) — wide
                               initial P so the first real sample dominates, matching
                               kalman.rs's own "infinite-initial-covariance" doc note */ }

    /// Advance one tick. `sample = None` for a genuinely absent metric this tick
    /// (e.g. `latency` pre-launch, `est_watts` off-Linux) — the filter still
    /// `predict()`s (time passes) but does not `update()` (no measurement), and the
    /// tick is reported `Missing`, never a fabricated surprise of 0.
    pub fn tick(&mut self, sample: Option<f64>) -> Observation { /* predict(); on Some,
        update(&[x]); read last_surprise(); classify via SURPRISE_ELEVATED/SPIKE */ }
}

/// Reuses `kernel::markov`'s engine (see §3.3) with a resource-specific alphabet.
pub struct StabilityDetector { /* BTreeMap<&'static str, MetricPredictor>,
    BTreeMap<&'static str, Vec<&'static str>> ring of ResourceState tokens per metric */ }

pub struct StabilityReport {
    pub observations: Vec<(&'static str, Observation)>,
    pub per_metric_verdict: Vec<(&'static str, crate::markov::Verdict)>,
    pub system_drift: crate::spectral::DriftClass, // §3.4
}
```

`BTreeMap` (not `HashMap`) for deterministic iteration order — matches the kernel-wide
fixed-order-serialization discipline (`fdr/json.rs`'s own doc: "Field order is fixed by call
order... deterministic output").

### 3.3 Generalizing `kernel::markov` — the one real kernel-core change

`markov::analyze_detailed`'s `potential`/`is_escape` (`markov.rs:30-39`) are free functions
matching literal strings. Extract them as parameters, **without changing existing behavior**:

```rust
// markov.rs — NEW, additive:
pub fn analyze_detailed_with<E, P>(states: &[&str], is_escape: E, potential: P) -> DetailedReport
where E: Fn(&str) -> bool, P: Fn(&str) -> f64
{ /* EXACT existing body of analyze_detailed, with the two free-function calls replaced
     by calls to the `is_escape`/`potential` closures — zero other change */ }

// EXISTING analyze_detailed becomes a thin wrapper — byte-identical behavior, pinned by
// the existing 12-case parity corpus (markov.rs tests, UNCHANGED, must stay green):
pub fn analyze_detailed(states: &[&str]) -> DetailedReport {
    analyze_detailed_with(states, is_escape, potential)
}
```

Then `resource_forecast.rs` supplies its own domain-honest closures — no string aliasing onto
`"run_ok"`/`"run_fail"`:

```rust
fn resource_is_escape(s: &str) -> bool { s == "calm" }
fn resource_potential(s: &str) -> f64 {
    match s { "calm" => 1.0, "spike" => -1.0, _ => 0.0 } // "elevated"/"missing" neutral
}
```

`StabilityDetector` feeds each metric's `Vec<ResourceState>` ring (rendered to `&["calm",
"elevated", "spike", "missing"]` tokens) through `markov::analyze_detailed_with(&tokens,
resource_is_escape, resource_potential)`. The resulting `Verdict` gets an honest reinterpretation,
documented here so nobody re-derives it ad hoc later:

- **Healthy** — the metric fluctuates but keeps returning to calm (or never leaves it).
- **LimitCycle** — the metric is **flapping**: rhythmically oscillating between calm and
  spike/elevated without settling. A real stability signal (e.g. a service restarting in a loop).
- **StrangeAttractor** — **chronic instability**: high-entropy churn through elevated/spike states
  that never mixes back to a calm-dominated stationary distribution. The clearest "this metric is
  in real trouble" signal.

### 3.4 Cross-metric graph-Laplacian coherence (the second, complementary signal)

Separately from §3.3's per-metric regime lens, build one small adjacency matrix per analysis run:
`n = METRICS.len()` nodes; edge weight `w[i][j]` = fraction of ticks in the current window where
metric `i` and metric `j` were BOTH in a non-`Calm` state simultaneously (a co-spike coincidence
rate, `[0,1]`, symmetric by construction). Feed this into the **already-existing**
`kernel::spectral::classify_drift(&adj)` / `graph_energy_report(&adj)` (no new spectral code) to
get one `DriftClass`:

- **Damped** — resource stresses are independent/isolated (low coupling) — noise, not a system
  issue.
- **Resonant** — metrics are moving together in a bounded oscillatory way.
- **Unstable** — a genuinely coupled, growing multi-metric degradation (e.g. mem_pct and est_watts
  and p99 all spiking together and getting worse) — the strongest possible stability warning, and
  the one case where the LLM narrative (§3.5) is unconditionally worth the token cost.

This directly answers the operator's explicit ask for the graph-Laplacian math to be used, as a
distinct lens from the Markov regime detector — not a redundant second implementation of the same
question.

### 3.5 I/O + narrative layer — `tools/telemetry/topics` (NOT the kernel)

The kernel module above is pure math: no file I/O, no network, no serde (matches kernel discipline
— `resource_forecast.rs` ships in the kernel's default `std`-only build, zero new dependencies).
All I/O lives in `tools/telemetry/topics`, which already depends on `dowiz-kernel`
(`tools/telemetry/topics/Cargo.toml:14`) and already uses `serde_json`.

- **Stateless replay, not a persisted predictor-state file.** On each `topics resources` run,
  after appending the new sample (§3.1), read the **entire** `resources-summary.jsonl` history,
  construct fresh `MetricPredictor`s, and replay every row through `tick()` in order — exactly the
  pattern `latency_summary`/`reconstruct_durations` already use for `metric.jsonl` (re-parse the
  whole log fresh, every call, no separate state file to keep in sync). The file is small (one row
  per hourly cron tick) so full replay is cheap and there is one fewer failure mode (a
  predictor-state file drifting from the log it's supposed to summarize).
- **New alert artifact: `resources-stability-alert.jsonl`**, appended to the SAME `log_dir()` as
  `resources-summary.jsonl` (`/root/ops/topics/`), only on a non-`Healthy` per-metric verdict or a
  non-`Damped` system `DriftClass`. **Explicitly a different file from the kernel's own
  `alert.jsonl`** (`span_metrics/breach.rs` — the P83 load-breach artifact, gated behind
  `--features telemetry`, written to `DOWIZ_SPAN_METRICS_DIR`, a different subsystem entirely) —
  named distinctly here specifically so the two "alert" artifacts are never conflated by a future
  reader.
- **LLM narrative, bounded and conditional.** Only when an alert actually fires (never on every
  tick — matches "advisory, non-gating" telemetry culture already established by
  `BLUEPRINT-ITEM-51-shadow-mode-divergence-telemetry`), call
  `llm_adapters::Harness::chat(ChatRequest{ ... })` (synchronous, already the harness's contract —
  `compose.rs:41-46`) with a small fixed prompt template summarizing the fired signal(s), to
  produce one human-readable paragraph for the Telegram message. **Degrades closed**: if
  `Harness::health()` fails (Ollama unreachable) or the call errors, fall back to a fixed
  templated sentence built from the `StabilityReport` fields directly (e.g. `"{metric}: {verdict}
  (surprise={surprise:.2}, {n} samples)"`) — the alert is never silently dropped for want of an
  LLM. `tools/telemetry/topics/Cargo.toml` gains `llm-adapters = { path = "../../../llm-adapters" }`
  as a new path dependency (in-repo, not a new external crate — no DECART report needed, matching
  this repo's existing convention that in-repo path deps don't carry the DECART ceremony reserved
  for new third-party crates).

## 4. Fits the existing architecture

- **Kernel stays pure-`std`, serde-free, zero new deps** — `resource_forecast.rs` uses only
  `kernel::kalman`/`kernel::markov`/`kernel::spectral`, all already in the default build.
- **`markov.rs`'s generalization is additive and non-breaking** — `analyze_detailed`'s existing
  callers (`autonomic.rs`, `tools/loop-signals`' `markov_attractor` bin) see byte-identical
  behavior; the 12-case parity corpus (`markov.rs` tests) is the regression guard.
- **I/O/LLM stays out of the kernel**, per the compile-firewall convention `CLAUDE.md` documents
  for the agent lane — `tools/telemetry/topics` is the right home, already at the seam between
  kernel math and the outside world (network, Telegram, `llm-adapters`).
- **No duplicate detector.** This is the explicit, load-bearing design constraint from the operator
  ask: §3.3 is a genuine reuse (shared function, shared parity tests), not a copy.

## 5. Acceptance criteria (RED → GREEN, per this repo's "verified, not claimed" culture)

1. **`markov::analyze_detailed_with` generalization is behavior-preserving.** RED: temporarily
   assert `analyze_detailed`'s existing 12-case parity corpus against a naive re-parametrized
   version to catch any accidental behavior drift during extraction. GREEN: full existing
   `kernel/src/markov.rs` test module passes unchanged after the refactor (zero test edits).
2. **Kalman prediction is falsifiable.** New test: feed `MetricPredictor` a synthetic series with a
   known injected step-change (e.g. constant 5.0 for 20 ticks, then a permanent jump to 50.0) and
   assert (a) `surprise` spikes above `SURPRISE_SPIKE` on the tick of the jump, (b) `surprise`
   decays back toward `SURPRISE_ELEVATED`'s threshold within a bounded number of ticks after the
   filter re-converges, (c) a constant series with injected N(0, σ) Gaussian noise produces a
   spike-classification rate within a documented tolerance band of the 3σ tail probability
   (~0.3%) — the calibration proof for §3.2's named thresholds.
3. **Regime detection matches the reused engine's own proof shape.** New test: a synthetic
   `ResourceState` sequence alternating `calm`/`spike` (the resource-domain analog of
   `markov.rs`'s `green_parity_limit_cycle_thrash`) must classify `LimitCycle` under
   `resource_is_escape`/`resource_potential`; a synthetic chronic-churn sequence (analog of
   `green_parity_strange_attractor_churn`) must classify `StrangeAttractor`.
4. **Cross-metric drift classification.** New test: a synthetic run where 3 metrics co-spike on the
   same ticks must classify system `DriftClass::Unstable`; a run with independent, non-overlapping
   spikes across metrics must classify `Damped`.
5. **Schema extension is backward-compatible.** New test in `tools/telemetry/topics`: parsing an
   OLD-shape `resources_summary` row (missing the new §3.1 keys) must not panic and must report
   every new field as absent, not a fabricated `0.0`/`false`.
6. **End-to-end, RED→GREEN on the real artifact.** Construct a synthetic
   `resources-summary.jsonl` fixture with an injected spike in `est_watts`; run the new
   replay-and-detect path; assert a `resources-stability-alert.jsonl` row is appended with the
   expected metric name and verdict. RED first (no alert file before the code exists / on a
   pre-fix binary), GREEN after.
7. **LLM narrative degrades closed.** Test the fallback template path with `Harness::health()`
   made to fail (e.g. point at an unreachable Ollama base URL) — assert the alert still gets a
   human-readable message (the template), never a dropped or panicking alert.

## 6. Explicitly deferred (named, not silently dropped)

- **Threshold recalibration against real production history.** `SURPRISE_ELEVATED`/`SURPRISE_SPIKE`
  are set from Gaussian-tail-probability reasoning (§3.2), not yet from real dowiz-dev host data
  (only 5 historical rows exist as of this session — too few to fit against). Recalibrate once
  ~2 weeks of hourly samples accumulate; track as a follow-up item, not a blocker to shipping v1.
- **Windowed co-spike correlation window size (§3.4)** is left as a named, tunable constant to set
  at implementation time against real cadence (hourly cron ⇒ likely a 24-48-tick / 1-2 day window)
  rather than guessed here without data.
