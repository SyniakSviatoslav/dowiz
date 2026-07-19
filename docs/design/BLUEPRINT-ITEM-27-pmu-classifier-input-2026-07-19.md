# BLUEPRINT — Item 27 (classifier-input half): PMU counters feeding `Verdict`/`DriftClass`

> Roadmap: `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §C (Tier 2), line 204.
> Synthesis: `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §18(c) (line 332), item 27
> proof line (line 342), §16 (lines 281–294), §21 (line 376).
>
> **Scope guard (from the roadmap itself):** this half gets real hardware counter data flowing
> INTO the existing classification pipeline as an additional recorded input signal. The response
> half — anything *acting* on a PMU-informed classification — is Tier 4, strictly behind item 9
> (breaker) and item 21 (autonomic gain-scheduling). Nothing in this blueprint changes a verdict,
> a threshold, or any behavior. Diagnostic grade, explicitly not a CI gate.

## 1. The real types, as they exist today (verified against source this session)

### 1.1 `markov::Verdict` — the Markov attractor verdict

- **Definition:** `kernel/src/markov.rs:42-50` —
  `pub enum Verdict { Healthy, LimitCycle, StrangeAttractor }`
  (`Copy + Eq`, fieldless; string form `verdict_str()` at `markov.rs:98-104`:
  `"HEALTHY" / "LIMIT_CYCLE" / "STRANGE_ATTRACTOR"`).
- **Carriers:** `Report` (`markov.rs:53-66` — verdict + entropy_rate_bits, escape_mass, drift,
  slem, period, gap, mixing_time) and `DetailedReport` (`markov.rs:84-96` — adds alphabet, eigs,
  stationary π, reason; its JSON shape is the frozen Python-parity contract, see §4.3).
- **Sole producer:** `analyze_detailed(states: &[&str]) -> DetailedReport` (`markov.rs:110`),
  thin view `analyze()` at `markov.rs:284`. **Current inputs:** a window of tool-outcome tokens
  (`"run_ok"`, `"edit_fail"`, …) → row-normalized transition matrix Â → entropy rate, escape
  mass (`is_escape`, `markov.rs:37`), Foster-Lyapunov drift (`potential`, `markov.rs:31-36`),
  slem/period/gap via `crate::spectral`. Pure function; fail-open cold start below
  `MIN_EVENTS = 8` (`markov.rs:25`).
- **Emission point (where a verdict leaves the kernel today):**
  `kernel/src/bin/markov_attractor.rs:35` — stdin tokens → `analyze_detailed` → hand-rolled JSON
  (format string at `markov_attractor.rs:55-59`), consumed by the harness (`check.sh`).

### 1.2 `spectral::DriftClass` — the DMD-style stability class

- **Definition:** `kernel/src/spectral.rs:674-682` —
  `pub enum DriftClass { Damped, Resonant, Unstable }` (`Copy + Eq`, fieldless).
- **Pinned wire contract:** `DriftClass::wire_code()` (`spectral.rs:686-699`) is the single
  authority for the numeric code on the kernel→engine FE-07 bridge (`Damped=0, Resonant=1,
  Unstable=2`), round-trip-pinned in `drift_wire_contract_matches_kernel`, decoded by
  `engine/src/bridge.rs::drift_from_code`. The engine's mirrored `DriftClass` is hermetic-audit
  finding RC-4 — this contract is load-bearing and must not change shape.
- **Sole producer:** `classify_drift(a: &[Vec<f64>]) -> DriftClass` (`spectral.rs:704-741`).
  **Current inputs:** a rebuilt operator matrix; ρ = `spectral_radius(a)` vs the unit circle
  with `DRIFT_BAND = 1e-6` (`spectral.rs:702`); fail-closed to `Unstable` on any non-finite
  entry, ragged rows, or unbuildable `Mat`. (This is the same `DriftClass` this session's
  `classify_drift` fail-closed work touched — one type, one file; there is no second one in the
  kernel.)
