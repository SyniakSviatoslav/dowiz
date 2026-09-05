# Fold specifications (T124, 2026-09-06)

Every std gate prints one i64; `bench/oracles/<gate>.py` recomputes it from the mathematical
definition, never from bebop. D11-E asked that the eight gates whose fold existed only in
code get a written specification. Each entry below is the normative statement; the oracle
is its executable twin and `bench/vs_rust/std_golden.sh` holds the frozen value.

## `csr` (frozen fold -6945622865743784444)

csr: base-131 i64-wrapping fold over row_ptr, then (col_idx, val) pairs of the five CSR GOLDENS graphs in golden.txt (P4, C3, K4W, B6, D2DUP)

- oracle: `bench/oracles/csr.py`; gate source: `selfhost/std/csr.bp`

## `bt` (frozen fold -5708805812714944038)

bt: .bt rank-4 codec — pack 2x3x2x2 tensor (magic BT4R, u32 ver=1, rank=4, dims, i64 LE data), FNV-1a 64 vs golden bt_fnv, unpack round-trip, stride offset(1,2,1,0)==22; fold = base-131 flag chain (i64 wrap)

- oracle: `bench/oracles/bt.py`; gate source: `selfhost/std/bt.bp`

## `store` (frozen fold 2245524994793680850)

store: pack rank-4 tensor dims (2,3,2,2), data[k]=((k*2654435761+7)&(2^44-1))-2^43 into the 220-byte "BT4R" v1 LE stream; FNV-1a 64 == golden bt_fnv; atomic tmp->rename publish, read back, FNV again, unpack round-trip. fold = i64-wrapped Horner base 131 over the check bits (7 ok bits, 24 mismatch bits, offset(1,2,1,0)==22, d2[0]==2 + d2[3]==2) atomic publish: write tmp, rename onto out, read back

- oracle: `bench/oracles/store.py`; gate source: `selfhost/std/store.bp`

## `tq` (frozen fold 722997760)

tq oracle: tensor query engine fold from the mathematical definition (fp Q32, fp_mul = trunc toward zero, fp_sqrt = isqrt(x<<26)<<3).

- oracle: `bench/oracles/tq.py`; gate source: `selfhost/std/tq.bp`

## `mvcc` (frozen fold 68412663603207)

Oracle for gate `mvcc` (T33): CoW versions + Grassmann reader tokens. Monomial form tok = sign*(2*mask+1); 1 = scalar (no readers); 0 = collapsed.

- oracle: `bench/oracles/mvcc.py`; gate source: `selfhost/std/mvcc.bp`

## `stm` (frozen fold 871596764015151)

Oracle for gate `stm` (T34): Z2 transactions -- odd-sector Grassmann write contexts, commit = nilpotency conflict test vs sheaf residual, abort = ctx^2 = 0. Monomial form ctx = sign*(2*mask+1); 0 = no context.

- oracle: `bench/oracles/stm.py`; gate source: `selfhost/std/stm.bp`

## `sort` (frozen fold 847859010857894)

sort: sort [3,1,4,1,5,9,2,6,5,3,5] ascending; fold = Horner hash acc = acc*31 + a[i] over the sorted array

- oracle: `bench/oracles/sort.py`; gate source: `selfhost/std/sort.bp`

## `rng` (frozen fold -552671757612340580)

rng: PCG64 RXS-M-XS seeded via SplitMix64 (rng_init(42,1), inc=3); fold = Horner acc = acc*31 + out over 8 outputs, i64 wrap

- oracle: `bench/oracles/rng.py`; gate source: `selfhost/std/rng.bp`
