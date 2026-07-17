# BLUEPRINT — Phase 24: Native Runtime Telemetry — Ring-Buffer Flight Recorder + Explainable Latency Events (2026-07-17)

> Planning document; writes no product code. Built under the Detailed Planning Protocol
> (`AGENTS.md` §"Detailed Planning Protocol"): ground-truth-first, inline DECART, 2-question doubt
> audit, Anu/Ananke check. Style contract: no metaphor; every load-bearing statement carries a
> `file:line` cite, a web citation, or is tagged **(proposal)**.
>
> **Branch context** (per the roadmap audit's own finding, `MASTER-ROADMAP …:320-324`): all cites
> verified live on `feat/harness-llm-backend` at `cc3d5c916`.

---

## 0. Scope banner + the P08 relationship, decided honestly

**Operator ask:** every feature/layer must use native, fastest-possible logging + telemetry +
benchmarking, with automatic real-time detection, reduction, and historical tracking of
latency/telemetry/log issues. Any spike/anomaly must be logged **and explained** (cause, not just
symptom). The techniques must be adapted from how Linux itself monitors resources (procfs,
perf_events, eBPF, cgroups, ring buffers). Binding constraint: the observability layer must never
become the bottleneck — no lock contention, no allocation on the hot path, no synchronous I/O.

**Is this P08 deepened, or a distinct phase?** Decided from what `BLUEPRINT-P08-typed-local-observability.md`
actually says, not assumed:

- P08 owns a **fixed anchor set** (M8, S7, S8, D7, E46, E47, F29, F31, F32, F36, F39, F40) and four
  mechanisms: typed `/proc/self` metrics (§2), typed log schema + spool-backed local sink (§3), the
  **CI** claim-latency anomaly detector (§4, the only part built — `tools/ci-truth/src/main.rs:446-659`),
  and signed-envelope/opt-in egress (§5).
- P08 does **not** cover: a hot-path emission primitive (its producer story is `Spool::append`,
  which allocates and needs external locking — §1.3 below), runtime latency baselines or runtime
  anomaly detection (its §4 is explicitly scoped to the CI commit-verification ledger; the code
  comment at `main.rs:547-554` says so verbatim: "Do NOT mistake this file for the complete M8
  observability system"), PSI/host cause-attribution, or bounded downsampled history.
- P08's own upgrade path already names this phase's core primitive without building it:
  `KERNEL-OBSERVABILITY-DECART-2026-07-15.md:23-24` — "DECISION: #1 C-ABI counter exports, with
  **#5 ring buffer as the upgrade path when event-level (not just aggregate) telemetry is needed**."

**Verdict: distinct phase — P24 — that DEPENDS on P08.** P24 consumes P08's typed-schema +
local-sink design (adding `LogEvent` variants, never a competing schema) and **generalizes P08 §4's
anomaly pattern from "CI commit latency" to "any runtime latency."** It re-owns none of P08's
anchors. P24 is also the already-named "#5 ring buffer" upgrade path of the kernel-observability
DECART, made concrete.

**SCOPE RULE** (same as P08 §0 and RCI §0): this is canonical-repo, single-host/per-hub local
observability. At runtime a hub is sovereign (M5/M9) and routes its own telemetry as it likes.
Everything here is local-only by construction; remote egress remains governed exclusively by P08 §5
(signed opt-in marker). P24 adds **zero** new egress paths.

---

## 1. Ground truth — what exists today (all cites live-verified this session)

### 1.1 Host gauges from `/proc` — two parallel implementations (a real drift)

- `tools/telemetry/hetzner-exporter/src/lib.rs` — pure-std gauges `[disk_pct, load1, mem_pct]`
  computed **on request, no background loop** (`Cargo.toml` description: "Near-zero CPU: computes
  on request"), from `/proc/loadavg` (`lib.rs:95-98`), `/proc/meminfo` (`lib.rs:106-125`), and —
  **note** — disk via a spawned `df -B1 /` subprocess (`lib.rs:75-86`), with its own comment trail
  showing the author talked themselves out of the syscall (`lib.rs:64-74`).
- `tools/telemetry/native-trackers/src/main.rs:716-831` (`hetzner-serve` subcommand) — the same
  gauges, but disk via **direct `statvfs64` FFI** (`main.rs:788-816`, "always linked on Linux; no
  Cargo dep"), plus `/proc/stat` CPU (since-boot ratio, `main.rs:744-762` — not a two-snapshot
  delta) and `/proc/net/dev` (`main.rs:832-839`).

Two disk-gauge implementations for the same number is exactly the dual-authority shape this repo
keeps finding (`kernel/src/markov.rs:1-8` hazard note; RCI resolution §0). And the `df` spawn is a
fork+exec (milliseconds) where a syscall is ~1 µs — an observability path that is itself the
slowest part of its own request. §6 DECART consolidates on the FFI.

### 1.2 Wire format + ledger conventions

- `tools/telemetry/native-ser/src/lib.rs` — canonical raw-LE-f64 wire (`wire_f64`/`unwire_f64`,
  `lib.rs:18-41`), schema-ordered obj↔native (`native_of`/`obj_of`), hand-rolled JSON edge, no
  serde. "The canonical form is the kernel C-ABI `field_metrics` wire shape (no parser, no
  allocator) — the speed floor" (`lib.rs:10-11`).
- `tools/telemetry/native-trackers/src/main.rs:1-22` — zero-dep JSONL ledger folds
  (`track_record.jsonl`, `false_claims.jsonl`), subcommand pattern, hand-rolled tolerant JSON
  parse (`parse_obj`, skips malformed lines, never panics).
- `docs/ledger/claim-latency.jsonl` + `docs/ledger/claim-latency-anomalies.jsonl` — the advisory
  JSONL ledger convention (append-only, greppable, one typed object per line).

### 1.3 Queues and hot-path primitives already in the kernel

- `kernel/src/spool.rs` — pure crash-safe claim/ack queue state machine (append→claim→ack,
  `reclaim` on crash, backpressure `is_full`). **Not** a hot-path primitive: `append` takes
  `&mut self` (needs an external `Mutex` cross-thread) and allocates a `String` per record
  (`spool.rs:70-82`). Its job is durable cross-process handoff, and it does that job well.
- `kernel/src/token_bucket.rs:14-25` — the kernel's existing lock-free pattern precedent:
  `AtomicU64` with f64 bit-cast, plain-std, no tokio.
- `kernel/src/geo.rs:39-41` — `ema_next(prev, sample, alpha)`: the scalar steady-state Kalman
  filter (per `kernel/src/kalman.rs:3-6`, which generalizes it to full n-D predict/correct).
- `tracing` spans exist on kernel hot paths (`order_machine.rs:145,151`, `domain.rs:130,138`) —
  P08 §7's S7 half, inherited, not rebuilt here.

### 1.4 The anomaly pattern already proven, in miniature

`tools/ci-truth/src/main.rs:446-659` (`claim-latency-check`) is the exact "log a spike AND explain
it" pattern, scoped to CI:

- One **named, documented, tunable floor constant** — `MIN_SECONDS_PER_100_LINES: f64 = 5.0`
  (`main.rs:464`), with the worked example in its doc comment.
- A **pure predicate** — `is_anomaly(delta_s, diff_loc)` calling `plausible_min_seconds(diff_loc)`
  (`main.rs:478-487`) — testable with no I/O.
- An **explained JSONL record**: every flagged row carries all inputs, the computed threshold, and
  a mechanically-composed `reason` string built only from typed fields (`main.rs:608-623`).
- **Advisory, exit 0 always** (`main.rs:651-658`), idempotent by `commit_sha` de-dup.

P24's runtime detector ports this *pattern* (named constants, pure predicate, explained record,
advisory posture), not this code — the inputs differ (streaming per-site durations vs. a
per-commit ledger).

### 1.5 The RCI H1 lesson — binding on this design's ingestion path

`docs/design/realtime-change-intelligence-2026-07-17/resolution.md:54-66` (H1): RCI round 1
pointed four cross-process producers at a chain whose substrate documents itself single-writer
(`event_log.rs` module doc; `set_tip` plain last-writer-wins, no compare-and-swap). The honest fix
was **not** to add CAS but to remove multi-producer ingestion entirely — "single-writer is enforced
by construction, not by protocol discipline" — with the pinned residue: any future chain requires
"CAS-or-single-writer" as a precondition. P24's ingestion is therefore designed **single-producer
per ring from the start** (§3.2): no CAS is needed because no slot is ever contended, by
construction.

### 1.6 Adjacent organs that stay separate

- `tools/loop-signals/` + `kernel/src/bin/markov_attractor` (`check.sh:26-30`) — the Markov
  attractor detector over **tool-outcome tokens** (agent-behavior domain). Different signal domain
  from runtime latency; both write advisory JSONL ledgers. §5 rules on sharing.
- RCI (Option D′) — **code-change** analysis, pull-based `rci derive`, dev-time. Different concern
  from runtime behavior; §5 rules on the one cheap, fail-open read P24 takes from it.

---

## 2. Linux's real-time monitoring architecture — the techniques being ported

Web-grounded synthesis (citations per item). Depth is limited to what §3 actually reuses.

### 2.1 procfs: counters as a byproduct, reading as a snapshot

`/proc/stat`, `/proc/[pid]/stat`, `/proc/meminfo` cost ~nothing to *maintain* because the kernel
already updates these counters as a side effect of work it must do anyway. The default
`TICK_CPU_ACCOUNTING` config charges each timer-tick jiffy to whatever mode the CPU was in —
"basic tick based cputime accounting… on per jiffies granularity" — with more precise tiers
(`VIRT_CPU_ACCOUNTING_*`) explicitly documented as costing "a small performance impact" per
kernel-boundary timestamp read ([init/Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/init/Kconfig)).
"Reading" is a formatting pass over already-existing counters via the `seq_file` iterator at read
time ([seq_file docs](https://docs.kernel.org/filesystems/seq_file.html)); there is no active
instrumentation agent. All fields are cumulative-since-boot in `USER_HZ` units
([proc_stat(5)](https://man7.org/linux/man-pages/man5/proc_stat.5.html)), so utilization is a
**consumer** derivation: two snapshots + delta over a monotonic interval — the kernel never
computes a percentage for you. This is exactly the discipline P08 §2 already specifies for
`/proc/self/stat` `utime`/`stime` (fields 14/15, clock ticks, `_SC_CLK_TCK`).

**PSI (Pressure Stall Information)**, `/proc/pressure/{cpu,memory,io}` (kernel ≥ 4.20): per-resource
`some` ("at least some tasks are stalled") and `full` (all non-idle tasks stalled) lines with
`avg10/avg60/avg300` (% of wall-time stalled) plus `total` (µs)
([PSI kernel doc](https://docs.kernel.org/accounting/psi.html)). PSI adds no instrumentation
points — it aggregates task-state transitions the scheduler already tracks. It is the kernel's own
*cause-attribution* primitive: Facebook's stated production payoff was being able to "trivially
link latency spikes and throughput drops to shortages of specific resources"
([LWN 759781](https://lwn.net/Articles/759781/)) — precisely the "why" half of an explainable
event, readable for the cost of one small file read. PSI also supports kernel-side **triggers**
(write `some 150000 1000000` to the file, then `poll()`) — noted as the push-notification upgrade
path if 250 ms drain-tick polling ever proves too coarse; not used in v1.

**Ported as:** never re-measure what the OS already accounts. The host/process context in every
anomaly capsule (§4) is read from `/proc` + PSI at flag time — off the hot path — not sampled by a
resident agent.

### 2.2 perf_events / eBPF: sample, aggregate in place, cost zero when off

Three load-bearing ideas:

1. **Sampling instead of full capture** — `perf_event_attr.sample_freq` self-adjusts the period to
   hit a target rate ([perf_event_open(2)](https://man7.org/linux/man-pages/man2/perf_event_open.2.html));
   Gregg's standard is `-F 99` ("instead of 100 Hertz, … to avoid accidentally sampling in
   lockstep with some periodic activity"), because overhead is bounded by the sample rate, not the
   event rate ([brendangregg.com/perf.html](https://www.brendangregg.com/perf.html)).
2. **In-kernel aggregation** — biolatency-style eBPF tools compute deltas and store them "in a
   log2 histogram"; "The summarization is all done in kernel context, for efficiency" — only the
   aggregated map crosses to userspace, periodically
   ([brendangregg.com/ebpf.html](https://www.brendangregg.com/ebpf.html)).
3. **Zero cost when disarmed** — static tracepoints compile to "a single atomic 'no-op'
   instruction (5 bytes on x86)" via static keys + runtime code patching
   ([static-keys doc](https://docs.kernel.org/staging/static-keys.html)); documented kprobe cost
   when armed is ~0.49–0.77 µs classic, ~0.05–0.06 µs optimized
   ([kprobes doc](https://docs.kernel.org/trace/kprobes.html)) — an unconsumed probe costs
   approximately nothing.

**Ported as:** per-site **always-on aggregates** (count/sum/max as relaxed atomics — the "in-kernel
aggregation") with **full ring events only for anomalous or 1-in-N-sampled durations** (the
"sampling"), and a compile-time/config gate that reduces a disarmed site to two `Instant::now()`
calls + three relaxed atomic ops (the "cheap when off" — true zero requires a feature gate, offered
but not default, §3.4).

### 2.3 Ring buffers: the standard low-latency producer/consumer primitive

- **perf ring buffer**: per-CPU, mmap'd, single-producer (kernel, that CPU) / single-consumer
  (userspace). `data_head` monotonically increases (index = `head & (size−1)`); "after reading the
  data_head value, user space should issue an rmb()". Two full-buffer modes exist: with
  `PROT_WRITE` the consumer acks via `data_tail` and "the kernel will not overwrite unread data"
  (lose-new-on-full); mapped read-only it overwrites oldest
  ([perf_event_open(2)](https://man7.org/linux/man-pages/man2/perf_event_open.2.html)). Per-CPU-ness
  is what removes cross-producer contention; the consumer merges per-CPU streams.
- **BPF ringbuf** (kernel ≥ 5.8): replaced per-CPU perf buffers for many uses with ONE MPSC ring —
  multi-producer reservation runs under "a lightweight spinlock"; reserve/commit means "if
  reservation succeeds, commit cannot fail"; cross-CPU event ordering is preserved; consumer is
  epoll-notified ([Nakryiko, BPF ring buffer](https://nakryiko.com/posts/bpf-ringbuf/);
  [kernel ringbuf doc](https://docs.kernel.org/bpf/ringbuf.html)). The instructive part for P24:
  the moment you want multiple producers on ONE ring, you pay a lock or CAS — there is no free MPSC.
- **io_uring SQ/CQ**: two SPSC rings in shared mmap'd memory; the documented idiom is
  `io_uring_smp_load_acquire()` on the peer's cursor before reading and
  `io_uring_smp_store_release()` to publish your own; indexing by `tail & *ring_mask`
  (power-of-two); with `SQPOLL` there is no syscall at all on the fast path
  ([io_uring(7)](https://man7.org/linux/man-pages/man7/io_uring.7.html)).
- **printk ring buffer** (fully lockless since **5.10**): two rings — a *descriptor ring*
  (per-record state) + a *data ring* (bytes); multi-writer safety via cmpxchg on descriptor state;
  drop-oldest-on-full within a fixed byte budget — what `dmesg` reads
  ([kernelnewbies 5.10](https://kernelnewbies.org/Linux_5.10); [LWN 800946](https://lwn.net/Articles/800946/)).
  It took the kernel a two-ring CAS-based descriptor scheme to make *multi-producer
  overwrite-oldest* lockless — evidence that this is the *hard* variant, to be avoided when SPSC
  suffices.

The SPSC memory-ordering discipline, exactly (this is the whole correctness argument):
**producer** writes the record bytes into the slot, *then* publishes `head.store(head+1, Release)`;
**consumer** reads `head.load(Acquire)` (which makes the record bytes visible), consumes slots
`[tail, head)`, *then* `tail.store(head, Release)`; producer checks `tail.load(Acquire)` for
capacity. The release store guarantees data writes are visible before the index that advertises
them; the acquire load prevents the consumer's data reads from being hoisted above the index read.
**One writer per cursor ⇒ no CAS anywhere**; MPSC breaks this because producers race for the same
reservation cursor.

**Ported as:** §3.2's `ring.rs` — SPSC by construction, one ring per producer thread, one drainer
merging rings (the perf per-CPU pattern), fail-on-full with a drop counter (the perf mode) rather
than overwrite-oldest (the printk mode) — see §3.2 for the reasoned rejection.

### 2.4 cgroups v2: accounting as a byproduct of mandatory work

`memory.current`, `cpu.stat`, `io.stat` are cheap for the same reason as §2.1: the memory
controller must charge pages to enforce limits anyway ("a memory area is charged to the cgroup
which instantiated it and stays charged… until the area is released" — charged once, at
allocation, not per read), `cpu.stat`'s `usage_usec`/`nr_throttled` roll up the same scheduler
accounting as `/proc/stat`, and `io.stat` increments in the block-layer submission path; reading a
cgroup file formats an existing number
([cgroup-v2 doc](https://docs.kernel.org/admin-guide/cgroup-v2.html)). **Ported as:**
P24's per-site aggregates are incremented at points that already compute the duration (a bench
loop, a request handler, the spool drainer's send path) — the counter update rides work that is
already happening; there is no separate measurement pass anywhere in the design.

### 2.5 Bounded history: round-robin archives + max-preserving consolidation

- **RRDtool**: an RRD "is created at its final, full size and filled with UNKNOWN data" — fixed
  preallocation, O(1) writes, no growth ever; samples consolidate into round-robin archives
  declared `RRA:CF:xff:steps:rows` with CF ∈ {AVERAGE, MIN, MAX, LAST} at multiple resolutions
  (full-res last hour, 1-min last day, …) ([rrdcreate(1)](https://oss.oetiker.ch/rrdtool/doc/rrdcreate.en.html)).
- **Prometheus TSDB / Gorilla**: XOR float compression + delta-of-delta timestamps ("about 96% of
  all timestamps can be compressed to a single bit") → ~1.37 bytes/sample (Gorilla, VLDB'15;
  [summary](https://blog.acolyer.org/2016/05/03/gorilla-a-fast-scalable-in-memory-time-series-database/));
  2 h blocks, retention by deleting whole expired blocks
  ([Prometheus storage docs](https://prometheus.io/docs/prometheus/latest/storage/)). Downsampling
  is external (recording rules / Thanos compactor: raw → 5 m after 40 h, 5 m → 1 h after 10 d) and
  — decisive detail — each downsampled point stores **count/sum/min/max** aggregates, not a bare
  mean ([Thanos compact docs](https://thanos.io/tip/components/compact.md/)).
- **printk/dmesg**: one fixed byte budget, oldest records overwritten (§2.3).

The known downsampling failure mode: mean-only consolidation erases spikes — the exact events this
phase exists to keep. RRD's MAX consolidation function and Thanos's per-point min/max both exist
specifically to prevent it. **Ported as:** §3.5's tiers store `{mean, max, count}` per slot — a
spike survives every tier because `max` is carried, not averaged away. Compression is *rejected*
(§6): fixed-slot LE f64 via `native-ser` is O(1)-seekable, crash-tolerant (a torn slot corrupts one
slot), and the byte budget (≈4.5 MB total) makes compression's complexity unjustifiable here.

### 2.6 Explainability precedents

- **PSI as cause attribution** (§2.1) — which resource stalled, kernel-maintained; `full` vs
  `some` further separates "everything blocked" from "partial contention"
  ([LWN 759781](https://lwn.net/Articles/759781/)).
- **USE method** (Brendan Gregg): "For every resource, check utilization, saturation, and errors"
  — a *complete causal checklist over resources* rather than a metric grab-bag
  ([brendangregg.com/usemethod.html](https://www.brendangregg.com/usemethod.html)). Mapping:
  `/proc/stat`/`cpu.stat` give U, PSI + `procs_blocked` give S, error counters give E. The
  capsule's context fields (§4.2) are exactly a one-shot USE snapshot.
- **OpenMetrics/Prometheus exemplars**: a histogram bucket may carry one reference out of the
  metric world — "Exemplars are references to data outside of the MetricSet. A common use case are
  IDs of program traces" ([OpenMetrics spec](https://github.com/prometheus/OpenMetrics/blob/main/specification/OpenMetrics.md)) —
  so a latency spike links to one concrete request instead of requiring log archaeology. Ported as
  the capsule's `prelude` (the ring events immediately preceding the flagged one: the concrete
  operations that were in flight) — one representative raw-event set per anomaly, near-zero extra
  cost, converts every aggregate anomaly into a drill-down handle.

---

## 3. Design — the minimal native primitive set

Four pieces. Zero new external dependencies. One new kernel module, three extensions.

### 3.1 Two-tier emission: aggregates always, events on anomaly/sample

Every instrumented site (a `u16` id from a static, checked-in site table — closed set, F32
discipline) maintains:

```
// per-site, static storage, written by the owning thread only
struct SiteAgg { count: AtomicU64, sum_ns: AtomicU64, max_ns: AtomicU64 }  // relaxed ordering
```

The hot-path cost of a span is: 2× `Instant::now()` (vDSO `CLOCK_MONOTONIC`, no syscall) + 3
relaxed atomic RMWs + one branch. A full `RingEvent` is pushed **only** when (a) the duration
trips the site's anomaly predicate (§4.1), or (b) 1-in-N sampling selects it (N per-site, default
64), or (c) the site is marked always-emit (rare, e.g. order-state transitions). This is §2.2's
aggregate-in-place + sample discipline: ring traffic scales with anomalies, not with load.

### 3.2 `kernel/src/ring.rs` — the SPSC ring (the one new module)

**(proposal — exact signatures, per protocol step 4):**

```rust
/// Fixed-size POD event. 32 bytes, no heap, no Drop.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct RingEvent {
    pub seq: u64,        // producer-local monotonic sequence
    pub t_mono_ns: u64,  // CLOCK_MONOTONIC ns at emission
    pub site: u16,       // static site-table id (closed set)
    pub kind: u8,        // EventKind: Duration | Gauge | Marker (closed enum)
    pub flags: u8,       // bit0: sampled, bit1: anomalous, rest reserved
    pub _pad: u32,
    pub value: u64,      // duration_ns for Duration; f64 bit-cast for Gauge
}

/// Single-producer single-consumer bounded ring. Capacity = power of two.
/// SINGLE-WRITER BY CONSTRUCTION (RCI resolution H1): exactly one producer
/// thread owns push(); exactly one drainer owns drain(). No CAS exists in
/// this module — enforced by the type system (Producer/Consumer halves are
/// !Clone and moved, not shared).
pub struct Ring { /* boxed [RingEvent; N], head: AtomicU64, tail: AtomicU64, dropped: AtomicU64 */ }

pub struct RingProducer(/* &'static Ring or Arc half */);
pub struct RingConsumer(/* the other half */);

pub fn ring(capacity_pow2: usize) -> (RingProducer, RingConsumer);

impl RingProducer {
    /// Lock-free, alloc-free, wait-free. On full: increments `dropped`,
    /// returns false. NEVER blocks, NEVER overwrites, NEVER silent-drops
    /// (the drop count is itself telemetry, drained with every batch).
    pub fn push(&self, ev: RingEvent) -> bool;
}
impl RingConsumer {
    /// Copies at most `out.len()` pending events; returns (n, dropped_since_last).
    pub fn drain(&self, out: &mut [RingEvent]) -> (usize, u64);
}
```

- **Memory ordering** is §2.3's discipline verbatim: `push` writes the slot then
  `head.store(Release)`; `drain` does `head.load(Acquire)` … `tail.store(Release)`; `push` checks
  `tail.load(Acquire)` for space. One writer per cursor.
- **Fail-on-full + drop counter, not overwrite-oldest** — reasoned, not defaulted: printk-style
  lockless overwrite-oldest requires the producer to also advance `tail`, making both sides write
  both cursors — the exact CAS-or-lock territory H1 forbids entering casually (and the kernel
  needed a descriptor-ring scheme to do it right, §2.3). With a live drainer the ring never fills;
  if the drainer dies, dropping *new* events with an exact count is honest and bounded. The drop
  count is emitted into the history store, so a saturated ring is itself an explainable event.
- **MPSC is composed, never built**: N producer threads ⇒ N rings (perf's per-CPU pattern), one
  drainer scans all registered rings. No shared write cursor exists anywhere.
- Pure `core::sync::atomic` + `alloc` — no_std-compatible, mirrors `spool.rs`'s pure-state-machine
  placement and `token_bucket.rs`'s atomics precedent. Property tests: seq gap-freeness (every
  gap == a recorded drop), FIFO order, and a 2-thread stress test asserting no torn reads
  (`RingEvent` round-trips bit-identical).
- **Relation to `spool.rs`** (no duplication): ring = in-process, lossy-by-declared-drop,
  nanosecond-scale emission. Spool = cross-process, crash-safe, claim/ack durability. The drainer
  is the bridge: ring → (aggregate, detect, capsule) → JSONL/spool. This mirrors perf exactly:
  in-kernel ring → userspace consumer → disk.

### 3.3 The drainer thread (per long-lived process)

One background thread per process (same posture as `rust-spool`'s drainer,
`rust-spool/src/main.rs:9-14`): every `DRAIN_INTERVAL_MS = 250` (named constant) it drains all
rings, folds aggregates, runs the anomaly predicate's slow half, writes history slots (§3.5), and
appends capsules (§4) to the local ledger. All file I/O lives here — the hot path never touches a
file, a lock, or an allocator. If the drainer panics, sites keep counting into aggregates and the
ring drop counter — degradation is bounded and visible, never a work-path stall.

### 3.4 Host/process context sampling — extend, don't rebuild

- Extend the **existing** gauge surface (one implementation after §6's consolidation) with:
  `/proc/pressure/{cpu,memory,io}` `some avg10` + `total` (PSI, §2.1), `/proc/self/stat`
  utime/stime **two-snapshot deltas** (fixing the since-boot-ratio limitation at
  `native-trackers/src/main.rs:744-762`), and `/proc/self/status` VmRSS. All are on-request
  snapshot reads (procfs discipline) — no resident sampler.
- P08 §2's typed `ProcCpuSample`/`MemSample` shapes are the schema these land in — consumed, not
  redesigned.
- Feature gate `telemetry-off` (**proposal**): compiles `span!` sites to nothing for the wasm/
  empty-import kernel build, honoring the bebop-core constraint recorded in
  `KERNEL-OBSERVABILITY-DECART-2026-07-15.md` — the ring is a *native-surface* primitive; the wasm
  kernel keeps C-ABI counters only.

### 3.5 Bounded history — RRD tiers over the native-ser wire

Fixed-size, preallocated, seek-addressed binary files (LE f64 slots via `native-ser::wire_f64` —
the existing canonical layout, `native-ser/src/lib.rs:18-24`), under the existing gitignored
telemetry logs dir:

| Tier | Resolution | Slots | Window | Per-slot payload |
|---|---|---|---|---|
| T0 | 10 s | 8 640 | 1 day | per-series `{mean, max, count}` |
| T1 | 5 min | 8 928 | 31 days | same, consolidated from T0 |
| T2 | 1 h | 8 760 | 1 year | same, consolidated from T1 |

Series = host gauges + PSI + per-site latency aggregates. ≈24 series × 3 f64 × 8 B × 26 328 slots
≈ **4.5 MB, fixed forever** (computed at creation; the file never grows — RRD's property).
Consolidation always carries `max` (§2.5 — spikes must survive every tier) and `count` (so means
recombine correctly). Write pattern: one `pwrite`-equivalent per tier per interval, from the
drainer only. A torn write corrupts one slot, detectable by its embedded slot-epoch (**proposal**:
first f64 of each slot = interval epoch; a mismatched epoch reads as a gap, never as fake data —
typed absence, P08 §2's honesty rule).

Capsule ledger (`…/anomalies.jsonl`) is bounded by rotation at `CAPSULE_ROTATE_LINES = 4096`
(named constant): rotate to `.1` and start fresh — two files max, printk's fixed-budget model at
file granularity.

---

## 4. The explainable event — what makes "explained" different from "flagged"

### 4.1 Detection: the claim-latency pattern generalized to streaming

Per-site baseline, updated by the **drainer** (never the hot path), using the kernel's existing
scalar filter:

```
mean' = ema_next(mean, x, LAT_EMA_ALPHA)            // geo.rs:39 — the 1-D steady-state Kalman
dev'  = ema_next(dev, |x - mean|, LAT_DEV_ALPHA)    // EMA of absolute deviation (robust scale)
flag  ⇔ x > mean + K_DEV * dev  AND  x > MIN_FLAG_NS
```

Named, documented, tunable constants in the claim-latency style (`main.rs:456-464` precedent):
`LAT_EMA_ALPHA = 0.05`, `LAT_DEV_ALPHA = 0.05`, `K_DEV = 6.0`, `MIN_FLAG_NS = 1_000_000` (1 ms
absolute floor so µs-scale sites don't flag noise) — each with a worked example in its doc
comment, each exercised by a pure RED→GREEN test (`is_latency_anomaly(x, mean, dev) -> bool` is
pure, like `is_anomaly` at `main.rs:485-487`). The floor direction (implausibly *fast*, the
BRAIN-TOPOLOGY pattern) remains available per-site via an optional `min_plausible_ns`, directly
mirroring `plausible_min_seconds` — the generalization the operator asked for.

**Kalman posture:** `ema_next` IS the 1-D Kalman steady state (`kalman.rs:3-6`); the full n-D
`kalman.rs` exists but is **not** wired in v1 — same named trigger as RCI resolution §F: "measured
EMA false-positive/negative rate insufficient for the anomaly job." Capacity is not need.

### 4.2 The capsule: context captured AT the moment, closed schema

One JSONL object per flagged event — every field typed, **no free-form text** (RCI M2 lesson:
closed schema, no captured stderr/env; the one composed `reason` string is built mechanically from
typed fields only, exactly like `main.rs:610-613`):

```
ExplainedAnomaly {
  // symptom + rule (falsifiable arithmetic, not vibes)
  ts_unix_s, site_id, site_name, kind,
  observed_ns, baseline_mean_ns, baseline_dev_ns, threshold_ns,
  rule: "K_DEV",                       // which named constant fired (closed enum)

  // cause attribution — which resource was stalling RIGHT THEN (PSI, §2.1)
  psi_cpu_avg10, psi_mem_avg10, psi_io_avg10, psi_io_total_delta_us,
  load1, mem_pct, disk_pct,            // existing gauge triple, reused
  self_cpu_delta_ticks, vm_rss_kb,     // this process's own share (two-snapshot delta)

  // concurrency context — what the system was doing
  ring_backlog, dropped_since_last, drainer_lag_ms, spool_pending,

  // recent-change context — cheap, fail-open, absent > fake
  git_head, head_age_s,                // one `git rev-parse` + commit ts, cached per drain tick
  rci_top_cochange: Option<[path; 3]>, // read-only peek at .rci/ ranking IF present (§5)

  // the concrete prelude — exemplar discipline (§2.6)
  prelude: [up to 8 preceding RingEvents from the same ring: {seq, t_mono_ns, site, kind, value}]
}
```

**Why this constitutes an explanation** (the reading protocol, stated once here and in the ledger
README): symptom = `observed vs baseline×rule`; *where it came from* = PSI triple (io-stall vs
cpu-stall vs mem-stall is disambiguated by the kernel's own accounting, not guessed) + the
process's own CPU delta (was it *us* or a neighbor); *what was in flight* = prelude + backlog;
*what changed recently* = head age + co-change peek. If PSI ≈ 0 and self-CPU is low, the cause is
internal serialization (backlog/lag fields carry it). Every field is something a human can act on;
none is a mood.

### 4.3 Real-time, reduction, historical tracking — mapped to the ask

- **Real-time detection**: predicate runs every drain tick (≤250 ms after emission), in-process.
- **Reduction**: aggregates-always + events-on-anomaly (§3.1) is the reduction — event volume
  scales with anomalies; history is downsampled with max-preservation (§3.5).
- **Historical tracking**: T0-T2 tiers + rotated capsule ledger; a year of latency shape in a
  fixed 4.5 MB, spikes never averaged away.

---

## 5. Boundaries: RCI and loop-signals (share vs. stay separate)

- **RCI**: different concern (code changes vs. runtime behavior). **No shared runtime primitive**
  — RCI D′ is pull-based with no chain and no producers (`resolution.md:55-60`), so there is
  nothing hot to share; sharing a ring would re-create the coupling H1 removed. The single
  touchpoint is the capsule's `rci_top_cochange` field: a **read-only, fail-open file peek** at
  `.rci/`'s derived ranking when it exists (absent ⇒ typed `None`, never an error, never a block).
  RCI never reads P24's data; P24 never writes RCI's.
- **loop-signals / markov_attractor**: different signal domain (tool-outcome tokens). Stays
  separate; both remain advisory JSONL ledgers side by side. If the Markov detector ever needs
  streaming ingestion, `ring.rs` is the primitive it should reuse (one new producer ring, same
  drainer) — noted as a future consumer, not wired now.
- **P08**: P24's capsule and latency events become `LogEvent` variants
  (`LogEvent::RuntimeLatencyAnomaly(ExplainedAnomaly)` alongside `ClaimLatencyAnomaly`) the moment
  P08 §3's typed sink lands; until then the JSONL ledger is the same honest stand-in the
  claim-latency code already documents (`main.rs:547-554`). Egress: P24 writes local files only;
  anything remote remains behind P08 §5's signed marker. Nothing in P24 opens a socket.

---

## 6. DECART — new vs. extend vs. already-solved

| Piece | Options considered | Decision + probe (strongest case against) |
|---|---|---|
| Hot-path ring | (a) crates.io `rtrb`/`crossbeam`/`heapless::spsc`; (b) new `kernel/src/ring.rs` (pure core/alloc); (c) reuse `spool.rs` | **(b).** (a) blocked twice over: crates.io 403 is live-documented (P08 appendix §ii) AND zero-dep rule; these crates are also *more* general than needed. (c) fails the hot path: `&mut self` + String alloc (`spool.rs:70-82`). **Probe:** hand-rolled lock-free code is where bugs live; mitigated by SPSC-only (no CAS to get wrong), 32-byte POD copies, the §3.2 property/stress tests, and the fact that this is the DECART-2026-07-15 #5 path already reviewed. If crates.io unlocks, re-run this row against `rtrb` (its SPSC design is the same algorithm). |
| Host/context sampling | (a) new sampler daemon; (b) extend existing gauges | **(b).** Extend `hetzner-exporter`/`native-trackers` with PSI + `/proc/self` deltas. **Consolidation sub-decision:** collapse the dual disk implementation onto the `statvfs64` FFI (`native-trackers/src/main.rs:788-816`), replacing the `df` fork+exec (`hetzner-exporter/src/lib.rs:75-86`) — the FFI is already written, already zero-dep. **Probe:** raw FFI struct layout is glibc-specific; the existing code already pads defensively (`main.rs:806-809`) and it's the incumbent implementation, not new risk. |
| Detector + capsule writer | (a) new binary; (b) drainer thread + a `latency-report` fold subcommand in `native-trackers` | **(b).** Real-time half lives in the drainer (in-process by necessity — it owns the ring consumer). Offline half (render/fold history, re-scan ledger) = one new subcommand in `native-trackers`, matching its existing subcommand pattern (`main.rs:14-21`). No new crate. **Probe:** a subcommand in a 984-line main.rs grows a hot file; acceptable at current size, split only when it hurts. |
| History store | (a) SQLite/pgrust; (b) Prometheus-style compression; (c) RRD fixed tiers over `native-ser` f64 wire | **(c).** (a) is a dependency + daemon for 4.5 MB of data; (b) trades O(1) seek + torn-write locality for ~5× smaller files nobody needs smaller. **Probe:** fixed tiers can't answer "raw event 3 weeks ago" — accepted: capsules (the events that matter) are kept verbatim in the rotated ledger; only *gauge shape* is downsampled. If the Living-Memory pgrust arc lands, migrating T1/T2 into pgrust is a named follow-up, not a v1 dependency. |
| Anomaly rule | (a) port claim-latency code; (b) port the *pattern* (named constants, pure predicate, explained record, advisory) with EMA baselines; (c) full Kalman | **(b).** (a) doesn't fit streaming inputs; (c) is capacity-not-need (RCI §F trigger adopted verbatim). Already-solved and reused as-is: `ema_next` (`geo.rs:39`), the ledger convention, the advisory exit-0 posture, the de-dup discipline. |

**New external dependencies: zero. New crates: zero. New modules: one (`kernel/src/ring.rs`).**

---

## 7. Build plan — dependencies re-derived, falsifiable done-checks

Order re-derived from real necessity (protocol step 2), not draft order. W1 units are mutually
independent (three lanes); W2 needs W1's lanes.

| # | Unit | Depends on | Falsifiable done-check |
|---|---|---|---|
| W1a | `kernel/src/ring.rs` (SPSC + tests) | — | `cargo test -p kernel ring::` green incl. 2-thread stress; `grep -c compare_exchange kernel/src/ring.rs` == 0 (H1 invariant, CI-greppable) |
| W1b | Gauge consolidation + PSI + `/proc/self` deltas | — | `hetzner-exporter --selftest` extended: PSI fields present on this host; `df` spawn gone (`grep -n '"df"' tools/telemetry/hetzner-exporter/src/lib.rs` empty); two-snapshot CPU% ∈ [0,100] under a spin-load fixture |
| W1c | RRD tier module in `native-ser` (`rrd.rs`) | — | Round-trip test: 3 days of synthetic 10 s samples → T0/T1 consolidation preserves global max exactly (bit-identical f64); file size constant before/after |
| W2a | Drainer + detector + capsule (wired into the 2 existing long-lived processes: `hetzner-exporter` serve loop, `rust-spool` drainer) | W1a-c | RED→GREEN: inject a synthetic 10× latency spike at a test site → exactly one capsule appears with `rule:"K_DEV"`, PSI fields populated, prelude non-empty; no capsule for baseline traffic. Overhead check: instrumented hot loop ≤ 1% slower than uninstrumented (bench, threshold named in the bench file) |
| W2b | `native-trackers latency-report` subcommand | W1c, W2a | Fold of a fixture ledger + tiers renders per-site p50/max/anomaly-count; exit 0; zero panics on a corrupted-slot fixture (torn slot reads as gap) |
| W3 | P08-integration stub: capsule as `LogEvent` variant | P08 §3 (unbuilt) | Deferred with P08; until then the JSONL ledger is the documented stand-in (`main.rs:547-554` pattern) |

**Adoption hook (Ananke, so P24 doesn't rot):** the done-check for W2a requires wiring into the
**two real long-lived processes that exist today** — there is no product server yet (`apps/*`
deleted at HEAD, roadmap §1.2), so P13/P16 inherit an already-proven substrate instead of a paper
design. A one-line CI grep guard (`ring.rs` must contain no `compare_exchange`) makes the H1
invariant structural, not remembered.

---

## 8. 2-question doubt audit

**Q1 — least confident about (concrete):**
1. `Instant::now()` cost is asserted from the vDSO argument (§3.1) but not measured **on this
   host**; W2a's overhead bench is the falsifier, and if it fails the fix is per-site disarm, not
   abandoning the design.
2. The PSI files require kernel ≥ 4.20 and may be absent in some container configs; the capsule
   treats absent PSI as typed `None` (fail-open), but I did not verify PSI availability inside the
   Fly.io runtime this repo deploys to — only that the design tolerates absence.
3. `DRAIN_INTERVAL_MS = 250` bounds detection latency but also sets worst-case ring occupancy;
   the ring capacity ↔ drain interval ↔ event rate arithmetic is stated as constants, not yet
   validated against a real burst profile (W2a's spike test covers one shape, not all).
4. The dual-gauge consolidation (W1b) touches a surface Gatus polls (`hetzner-exporter` JSON field
   order is load-bearing, `main.rs:58`) — appending fields is safe by that comment's own logic,
   but I did not read the live Gatus config to confirm no strict-schema match.

**Q2 — biggest thing I might be missing:** the same blind spot the roadmap audit found twice
(P08 appendix Q2): cross-boundary assumptions. Here the risk shape is *P24 assuming P08's sink
will land* — mitigated by making the JSONL ledger the stand-in from day one (W3 is deferred WITH
P08, nothing in W1/W2 waits on it). Second candidate: with no product server at HEAD, the honest
measured need for *runtime* latency telemetry today is the harness/tools layer itself — if the
operator intends P24 to primarily serve the future delivery spine, the W2a wiring targets should
be re-pointed at P13's binaries when they exist; the substrate is identical either way.

## 9. Anu / Ananke check

**Anu (derivable, not asserted):** the two load-bearing choices both trace to evidence already in
front of this document — SPSC-no-CAS derives from RCI H1's verified failure
(`resolution.md:54-66`) plus the §2.3 memory-ordering argument; aggregate-always/event-on-anomaly
derives from perf/eBPF's documented overhead model (§2.2) and is falsified-or-confirmed by W2a's
1% bench gate. The phase-numbering decision (§0) is derived from P08's own text, quoted, not from
memory of what P08 "probably" covers. Weakest Anu link, named: the §3.5 slot-epoch torn-write
scheme is designed-by-argument, not yet property-tested — W1c's corrupted-slot fixture is the
required check before it counts as true.

**Ananke (structural, not hoped):** drops are counted, never silent (ring semantics); spikes
survive downsampling because `max` is carried by construction, not by operator vigilance; the H1
invariant is a CI grep, not a convention; history cannot grow unboundedly because tier files are
fixed-size at creation; the observability layer cannot stall the work path because the hot path
contains no lock, no allocation, no file I/O *by module boundary* (only the drainer owns I/O).
Named residual that is NOT structurally enforced: nothing forces future new subsystems to
instrument their sites (adoption is convention) — the W2a wiring-into-real-processes done-check
covers today's processes; a future lint ("long-lived binary must register ≥1 ring") is flagged as
the follow-up if adoption drifts.

---

## Appendix — phase-table registration

Registered in `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §8 as **Phase 24** (24 was
confirmed free at registration time: §8.1 ends at P23; §8.2 explicitly declined to use 24).
Depends on: **8** (typed schema/sink substrate; P24 generalizes P08 §4's anomaly pattern from CI
commit latency to runtime latency). Off-critical-path lane, same class as P5/P8/P11/P12.

---

## Audit addendum (2026-07-17, appended — Phase-27 fault-isolation audit; design above unchanged)

Three verified defects in the telemetry surface this blueprint builds on, raising the priority of
work it already plans and adding one line-level fix to its wave list
(`BLUEPRINT-FAULT-ISOLATION-DECENTRALIZED-ARCHITECTURE-2026-07-17.md` §1.2 for full context):

1. **The unbounded-growth class this blueprint's RRD tiers answer is live in production now
   (A6-class, HIGH):** `tools/telemetry/logs/metric.jsonl` measured at 2,758,165 bytes, actively
   written today, with zero rotation/retention logic anywhere in `lib.sh`/`governance.sh`/
   `report.sh` (grep-verified). The RRD max-preserving tier design here is the fix — this finding
   converts it from "good hygiene" to "closing an active leak"; the existing JSONL files must be
   brought inside the tiering, not left as a parallel untended surface.
2. **Head-of-line wedge in the flight path this blueprint inherits (A1, CRITICAL):**
   `rust-spool/src/main.rs:240-247` retries the queue head forever with no send-failure
   deadletter — one permanently-rejected Telegram message silently ends all future operator
   alerting (`lib.sh:35-43` auto-launches this binary). Owned by Phase-27 Wave F0; recorded here
   because any P24 alarm that exits via this spool inherits the wedge until F0 lands.
3. **One missing timeout (A5, HIGH):** `tools/telemetry/topics/src/main.rs:66` — the only ureq
   call site in the repo with no `.timeout(...)`; a connect-then-silent endpoint blocks the
   caller indefinitely. One-line fix, Phase-27 Wave F0.
