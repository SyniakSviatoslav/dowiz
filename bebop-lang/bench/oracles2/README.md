# bench/oracles2 — second INDEPENDENT oracles (ROADMAP D11-E)

Written 2026-09-05 from prose specifications only. Per the D11-E rule the
`.bp` implementations (`selfhost/std/<gate>.bp` code, `bench/vs_rust/std_tests/`),
the first-generation mirrors (`bench/oracles/<gate>.py`), the gate lines of
`std_golden.sh` and the fold lines of `docs/exp.journal` were NOT opened.
Old oracles were executed only as black boxes by `run_all2.sh`.

Run: `bash bench/oracles2/run_all2.sh` (≈70 s; nnidx dominates). Each oracle is
python3 stdlib, deterministic, prints its fold as the LAST line; guessed
quantities are `env` parameters named in the module docstring.

Disclosure: while grepping the `#` header comments of `std_golden.sh` for the
sha256/crc inputs (numbers ≥ 6 digits were masked with sed), the un-masked first
pass printed the store comment "fold 2245524994793680850" (file line 114). That
number was not used to derive anything — store.py's default fold is the FNV of the
stream and MISMATCHes it, as expected. The csr/bt/tq/mvcc/stm/money/ordfsm frozen
values are quoted in ROADMAP prose itself (allowed); only for csr/bt a small family
of natural folds was tried against them (none matched; see below).

## run_all2 result

| gate | result | new (oracles2) | old (bench/oracles) |
|---|---|---|---|
| csr | MISMATCH | 6393907679023662745 | -6945622865743784444 |
| bt | MISMATCH | -6204655307031605165 | -5708805812714944038 |
| store | MISMATCH | -6204655307031605165 | 2245524994793680850 |
| tq | MISMATCH | 593690491 | 722997760 |
| mvcc | MISMATCH | 21255159101507 | 68412663603207 |
| stm | MISMATCH | 590346954008031 | 871596764015151 |
| sha256 | MATCH | -4000131497313522475 | -4000131497313522475 |
| crc | MATCH | 3421780262 | 3421780262 |
| sort | MISMATCH | 7338407803577109147 | 847859010857894 |
| rng | MISMATCH | 8827004508344512654 | -552671757612340580 |
| money | MATCH | 872656672063013 | 872656672063013 |
| ordfsm | MATCH | 346243789026198 | 346243789026198 |
| nnidx (3x3 window fold = the gate) | MISMATCH vs old last line | 721792946 | 261942249 |
| nnidx (exact-nearest fold) | MATCH | 261942249 | 261942249 |

Reading: 4 gates (+ the nnidx truth half) are reproducible from the specification
alone and agree bit-exactly with the first-generation oracle → those oracles are
now doubly independent. 8 MISMATCHes are all "the fold is not written down
anywhere in prose" (details per gate) — none is evidence of a wrong first oracle,
but none confirms it either. The nnidx old oracle prints the brute-force fold as
its last line while the gate (`nnidx.bp` comment) folds the windowed answer; the
window/exact disagreement is 2/1000 queries, exactly what RESULT.md records.

## Per gate

Notation: "spec" = prose sources used (file:lines), "gap" = quantity missing from
prose (made a parameter), "conf" = confidence that the oracle computes the gate's
number given the spec (not that it matches the frozen value).

### csr — MISMATCH
- spec: ROADMAP.md 2371-2376 (from_edges contract); selfhost/std/csr.bp comment
  lines 1-18 (rp/ci/vv layout, fp Q32, merge/ignore rules, caps); std_golden.sh
  header comment 71-72 ("fold over rp+ci+vv of the five golden graphs");
  bench/vs_rust/spectral_golden/README.md 44-49; golden.txt CSR GOLDENS section
  (expected rp/ci/vv — asserted); generator/src/main.rs 133-152 (edge lists,
  both directions); crates/dowiz-core/src/csr.rs from_edges (the reference).
