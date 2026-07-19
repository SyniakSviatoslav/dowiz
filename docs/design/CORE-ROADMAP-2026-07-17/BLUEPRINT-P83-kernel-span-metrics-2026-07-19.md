# BLUEPRINT P83 — Kernel span metrics + breach-triggered spike profiler (2026-07-19)

> **Standalone OBSERVABILITY blueprint (`dowiz/kernel` + `tools/telemetry`).** One coherent,
> independently buildable unit against the 20-point contract in `CORE-ROADMAP-STANDARD-2026-07-17.md`
> §2. Research source: `docs/research/OPUS-PERF-METRICS-ARCHITECTURE-2026-07-18.md` (whole report).
> Reconciled in `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §3.3-C4 / §5 (Tier C, Wave W3, unit
> **P83**). Format precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`. Grounding tree:
> `/root/dowiz/kernel` + `/root/dowiz/tools/telemetry` at HEAD, read live this pass.
>
> **One sentence:** the kernel already *pays for* `tracing` spans on its money/order hot paths but
> nothing turns span open/close into a per-function latency metric — P83 adds a ~120-line zero-dep
> `tracing` `Layer` that folds span durations into `metric.jsonl` (Layer 1, continuous) plus a
> breach-triggered system-wide `perf record` on the existing `load1/nproc ≥ 4` friction hook (Layer 2,
> on-demand), giving per-*function* attribution where the operator today has only per-*host*.

---

## VERDICT (stated up front, per session research discipline)

**GO — zero new dependencies, provably-small hot-path cost, parallel-safe.** Unlike P81/P82 this
blueprint changes real kernel source (it adds five spans and a `Layer`), so it carries a genuine
hot-path-overhead obligation — discharged in §7 with measured numbers, not assertion. Three honest
framings bound it:

1. **We are adding a *consumer* of instrumentation that already runs, not new instrumentation.** Three
   spans are already placed and paid for (`domain.rs`, `order_machine.rs`); `tracing` +
   `tracing-subscriber` are already **non-optional** kernel deps. The gap is that `init_tracing()`
   installs a *printing* subscriber that aggregates nothing (§0.2). Layer 1 is ~120 lines against an
   already-linked crate — **zero new dependencies** (R6 §1 "the decisive finding").

2. **Layer 2's honest reframe of the operator's own question.** The `load1` spike the operator saw
   (1.09→2.15→3.52→4.51→3.73 over one minute) **most likely occurred during a `cargo build`** — the
   cores were eaten by `rustc`, not kernel functions (R6 §3). A **system-wide** `perf record -a` is
   the *only* one of the candidate tools that can answer "is this spike my kernel, or just `rustc`?"
   before drilling into which kernel function. P83 preserves that caveat rather than pretending
   per-function tracing alone would have explained the spike — it would not have.

3. **The one deliberate non-goal:** P83 does **not** instrument inner loops (`assert_transition`,
   eigensolver kernels). A ~100 ns span on a per-edge inner loop would *violate* the hot-path
   constraint it exists to protect (R6 §5). The cut line is "per business event, not per iteration,"
   and a CI grep enforces it (§4.4).

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> Read from `/root/dowiz/kernel` + `/root/dowiz/tools/telemetry` this pass. **Two corrections to the
> research doc are load-bearing and recorded here — a correct blueprint requires them.**

### 0.1 Correction A — `kernel/src/telemetry.rs` ALREADY EXISTS and is a *different* concern

R6 §6 step 1 prescribes "add `SpanMetricsLayer` … in a **new** `kernel/src/telemetry.rs`." **That
filename is taken.** Live, `kernel/src/telemetry.rs` is the **self-improvement-loop trigram pattern
surface** (`surface_recurring_patterns`, "W19 integration point for `trigram.rs`", zero-dep HashMap
n-gram counting) — an unrelated module. **Correction:** the `SpanMetricsLayer` goes in a **new module
`kernel/src/span_metrics.rs`**, not `telemetry.rs`. Reusing the existing file would fuse two unrelated
responsibilities (self-improvement pattern detection vs latency aggregation) and break its zero-dep
single-purpose doc contract. This is the single most important correction in the blueprint.

### 0.2 The instrumentation that already exists (verified this pass)