- **Also carried in:** `GraphSpectrum.drift` (`spectral.rs:612-619`, filled at `spectral.rs:638`).
- **Consumers / emission points:** `spectral_cache.rs:21` (`RetainedBase::admit` — the snapshot
  admission gate), `event_log.rs:405-435` (classify-BEFORE-retain integrity hole closure),
  `order_machine.rs:354-361` (drift-word cross-check), `wasm.rs:716/742-746/801-803` (JS +
  flat-wire encoders), `hydra.rs`.

### 1.3 Disambiguation

`kernel/src/verify_retrieval.rs:20` defines an unrelated `pub enum Verdict { Pass, Fail }`
(claim re-verification). It is NOT this item's target and gains nothing here.

## 2. Reuse target: the Tier-1 FDR schema (do NOT build a parallel mechanism)

This session's logger/FDR rewrite already built exactly the hardware-telemetry machinery this
item needs — on branch `exec/space-grade-tier0-2026-07-19` (worktree
`/root/dowiz-wt-space-grade-exec`), introduced in commit `f04142f89`, **not yet merged to main**
as of this writing:

- `kernel/src/fdr/schema.rs:25-37` — `enum Absence` (closed reason set: `NonLinuxHost`,
  `NoRaplInterface`, `PermissionDenied`, `ReadError`, `SamplingDisabled`), serialized by name.
- `schema.rs:54-58` — `enum Reading<T> { Value(T), Unavailable(Absence) }`; `write_field`
  (`schema.rs:64-75`) ALWAYS emits the key — value or `{"unavailable":"<reason>"}` — the
  "named absence, never silent omission, never a fake 0" guarantee, mechanically.
- `schema.rs:86-95` — `struct HwStamp { cpu_ticks, rss_kb, joules_uj: Reading<u64> }`,
  non-optional on every `FdrEvent` (`schema.rs:205-217`); `/proc` readers reused from
  `typed_metrics.rs` (`ProcCpuSample` at `typed_metrics.rs:36`, `MemSample` at `:75`).
- `schema.rs:100-104` — `enum StampPolicy { Full, Cheap }` — honest cost control: hot-path
  records carry a first-class `SamplingDisabled`, not a silent omission.
- `schema.rs:153-175` — `read_joules_uj()` (RAPL): errno→`Absence` mapping, degrades to a named
  absence on every failure mode. The module doc pins the losslessness rule: *the kernel emits
  raw monotone counters only; rates/deltas are a consumer concern* (`schema.rs:91-93`).

**Decision: item 27 extends this schema.** The PMU stamp is a sibling of `HwStamp` built from
the same `Reading<T>`/`Absence`/`StampPolicy` machinery and the same raw-monotone-counters rule.
Building any second telemetry mechanism would violate Hermetic P2 (one mechanism, not two) and
the synthesis §18(c) directive verbatim ("never a separate, parallel monitoring system").

**Dependency:** implementation starts after the Tier-1 FDR branch lands on main (or lands as a
follow-on commit on that branch). Item 27 is Tier 2 — "parallelizable" but structurally after
items 4+29.

## 3. PMU access in zero-dep Rust — research findings

The kernel's default build has zero external dependencies (roadmap §B, confirmed landed on the
exec branch at `6605166cd`). No `libc`, no `perf-event` crate. What remains:

### Tier A — always available, zero permission (std file reads + one stable intrinsic)

| Signal | Source | Notes |
|---|---|---|
| `tsc_cycles` | `core::arch::x86_64::_rdtsc()` | Stable intrinsic, no feature gate; `tsc`+`rdtscp` in host cpuflags. Reference cycles, not core cycles; virtualized-TSC caveat on KVM. `unsafe` but never faults in ring 3 (CR4.TSD unset in practice). aarch64 analogue: `CNTVCT_EL0` via `asm!`, or omit (named absence). |
| `minflt` / `majflt` | `/proc/self/stat` fields 10/12 | Verified readable on this host. Page-fault counters — the zero-permission proxy for memory-pressure/cache behavior. |
| `vol_ctxt_switches` / `nonvol_ctxt_switches` | `/proc/self/status` | Verified readable on this host. Nonvoluntary switches are the zero-permission proxy for CPU contention. |
| `cpu_ticks` (utime+stime) | already in `HwStamp` via `typed_metrics::ProcCpuSample` | Reuse — do NOT re-read in the PMU stamp. |

