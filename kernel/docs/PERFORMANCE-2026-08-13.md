# dowiz-kernel — Performance & Metrics Report

Captured 2026-08-13 (aarch64 host, Linux 6.17.0-PRoot-Distro).
This is the post-rewrite snapshot (glyph-geometry migration, phases A–F complete).

## Build / Size

| Metric | Value |
|---|---|
| `cargo check --lib` | exit 0 (clean) |
| `cargo build --release` (LTO) | 1m 38s |
| `target/release` total | **34 MB** |
| `libdowiz_kernel.rlib` | **26.83 MB** |
| `libdowiz_kernel.so` | 0.07 MB (cdylib shell) |
| rlib code (text) | **2.45 MB** (2,567,907 bytes) |
| rlib data | 77,805 bytes |
| largest binary (`enrich`) | 0.72 MB |

## Source / Symbol counts

| Metric | Value |
|---|---|
| `.rs` files (src/) | **301** |
| top-level modules (src/*.rs) | 181 |
| total source lines (src/*.rs) | **149,929** |
| docs .md files / lines | 4 / 823 |
| defined symbols (rlib) | **8,262** |
| code symbols (T/t/W/w) | 6,035 |

## Test suite

| State | Value |
|---|---|
| full `cargo test --lib` | **3092 passed, 0 failed, 1 ignored** |
| (before fix) | 3091 passed, 4 failed (PMU ×3 + hw_profile) |
| fix | PMU tests arch-gated to x86_64; hw_profile ARM64 1:1 fallback |

## Latency benchmarks (criterion, `--warm-up 1 --measure 2 --sample 10`)

Baseline (`benches/baseline.json`) was captured 2026-07-13 on **x86_64
(Linux 6.8.0-124-generic)**. This host is **aarch64**, so the two columns are
NOT a like-for-like A/B — they differ in ISA, microarchitecture and clock.
Reported side-by-side for completeness; treat as informational only.

| benchmark | baseline x86_64 (ns) | current aarch64 (ns) |
|---|---|---|
| place_order/5_items | 74.9 | 130.7 |
| fold_transitions/5_hops | 4.27 | 14.7 |
| ppr/rank_32x32_k20 | 8,043 | 22,955 |
| retrieval/recall_at_k_5 | 8,397 | 15,212 |
| attention/matmul_8x8 | 1,278 | 1,744 |
| token_bucket/try_acquire_permit | 51.9 | (not re-run) |
| spine_build/16 | 18,500 | 38,930 |
| spine_build/64 | 62,143 | 130,590 |
| spine_build/256 | 229,310 | 467,350 |
| spine_build/1024 | 902,070 | 1,969,900 |
| spool_drain/16 | 942 | 1,818 |
| spool_drain/64 | 3,938 | 8,059 |
| spool_drain/256 | 17,033 | 35,190 |
| spool_drain/1024 | 68,381 | 133,920 |
| graph_rebuild_rank/heap | 120,300 | 286,910 |

## Microbench (same-machine A/B, release, aarch64)

| measurement | result |
|---|---|
| drift classification: if/else | 0.601 ns/op |
| drift classification: branchless LUT | **0.456 ns/op** |
| LUT speedup | **1.32×** |
| hypervector rank over 1000 docs | 113.6 ns/doc |
| single hypervector similarity (O(1), 16×u64) | 1.042 ns |
| squash: 10 KB repetitive | 81 bytes (**123.5×** shrink) |
| squash: 10 KB monotonic (wrapping) | raw fallback (1.0×, correct) |

## size_of (bytes)

| type | size |
|---|---|
| Hypervector (1024-bit VSA) | 128 |
| Squash (compressed blob) | 40 |
| HvDocument | 160 |
| HypervectorIndex | 24 |

## Energy efficiency

**Not measurable on this host.** aarch64 container has no RAPL / no
`perf_event_open` (that is exactly why the PMU tests were arch-gated). A real
energy number requires a bare-metal x86_64 host with RAPL or `perf stat`
access. Honest status: energy delta is unmeasured, not "zero".

## Rewrite summary (what shipped)

Phases A–F of `docs/architecture/GLYPH-GEOMETRY-MIGRATION.md`:

- A: `src/lut.rs` (n(0) LUT, branchless) + `src/constants.rs` (single authority)
- B: geometry on `eigen`/`tensor` (`Phase`, `angle_with`)
- C: `src/glyph_dashboard.rs` (sparkline/heatmap/scatter/braille bridge)
- D: `src/hypervector_index.rs` (VSA document index + cosine oracle)
- E: `src/squash.rs` (zero-dep RLE/delta compression)
- F: sweep — `sys_dashboard` if/else → LUT, `telemetry_aggregator` glyph report