| Element | Cite (live) | State |
|---|---|---|
| `tracing` dependency | `kernel/Cargo.toml:117` `tracing = "0.1"` | **non-optional**, not feature-gated |
| `tracing-subscriber` | `kernel/Cargo.toml:122` `= "0.3"` features `["env-filter"]` | linked; exposes the `Layer`/`Registry` API P83 uses |
| span on `place_order` | `kernel/src/domain.rs:175` `tracing::info_span!(...)` | **already placed + paid for** |
| span on `place_order_priced` | `kernel/src/domain.rs:219` `tracing::info_span!(...)` | **already placed** (catalog-authoritative money path) |
| span on `fold_transitions` | `kernel/src/order_machine.rs:161` `info_span!("fold_transitions", …).entered()` | **already placed** |
| `init_tracing()` | `kernel/src/lib.rs:369` → `tracing_subscriber::fmt()...try_init()` (`:372`) | installs a **printing** subscriber; **aggregates nothing** — the entire gap |

**Correction B (line drift):** R6 cites `init_tracing()` at `lib.rs:323` and the two `domain.rs`
spans at `:164`/`:207`; live they are `lib.rs:369`, `domain.rs:175`, `domain.rs:219` (the `info_span!`
call lines). Same functions, drifted since the 2026-07-18 read. The blueprint uses the live lines.
`kernel/Cargo.toml:121` explicitly notes the subscriber is "dev/CLI only, never called from wasm" —
which is why Layer 1 is safe (§7, no wasm impact).

### 0.3 The functions to add spans to (verified this pass)

| # | Function | Cite (live) | Span action |
|---|---|---|---|
| 4 | `event_log::EventLog::commit_after_decide` | `kernel/src/event_log.rs:366` | ADD `info_span!("commit_after_decide")` (decide→commit + SHA3 chain append) |
| 5 | `ports::payment::decide_settlement` | `kernel/src/ports/payment.rs:367` | ADD `info_span!("decide_settlement")` (money settlement + cap-auth verify) |
| 6 | `ports::agent::cap::verify_chain` | `kernel/src/ports/agent/cap.rs` (R6 §5) | ADD `info_span!("verify_chain")` (capability-cert chain verify) |
| 7 | `pq::dsa::verify` | `kernel/src/pq/dsa.rs:1003` (R6 §5) | ADD `info_span!("mldsa_verify")` **behind `cfg(feature = "pq")`** so the default build is untouched |
| 8 | `router::route` | `kernel/src/router.rs:90` | ADD `info_span!("route")` (dispatch/routing hot path) |

Functions 1–3 (`place_order_priced`, `place_order`, `fold_transitions`) already have spans (§0.2) —
they need only the aggregating Layer, **no source edit**.

### 0.4 The telemetry pipeline P83 extends (verified this pass)

| Element | Cite (live) | State |
|---|---|---|
| `telemetry monitor` loop | `tools/telemetry/telemetry:84` | samples host every ~15 s, emits a `metric` event via `log_event` |
| **the friction breach branch** | `tools/telemetry/telemetry:105` | `awk … 'l/n>=4'` → sets `friction`; `:107-108` `tg_deliver "🔥 friction: …"` — **the hook Layer 2 attaches to** |
| `telemetry kernel` subcommand | `tools/telemetry/telemetry:67` (`kernel)`) | runs a kernel probe, folds the result via `log_event` — **the pattern `kernel-spans` mirrors** |
| `native-trackers` precedent | `tools/telemetry/native-trackers/` (R6 §0) | zero-dep pure-`std` native binary reading the same JSONL — the "hand-roll over a fixed schema, don't link a crate" discipline Layer 1's histogram matches |

---

## 1. Prior-art map — adopt, don't invent (standard §2 item 19)