- gap: the fold function. Tried fnv_cells / poly31 / MMIX-mix / T66-mix, with and
  without (n, nnz) prefixes: none gives the ROADMAP-quoted value. Default: FNV-1a
  over every cell of rp, ci, vv, graph after graph (`FOLD=fnv|poly31|mmix|mix1000003`,
  `META=1` adds n,nnz).
- conf: from_edges half exact (asserted against golden.txt); fold 0.

### bt — MISMATCH
- spec: selfhost/std/bt.bp comment 1-21 (format v1); spectral_golden/README.md
  50-57; golden.txt .bt section (220 bytes, FNV 12242088766677946451 — asserted);
  generator/src/main.rs 155-178 (data formula).
- gap: how "pack/FNV/unpack/stride roundtrip flags" become one number. Default =
  the FNV (signed); `FOLD=fnv_flags3|fnv_flags1000|fnv_plus_flags|fnv_x31_chain`.
- conf: stream/FNV/unpack/stride exact; fold 0.

### store — MISMATCH
- spec: std_golden.sh header comment 112-115 (tmp → export → rename → read back →
  unpack vs golden, same 220-byte stream); ROADMAP.md 1024-1026, 1556-1559
  (publish rule). The file round trip is performed for real (tempfile + os.replace).
- gap: the fold over (stream, round-trip flags). Default = FNV of the bytes read back.
- conf: round trip exact; fold 0.

### tq — MISMATCH
- spec: ROADMAP.md 902-910 (T20 GOAL/DONE-CHECK); selfhost/std/tq.bp comment 1-22
  (R^4 fp Q32, ev0/ev1 projection, GRID_RES=8, K anchors, 3-segment walk, break
  best_d < 0.1, 3x3 window, fp_mul/fp_sqrt definitions, fold = polynomial hash mod
  1e9+7 of (idx, dist, count)); crates/dowiz-core/src/parametric_spectral.rs
  grid_cell/search_spins and memory_search.rs geodesic_distance (reference);
  selfhost/prelude/fp.bp (fp_mul, isqrt).
- gap: N, the point/query generator, how ev0/ev1 are obtained (a fp top-2 eigen
  solve of an unspecified matrix), K and which points are anchors, the polynomial
  base, the tie rule. Parameters `N K Q BASE SEED`; ev0/ev1 = axis vectors.
- conf: geometry/fp arithmetic per prose ~0.7; data/hash 0.

### mvcc — MISMATCH
- spec: ROADMAP.md 1229-1240 (T33); selfhost/std/mvcc.bp comment 1-20 (token
  encoding, acquire/release/collapse rules, 64 steps, fold formula); ROADMAP.md
  1036-1041 (T22 Grassmann product rule). The token algebra (sign*(2*mask+1),
  nilpotent acquire, contraction) is implemented per the T22 rule.
- gap: which LCG and how it selects (op, reader), the mixing hash, the initial
  value, exact meaning of `surv` and `acct`. Parameters `SEED`; mix = T66 mix.
- conf: 0.1.

### stm — MISMATCH
- spec: ROADMAP.md 1242-1252 (T34); selfhost/std/stm.bp comment 1-20 (odd-sector
  context, two slots, `win`, conflict detectors, Stokes check, fold formula).
- gap: LCG, schedule (begin/write/commit mix, writes per txn, values), mix, store
  hash, step count. Parameters `SEED STEPS`.
- conf: 0.1.

### sha256 — MATCH
- spec: FIPS 180-4; selfhost/std/sha256.bp comment 1-21 (fold acc*31+word over the
  8 words, u64 wrap); std_golden.sh header comment 45-48 (input "abc").
- gap: acc0 (taken 0) and signedness of the printed value (i64 cell) — both
  resolved by the match. hashlib digest asserted against the FIPS vector.
- conf: 1.0.

