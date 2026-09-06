# B2-PREP: decisive twins, preparation half (2026-09-06)

Prepared while the register-model rewrite of `bebop.bp` is in flight on another worker (its
binary is not promoted yet). Nothing here edits `bebop.bp`, `tools/`, `docs/`, or `selfhost/` —
every file listed below is new, under `bench/`. Scope: docs/blueprints/B2-decisive-twins.md §3
(i) join-as-SpGEMM, (ii) specialise-then-run scan, (iii) CSR-build profile method.

## What exists

| file | role |
|---|---|
| `bench/vs_rust/std_tests/join_twin.bp` | bebop join kernel (i): generator + local plain-array `csr_build` + Gustavson probe + folds. ~175 lines incl. comments. argv: `n dist mode` (`dist` = u/z, `mode` = c/s/f/b/p/t). |
| `bench/oracles/join_twin.py` | python oracle (stdlib): same generator + `join_fold`, bit-identical to the `.bp` twin; also imported as a library by `join_sqlite.py` and `twins_b2.sh`. std_golden candidate gate `join_twin`. |
| `bench/tq_sqlite/join_sqlite.py` | sqlite twin via ctypes (in-memory db, prepared statements), indexed and no-index `SELECT count(*), sum((a*b) % 2^61) FROM R JOIN S USING(k)`, `EXPLAIN QUERY PLAN` capture, LANG-DB §8 `ctypes_floor_us()` helper. |
| `bench/vs_rust/rust_once/join_hash.rs` | Rust twin: `HashMap<i64, Vec<u32>>` build+probe. |
| `bench/vs_rust/rust_once/join_merge.rs` | Rust twin: sort-merge join (two pointers, equal-key runs). |
| `bench/vs_rust/std_tests/gen_scan.py` | template writer for (ii): emits a self-contained `scan_<digest>.bp` for one concrete schema+predicate. Python, not `.bp` — see "Deliberate choices" below. |
| `bench/oracles/scan_twin.py` | python oracle for the scan (same LCG generator/predicate/aggregate). |
| `bench/vs_rust/rust_once/scan_const.rs` | Rust twin: schema+predicate as compile-time constants (the "best form" bebop's generated program represents). |
| `bench/vs_rust/rust_once/scan_generic.rs` | Rust twin: schema+predicate read at runtime through `Vec<Pred>`, applied by a 3-branch `match` interpreter (the honest "generic" form, source published per the blueprint's risk #4). |
| `bench/vs_rust/std_tests/csr_build_profile.bp` | (iii): local, instrumented copy of `selfhost/std/sgraph2.bp`'s `phase_build`/`csr_build`, printing a `clock_ms` delta per sub-phase (pair generation, counting pass, prefix sum, fill pass, `st_seal`, `st_commit`) via `sys_write`. Opens its own `b2_csrprofile.store`, never touches `sgraph.store`/`sgraph2.store`. |
| `bench/vs_rust/twins_b2.sh` | harness for (i)+(ii): compiles/builds every twin, checks folds first, times `R` reps each, appends gate verdicts to `bench/vs_rust/REPORT-b2.md`. |
| `bench/vs_rust/csr_profile_b2.sh` | driver for (iii): compiles/runs `csr_build_profile.bp` at the sgraph2 scale (n=1e6, e=5e6) and prints the phase table + top phase. |

## The one-command run for later (A1 GREEN, register-model `bebop.bin` promoted)

```
BEBOP_BIN=./bebop.bin BEBOP_TMP=$OUT R=11 bash bench/vs_rust/twins_b2.sh
BEBOP_BIN=./bebop.bin BEBOP_TMP=$OUT        bash bench/vs_rust/csr_profile_b2.sh
```

`twins_b2.sh` writes `bench/vs_rust/REPORT-b2.md` (append) with the join and scan gate rows
per blueprint §7/§9. `csr_profile_b2.sh` prints the phase table for a human/script to fold into
the same report (kept separate: it never touched sgraph2.bp, and the blueprint's own §5 keeps
it as a distinct row).

## What was verified functionally, now, against the committed `./bebop.bin` (md5 `f7a25d38`)

All folds compared bebop vs python oracle vs Rust vs sqlite (join) or bebop vs python vs Rust
(scan). No timing number is claimed or recorded anywhere in this prep — every `ms` printed
during verification below is from the current STACK-MACHINE `bebop.bin`, on A55 cores 0-3
(nice -n 10, one process at a time, per this session's box rules), and is meaningless as a
performance number; it is here only to show the checks completed in seconds, not that the
kernels were fast.

**Join, n=1,000,000 (the blueprint's actual target scale — not just a reduced-N spot check):**

| dist | bebop count/checksum | python | rust_hash | rust_merge | sqlite idx | sqlite no-idx |
|---|---|---|---|---|---|---|
| uniform | 1000885 / 1077689888743786 | equal | equal | equal | equal | equal |
| zipf | 9485500 / 10191336072212230 | equal | equal | equal | equal | equal |

Combined fold (`count*1000000007 + checksum`) matched bit-for-bit across bebop, python, both
Rust twins, and both sqlite plans, on both distributions, at full N. Zipf pair count (9.49M)
lands inside the blueprint's own "~10M pairs" target (§8 risk row) by construction, not by an
extra cap — see "Deliberate choices" below.

**Scan, N checked at 20,000 and 100,000** (bebop's stack-machine interpreter makes 1,000,000
run in a few seconds too, but 20k/100k already exercise the same straight-line code the
generator emits): bebop, python oracle, `rust_const`, `rust_generic` all agree exactly (e.g.
N=100,000: sum=7,480,008,934 on all four).

**CSR-build profile mechanism**: compiled clean against `./bebop.bin`; runtime-smoke-tested at
n=1,000/e=2,000 (prints all 6 phase lines, commits, exits 0). The real n=1e6/e=5e6 profile
(45-90s per HISTORY) was deliberately **not run** — deliverable 5 says so explicitly, and nice
was to keep this session inside the A55/nice/one-process box rules rather than race the other
worker's A78 timing lanes.

A real bug was caught and fixed during this verification: the Rust scan twins initially used
`i64` for the LCG state, so Rust's `>>` did an arithmetic (sign-extending) shift where
bebop's/python's is logical (unsigned) — this silently produced a different key stream and a
wrong sum until both files were switched to `u64` state (see the header comments in
`scan_const.rs`/`scan_generic.rs`). The join Rust twins were written with `u64` state from the
start and never hit this. Also hit and fixed: `st_digest` (used by the CSR profile) needs
`selfhost/prelude/sha256.bp` imported alongside `selfhost/prelude/store.bp` (it calls
`sha256_lo64` internally) — `sgraph2.bp` already imports both; my first draft only imported
`store.bp` and got `trap 87: call to an unresolved function` at runtime.

## Deliberate choices (say-what-was-decided, per the blueprint's own escape hatches)

- **`csr_build` is a local plain-array counting sort**, not the store-object `csr_build` in
  `selfhost/std/sgraph2.bp:24`. That one runs inside an `st_begin`/`st_alloc`/`st_seal`/
  `st_commit` transaction and persists `rp`/`ci` as store objects — plumbing this twin doesn't
  need (it never opens a store file for the join) and that would add file I/O to exactly what's
  being measured. Same counting-sort algorithm, verbatim in spirit.
- **Zipf(1.1) is implemented as a two-bucket heavy/light split** (1% of keys, chosen via a
  30%-probability LCG coin, carry 30% of the rows), not a literal Zipf(1.1) rank-frequency CDF.
  bebop has no floats, so a literal Zipf CDF sampler would need fixed-point or a precomputed
  table shared across languages — extra machinery for a property the blueprint states directly
  ("1% of keys carry 30% of the rows"). This construction reproduces that property exactly by
  construction and, as a bonus, naturally lands the pair count at the blueprint's own "~10M
  pairs" target at N=1M (9.49M measured) without a separate multiplicity cap. If a future row
  needs the actual Zipf(1.1) exponent (e.g. for a paper-comparable curve), this needs revisiting.
- **Payload range is bounded (`[0, 65536)`) so the true mathematical `sum(a*b)` never
  approaches i64 overflow** at the pair counts this generator produces (~4.3e16 at N=1M, vs
  9.2e18 the i64 ceiling), rather than relying on wraparound semantics as the blueprint's prose
  suggests ("checksum ... exact in wraparound i64"). This sidesteps SQLite's silent
  integer-to-REAL promotion on arithmetic overflow (undocumented-enough behavior that gating
  correctness on all four engines wrapping identically felt like the wrong thing to bet on). The
  `% 2^61` step is still present in every twin for spec fidelity; it is a no-op at these
  magnitudes. If payload ranges ever need to be wider, this needs revisiting together with
  SQLite's overflow behavior.
- **The scan template writer (`gen_scan.py`) is python, not `.bp`** — the blueprint's own text
  allows either ("a tiny generator gen_scan.bp (or python for now, blueprint decides)"). The
  gated quantity is compile-time + run-time of the *generated* program, not the time to expand
  the template, so this changes nothing about what the gate measures; a native `.bp` generator
  (using `sys_write`/`st_str`, both confirmed working in `csr_build_profile.bp`) can replace it
  later without touching the gate math in `twins_b2.sh`.
- **`twins_b2.sh` uses R full-process-run medians (honest.sh's outer-R method), not an
  in-process REPS=100 loop** for the join/scan kernels themselves. honest.sh needed REPS=100 to
  get the sub-ms `k*h` kernels above `clock_ms`'s 1ms resolution; a 1M-row join or 1M-row scan
  is already multiple ms, so REPS=1 (default, still wired into every Rust binary and into
  `join_twin.bp`'s mode system) is enough. This can be bumped later without code changes.

## Open (deferred to the register-model binary / later sessions)

- Every `ms` row of B2 (join uniform/zipf, scan compile+run, CSR-build phases) — by design,
  per the task: "the register-model binary is NOT available yet."
- The gate verdicts themselves (`>= 10x sqlite native AND >= 0.7x best Rust`, scan
  `<= rust_generic/5`, `<= 1.5x rust_const`) — computed by `twins_b2.sh` but not trustworthy
  until run against the promoted binary.
- `bench/vs_rust/REPORT-b2.md` does not exist yet — it is created by the first real run of
  `twins_b2.sh`. (A smoke-test run at N=3000/R=2 was done against a scratch report path during
  prep, confirming the script's orchestration has no crashes; it never touched the real
  `REPORT-b2.md`.)
- The sqlite timing rows in `twins_b2.sh` measure raw `sqlite3_step` wall time; LANG-DB §8's
  ctypes-floor subtraction is wired (`join_sqlite.ctypes_floor_us()`) but not applied inline,
  since the floor is per ctypes *call* (~1-7 us here, machine-dependent — measured 1.17 us/call
  in LANG-DB §8's own run, 6-7 us/call in this session's proot/A55 environment) and this loop
  makes exactly 2 calls/rep against a query that (at 1M rows) will cost single-digit-to-tens of
  ms — likely negligible, but state it rather than assume it when the real numbers land.
- A literal Zipf(1.1) CDF generator, if a future blueprint needs the exact exponent rather than
  the "1%/30%" property (see "Deliberate choices").