| Prior art | What it is | How P83 uses it — and what it does NOT take |
|---|---|---|
| **`tracing-subscriber` `Layer` over `Registry`** | per-span `extensions` scratch storage; `on_new_span`/`on_close` callbacks | **Adopt** — `SpanMetricsLayer` stamps `Instant` on enter, folds `elapsed` on close. **NOT taken:** any distributed-trace exporter (OTLP/Jaeger) — over-engineering for a single-box kernel; a `metric.jsonl` line is the whole need. |
| **`native-trackers` hand-rolled JSON over a fixed schema (zero-dep)** | the operator's chosen "don't link a crate for JSON" pattern | **Adopt** — the histogram is a hand-rolled fixed-bucket log-scale array, **not** `hdrhistogram` (which would be a new dependency, violating the minimal-deps constraint). |
| **The existing `load1/nproc ≥ 4` friction branch** (`telemetry:105`) | already-wired breach detector → `tg_deliver` | **This is the seam Layer 2 fills** — add a `perf record` capture next to the existing alert; no new alerting machinery. |
| **`perf record -a -g -F 99` (system binary)** | zero-instrumentation sampling profiler; can run system-wide | **Adopt as the deep-dive slot** — zero new deps, and its **system-wide mode uniquely answers "kernel or `rustc`?"** (§0/VERDICT.2). `-F 99` (off round numbers) is the documented production rate. |
| **`pprof-rs` (in-process SIGPROF sampler)** | flamegraph via `setitimer`; **one new dep** (`backtrace-rs`) | **NOT taken as primary** — the minimal-deps constraint tips it to `perf`. Kept only as a **feature-gated (`profiling`), off-by-default fallback** if `perf_event_paranoid` is locked down in some target env (mirrors how the kernel already gates `chaos`/`pq`/`gpu`). |
| **`hdrhistogram` / `metrics` crates** | ready-made latency histograms | **NOT taken** — each is a new dependency; the hand-rolled 24-bucket log histogram gives `count`/`sum`/`max`/`p50`/`p99` at zero dep cost (R6 §1). Stated so nobody "upgrades" to a crate without re-reading the zero-dep rationale. |

---

## 2. Scope — what P83 owns vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P83 OWNS

1. **NEW `kernel/src/span_metrics.rs`** (§0.1 correction): `SpanMetricsLayer` (~120 lines) — per-span
   `Instant` stamp on enter, `on_close` fold into a per-name hand-rolled log-bucket histogram, a
   15 s background flush emitting `SPANMETRICS <json>` lines to stdout. Zero new deps.
2. **EDIT `kernel/src/lib.rs:369` `init_tracing()`**: when `DOWIZ_SPAN_METRICS=1`, install
   `SpanMetricsLayer` instead of the `fmt` layer (one `if`). Default/wasm behavior unchanged.