### Tier B — true PMU events (instructions, cycles, cache-misses, branch-misses → IPC, miss rates)

- The ONLY sane access path is `perf_event_open(2)`. glibc provides no wrapper; Rust std has no
  binding. Zero-dep implementation = hand-rolled raw syscall via `core::arch::asm!` (stable
  since Rust 1.59): syscall № 298 (x86_64) / 241 (aarch64), a hand-declared `#[repr(C)]`
  `perf_event_attr` with `size = PERF_ATTR_SIZE_VER0 (64)` for maximal ABI compatibility,
  `exclude_kernel = 1, exclude_hv = 1`, then `read(2)` on the returned fd yields the u64 count;
  `ioctl` `PERF_EVENT_IOC_{RESET,ENABLE}` to bracket. Estimated ~150–200 LOC including the
  errno→`Absence` mapping. Feasible, but see §5 — it will return a named absence on the current
  dev/self-management host (a cloud KVM guest — NOT the arc's deploy target; phrasing corrected
  2026-07-19, consistency audit §1.4).
- **`_rdpmc` — rejected as an independent path.** It #GP-faults (SIGSEGV) from userspace unless
  either (a) executed inside a `perf_event_open`+mmap self-monitoring setup with
  `/sys/bus/event_source/devices/cpu/rdpmc >= 1` (this host: `1` — mmap-scoped only), or
  (b) root sets `rdpmc = 2`. It is a latency optimization *of* the perf path, never an
  alternative *to* it. A kernel that SIGSEGVs when telemetry is unavailable is the opposite of
  the named-absence doctrine.
- **RAPL/energy** — already handled: `read_joules_uj()` exists in the FDR schema; this host has
  an empty `/sys/class/powercap/` and serializes `no_rapl_interface`. Nothing to add.

## 4. Design — minimal footprint

### 4.1 The enums do not change

`Verdict` and `DriftClass` stay byte-identical. Three independent reasons:

1. Both are fieldless `Copy + Eq` enums matched exhaustively across the kernel; adding payload
   is a redesign, not an input.
2. `DriftClass::wire_code()` is a pinned round-trip wire contract with the engine
   (`spectral.rs:686-699`; RC-4 mirror on the far side). Changing its shape breaks the FE-07
   bridge.
3. The item's scope is classifier-INPUT. The classification logic and its outputs are the
   response half's territory (Tier 4), and even there the enums likely survive unchanged.

### 4.2 New sibling stamp: `PmuStamp` in `kernel/src/fdr/pmu.rs`

```rust
/// All fields Reading<u64>; ALWAYS serialized; raw monotone counters only
/// (deltas/IPC/miss-rates are consumer-side, same rule as HwStamp.joules_uj).
pub struct PmuStamp {
    // Tier A — sampled fresh per stamp (two /proc reads + one rdtsc; µs-scale)
    pub tsc_cycles: Reading<u64>,
    pub minflt: Reading<u64>,
    pub majflt: Reading<u64>,
    pub vol_ctxt_switches: Reading<u64>,
    pub nonvol_ctxt_switches: Reading<u64>,
    // Tier B — perf_event_open; on this host today: Unavailable(PermissionDenied)
    pub hw_instructions: Reading<u64>,
    pub hw_cpu_cycles: Reading<u64>,
    pub hw_cache_misses: Reading<u64>,
    pub hw_branch_misses: Reading<u64>,
}
```

