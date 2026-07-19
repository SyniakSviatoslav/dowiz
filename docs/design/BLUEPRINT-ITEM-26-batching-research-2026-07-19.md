# BLUEPRINT — Item 26: Batching Research Pass (measurement-only)

**Date:** 2026-07-19 · **Tier:** 2 (roadmap §C) · **Priority:** low · **Prereqs:** none
**Sources:** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §C Item 26;
`SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §18(b) + addendum item 26.

**Scope law (from the roadmap's own proof clause):** *"no batching code lands in the kernel
before its path's measurements exist."* This blueprint therefore proposes measurements only.
Since Item 26 is itself a research pass, §1–§3 below **are** the pass's first half (the
inventory the synthesis said "the enumeration itself is part of the task"); §4 is the
measurement half for the executor to run.

---

## §1. Existing-batching inventory (verified against source this session)

The synthesis §18(b) named three candidate hot paths — event-log commit, decision-unit
import, arena allocation — and asked for the real enumeration. Here it is.

| # | Mechanism | Where | Kind | Policy |
|---|-----------|-------|------|--------|
| 1 | SoA SIMD lane (`softmax_batch_lane`, `kalman_batch_step`) | `kernel/src/simd.rs:164,296,359` | compute batching | lane width 4 = `f64x4` hardware width; batch = caller's slice |
| 2 | FDR A/B segment ring | `kernel/src/fdr/ring.rs` (exec worktree, see §3) | fsync batching | fsync per **segment** (1 MiB cap) + per-record for Alarm class |
| 3 | Event-log commit (`FileEventStore::insert`) | `kernel/src/hydra.rs:1040-1063` | **no batching** | one `write`+`flush`+`sync_all` per event |
| 4 | Decision-unit import | `kernel/src/decision/import.rs:81` (`import_unit`) | **no batching** | per-unit admission; batching = caller loop |
| 5 | `BumpArena` | `kernel/src/arena.rs:56,75,114` | allocation batching | one upfront reserve, pointer-bump, never grows |
| 6 | Contiguous sample matrix | `kernel/src/causal.rs:966` | allocation batching | "one allocation for the whole batch" (row-major) |
| 7 | Warm-pool refill | `kernel/src/hub_provisioning.rs:274` | domain batch size | `POOL_REFILL_BATCH: u32 = 12` |
| 8 | WASM batch ingest (`channel_ledger_js`) | `kernel/src/wasm.rs:334` | boundary amortization | batch = whatever the JS caller sends; no policy |
| 9 | Online-learner mini-batch | `kernel/src/online.rs:280,330` | compute batching | caller-supplied local batch |

**Anti-batching by design (must stay excluded from any future batching work):**
- `apps/courier/src/surface.rs:13` — **P52 no-batching law**: at-most-one live courier offer.
  Product invariant, not a performance gap.
- `kernel/src/decision/mod.rs:184` — `DispatchOut { batch: bool }` is a *domain decision
  output* (whether to batch orders for dispatch), not an infrastructure batching mechanism.
  Named here so nobody miscounts it.
- `kernel/src/ports/notification.rs:406` — fan-out is per-message across transports with
  mid-batch dead-token eviction; retry pacing via `TokenBucket` is throttling, not batching.

**Non-findings (grep-verified, zero `batch` hits):** `kernel/src/mesh.rs` (gossip/admission
is per-event), `kernel/src/retrieval/` (bm25/diffusion are per-query), and the entire
`engine/` crate. `kernel/src/backup.rs:216-229` is whole-blob write→fsync→rename
(crash-atomic single write, not a batching policy).

## §2. Tuning-rationale assessment — honest, per mechanism

| Mechanism | Rationale quality |
|-----------|-------------------|
| SIMD lane (simd.rs) | **Best in repo.** Width 4 is hardware-derived, not magic. Bit-identity parity tests (`kalman_batch_bit_identical`, simd.rs:533) AND a recorded speedup benchmark (`kalman_batch_benchmark_speedup_recorded`, simd.rs:636-712) gating ≥1.0× with the real ratio printed. The one batching mechanism that is both proven correct and measured. |
| FDR segment cap 1 MiB (ring.rs:32) | Architecture is deliberate (two-tier durability, doc-commented); the **number** is retention-motivated ("bounded size, last-N-seconds"), never throughput-measured. Untuned default. |
| Event-log per-event fsync (hydra.rs) | Deliberate durability-first policy with a typed fault taxonomy (`StoreError::Flush/Sync`, event_log.rs:172-186) — but its *cost* has never been measured. The classic group-commit candidate. |
| `POOL_REFILL_BATCH = 12` | Magic number, self-labeled "tunable defaults" (§6.3). Cold path (server provisioning) — low measurement value; note-only. |
| `BumpArena` capacity | Fixed at construction; `high_water()` (arena.rs:122) is a built-in measurement hook that no recorded baseline uses yet. |
| WASM batch ingest / online mini-batch | Caller-determined; no kernel policy to tune. |

## §3. Cross-reference — the FDR ring as batching, and the missing baseline

The FDR module (this session, exec worktree `/root/dowiz-wt-space-grade-exec`, commits
`f04142f89`→`eb350464e`, not yet on this checkout's main) **is** a form of write batching:
`FdrRing::append` (ring.rs:~117) does per-record `write(2)` to the page cache (kill-9
durable) and pays `sync_data` only on segment switch (`switch()`, ring.rs:140-152) or for
`Alarm`/`PostMortem` records — i.e., the durability barrier is amortized across up to 1 MiB
of ordinary records. That is exactly the amortization-vs-tail-latency shape §18(b) describes.

**But it is unmeasured.** Grep of the module and of the staged
`BLUEPRINT-ITEMS-04-29-logger-fdr-rewrite-2026-07-19.md` finds no throughput numbers; the
only perf artifact there is a criterion pre/post guard on `place_order`. So the FDR ring
does **not** yet supply this pass a baseline — establishing one is measurement M2 below.

## §4. Measurement plan (prioritized, for the Opus executor)

**Instrumentation stack — reuse, don't build.** Criterion harness + trend tracking already
exist (`kernel/benches/`: `criterion.rs`, `contention.rs`, `baseline.json`,
`bench_track.py`, `BENCH_HISTORY.md`; CI-gate work in p75/p80/p81 worktrees). PMU numbers
come from external `perf stat` (instructions, cache-misses, branch-misses, context-switches)
— per synthesis §18(c), **no new in-kernel profiling machinery**; any PMU samples collected
here double as trial input data for item 27's classifier-feature half. dudect stays reserved
for crypto paths (hardening-checklist precedent); it is not the right tool for throughput and
is deliberately not used here. Allocation counts: `BumpArena::high_water()`/`reset_count()`
where available, else a counting global allocator inside the bench crate only. Syscall
counts: `strace -c`. **P99 reported alongside throughput for every measurement** (roadmap
item 26's own proof clause).

- **M1 (highest value): event-log commit.** Criterion bench of
  `FileEventStore::insert`-backed `MeshLog::append` (hydra.rs:1040, event_log.rs:302):
  per-append P50/P99 wall-clock and appends/sec, with `strace -c` confirming 1 fsync/event.
  Then, **in the bench crate only** (a measurement scaffold, no kernel change), measure raw
  fsync amortization: same byte stream synced every 1/4/16/64 records → curve of
  throughput gained vs P99 latency added per grouping level. **Output:** a falsifiable
  batch/don't-batch line for group-commit, with the durability-semantics cost stated
  (events acknowledged before fsync would change the crash contract — that trade must be
  priced, not assumed).
- **M2: FDR ring baseline.** Criterion bench of `FdrRing::append` in the exec worktree:
  records/sec for normal-kind (page-cache path) vs Alarm-kind (per-record `sync_data`);
  segment-switch cost in isolation; sweep segment cap 256 KiB / 1 MiB / 4 MiB to learn
  whether the cap is even throughput-relevant. **Output:** the baseline the module landed
  without; a keep/change verdict on the 1 MiB default with numbers.
- **M3: decision-unit import.** Per-unit cost of `import_unit` (import.rs:81) over an
  N-unit stream: wall-clock + allocations/unit + `perf stat` IPC and cache-miss rate.
  Hypothesis to falsify cheaply: per-unit cost is CPU-trivial and batching would buy
  nothing. A measured "don't batch" is a fully successful outcome.
- **M4 (conditional on M3 showing allocation cost matters): arena.** Record
  `high_water`/`reset_count` across a representative workload and compare cache-miss rate
  (perf stat) of arena-bump vs per-item `Vec` allocation for one real call shape
  (causal.rs:966 is the template). Skip entirely if M3 says allocation is noise.

**Explicit exclusions:** the P52 courier-offer path (law, §1); `DispatchOut.batch`
(domain semantics); `POOL_REFILL_BATCH` (cold path — recorded above as a magic number,
not worth bench time); notification fan-out (throttling concern, item 27/21 territory).

**Done-when (mirrors roadmap item 26 proof):** a research doc exists with measured
baselines for M1–M3 (M4 conditional), P99 beside every throughput number, and one
falsifiable batch/don't-batch recommendation per path. No batching code in the kernel.