3. **ADD five spans** (functions #4–#8, §0.3) matching the existing `info_span!` style; #7 behind
   `cfg(feature = "pq")`.
4. **NEW `telemetry kernel-spans` subcommand** (`tools/telemetry/telemetry`): mirror `telemetry
   kernel` (`:67`); run the instrumented target under `DOWIZ_SPAN_METRICS=1`, fold emitted
   `SPANMETRICS` lines via `log_event metric "kind=kernel_span" "fn=<name>" "sample=<escaped json>"`.
5. **EXTEND the `monitor` friction branch** (`telemetry:105`): on `load1/nproc ≥ 4`, in addition to
   the Telegram alert, run `perf record -a -g -F 99 -o /tmp/spike-<ts>.data -- sleep 10` (system-wide
   first) and emit `log_event alert "kind=cpu_spike" "profile=<path>"` into the existing `alert.jsonl`.
6. **(Deferred, feature-gated) `profiling` feature** pulling `pprof` into a kernel bin as the Layer-2
   fallback — declared OFF by default; body only if `perf` perms prove unavailable.

### 2.2 P83 does NOT own (anti-scope)

- **Spans on inner loops.** `assert_transition` (per-edge, inside `fold_transitions`), eigensolver
  kernels, histogram bumps — **deliberately never instrumented** (R6 §5; a span there violates the
  hot-path constraint). A CI grep enforces the exclusion (§4.4). Layer 2's sampler covers them at zero
  per-call cost.
- **The offline benchmark-coverage gap** — that is P80/P81/P82 (criterion `baseline.json`). P83 is
  **live production attribution**, a *distinct* concern (R6 title line): span durations of the running
  kernel, not `cargo bench` numbers. It writes `metric.jsonl`, not `baseline.json`. No overlap, no
  P75 dependency.
- **Any new alerting/telemetry system.** P83 reuses `log_event`, the `monitor` loop, and the JSONL
  ledgers — it forks nothing (R6 §0 constraint).
- **A `metrics`/`hdrhistogram`/OTLP dependency.** Explicitly rejected (§1) — zero-dep is a hard
  property.

### 2.3 Dependencies (named by artifact — standard §2 item 7)

**Hard inputs:** none — R6 confirms `tracing`/`tracing-subscriber` are already linked and three spans
already exist. **Parallel-safe** with W1/W2 (S1 §5.3 lists P83 "anytime"; MASTER-STATUS-LEDGER §3
Wave-2 "P83 (anytime)"). It touches `kernel/src` files **disjoint** from P77 (`spool.rs`/`spine.rs`)
and P79 (`causal.rs`/`spectral.rs`) — collision-free.
**System requirement (not a build dep):** Layer 2 needs `perf_event_paranoid` low enough (or
`CAP_PERFMON`) + frame pointers/DWARF for good stacks — the `profiling`/`pprof` fallback exists for
environments where that is unavailable.

### 2.4 Honest reconciliation with the operator's spike question (standard §2 item 6)

The operator asked, in effect, "what function caused my `load1` spike?" R6's honest answer, preserved
here: **Layer 1 alone cannot answer it if the spike was a build** — during `cargo build` the kernel
isn't even running; the hot frames are `rustc`. Layer 1 answers the *trend* question ("is
`decide_settlement` p99 climbing week over week?"); Layer 2's **system-wide** capture answers the
*attribution* question ("kernel or `rustc`, right now?"). P83 ships both because the operator's real
need spans both, and it does **not** overstate that continuous per-function tracing would have caught
a build spike — it would not have (§7 limits).

---

## 3. Predefined types & constants — named BEFORE implementation (standard §2 item 4)

```rust
// kernel/src/span_metrics.rs  (NEW — NOT telemetry.rs, which is the trigram surface, §0.1)

/// Number of log-scale duration buckets. 24 power-of-two buckets from 16 ns to ~256 ms
/// (16ns << 2^24 ns ≈ 268 ms) — spans the whole realistic latency range of the instrumented
/// per-event functions without a per-close allocation (matches native-trackers' zero-dep discipline).
pub const HIST_BUCKETS: usize = 24;
pub const HIST_MIN_NS: u64 = 16;          // floor bucket edge

/// Flush cadence — matches the host sampler's 15 s so metric.jsonl rows line up temporally.
pub const FLUSH_INTERVAL_SECS: u64 = 15;

/// The ONLY span names this Layer aggregates (a fixed allow-set — a span not in this set is
/// ignored, so an accidental future span on a hot inner loop costs at most the filter check,
/// never a histogram slot). This is the single place the instrumented surface is defined.
pub const INSTRUMENTED_SPANS: &[&str] = &[
    "place_order", "place_order_priced", "fold_transitions",
    "commit_after_decide", "decide_settlement", "verify_chain",
    "mldsa_verify", "route",
];

/// One function's rolling latency accumulator. Fixed-size — no per-close heap allocation.
#[derive(Debug, Default, Clone)]
pub struct SpanStat {
    pub count: u64,
    pub sum_ns: u64,
    pub max_ns: u64,
    pub buckets: [u64; HIST_BUCKETS],   // log-scale histogram → bucket-interpolated p50/p99
}

/// The aggregating Layer. Holds a small fixed map name→SpanStat behind a Mutex; the map has at
/// most INSTRUMENTED_SPANS.len() entries, so the lock is O(1)-bounded and never grows.
pub struct SpanMetricsLayer { /* Mutex<BTreeMap<&'static str, SpanStat>>, flush thread handle */ }

/// The flushed line shape (one per instrumented fn per interval) — byte-compatible with the host
/// `metric` line so every existing metric.jsonl reader keeps parsing (R6 §4 Layer-1 integration).
///   SPANMETRICS {"fn":"decide_settlement","count":N,"sum_ns":..,"max_ns":..,"p50_ns":..,"p99_ns":..}
```

**Constants (Layer 2, in `tools/telemetry/telemetry`):**

```sh
PERF_FREQ=99                 # -F 99: documented production sampling rate, off round numbers
PERF_WINDOW_SECS=10          # bounded one-shot capture per breach
SPIKE_DIR=/tmp               # spike-<ts>.data lands here; indexed via log_event alert
```

---

## 4. Build items — spec → RED test → code, each adversarial-guarded (standard §2 items 2, 3, 5)

### 4.1 M1 — `SpanMetricsLayer` in `kernel/src/span_metrics.rs`

- **Spec:** implement `tracing_subscriber::Layer`: `on_new_span`/`on_enter` stamp `Instant::now()`
  into the span's `extensions`; `on_close` compute `elapsed`, look up the static name, **ignore names
  not in `INSTRUMENTED_SPANS`**, else fold into the `SpanStat` (count++, sum, max, bucket-bump —
  **no allocation on this path**). A background thread flushes every `FLUSH_INTERVAL_SECS` as
  `SPANMETRICS <json>` to stdout and resets nothing (rolling) or resets per interval (configurable;
  default rolling with per-interval delta emit).
- **RED `red_span_durations_not_aggregated`:** with the current `fmt` subscriber, run a workload that
  calls `place_order_priced` 1000× → **no** per-function latency exists anywhere (only printed log
  lines). RED today; GREEN once the Layer emits a `SPANMETRICS` line with `count == 1000`.
- **RED `red_uninstrumented_span_ignored`:** a span named `"scratch"` (not in the allow-set) produces
  **no** `SpanStat` entry → proves the fixed allow-set bounds the map.
- **Adversarial `red_on_close_does_not_allocate`:** a test (or `#[bench]`) asserting the `on_close`
  path performs zero heap allocations (fixed-bucket array, no `Vec`/`String` build) — because a Layer
  that allocated per span-close would defeat its own overhead budget and become the very regression it
  hunts. This is the load-bearing hazard guard.

### 4.2 M2 — `init_tracing()` gate + the five new spans

- **Spec:** in `kernel/src/lib.rs:369`, branch on `std::env::var("DOWIZ_SPAN_METRICS")`: if `"1"`,
  `tracing_subscriber::registry().with(SpanMetricsLayer::new()).try_init()`; else the existing `fmt`
  path unchanged. Add the five `info_span!(...).entered()` lines (functions #4–#8, §0.3); #7 behind
  `cfg(feature = "pq")`.
- **RED `red_default_build_unchanged`:** without the env flag, `init_tracing()` installs the `fmt`
  layer exactly as today (assert no `SpanMetricsLayer` in the default path) — the production
  `cdylib`/wasm build is byte-unaffected.
- **RED `red_five_spans_emit`:** under `DOWIZ_SPAN_METRICS=1`, a workload exercising all eight paths
  emits eight distinct `fn` names in the flush; `mldsa_verify` appears only when `--features pq`.
- **Adversarial `red_no_span_on_assert_transition`:** a **CI grep** asserting `assert_transition`
  (and the eigensolver inner-loop functions) contain **no** `info_span!` — the smart-index for the
  "someone instrumented a hot inner loop" bug class (§4.4).

### 4.3 M3 — `telemetry kernel-spans` subcommand

- **Spec:** mirror `telemetry kernel` (`:67`): run the instrumented kernel target (a bin/bench) under
  `DOWIZ_SPAN_METRICS=1`, read the emitted `SPANMETRICS <json>` lines, and for each emit
  `log_event metric "kind=kernel_span" "fn=<name>" "sample=<_jesc json>"` — byte-compatible with the
  host `metric` line (`kind=<x>` + escaped `sample`, R6 §4).
- **RED `red_kernel_spans_absent`:** `telemetry kernel-spans` does not exist → no per-function row ever
  reaches `metric.jsonl`. GREEN once a run appends `kind=kernel_span` rows queryable by `fn`.
- **Adversarial `red_metric_line_shape_preserved`:** assert a `kind=kernel_span` row parses under the
  *existing* `metric.jsonl` reader (the double-serialized `sample` contract, R6 §0) — a malformed
  line that broke current consumers is the regression this guards.

### 4.4 M4 — Layer 2: breach-triggered `perf record` + the inner-loop CI fence

- **Spec:** extend the `friction` branch (`telemetry:105`): when `load1/nproc ≥ 4` fires, run
  `perf record -a -g -F "$PERF_FREQ" -o "$SPIKE_DIR/spike-<ts>.data" -- sleep "$PERF_WINDOW_SECS"`
  (**system-wide first** — answers "kernel or `rustc`?") and `log_event alert "kind=cpu_spike"
  "profile=<path>"`. Guard with a "perf available?" check; if absent, log the alert without the
  capture (never fail the monitor loop).
- **RED `red_spike_not_captured`:** today a breach fires only a Telegram alert — no profile artifact,
  no `alert.jsonl` `cpu_spike` row. GREEN once a simulated breach writes the artifact + the indexed row.
- **Adversarial `red_perf_absent_degrades_gracefully`:** with `perf` unavailable (or
  `perf_event_paranoid` locked), the monitor loop still emits the alert and **does not crash or hang**
  — degrade-closed, not degrade-broken. (This is where the `profiling`/`pprof` fallback would engage,
  feature-gated OFF by default.)
- **The inner-loop CI fence (M2's `red_no_span_on_assert_transition`) lands here as a committed CI
  step** so the exclusion is permanent, not a one-time review note.

---

## 5. DoD — falsifiable, RED→GREEN, machine-checkable (standard §2 item 2)

| # | Done when… | Falsifier (check) |
|---|---|---|
| D1 | `SpanMetricsLayer` folds span durations into per-`fn` histograms | `red_span_durations_not_aggregated` (count==N), `red_uninstrumented_span_ignored` |
| D2 | the default (no-env) and wasm builds are byte-unaffected | `red_default_build_unchanged`; `cargo build -p dowiz-kernel` + the wasm target diff clean |
| D3 | all 8 functions emit a named metric under `DOWIZ_SPAN_METRICS=1`; `mldsa_verify` only under `pq` | `red_five_spans_emit` |
| D4 | `on_close` performs **zero** heap allocation (overhead budget honored) | `red_on_close_does_not_allocate` |
| D5 | `telemetry kernel-spans` appends `kind=kernel_span`/`fn=…` rows to `metric.jsonl`, parseable by existing readers | `red_kernel_spans_absent`, `red_metric_line_shape_preserved` |
| D6 | a `load1/nproc ≥ 4` breach captures a system-wide `perf` profile + indexes it in `alert.jsonl` | `red_spike_not_captured`; degrade check `red_perf_absent_degrades_gracefully` |
| D7 | no span exists on `assert_transition` / eigensolver inner loops | `red_no_span_on_assert_transition` (committed CI grep) |
| D-DEPS | **zero** new runtime dependencies added (pprof only behind off-by-default `profiling`) | `cargo tree -e normal -p dowiz-kernel` unchanged vs HEAD; `profiling` feature declared, default OFF |
| D-OVH | a measured recorded-span cost ≤ ~150 ns and a filtered span ≈ free (item 10) | the M1 micro-bench numbers, recorded in the pass |

---

## 6. Benchmarks + telemetry (standard §2 item 10)

- **This blueprint IS a telemetry mechanism** — its deliverable is continuous per-function
  `metric.jsonl` trend data (p50/p99 over time) for the 8 named functions, plus on-demand spike
  flamegraphs. That directly satisfies item 10's "telemetry hook so regressions show up automatically,
  not only at review time."
- **Required measured number (D-OVH):** a micro-bench of `SpanMetricsLayer` proving a **recorded**
  span costs ≤ ~150 ns (dominated by two `Instant::now()` reads ≈ 50 ns + map lookup + bucket bump,
  R6 §1) and a **filtered/uninstrumented** span is effectively free (one relaxed atomic compare). At
  the real event rate (hundreds–low-thousands/s), that is ≤ ~0.01 % CPU — provably small on these
  per-event paths. The number is *measured and recorded*, per the standing rule; it is not asserted.
- **Scaling axis (item 8):** the metric store is **O(INSTRUMENTED_SPANS.len())** = 8 fixed entries —
  it does not grow with call volume (counts fold in place). It would only change shape if the
  instrumented set grew to hundreds of functions (it will not — the cut line is per-event functions).
  Layer 2's cost scales with **breach frequency × 10 s window**, not with steady-state load.

---

## 7. Cross-cutting obligations + honest limits (standard §2 items 6, 8, 9, 11–16, 20)

- **Hazard-safety (item 6):** the hazard P83 introduces is **the observer becoming the bottleneck** —
  a Layer that slows the hot path it measures. Made unrepresentable by: (i) the fixed allow-set
  (`INSTRUMENTED_SPANS`) so an errant span costs only a filter check; (ii) the **zero-alloc `on_close`**
  invariant, guarded by `red_on_close_does_not_allocate`; (iii) the **hard exclusion of inner loops**,
  guarded by the committed CI grep `red_no_span_on_assert_transition`; (iv) the measured D-OVH budget.
  Reachability of "the metrics layer regressed the kernel" is argued from these four structural guards,
  not from a promise.
- **Isolation / bulkhead (item 11):** Layer 1 is **opt-in via `DOWIZ_SPAN_METRICS=1`** — the default
  and wasm builds never install it (D2), so a Layer bug cannot reach production. Layer 2 is
  out-of-process (`perf` is a separate binary) and **degrade-closed** — if `perf` is unavailable the
  monitor loop still alerts and never crashes (`red_perf_absent_degrades_gracefully`). Each layer's
  failure is contained to itself.
- **Mesh awareness (item 12):** N/A — this is single-node host observability; it gossips nothing and
  touches no transport. (The mesh's *own* per-frame cost is P82's bench lane, a separate concern.)
- **Rollback/self-heal as math (item 13):** **Self-termination** = the env-gate — with
  `DOWIZ_SPAN_METRICS` unset the Layer simply does not exist (the unsafe "observer on the prod hot
  path" state is unrepresentable by construction, not by a supervisor). Rollback = unset the env var
  (Layer 1) / remove one shell branch (Layer 2); the five spans are inert without the aggregating
  Layer. No runtime state to heal.
- **Error-propagation / smart index (item 14):** the bug class this could introduce — *a span placed
  on a hot inner loop* — is turned into a **CI-time** failure by the committed grep (M4/D7), not a
  runtime surprise. The bug class it *catches* — *a per-event function silently regressing in prod* —
  surfaces as a rising p99 in `metric.jsonl`, queryable by `fn`.
- **Living-memory awareness (item 15):** the `metric.jsonl` rows are **time-scoped** trend data — the
  natural input to the self-improvement loop's temporal analysis (the same ledger family
  `markov.rs`/the existing `telemetry.rs` trigram surface consume). P83 feeds living memory a
  per-function latency time-series; it does not itself persist beyond the JSONL append.
- **Tensor/spectral (item 16):** N/A, honestly — a histogram fold is not a linear-algebra kernel;
  forcing `spectral.rs` here would be over-engineering (ponytail). Stated.
- **Linux discipline (item 9):** **EXTENDS** the existing `tools/telemetry` pipeline (a new
  subcommand + a friction-branch capture) and the already-linked `tracing-subscriber` (a new Layer);
  **REINFORCES** the native-trackers "hand-roll over a fixed schema, don't link a crate" discipline
  and the `log_event`/JSONL contract; **DOES-NOT-TRANSFER** — no new daemon, no new alerting system,
  no distributed tracer.
- **Hermetic principles (item 20):** **Cause & Effect** — P83 turns "the box felt slow" into a named
  cause (a specific `fn`'s rising p99, or a `perf` frame proving it was `rustc`); attribution replaces
  correlation. **Correspondence** — the `metric.jsonl` `fn` row corresponds exactly to the span it
  measures; the fixed allow-set forbids a metric that corresponds to nothing.

**Honest limits (R6 §1, preserved, not softened):** Layer 1 measures **wall-clock of the span, not
where CPU went inside it** — if `route` blocks on a lock or page-fault the span is "slow" but not
*why*; that is Layer 2's job. Layer 1 covers **only the 8 chosen functions** — a regression in an
uninstrumented function is invisible to it (Layer 2's sampler covers the rest at spike time). And
Layer 1 is **blind to a build-time spike** — the reason Layer 2's system-wide `perf` is not optional.

---

## 8. Standard-compliance map (all 20 points — standard §2)

| # | Standard item | Where satisfied |
|---|---|---|
| 1 | Ground truth, live `file:line` | §0 (Correction A: `telemetry.rs` collision; Correction B: line drift; every span/pipeline cite re-verified) |
| 2 | Falsifiable DoD | §5 (D1–D-OVH, incl. zero-alloc + injected checks) |
| 3 | Spec→test→code, event-ordered | §4 (spec-first per M; RED-before / GREEN-after) |
| 4 | Predefined types & constants | §3 (`SpanMetricsLayer`, `SpanStat`, `INSTRUMENTED_SPANS`, `SPANMETRICS` shape, PERF_* consts) |
| 5 | Adversarial cases | §4 (zero-alloc, uninstrumented-ignored, no-span-on-inner-loop, perf-absent-degrades) |
| 6 | Hazard-safety from structure | §7 (observer-as-bottleneck made unrepresentable by 4 structural guards) |
| 7 | Links to docs & memory | §9 |
| 8 | Schemas with scaling axis | §6 (fixed 8-entry store; Layer-2 scales with breach freq, not load) |
| 9 | Linux engineering discipline | §7 (EXTENDS/REINFORCES/DOES-NOT-TRANSFER) |
| 10 | Benchmarks + telemetry | §6 (P83 *is* telemetry; D-OVH measured overhead number required) |
| 11 | Isolation / bulkhead | §7 (env-gated Layer 1; out-of-process degrade-closed Layer 2) |
| 12 | Mesh awareness | §7 (N/A, single-node host observability, stated) |
| 13 | Rollback/self-heal as math | §7 (self-termination = env-gate; unsafe state unrepresentable when unset) |
| 14 | Error-propagation / smart index | §7 (inner-loop span → CI grep; prod regression → metric.jsonl p99) |
| 15 | Living-memory awareness | §7 (time-scoped per-fn latency series feeds the self-improvement loop) |
| 16 | Tensor/spectral where applicable | §7 (N/A honestly; a histogram fold is not linear algebra) |
| 17 | Regression tracking | §9 (REGRESSION-LEDGER note; the CI grep is a permanent fence) |
| 18 | Clear worker instructions | §9 |
| 19 | Reuse-first, upgrade-if-needed | §1 (adopt Layer/perf/native-trackers; reject hdrhistogram/OTLP/pprof-primary with reason) |
| 20 | Hermetic principles | §7 (Cause & Effect, Correspondence) |

---

## 9. Links to docs & memory + instructions for other agentic workers (standard §2 items 7, 18)

**Depends on / cites:**
- `docs/research/OPUS-PERF-METRICS-ARCHITECTURE-2026-07-18.md` — §0 (existing pipeline), §1 (Layer 1
  design + overhead numbers), §3 (the `perf` system-wide "kernel vs rustc" reframe), §4
  (the hybrid recommendation), §5 (the 8-function set + the `assert_transition` exclusion), §6 (build
  order — **corrected here for the `telemetry.rs` filename collision, §0.1**).
- `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §3.3-C4, §5 (Tier C, Wave W3, unit P83, "no deps,
  parallel-safe").
- `MASTER-STATUS-LEDGER-2026-07-19.md` §1 (P83 row), §3 Wave-2 ("P83 anytime").
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract).
- Format precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`.
- Memory: `performance-priority-over-minimal-change-2026-07-17.md`, `verified-by-math-2026-07-07.md`,
  `markov-attractor-loop-signal-2026-07-13.md` + `knowledge-as-circuits-and-eye-2026-07-05.md` (the
  self-improvement loop that consumes the time-series P83 emits).

**Existing code this blueprint edits/creates (exact targets — `dowiz` repo):**
- **NEW** `kernel/src/span_metrics.rs` — `SpanMetricsLayer` + `SpanStat` + histogram + flush thread
  (§3/§4.1). **NOT `telemetry.rs`** (taken — §0.1).
- **EDIT** `kernel/src/lib.rs:369` `init_tracing()` — the `DOWIZ_SPAN_METRICS` branch + `mod
  span_metrics;`.
- **EDIT** `kernel/src/event_log.rs:366`, `kernel/src/ports/payment.rs:367`,
  `kernel/src/ports/agent/cap.rs`, `kernel/src/pq/dsa.rs:1003` (behind `cfg(feature = "pq")`),
  `kernel/src/router.rs:90` — one `info_span!(...).entered()` each (functions #4–#8).
- **EDIT** `tools/telemetry/telemetry` — NEW `kernel-spans` subcommand (mirror `:67`); EXTEND the
  friction branch (`:105`) with the `perf record -a` capture + `alert.jsonl` `cpu_spike` row.
- **EDIT** `kernel/Cargo.toml` — declare an OFF-by-default `profiling` feature (the `pprof` fallback
  boundary); **add no default dependency**.
- **DO NOT TOUCH** `kernel/src/telemetry.rs` (the trigram surface), `assert_transition`, or any inner
  loop.

**For the worker with zero session context — exact acceptance path:**
1. Create `kernel/src/span_metrics.rs` with the zero-alloc `on_close` histogram; prove D4
   (`red_on_close_does_not_allocate`) before wiring anything else — the overhead budget is the gate.
2. Add the `DOWIZ_SPAN_METRICS` branch in `init_tracing()` and the five spans; prove D2
   (default/wasm build byte-unchanged) and D3 (all 8 emit).
3. Add `telemetry kernel-spans`; prove D5 (rows parse under the existing `metric.jsonl` reader).
4. Extend the friction branch with system-wide `perf record`; prove D6 + the degrade-closed path.
5. Commit the `red_no_span_on_assert_transition` CI grep (D7) and a REGRESSION-LEDGER row: "kernel
   per-function span metrics + spike profiler (P83); observer-overhead budget + inner-loop fence gated."
6. Record the D-OVH measured overhead number in the pass output (per the standing rule — a measured
   number, not an estimate).
7. Anti-scope: do **not** instrument any inner loop; do **not** add a default dependency (pprof stays
   feature-gated OFF); do **not** reuse `telemetry.rs`; do **not** make Layer 1 default-on.