- `Absence` gains exactly ONE variant: `NoPmuInterface` — mapped from `ENOENT`/`EOPNOTSUPP`
  (no PMU exposed, the typical virtualized outcome) and `ENOSYS` (seccomp-filtered syscall).
  `EPERM`/`EACCES` (perf_event_paranoid) map to the EXISTING `PermissionDenied`. The enum stays
  closed; every `as_str` match arm is exhaustive, so the compiler forces the serialization name.
- `PmuStation`: opens the four Tier-B fds ONCE on first use; on failure caches the
  errno-derived `Absence` and never retries per-sample (cost + log-noise control). Tier A is
  sampled fresh each call. `PmuStation::delta(start, end) -> PmuStamp` produces
  wrapping-sub deltas, absence-propagating (`Value` only when both endpoints are `Value` with
  the same absence discipline as `Reading`).
- Cost discipline reuses `StampPolicy` semantics: PMU stamping happens only at Full-policy,
  low-frequency emission points (verdict emissions), NEVER inside `classify_drift` or
  `analyze_detailed` themselves (`RetainedBase::admit` is a hot path).

### 4.3 "Feeding" = window-bracketed delta joined on the SAME FDR record as the verdict

The companion-record option, chosen over a new field on the classifier outputs:

- **Verdict lane:** the caller that emits a verdict (`bin/markov_attractor.rs`, and any future
  kernel-side emitter) brackets the classification window — `station.sample()` before building
  the token window, `station.sample()` after `analyze_detailed` — and logs ONE `FdrEvent`
  (`kind: Event`, `name: "markov_verdict"`) whose `fields` carry `verdict_str()` plus the
  `PmuStamp` delta via `write_field`. `analyze_detailed` stays pure and byte-identical; the
  frozen Python-parity JSON contract (`markov_attractor.rs:55-59`, `green_parity_*` tests) is
  untouched.
- **DriftClass lane:** identical bracketing at the admission/integrity emission points
  (`spectral_cache::RetainedBase::admit`, the `event_log.rs:405-435` snapshot gate): the FDR
  record that logs the admit/reject decision carries the PMU delta. `classify_drift` stays pure.
- Correlation is exact and free: same `FdrEvent`, same `seq` — no join key, no second log.

**Why companion-record beats a new struct field**, point by point:

1. **P6 determinism preserved by construction.** Verdicts remain a replayable pure function of
   their existing inputs; PMU data is *recorded input*, not yet a decision variable. The
   roadmap's replay proof ("a replayed counter sequence reproduces the identical verdict
   sequence") is satisfied vacuously in this half — verdicts do not depend on counters at all —
   and becomes a real obligation only when the Tier-4 response half wires them in through item
   21's bounded-control-law table.
2. **Both pinned contracts survive:** the engine wire code (§4.1) and the Python-parity JSON.
3. **One classification mechanism, grep-provable** (item 27 proof (a)): no new classifier, no
   parallel monitor — PMU data rides the existing verdict emission through the existing FDR.
4. **Reversible.** Deleting `fdr/pmu.rs` and the bracketing lines restores today's kernel
   exactly. A field added to `Report`/`DetailedReport` would leak into every consumer signature.

Deferred variant (recorded, not chosen): `DetailedReport` gaining `pub pmu: Option<PmuStamp>`
attached via a builder (`with_pmu`). Rejected for this half — it perturbs the frozen parity
surface for zero consumer benefit before item 21 exists. Revisit at the response half if the
autonomic table wants the stamp inside the report rather than beside it.

### 4.4 Plane doctrine (one sentence, enforced in the module doc)

