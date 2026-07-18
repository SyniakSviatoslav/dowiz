# Per-Function Production Observability for the dowiz Kernel

**Author:** Opus research pass · 2026-07-18
**Scope:** LIVE / production per-function attribution for the Rust kernel — distinct from the
offline benchmark-coverage gap. Answers the operator's blind spot: `load1` spiked
1.09 → 2.15 → 3.52 → 4.51 → 3.73 over one minute and the existing telemetry could name the
*host* but not the *function*.
**Constraints honored:** Rust-native (no Node/TS), minimal new dependencies, provably-small
hot-path overhead, and *extend the existing `tools/telemetry/` pipeline — do not fork a parallel
system*.

---

## 0. Ground truth — what already exists (verified against source, not memory)

Before proposing anything, the current mechanism, read end to end:

**The collector.** `tools/telemetry/telemetry monitor [iv=15]` runs an infinite `while` loop.
Every `iv` seconds it calls `resource_sample()` (in `lib.sh:146`) which reads `/proc/loadavg`,
`/proc/meminfo`, and `df /`, then emits **one line** via `log_event metric`:

```
{"ts":"2026-07-18T21:50:56Z","kind":"metric","host":"dowiz-dev","kind":"host",
 "sample":"{\"load1\":1.33,\"load5\":0.60,\"mem_pct\":16.4,...,\"nproc\":8}"}
```

The payload is a **JSON string** carried in the `sample` field (double-serialized on purpose —
`log_event` in `lib.sh:69` escapes each value once; the inner object is a `_jesc`-escaped blob).
The literal `"kind"` key appears twice: `log_event` writes `"kind":"metric"` positionally, then
the caller's `kind=host` k/v appends `"kind":"host"` (last-wins). This is the established shape.
Whatever we add must ride the same `log_event metric "kind=<x>" "sample=<escaped-json>"` contract
so the existing `.jsonl` consumers keep parsing.

**The alert path.** The *same* `monitor` loop already implements threshold-triggered friction
alerts (`telemetry:99-108`):

```bash
awk -v l="$load1" -v n="$nproc" 'BEGIN{exit !(n>0 && l/n>=4)}' \
  && friction="CPU saturation load1=$load1 nproc=$nproc"
...
[ -n "$friction" ] && tg_deliver "🔥 friction: $friction"
```

So the repo **already has** a `load1/nproc >= 4` breach detector wired to a delivery mechanism.
This is the hook the on-demand profiler attaches to — no new alerting machinery is needed.

**The native-Rust precedent.** `tools/telemetry/native-trackers/` (1 025 lines, `[dependencies]`
empty — pure `std`, hand-rolled JSON) is the operator's chosen pattern for replacing
interpreter-heredoc compute with a zero-dep native binary reading the *same* JSONL ledgers.
`rust-spool/` (the `telemetry-spool` Telegram drainer) is the same discipline. Any new native
piece I propose must match this: `std`-only, hand-rolled over the fixed schema, no serde on the
hot path.

**What the kernel already has (this is the decisive finding).** `tracing` is a **non-optional,
already-linked dependency** of the kernel — not behind a feature flag:

```toml
# kernel/Cargo.toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

and spans are **already on the hot paths**:

| File:line | Existing span |
|---|---|
| `kernel/src/domain.rs:164` | `info_span!("place_order", id, n_items, channel)` |
| `kernel/src/domain.rs:207` | `info_span!("place_order_priced", ...)` (catalog-authoritative money path) |
| `kernel/src/order_machine.rs:161` | `info_span!("fold_transitions", start, n_steps)` |

`kernel/src/lib.rs:323` ships `init_tracing()` — but it installs
`tracing_subscriber::fmt()` (a *printing* subscriber). It emits log lines; **it does not
aggregate span durations into anything**. That is the entire gap: the instrumentation points
exist and are paid for, but nothing turns span open/close into a per-function latency metric, and
nothing feeds `metric.jsonl`. We are not adding instrumentation. We are adding a *consumer* of
instrumentation that already runs.

---

## 1. Option A — `tracing` spans → duration histograms → `metric.jsonl`

### Mechanism
`tracing-subscriber` (already a dependency) exposes the `Layer` trait over a `Registry`. A ~120-line
custom `Layer` does exactly this and nothing more:

- `on_new_span` / `on_enter`: stamp `Instant::now()` into the span's `extensions` (per-span
  scratch storage the Registry provides).
- `on_close`: `elapsed = now - stamp`; look up the span's static name (`"place_order_priced"`,
  `"decide_settlement"`, …); fold `elapsed` into a per-name accumulator.
- A background flush (every 15 s, matching the host sampler cadence) drains the accumulators to
  stdout as one line per function, which the shell folds into `metric.jsonl`.

The accumulator is a **hand-rolled fixed-bucket log-scale histogram** (e.g. 24 power-of-two
buckets from 16 ns to 256 ms) — matching the `native-trackers` zero-dep discipline. It yields
`count`, `sum_ns`, `max_ns`, and bucket-interpolated `p50`/`p99` **without** pulling `hdrhistogram`
(which would be a new dependency and violate the minimal-deps constraint). This is the same
"hand-roll over a fixed schema rather than link a crate" trade the repo already made for JSON in
`native-trackers`.

### Overhead — real numbers
Two costs matter, and they are very different:

1. **A span that is filtered out** (below the subscriber's level, or its target isn't in the
   registered set) costs approximately a single relaxed atomic load + compare. `tracing`'s design
   is a static `max_level` feature gate plus a runtime `Interest`/`LevelFilter::current()` cache;
   sub-threshold spans are "completely omitted from compilation" or reduced to that one check
   ([tracing docs](https://docs.rs/tracing), and the same zero-when-disabled property Fastrace
   documents for the model — [Fastrace blog](https://fast.github.io/blog/fastrace-a-modern-approach-to-distributed-tracing-in-rust/)).
   Effectively free on the hot path when the layer is not attached.

2. **A span that IS recorded** is dominated by **two `Instant::now()` reads** (enter + close). The
   Fastrace analysis measured `std::Instant::now()` at **~50 ns for two timestamps** (~25 ns each)
   and calls this "one of the reasons most Rust tracing libraries do not trace efficiently"
   ([Fastrace blog](https://fast.github.io/blog/fastrace-a-modern-approach-to-distributed-tracing-in-rust/)).
   Add the map lookup + histogram bump: budget **~60–120 ns per recorded span**.

Whole-system context: PingCAP traced a production KV database with the `tracing`-family model at
**< 5 % performance impact** end to end ([PingCAP](https://www.pingcap.com/blog/how-we-trace-a-kv-database-with-less-than-5-percent-performance-impact/)).
That is with spans *everywhere*. Our proposal instruments **seven** functions, so the ceiling is far
below that.

**Is ~100 ns/call acceptable on our hot paths?** Yes, provably, for these functions.
`place_order_priced`, `decide_settlement`, `commit_after_decide`, `verify_chain`, `dsa::verify`,
and `route` are called **per business event** (an order, a settlement, a signature) — hundreds to
low-thousands per second at most, not in tight numeric inner loops. At 1 000 calls/s × 100 ns =
**100 µs/s = 0.01 % CPU**. The one function to watch is `assert_transition` (called once per edge
inside `fold_transitions`); if a fold replays thousands of transitions it should get a *coarse*
span at the `fold_transitions` level only, not a per-edge span. (See §4 instrumentation list — I
deliberately do not put a span on `assert_transition`.)

### Fit to the pipeline
Excellent. Zero new dependencies (Layer is built from the already-linked `tracing-subscriber`).
Output is the existing `metric.jsonl` line shape. It gives **continuous, 24/7, per-function
latency trend** data (p50/p99 over time) — exactly "is function X getting slower".

### Limits (stated honestly)
- Measures **wall-clock of the span**, not where CPU went *inside* it. If `route` blocks on a lock
  or a page fault, the span is "slow" but you can't see *why* from the histogram alone.
- Only covers functions you chose to instrument. A regression in an *uninstrumented* function is
  invisible.
- It measures the kernel *when the kernel runs*. It says nothing about a `load1` spike caused by
  `rustc` during a build (see §3 caveat).

---

## 2. Option B — `pprof-rs` (in-process sampling profiler, on demand)

### Mechanism
`pprof-rs` (tikv) arms a `setitimer` timer that raises `SIGPROF` at a fixed frequency; the signal
handler captures a backtrace and increments a per-stack counter, producing a flamegraph or
`pprof`-proto output ([pprof-rs README](https://github.com/tikv/pprof-rs)). It runs **in-process**,
needs no external tooling, and is signal-safe. Crucially it can be **started and stopped on
demand** — arm a `ProfilerGuard`, run for N seconds, drop it, serialize the flamegraph.

### Overhead — real numbers
Sampling overhead scales with frequency, independent of call count (the decisive advantage over
per-call instrumentation). Measured overhead table
([oneuptime](https://oneuptime.com/blog/post/2026-01-30-low-overhead-profiling/view)):

| Frequency | Overhead | Use case |
|---|---|---|
| 10 Hz | ~0.1 % | always-on baseline |
| 50 Hz | ~0.5 % | production continuous |
| 99 Hz | ~1 % | production standard |
| 500 Hz | ~3 % | incident investigation |
| 1000 Hz | ~5–10 % | short-term debugging |

Same source: instrumentation-based profilers are **10–50 %**, low-frequency sampling **1–5 %**.
This is *why* a sampler is the right tool for "what is hot RIGHT NOW during a spike" and
per-call instrumentation is not.

### Fit
Answers the question Option A cannot: full-stack CPU attribution across **every** function
(instrumented or not) at the moment of the spike. Triggerable from the existing
`load1/nproc >= 4` breach in `telemetry monitor`.

### Cost to adopt
Adds **one new dependency** (`pprof`, which pulls `backtrace-rs`), and the profiler must be
**compiled into a running binary** that the kernel is exercised through. The kernel is primarily
a library (`crate-type = ["cdylib", "rlib"]`); the natural host is the `lm` / `markov_attractor`
bins or a bench, or a small always-resident kernel service if one is ever run. It cannot attach to
an *already-running foreign* process (unlike `perf`).

---

## 3. Option C — Linux `perf` (external, zero-instrumentation, on demand)

### Mechanism
No code changes at all. On a load breach, shell out to:

```bash
perf record -g -F 99 -p "$(pgrep -f target/release/<kernel-bin>)" -- sleep 10
perf script | stackcollapse-perf.pl | flamegraph.pl > spike-$(date +%s).svg
```

`-F 99` (99 Hz, deliberately off round numbers to avoid lockstep with other timers) is the
documented production-standard sampling rate; attaching to a live PID for a fixed window is the
documented non-invasive pattern ([oneuptime perf guide](https://oneuptime.com/blog/post/2026-01-07-rust-profiling-perf-flamegraph/view)).
Overhead is the same ~1 % at 99 Hz as any sampler (§2 table).

The `perf-event` / `perf-event2` crates wrap `perf_event_open` for *in-process* counter reads, but
note the hardware constraint: "if more counters are requested than the hardware can support the
kernel will timeshare them" ([perf-event2 docs](https://docs.rs/perf-event2/)) — that path is for
counting cache-misses/instructions, not for the per-function flamegraph we want here. For "which
function is hot", `perf record` (sampling) is the right `perf` mode, not the counter crate.

### Fit — and the honest caveat that reframes the whole problem
**Zero new dependencies** (`perf` is a system binary), **zero code instrumentation**, works on the
release binary as-is, and can even run **system-wide** (`perf record -a -F 99 -- sleep 10`).

That system-wide capability exposes something the operator's framing needs to hear:
**the `load1` spike he observed occurred during a *build*.** During `cargo build`, the processes
eating all `nproc` cores are `rustc` / `cargo`, **not** kernel functions. Per-*kernel*-function
attribution during a build spike would show `rustc` frames, because the kernel isn't the thing
running. A **system-wide** `perf record -a` at the moment of breach is the *only* one of these
three options that can answer the actual first question — "is this spike my kernel, or is it just
`rustc`?" — before drilling into which kernel function. This is a real distinction between
*build-time* load and *runtime* kernel hot paths, and the architecture must serve both.

### Cost to adopt
Needs `perf_event_paranoid` low enough (or root/`CAP_PERFMON`) and frame pointers or DWARF
(`-g`) for good stacks — release LTO builds may need `force-frame-pointers`. Output is an SVG /
`perf.data`, not a `metric.jsonl` number, so it's a *deep-dive artifact*, not a continuous metric.

---

## 4. Recommendation — the hybrid, and why it beats any single option

**Primary mechanism = two complementary layers, not three parallel systems:**

### Layer 1 (continuous, always-on): custom `tracing` `Layer` → `metric.jsonl`
- **Zero new dependencies.** Built from `tracing-subscriber`, already linked. This is the single
  biggest reason it wins Option A's slot: the instrumentation is *already paid for*; we only add a
  consumer.
- Aggregates span durations for a **small named set** (below) into a hand-rolled log-bucket
  histogram; flushes every 15 s.
- **Integration point:** mirror the existing `telemetry kernel` subcommand (`telemetry:64-82`),
  which already runs a kernel probe and folds the result via `log_event`. Add a sibling
  `telemetry kernel-spans` (or fold into `bench_run`) that runs the kernel under
  `DOWIZ_SPAN_METRICS=1` (env flag → `init_tracing()` installs the aggregating Layer instead of
  the fmt layer), reads the emitted `SPANMETRICS <json>` lines, and writes:

  ```
  log_event metric "kind=kernel_span" "fn=decide_settlement" \
    "sample=<escaped {count,sum_ns,max_ns,p50_ns,p99_ns}>"
  ```

  This is byte-compatible with the host `metric` line shape (`kind=<x>` + escaped `sample`), so
  every current `metric.jsonl` reader keeps working and the new rows are queryable by `fn`.
- **Overhead:** ~60–120 ns per recorded span (§1), ~0.01 % CPU at realistic event rates. Filtered
  spans are effectively free. Provably small.
- **Answers:** "Is `place_order_priced` / `dsa::verify` regressing week over week?" — the trend
  question, continuously, cheaply.

### Layer 2 (on-demand, triggered): `perf record` from the existing load-breach hook
- **Zero new dependencies** — `perf` is a system binary. This is the deliberate choice of `perf`
  over `pprof-rs` for the deep-dive slot: the minimal-dependency constraint tips it, and `perf`'s
  system-wide mode uniquely answers the "`rustc` vs kernel" question (§3).
- **Integration point:** extend the friction branch already in `telemetry monitor`
  (`telemetry:105`). When `load1/nproc >= 4` fires, in addition to the Telegram alert, capture a
  bounded profile:

  ```bash
  perf record -a -g -F 99 -o "/tmp/spike-$(date +%s).data" -- sleep 10   # system-wide first
  # then, if a kernel PID is resident:
  perf record -g -F 99 -p "$(pgrep -f target/release/lm)" -- sleep 10
  ```

  Emit a `log_event alert "kind=cpu_spike" "profile=<path>"` so the artifact is indexed in the
  existing `alert.jsonl` ledger. One-shot, ~1 % overhead for the 10 s window, no steady-state cost.
- **Fallback:** if `perf_event_paranoid` is locked down in some target environment, swap in
  `pprof-rs` compiled into the kernel bin (Option B) behind an opt-in `profiling` feature flag —
  same trigger, in-process, no `perf` permissions needed. Keep it feature-gated and OFF by default
  so the production cdylib carries zero profiler symbols (the same discipline `kernel/Cargo.toml`
  already applies to `chaos`, `pq`, `gpu`).

### Why the hybrid beats either alone
- **Tracing alone** gives cheap continuous trends but only for the 7 functions you picked, only
  wall-clock (not CPU cause), and is blind to a build-time spike. It can tell you `route` got
  slower; it cannot tell you *why* or whether the box-level spike was even the kernel.
- **Sampling alone** gives full-stack "what's hot now" but only when triggered — it has no memory,
  no 24/7 trend, and can't cheaply assert "`decide_settlement` p99 has been climbing for a week."
  Running it continuously at a trend-useful frequency would cost 1–10 % forever.
- **Together:** Layer 1 is the always-on smoke detector feeding `metric.jsonl` (find *that* a
  function regressed, near-free); Layer 2 is the on-demand camera that opens only when the box is
  actually on fire (find *why*, ~1 %, ephemeral). Each covers precisely the other's blind spot,
  and both reuse `tools/telemetry/` plumbing (`log_event`, the `monitor` loop, the JSONL ledgers)
  rather than forking a new system.

---

## 5. First functions to instrument (Layer 1) — with verified symbols

The sibling audits `OPUS-PERF-KERNEL-AUDIT-2026-07-18.md` / `OPUS-PERF-BEBOP-AUDIT-2026-07-18.md`
do **not exist yet** at time of writing (confirmed: `find /root/dowiz -name 'OPUS-PERF*'` → empty),
so this set is derived from the session's established critical-path knowledge and cross-checked
against actual source symbols. When those audits land, reconcile against their hot-path findings.

| # | Function (verified path) | Why first | Instrumentation |
|---|---|---|---|
| 1 | `domain::place_order_priced` — `kernel/src/domain.rs:207` | Money: catalog-authoritative pricing, the client-price-tamper close. | **Span already exists** — just needs the aggregating Layer. |
| 2 | `domain::place_order` — `kernel/src/domain.rs:164` | Money: legacy untrusted-price path (compare vs #1). | **Span already exists.** |
| 3 | `order_machine::fold_transitions` — `kernel/src/order_machine.rs:161` | Order Law: the deterministic reducer the WS bus replays. | **Span already exists.** Keep the span here; do **not** span `assert_transition` (per-edge, inner loop). |
| 4 | `event_log::EventLog::commit_after_decide` — `kernel/src/event_log.rs:366` | The decide→commit Law + SHA3 hash-chain append — the write hot path. | Add one `info_span!("commit_after_decide")`. |
| 5 | `ports::payment::decide_settlement` — `kernel/src/ports/payment.rs:367` | Money settlement + capability-auth verify (calls `verify_chain`). | Add `info_span!("decide_settlement")`. |
| 6 | `ports::agent::cap::verify_chain` — `kernel/src/ports/agent/cap.rs` | Capability-cert chain verify — admission/dispatch gate, signature-bearing. | Add `info_span!("verify_chain")`. |
| 7 | `pq::dsa::verify` — `kernel/src/pq/dsa.rs:1003` | ML-DSA-65 signature verify — the single most CPU-expensive verify on any hot path; the prime deep-dive suspect. | Add `info_span!("mldsa_verify")` (gate with `pq` feature so default build is untouched). |
| 8 | `router::route` — `kernel/src/router.rs:90` | Dispatch: A*/contraction-hierarchy routing (the matcher/assignment hot path). | Add `info_span!("route")`. |

Rationale for the cut line: these are the money / order / dispatch / crypto-verify paths named as
critical, they are **per-event** (safe for ~100 ns spans), and five of eight are signature- or
hash-bearing (the functions most likely to dominate a real CPU spike). Numeric inner loops
(`assert_transition`, eigensolver kernels, histogram bumps) are **deliberately excluded** — a span
there would violate the "must not slow the hot path" constraint, and a sampler (Layer 2) covers
them at zero per-call cost anyway.

---

## 6. Concrete build order

1. **Layer 1 core** — add `SpanMetricsLayer` (~120 lines) in a new `kernel/src/telemetry.rs`,
   built on `tracing-subscriber` (no new dep). Hand-rolled log-bucket histogram, `on_close`
   aggregation, `flush()` → `SPANMETRICS <json>` on stdout.
2. **Wire `init_tracing()`** — when `DOWIZ_SPAN_METRICS=1`, install `SpanMetricsLayer` (else the
   existing fmt layer). One `if` in `kernel/src/lib.rs:323`.
3. **Add spans #4–#8** — one `info_span!(...).entered()` line each, matching the existing style in
   `domain.rs` / `order_machine.rs`. (#7 behind `cfg(feature = "pq")`.)
4. **`telemetry kernel-spans` subcommand** — mirror `telemetry kernel` (`telemetry:64-82`); run the
   instrumented target, fold emitted lines via `log_event metric "kind=kernel_span" "fn=..."`.
5. **Extend the `monitor` friction branch** (`telemetry:105`) — on `load1/nproc >= 4`, fire
   `perf record -a -g -F 99 -- sleep 10` and `log_event alert "kind=cpu_spike" "profile=<path>"`.
6. **(Deferred fallback)** — `profiling` feature flag pulling `pprof` into a kernel bin, same
   trigger, only if `perf` permissions prove unavailable in a target environment.

Net new dependencies for the primary design: **zero** (Layers 1 and 2 both reuse already-present
tooling). The only dependency the design ever adds is the *optional, feature-gated, off-by-default*
`pprof` fallback — consistent with how the kernel already gates `chaos`/`pq`/`gpu`.

---

## Sources

- Overhead-by-frequency table + instrumentation-vs-sampling percentages — [oneuptime, "How to Create Low-Overhead Profiling" (2026-01-30)](https://oneuptime.com/blog/post/2026-01-30-low-overhead-profiling/view)
- `perf record -F 99 -p PID`, attach-to-running, 99 Hz rationale — [oneuptime, "Profile Rust with perf, flamegraph, samply" (2026-01-07)](https://oneuptime.com/blog/post/2026-01-07-rust-profiling-perf-flamegraph/view)
- pprof-rs SIGPROF/`setitimer` sampling mechanism + frequency knob — [tikv/pprof-rs README](https://github.com/tikv/pprof-rs)
- Whole-DB tracing at < 5 % impact (tracing-family model) — [PingCAP, "How We Trace a KV Database with Less than 5% Performance Impact"](https://www.pingcap.com/blog/how-we-trace-a-kv-database-with-less-than-5-percent-performance-impact/)
- `Instant::now()` ≈ 50 ns for two timestamps; zero-cost-when-disabled model — [Fastrace blog](https://fast.github.io/blog/fastrace-a-modern-approach-to-distributed-tracing-in-rust/)
- `tracing` static max-level filtering / disabled-span cost model — [tracing docs](https://docs.rs/tracing)
- `perf_event_open` counter timesharing constraint — [perf-event2 docs](https://docs.rs/perf-event2/)
