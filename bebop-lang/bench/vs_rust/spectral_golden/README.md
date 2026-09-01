# Spectral + HDC golden vectors (extracted from Rust BEFORE Zero-C)

Per ROADMAP spectral-tier law: "витягти golden-вектори (еталонні спектри +
Householder-vs-Faddeev паритет-набір) з C/Rust ДО видалення native/src. Після —
паритет тільки проти golden."

## Contents

`golden.txt` — sections:
1. `════ HDC GOLDENS ════` — Ф1 consumer format: `code(seed)` = splitmix64-filled
   16×u64 (D=1024), bind=XOR (+self-inverse proof line), bundle=majority
   (ties→0), permute=bit-rotation (1/64/255/1023 + 0≡1024 identity),
   hamming/popcount ints, similarity as f64 bits, spectral-role XOR pattern
   (code(0xA1) role, code(11) item).
2. `== <graph> ... topk_symmetric` — SPECTRAL tier: power+Hotelling over Csr
   spmv, 32 iters, LCG start (0x9E3779B97F4A7C15; ×6364136223846793005;
   +1442695040888963407; frac=(rng>>11)/2^52), sign = first-nonzero>0,
   descending |λ|. Graphs: P4, C3, S5, C4, K4-weighted, B6-bridge.
3. `== eigh ...` — Householder dense (n≤32) reference for the same graphs
   (parity oracle: power-vs-Householder λ agreement ≤ few LSBs at fp32 scale).
4. `== charpoly` — LeVerrier coefficients (exact in i64 only for n≤16).

Format per vector: `*_fp32:` = i64 fixed-point scale 2^32 (the .bp port's
consumer format), `*_bits:` = raw f64 bit patterns (hex, for bit-exact port
debugging). Words in `hv` lines are hex u64, word 0 first.

## Regenerate

```sh
cd generator && cargo run --release > ../golden.txt
```
(dowiz-core path dep: ../../../../crates/dowiz-core)

## Consumer contract (Ф-port laws)

- start vector: LCG above, x[i]=frac*2−1, then normalize; orthogonalize start
  AND each iteration against already-found pairs (per-spmv Hotelling, Csr
  never densified).
- fixed summation order = Csr spmv row/col-ascending order.
- Rayleigh λ = xᵀ(Ax) on the deflated space, recompute-deflate-then-dot.
- zero-vector placeholder if deflated away (nx==0).
- Ф7 fingerprint: sorted top-k λ + sign-normalized Fiedler = layout-invariant.

## Ф2 additions (2026-09-01)

- `════ CSR GOLDENS ════` — structural from_edges reference (P4/C3/K4W/B6/
  D2DUP, symmetrized): row_ptr / col_idx / val_fp32. D2DUP exercises the
  duplicate-column merge (wrapping sums are order-independent). Consumer:
  selfhost/std/csr.bp (gate `csr` in std_golden.sh, frozen -6945622865743784444).
- `════ .bt RANK-4 GOLDEN ════` — the canonical .bt artifact format v1:
  magic "BT4R" (4B), u32 version=1, u32 rank=4, u32 dims[4], dense i64 LE
  data (row-major rank-4 strides: ((i*D1+j)*D2+k)*D3+l). Golden: dims
  2x3x2x2, data[k] = ((k*2654435761+7) mod 2^44) - 2^43, 220 bytes,
  bt_fnv = FNV-1a 64 over the byte stream (signed -6204655307031605165).
  Consumer: selfhost/std/bt.bp (gate `bt`, frozen -5708805812714944038 —
  pack/FNV/unpack/stride roundtrip flags folded).
