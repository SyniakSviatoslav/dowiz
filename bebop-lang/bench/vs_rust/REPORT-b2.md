
## B2 decisive twins (2026-09-13, bebop.bin 072c01d1, core 4, R=11, N_JOIN=1000000, N_SCAN=1000000, SCAN_REPS=200) — folds equal

### (i) join as SpGEMM — bebop CSR build + Gustavson probe, generation untimed everywhere

| dist | bebop total ms | (build) | (probe) | rust_csr32 ms | rust_csr64 ms | rust_hash ms | rust_merge ms | sqlite_idx ms | sqlite_noidx ms | pairs | sqlite_best/bebop | rust_best/bebop | gate |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| u | 234.0 | 75.0 | 158.0 | 149.4 | 164.9 | 798.2 | 353.0 | 2272.0 | 7128.4 | 1000885 | 9.7x | 0.64x | UNMET |
  - plan idx: SCAN R | SEARCH S USING INDEX idx_s_k (k=?); plan no-idx: SCAN R | BLOOM FILTER ON S (k=?) | SEARCH S USING AUTOMATIC COVERING INDEX (k=?); ctypes floor: 2.08 us/call (2 calls/rep, not subtracted)
| z | 569.0 | 67.0 | 493.0 | 356.7 | 371.4 | 990.0 | 386.7 | 11587.5 | 8240.1 | 9485500 | 14.5x | 0.63x | UNMET |
  - plan idx: SCAN R | SEARCH S USING INDEX idx_s_k (k=?); plan no-idx: SCAN R | BLOOM FILTER ON S (k=?) | SEARCH S USING AUTOMATIC COVERING INDEX (k=?); ctypes floor: 2.08 us/call (2 calls/rep, not subtracted)

`rust_best` is the minimum over ALL four Rust twins — including rust_csr, which runs bebop's own algorithm. Reading the gate against rust_hash/rust_merge alone would credit bebop with an algorithm choice; see this script's header.

### (ii) specialise-then-run scan over a materialised 3-column table

| row | ms |
|---|---|
| bebop cold specialisation (compile a never-seen query), median of 11 | 33.7 |
| bebop scan, per pass (median of 11 runs of 200 in-process passes) | 3.145 |
| **bebop FIRST: compile + one scan** | **36.8** |
| **bebop REPEAT: one scan, compile memoised away** | **3.145** |
| rust_generic (predicate interpreted at run time) | 9.825 |
| rust_const (query known to rustc — the unattainable floor) | 1.758 |

crossover: bebop saves 6.680 ms per 1000000-row scan against rust_generic, so the 33.7 ms specialisation pays for itself after **5.0 executions** of this query, or in a single query over **5.0M rows**.
specialisation is worth 5.59x inside Rust itself (rust_generic / rust_const); bebop's specialised scan is 1.79x rust_const and 3.12x faster than rust_generic.

### (iii) CSR-build profile

not run by this script — two commands, both needed, and they must be read together:
```
BEBOP_BIN=./bebop.bin BEBOP_TMP=$OUT bash bench/vs_rust/csr_profile_b2.sh   # store arm
./seed/build/seed $OUT/b2/csr_build_plain.bin 1000000 5000000                # plain arm
```
The store arm (bench/vs_rust/std_tests/csr_build_profile.bp) is sgraph2.bp's phase_build with rp/ci as store objects; the plain arm (csr_build_plain.bp) is the identical counting sort over `zeros()` arrays. Both print the same fold, so the difference between their phase tables is the store and nothing else.

## B2 decisive twins (2026-09-13, bebop.bin 072c01d1, core 4, R=11, N_JOIN=1000000, N_SCAN=1000000, SCAN_REPS=200) — folds equal

### (i) join as SpGEMM — bebop CSR build + Gustavson probe, generation untimed everywhere

| dist | bebop total ms | (build) | (probe) | rust_csr32 ms | rust_csr64 ms | rust_hash ms | rust_merge ms | sqlite_idx ms | sqlite_noidx ms | pairs | sqlite_best/bebop | rust_best/bebop | gate |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| u | 233.0 | 81.0 | 145.0 | 152.7 | 169.5 | 788.3 | 356.0 | 2291.7 | 7129.9 | 1000885 | 9.8x | 0.66x | UNMET |
  - plan idx: SCAN R | SEARCH S USING INDEX idx_s_k (k=?); plan no-idx: SCAN R | BLOOM FILTER ON S (k=?) | SEARCH S USING AUTOMATIC COVERING INDEX (k=?); ctypes floor: 2.21 us/call (2 calls/rep, not subtracted)
| z | 570.0 | 66.0 | 494.0 | 357.0 | 379.1 | 1005.0 | 435.8 | 11799.4 | 8278.5 | 9485500 | 14.5x | 0.63x | UNMET |
  - plan idx: SCAN R | SEARCH S USING INDEX idx_s_k (k=?); plan no-idx: SCAN R | BLOOM FILTER ON S (k=?) | SEARCH S USING AUTOMATIC COVERING INDEX (k=?); ctypes floor: 1.91 us/call (2 calls/rep, not subtracted)

`rust_best` is the minimum over ALL four Rust twins — including rust_csr, which runs bebop's own algorithm. Reading the gate against rust_hash/rust_merge alone would credit bebop with an algorithm choice; see this script's header.

### (ii) specialise-then-run scan over a materialised 3-column table

| row | ms |
|---|---|
| bebop cold specialisation (compile a never-seen query), median of 11 | 32.7 |
| bebop scan, per pass (median of 11 runs of 200 in-process passes) | 3.090 |
| **bebop FIRST: compile + one scan** | **35.8** |
| **bebop REPEAT: one scan, compile memoised away** | **3.090** |
| rust_generic (predicate interpreted at run time) | 9.754 |
| rust_const (query known to rustc — the unattainable floor) | 1.597 |

crossover: bebop saves 6.664 ms per 1000000-row scan against rust_generic, so the 32.7 ms specialisation pays for itself after **4.9 executions** of this query, or in a single query over **4.9M rows**.
specialisation is worth 6.11x inside Rust itself (rust_generic / rust_const); bebop's specialised scan is 1.93x rust_const and 3.16x faster than rust_generic.

### (iii) CSR-build profile

not run by this script — two commands, both needed, and they must be read together:
```
BEBOP_BIN=./bebop.bin BEBOP_TMP=$OUT bash bench/vs_rust/csr_profile_b2.sh   # store arm
./seed/build/seed $OUT/b2/csr_build_plain.bin 1000000 5000000                # plain arm
```
The store arm (bench/vs_rust/std_tests/csr_build_profile.bp) is sgraph2.bp's phase_build with rp/ci as store objects; the plain arm (csr_build_plain.bp) is the identical counting sort over `zeros()` arrays. Both print the same fold, so the difference between their phase tables is the store and nothing else.