### crc — MATCH (old oracle name crc32)
- spec: CRC-32/ISO-HDLC; selfhost/std/crc.bp comment 1-12; std_golden.sh header
  comment 52 (check value for "123456789"). Own table implementation, asserted
  against zlib.crc32.
- gap: none after the match (the gate is the bare check value).
- conf: 1.0.

### sort — MISMATCH
- spec: selfhost/std/sort.bp comment 1-3 only (is_sorted, find_min_idx ⇒ selection
  sort); ROADMAP.md 152-154 lists the gate without a definition; std_golden.sh
  header comment 33 is an empty section marker.
- gap: EVERYTHING about the driver — input array, n, fold. Parameters `N SEED`.
  Algorithm half (first-minimum selection sort, is_sorted) exact.
- conf: 0.

### rng — MISMATCH
- spec: selfhost/std/rng.bp comment 1-24 (constants, splitmix contract, pcg_step).
  rng.c was deleted at M7 (ROADMAP.md 155), so the RXS-M-XS output permutation is
  the public PCG definition. SplitMix64 verified against golden.txt hv code(0)
  word 0 (0xE220A8397B1DCDAF for seed 0).
- gap: seed, increment, number of draws, fold. Parameters `SEED INC DRAWS`.
- conf: primitives 0.9 (splitmix), 0.6 (pcg output permutation); driver 0.

### money — MATCH
- spec: ROADMAP.md 1897-1906 (T66); selfhost/std/money.bp comment 1-22 (op codes,
  tag/reason codes, fold); crates/dowiz-core/src/money.rs (the production law,
  ported to Python: i128 = int, truncating division, math::round half-away);
  bench/oracles/rust/src/bin/money.rs (allowed: case table, LCG seed 4242, mix).
- gap: none — the harness is documented in the bin and the arithmetic is the
  production code. The oracle does not call cargo, so it is independent of the
  Rust build.
- conf: 1.0 (bit-exact with the cargo oracle).

### ordfsm — MATCH
- spec: ROADMAP.md 1897-1906; selfhost/std/ordfsm.bp comment 1-22 (states, codes,
  sections A/B1/B2/C, golden signature); crates/dowiz-core/src/order_machine.rs
  (allowed_next, assert_transition, fold_transitions, reachable, Kahn, has_cycle,
  cyclomatic, signature); bench/oracles/rust/src/bin/ordfsm.rs (hand table, seed 1234).
- gap: none. conf: 1.0.

### nnidx — MATCH on the exact-nearest fold; window fold has no old last line
- spec: bench/tq_sqlite/oracle.py docstring lines 2-15 (LCG seed 12345, u/v law,
  cell law, fold = Σ id·131^i mod 1e9+7); RESULT.md 1-14 (998/1000 window hits);
  bench/tq_sqlite/nnidx.bp comment 1-24 (3x3 window, lowest d then lowest id, -1
  if empty, returns fold*1e6 + query_ms).
- gap: whether the seed is stepped before the first draw (`ADVANCE_FIRST`, True
  reproduces the truth fold); the ms part of the gate value is timing and is not
  reproduced. Query-window clipping at the grid border assumed (search_spins style).
- conf: 0.95 (window fold), 1.0 (exact fold, matches the cached truth).

## Underspecified quantities (the finding list)
1. csr: the fold over rp/ci/vv (function, initial value, whether n/nnz enter).
2. bt: the combination of FNV + len/unpack/stride flags into one number.
3. store: the fold over the published stream + round-trip flags.
4. tq: N, point/query generator, ev0/ev1 derivation, K + anchor choice, hash base, tie rule.
5. mvcc: LCG + schedule decoding, mixing hash, definitions of `surv`/`acct`.
6. stm: LCG + schedule, write values, mix, store hash, step count.
7. sort: input array, n, fold (nothing in prose beyond two helper names).
8. rng: seed, increment, number of draws, fold; PCG output permutation only public.
9. sha256: acc0 and signedness (resolved by the match, but not written down).
10. nnidx: seed-advance convention; the gate value's timing component.
