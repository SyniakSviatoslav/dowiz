# AUDIT — Item 26: Batching Measurement Results (measurement-only)

**Date:** 2026-07-19 · **Tier:** 2 (roadmap §C) · **Priority:** low · **Status:** ✅ DONE
**Blueprint:** [`BLUEPRINT-ITEM-26-batching-research-2026-07-19.md`](BLUEPRINT-ITEM-26-batching-research-2026-07-19.md)
**Scope law (roadmap proof clause):** *"no batching code lands in the kernel before its path's
measurements exist."* This pass produced the measurements. **No batching code was written; no
kernel production behaviour changed.** Scaffolding is bench/test-only.

**Environment:** AMD EPYC-Milan, 8 cores, AVX2. Storage `/dev/sda1` **ext4 on a real block
device** (not tmpfs — fsync probe: ~500 µs/call, so every fsync number below is real IO, not a
no-op). Toolchain `cargo/rustc 1.96.1`. Worktree `/root/dowiz-wt-space-grade-exec`, branch
`exec/space-grade-tier0-2026-07-19`.

**PMU / perf: UNAVAILABLE.** `/proc/sys/kernel/perf_event_paranoid = 4` (unprivileged
`perf_event_open` blocked) and no `perf` binary is installed. As the blueprint anticipated, the
pass fell back to **wall-clock (`Instant` per op) + syscall counts (`strace -c`) + allocation
reasoning**. No cache-miss/IPC numbers are reported because none could be honestly obtained — none
are fabricated.

---

## §0. Inventory re-verification (independent, against the live worktree)

Every §1 blueprint citation re-checked against the current checkout — **all accurate**:

| Citation | Verified |
|---|---|
| `simd.rs:164` `softmax_batch_lane`, `:296` `kalman_batch_step`, `:359` `kalman_batch_step_trust` | ✓ lane width 4 = `f64x4` |
| `fdr/ring.rs` `FdrRing::append` — `write(2)` to page cache; `sync_data` only on segment switch **or** `Kind::Alarm`/`PostMortem` (`:134`), `switch()` at `:141`, `DEFAULT_SEG_CAP = 1<<20` (`:33`) | ✓ built this session |
| `hydra.rs:1036` `FileEventStore::insert` — `open(append)`+`write_all`+`flush`+`sync_all` **per event**, index advances only after fsync | ✓ zero batching |
| `event_log.rs:302` `EventLog::append` (the `MeshEvent` entry point) | ✓ |
| `decision/import.rs:81` `import_unit` — per-unit; 2× SHA3-256 (integrity + `event_id`), replay loop, no IO on the append (`MemEventStore`) | ✓ |
| `hub_provisioning.rs:274` `POOL_REFILL_BATCH = 12` — self-labeled tunable, cold path | ✓ excluded (correctly) |

---

## §1. M1 — Event-log commit (`FileEventStore`-backed `EventLog::append`)

Per-event, direct per-op percentiles (n=3000, clean run):

| p50 | p99 | p99.9 | max | mean | throughput |
|---|---|---|---|---|---|
| **637 µs** | **1343 µs** | 5415 µs | 6766 µs | 661 µs | **1,513 events/s** |

Criterion cross-check: mean 628.6 µs, median 626.6 µs (30 samples) — agrees.

**`strace -c` (3000 events) — confirms the cost shape:** `fsync = 3000`, `write = 3011`,
`openat = 3006`, `close = 3006`. So it is **exactly one fsync per event, and also one
open+write+close per event** — `insert` re-opens the file every call. fsync is ~49% of syscall
time; the open/close overhead is real but secondary.

### Fsync-amortization curve (bench-crate group-commit model, n=20,000, *no kernel change*)

| sync every | per-record | throughput | vs g=1 | fsync-barrier p50 / p99 |
|---|---|---|---|---|
| 1 | 568 µs | 1,760 rec/s | 1.0× | 555 / 806 µs |
| 4 | 160 µs | 6,267 rec/s | 3.6× | 600 / 838 µs |
| 16 | 35.8 µs | 27,957 rec/s | 15.9× | 505 / 805 µs |
| 64 | 10.8 µs | **92,811 rec/s** | **52.7×** | 502 / 862 µs |

**Key finding (surprising, strongly in favour):** group-commit would be worth **~53× throughput**
at batch-64. Critically, **the fsync barrier latency itself stays ~500–860 µs regardless of batch
size** — batching does *not* inflate the barrier; the only latency it adds is the time a record
waits for its batch window to close. This is the classic group-commit trade in its most favourable
form.

**Recommendation — M1: BATCH-WORTHY, but operator-gated (durability-contract change).** This is
the single highest-value batching opportunity in the kernel. However, group-commit **changes the
crash contract**: events would be acknowledged before their fsync completes, so a crash could lose
the last unfsynced batch. The current per-event fsync is a *deliberate* durability-first choice
(typed `StoreError::Sync`, index-advances-after-sync ordering). The number to weigh: **~1,500
events/s today vs ~93,000/s at batch-64**. Verdict: file a design proposal for an *opt-in*
group-commit mode with an explicit acknowledged-before-durable window; do **not** silently change
the default. The per-event `open`/`close` is a cheaper, contract-neutral win worth noting
separately (keep the fd open across appends).

---

## §2. M2 — FDR ring (`FdrRing::append`)

Direct per-op percentiles:

| path | n | p50 | p99 | p99.9 | mean | throughput |
|---|---|---|---|---|---|---|
| normal (`Kind::Event`, page cache, no fsync) | 200,000 | 2.56 µs | 10.68 µs | 20.99 µs | 3.87 µs | 248,634 rec/s |
| alarm (`Kind::Alarm`, `sync_data`/record) | 3,000 | 571 µs | 845 µs | 1347 µs | 571 µs | 1,748 rec/s |
| forced segment-switch (fsync + reopen/truncate) | 2,000 | 720 µs | 1243 µs | 3674 µs | 746 µs | 1,340 rec/s |

Criterion cross-check: normal 3.77 µs, alarm 600.9 µs, switch 751.1 µs — agrees. The alarm/normal
ratio is **~148×** — the fsync entirely dominates; the record encode (`to_json` + crc32 + write) is
~3.9 µs.

### Segment-cap sweep (4096-record normal burst, per-record cost)

| cap | per-record | note |
|---|---|---|
| 256 KiB | 4.60 µs | more frequent `switch()` fsyncs |
| 1 MiB (default) | 3.69 µs | |
| 4 MiB | 3.30 µs | fewest switches |

**Key finding:** the 1 MiB → 4 MiB gain is only **~11%** on a sustained burst, because the record
write (~3.3 µs) dominates and the segment-switch fsync is amortized across **~8,000 normal records
per 1 MiB segment**. The FDR ring **is already the right batching design** — it amortizes the
571 µs fsync barrier over an entire segment, and pays per-record fsync only for `Alarm` records
that genuinely must be power-loss-durable immediately.

**Recommendation — M2: KEEP AS-IS. Do not batch, do not enlarge the cap.** The 1 MiB default is
sound: enlarging to 4 MiB buys ~11% throughput while widening the crash-recovery replay window and
the kill-9 loss surface — a bad trade for a forensic log. The Alarm-per-record fsync is correct by
design. This closes the blueprint's open "the FDR ring landed without a baseline" item: the
baseline now exists and it **validates the existing design**.

---

## §3. M3 — Decision-unit import (`import_unit`)

Per-unit CPU cost, replay-set size swept (no IO — `MemEventStore`):

| N cases | p50 | p99 | p99.9 | mean | throughput |
|---|---|---|---|---|---|
| 1 | 0.87 µs | 2.54 µs | 6.07 µs | 0.99 µs | 961,403/s |
| 8 | 0.87 µs | 1.14 µs | 5.86 µs | 0.89 µs | 1,079,069/s |
| 64 | 0.91 µs | 1.23 µs | 6.27 µs | 0.93 µs | 1,037,147/s |

Criterion cross-check: 849 / 853 / 888 ns — agrees. **Marginal per-case cost ≈ 0.6 ns** (the
1→64-case delta is ~38 ns *total*). The fixed ~0.9 µs is dominated by the **two SHA3-256 hashes**
(artifact integrity + `event_id`), not the replay loop and not allocation.

**Recommendation — M3: DON'T BATCH (measured, confirmed).** `import_unit` is sub-microsecond,
CPU-only, ~1M units/s single-threaded, with essentially zero marginal per-case cost. Batching
admission would buy **nothing** — there is no fsync or lock to amortize. The blueprint's hypothesis
("per-unit cost is CPU-trivial and batching would buy nothing") is a **measured, fully successful
"don't batch."** If anything is ever worth optimizing here it is the two SHA3 hashes, not batching.

---

## §4. M4 — Arena vs per-item allocation

**Skipped — correctly, per the blueprint's own gate** ("Skip entirely if M3 says allocation is
noise"). M3 shows allocation is noise: growing the replay set 1→64 adds ~0.6 ns/case, and the
per-unit cost is dominated by fixed hashing, not allocation. There is no allocation-cost signal to
chase, and PMU cache-miss counters (the tool M4 would use) are unavailable anyway (§ header).

---

## §5. Summary — one falsifiable verdict per path

| Path | Baseline (p50 / p99 / throughput) | Verdict |
|---|---|---|
| **M1 event-log commit** | 637 µs / 1343 µs / 1,513 ev/s | **BATCH-WORTHY (~53×), operator-gated** — group-commit changes the crash contract; propose opt-in, don't default. Also: stop re-opening the fd per event. |
| **M2 FDR ring** | normal 2.56/10.7 µs; alarm 571/845 µs | **KEEP AS-IS** — already amortizes fsync over a 1 MiB segment; cap enlargement buys only ~11%. Baseline now on record. |
| **M3 import_unit** | 0.87 µs / 1.14 µs (N=8) / ~1M u/s | **DON'T BATCH** — CPU-trivial, ~0.6 ns/case marginal, no IO to amortize. |
| **M4 arena** | — | **Skipped** (M3 gate: allocation is noise; PMU unavailable). |

**Done-when (roadmap item 26 proof) satisfied:** measured baselines exist for M1–M3 (M4 gated out),
P99 sits beside every throughput number, one falsifiable batch/don't-batch line per path, and **no
batching code landed in the kernel**.

---

## §6. Reproduction

Scaffolding (bench + `#[ignore]` test) committed on `exec/space-grade-tier0-2026-07-19`:

```
# criterion central estimates (sample-level):
cargo bench -p dowiz-kernel --bench batching

# direct per-op P50/P99 tail latencies:
cargo test -p dowiz-kernel --release --test item26_latency_probe -- --ignored --nocapture --test-threads=1

# 1-fsync-per-event confirmation:
strace -f -c -e trace=fsync,fdatasync,openat,write,close \
  target/release/deps/item26_latency_probe-<hash> --ignored m1_event_log_append_percentiles
```

Files: `kernel/benches/batching.rs`, `kernel/tests/item26_latency_probe.rs` (both measurement-only;
zero production-code change).