PMU values are nondeterministic by nature and live on the P3 forensic/display plane, exactly
like FDR wall-clock timestamps (roadmap item 13's secondary note): they are categorically
excluded from every hash, signature, idempotency, or gate-verdict surface. This sentence goes
verbatim into `fdr/pmu.rs`'s module doc.

### 4.5 Proof obligations for THIS half (restricted from the item-27 proof line)

1. Grep shows one classification mechanism — no parallel monitor (`rg 'Verdict|DriftClass'`
   finds only the existing two producers plus the unrelated `verify_retrieval` pair).
2. Every `PmuStamp` field is always serialized; a unit test forces each errno path and asserts
   the named absence string (mirrors the existing `read_joules_uj` degradation tests).
3. No CI job is keyed to any PMU value; the write-up (this document) labels the signals
   diagnostic-grade and quotes the synthesis §4 caveat verbatim: PMU-based verification is
   *"the weakest-precedented element — more studied as an attack vector than deployed as
   routine CI."*
4. Deterministic-replay of verdict-from-counters: deferred to the response half, and satisfied
   vacuously here (verdicts are independent of counters in this half — asserted by the purity
   of `analyze_detailed`/`classify_drift`, which this item does not modify).

## 5. Honest platform reality (probed live on this host, 2026-07-19)

| Probe | Result | Consequence |
|---|---|---|
| `/proc/sys/kernel/perf_event_paranoid` | **4** | Stricter than mainline's max (2) and Debian's patch level (3): unprivileged `perf_event_open` is fully blocked — hardware AND software events → `EACCES` → `Unavailable(PermissionDenied)`. |
| `/sys/bus/event_source/devices/cpu/rdpmc` | `1` | `_rdpmc` only inside a perf-mmap self-monitoring setup — which paranoid=4 prevents. No standalone rdpmc path. |
| `/sys/class/powercap/` | empty | No RAPL (already a named absence in the FDR schema). |
| `/proc/cpuinfo` | `hypervisor` flag; AMD EPYC-Milan (KVM guest); `perfctr_core` advertised | A vPMU is nominally advertised, but unreachable at paranoid=4. Docker's default seccomp profile additionally filters `perf_event_open`. |
| `/proc/self/stat` minflt/majflt, `/proc/self/status` ctxt switches, `tsc`/`rdtscp` flags | all present/readable | **Tier A flows real data today, zero permissions.** |

**Plain statement (corrected 2026-07-19 — consistency audit §1.4; the original conflated the dev
box with the deploy target):** on the *current dev/self-management host* — an AMD EPYC-Milan KVM
cloud guest (`hypervisor` flag, empty powercap) — every Tier-B hardware counter reads
`{"unavailable":"permission_denied"}` today. **This host is NOT the arc's deploy target.** On the
actual target — local, offline-first consumer hardware — the availability picture typically
*inverts*: RAPL (`/sys/class/powercap/intel-rapl`) usually EXISTS on consumer Intel/AMD Linux
boxes, and `perf_event_paranoid` is normally 2 (the mainline default), not 4, so Tier B is often
reachable there. The named-absence design itself is deployment-agnostic and unchanged: it covers
the hosts where RAPL/PMU are absent with a truthful, greppable, per-reason signal instead of a
fabricated zero or a crash — and Tier A (rdtsc deltas, fault counts, context switches) delivers
real hardware-adjacent input on every host immediately. If real IPC/cache-miss data is ever wanted, the two
levers are `sysctl kernel.perf_event_paranoid=2` or granting `CAP_PERFMON` (kernel ≥5.8) to the
kernel's process — both host-level changes, therefore **NEEDS-OPERATOR-DECISION**, recorded here
in the `slot_arena.rs` evidence-then-ruling format and NOT assumed by this design.

## 6. Sequencing and out-of-scope

- **Depends on:** Tier-1 FDR (items 4+29) merging to main — branch
  `exec/space-grade-tier0-2026-07-19`, NOT merged as of this writing. `PmuStamp` is meaningless
  without `Reading<T>`/`FdrEvent` on main.
- **Blocks nothing.** Item 27's response half (Tier 4) consumes this half's recorded signal but
  is gated on items 9 + 21, not on any interface decision made here — the FDR record shape IS
  the interface.
- **Out of scope, explicitly:** any verdict/threshold change; any autonomic response; any CI
  gate keyed to PMU values; `_rdpmc` fast paths; per-core or system-wide (non-self) monitoring;
  GPU counters; extending `HwStamp` itself (it stays per-record cheap; `PmuStamp` is
  per-classification-window).
