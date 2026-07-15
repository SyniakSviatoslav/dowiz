# Kernel Observability — Decart Comparison (2026-07-15)

Context: `rust-core` (bebop-core) is a **zero-dependency, `core`/`alloc`-only, air-gapped** kernel.
It exposes state ONLY via C-ABI fns over linear memory (`field_*`). It has **no logging/tracing
infra at all**. The host (dowiz `tools/telemetry`) already samples + ships to Telegram; the kernel
just needs to *expose* counters cheaply. There is a natural metric surface already: `GraphState.
field_energy: Vec<f64>` — per-node `|Δu|` accumulated every propagation (line 35). Also `STATE`
is `std::sync::Mutex<GraphState>` → the crate is `std`-capable natively, but the **wasm32
empty-import gate** (sovereign-core, no phone-home) forbids `std` at wasm build.

Ranking criterion = the operator's binding constraints: sovereign (no phone-home), no_std-compatible,
air-gapped (no network at build), empty-import wasm, falsifiable, reversible, low supply-chain.

| # | Approach | bare-metal fit (no_std/wasm) | falsifiable correctness | perf cost | supply-chain / license | maintainability | reversibility | evidence |
|---|----------|------------------------------|-------------------------|-----------|------------------------|-----------------|----------------|----------|
| 1 | **C-ABI counter exports** (add `field_metrics(out:*mut f64, n)` exposing `field_energy` + step counts) | ✅ pure `core`/`alloc`, same ABI as existing fns | ✅ host asserts `Σ field_energy` monotonic per propagate; unit-testable on native | ~0 (copy a Vec into linear memory) | none (no dep) | low (one fn mirrors existing pattern) | trivial (one added fn) | matches existing 14 `extern "C"` fns; `field_energy` already updated per step |
| 2 | **`defmt`** (embedded defmt fmt, no_std) | ✅ designed for no_std; tiny | ✅ compiles to a linker table, deterministic | low RAM, but adds a `defmt` + `defmt-rtt`/probe backend dep | MIT, but a 1-crate dep (breaks air-gap purity slightly) | med (macro attr on fns) | med (per-fn attrs) | defmt is the embedded-Rust standard for exactly this |
| 3 | **`tracing` + `tracing-*`** | ❌ needs `std` + a subscriber; wasm needs `tracing-wasm` + browser console (phone-home-ish, not empty-import) | ✅ spans/events are structured | high (alloc per event, boxing) | many crates (supply-chain) | high | hard (pervasive) | `tracing` is the std-Rust choice but violates sovereign gate |
| 4 | **`log` crate + host-side drain** | ⚠️ `log` is no_std-compatible (facade only) but a drain still needs an output; wasm has no stdout | ✅ facade is stable | low-mid | 1 facade crate | med | med | `log` facade is the minimal-Rust default |
| 5 | **Ring buffer in linear memory** (kernel writes events to a `static` ring; host polls) | ✅ pure `core` (needs an allocator or fixed array) | ✅ host replays buffer, asserts sequence numbers | ~0 (append) but fixed cap = dropped events | none | med (lock-free ring + versioning) | med | classic embedded pattern; matches "no phone-home, host pulls" |
| 6 | **eBPF / USDT probes** (observe from host OS) | ❌ needs Linux kernel + BPF, not wasm, not air-gapped build | ✅ tracepoints are verifiable | near-zero in-kernel | kernel-side only | high (BPF toolchain) | hard | for the *native* CLI only, not the wasm core |

DECISION: **#1 C-ABI counter exports**, with **#5 ring buffer** as the upgrade path when
event-level (not just aggregate) telemetry is needed. Rationale (falsifiable): it is the ONLY
option that (a) adds ZERO dependencies (preserves air-gap + empty-import wasm), (b) reuses the
exact C-ABI/linear-memory contract already proven by 14 existing `field_*` fns, (c) is unit-
testable on native `cargo test` (assert `field_energy` is monotonic across `field_spectral`
calls), and (d) keeps the host (dowiz telemetry) as the sole exporter — so the kernel never
"phones home", satisfying the sovereign gate. `defmt` (#2) is the only honest runner-up IF the
operator later accepts one MIT dep; `tracing` (#3) is rejected on the merits (violates the
empty-import/no-std wasm gate), not on age.

Mandatory probe (strongest honest argument AGAINST the choice):
> C-ABI counter exports only expose *aggregates* the kernel already computes (energy, step
> counts). They CANNOT capture *event-level* traces (which node flipped, when a propagation
> stalled) without either (a) adding a ring buffer (#5) — reintroducing a fixed memory cap and
> drop semantics, or (b) bloating linear memory with per-event structs. If the operator's goal
> is full distributed-tracing of the kernel, #1 alone is insufficient and #5 (or defmt on native)
> becomes necessary. We accept aggregate-only telemetry as the correct MVP because the host layer
> already owns event/task/session logging; the kernel's job is to expose its *numeric state*, not
> to become a logger.

Older-tech-as-adapter note: the existing `tools/telemetry` bash bridge (JSONL + Telegram) is kept
as the EXPORTER; it is not replaced. The kernel change is purely additive C-ABI surface (#1). The
JSONL encoding itself is out of scope — the bottleneck was DISK (91%→88% reclaimed, +2.5GB), not
serialization format; per-event size is ~240B and immaterial at current volume.

Next step (if approved): add `field_metrics(out: *mut f64, n: i32) -> i32` to `rust-core` exposing
[step_count, last_propagation_ms(approx via cycle count), Σfield_energy, max field_energy, active_set_size]
and a native `cargo test` asserting monotonicity; wire the host `telemetry bench_run` to sample it.
