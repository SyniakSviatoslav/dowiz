# OPUS — ML-KEM-768 NTT: implementation + exhaustive correctness proof

**Date:** 2026-07-18
**Author:** Opus (research + implementation pass, follow-up to `OPUS-PERF-BEBOP-AUDIT-2026-07-18.md` finding **P3**)
**Red-line class:** post-quantum crypto (ML-KEM key exchange). Treated with the repo's crypto-change discipline: the fast path is added **alongside** the schoolbook multiply, **not wired into the live KEM**, and gated on a bit-identity proof.

**Status: VERIFIED CORRECT.** A correct FIPS-203 incomplete NTT for the ML-KEM ring was implemented and **exhaustively proven** bit-identical to the existing schoolbook `poly_mul` (all 65 536 monomial basis pairs, 0 mismatches — a *complete* proof, not a sample). The code is committed to `bebop-repo` as new, non-default-active functions. It is **not** wired into `keygen`/`encaps`/`decaps`; the call-site swap is left for explicit sign-off, per the prior-incident discipline.

---

## 1. What was found (ground truth, read from source — not assumed)

### 1.1 bebop2/core `pq_kem.rs` — the real ML-KEM-768, schoolbook by deliberate choice
- Ring multiply is `poly_mul` (`pq_kem.rs:296`), schoolbook **negacyclic** convolution in R_q = Z_q[x]/(x²⁵⁶+1), q=3329. Correctness signature: the wraparound at `idx = i+j ≥ N` **subtracts** (`pq_kem.rs:313`), which is the negacyclic sign flip x²⁵⁶ ≡ −1. This is the correct ML-KEM ring.
- Parameters confirmed correct for ML-KEM-768: n=256, q=3329, k=3, **η1=2**, η2=2, du=10, dv=4 (`pq_kem.rs:245–259`).
- The prior NTT was **deliberately ripped out** (`pq_kem.rs:329–335`): the shipped forward/inverse were not a valid pair and its basemul did not reproduce schoolbook products. The comment sets the exact re-introduction bar: any NTT must ship with a verifier proving `intt(ntt(a))==a` **and** `intt(basemul(ntt(a),ntt(b)))==schoolbook(a,b)`. This pass meets that bar and exceeds it (exhaustive, not sampled).
- **Interop caveat (pre-existing, not introduced here):** bebop's KEM stores `t`/`s` in the **coefficient domain**, not the NTT domain (`pq_kem.rs` keygen comments). It is therefore already **not wire-interoperable** with NIST ACVP ML-KEM vectors regardless of the multiply used. The correctness gate that is both *achievable* and *load-bearing* here is bit-identity to the schoolbook `poly_mul` — which this pass proves.

### 1.2 dowiz kernel `kernel/src/pq/kem.rs` — has an NTT, but it is the WRONG ring (do NOT use as reference)
The brief asked whether the dowiz kernel already solved this correctly, to port it if so. **It has not.** Two independent non-compliances, found by reading the code and its own test:
1. **Cyclic, not negacyclic.** Its NTT (`kem.rs:57`) is a *complete* length-256 transform with ROOT=17, and its own multiplication test (`kem.rs:429`) uses `idx = (i + j) % N` — i.e. it implements Z_q[x]/(x²⁵⁶**−1**) (cyclic), not the ML-KEM ring Z_q[x]/(x²⁵⁶**+1**) (negacyclic). There is no ψⁱ pre/post-weighting anywhere that would convert cyclic→negacyclic.
2. **η1=3** (`kem.rs:21`), which is the ML-KEM-**512** value; ML-KEM-768 requires η1=2.

The kernel KEM is *internally self-consistent* (encaps/decaps use the same ring throughout, so its own round-trip tests pass), but it is **not ML-KEM-768** and not FIPS-203. **Porting its NTT into bebop would have reintroduced exactly the class of subtle ring bug that got the prior NTT ripped out** — it fails the bit-identity gate by construction. This is reported honestly as a separate finding for the kernel lane; fixing it is out of scope here (separate red-line surface, needs its own review + sign-off).

### 1.3 Why the transform must be *incomplete* (the structural crux, proven numerically)
A complete length-256 negacyclic NTT would need a primitive **512th** root of unity mod 3329. It does not exist:

> q − 1 = 3328 = 2⁸ · 13. 512 = 2⁹ **does not divide** 3328. The 2-Sylow subgroup of Z*₃₃₂₉ has order 2⁸, so the largest power-of-two root order available is 256.

ζ = 17 has order **exactly 256** (17¹²⁸ ≡ −1, 17²⁵⁶ ≡ 1 — verified). Hence x²⁵⁶+1 splits only into **128 quadratic** factors (x² − ζ^{2·brv7(i)+1}), and the correct transform is a **7-layer** (not 8-layer) NTT leaving 128 degree-1 residues, multiplied pairwise by a quadratic `basemul`. This is precisely the FIPS-203 / Kyber structure — and precisely why ML-DSA (`pq_dsa.rs`, q=8380417, which *does* have a 512th root) can use a *complete* NTT while ML-KEM cannot. The two PQ schemes diverge here for a real mathematical reason, not by oversight.

---

## 2. The implementation (committed, non-wired)

Added to `bebop2/core/src/pq_kem.rs`, immediately after the schoolbook `poly_mul`:

| Symbol | Role |
|---|---|
| `const ZETAS_KEM: [i32;128]` | ζ table, `ZETAS_KEM[i] = 17^{brv7(i)} mod q`, built at **compile time** via `const fn` (no runtime init, no alloc). |
| `ntt_fwd_kem(&mut [i32;256])` | Forward incomplete NTT, Cooley-Tukey, 7 layers (len 128→2). |
| `ntt_inv_kem(&mut [i32;256])` | Inverse (Gentleman-Sande, same table backward) + final ×128⁻¹ (=3303 mod q). |
| `basemul_kem(a0,a1,b0,b1,ζ)` | (a0+a1x)(b0+b1x) mod (x²−ζ). |
| `poly_mul_ntt(a,b) -> [i32;256]` | Full ring multiply: `intt(basemul(ntt(a),ntt(b)))`. O(N log N). |

Design choices (all justified against the prior incident):
- **Plain modular arithmetic** via the crate's existing `red()` helper, not Montgomery. Montgomery/Barrett buy *speed*, not correctness; the KEM runs per-handshake (not per-frame, per the audit), so the correctness-transparent plain form is the right first cut. Montgomery can be layered later behind the same bit-identity gate if a bench justifies it. i64 intermediates bound trivially (max product 3328·3328 ≈ 1.1e7; basemul ≈ 3.7e10 ≪ i64::MAX).
- **Same ζ-table traversed backward for the inverse**, matching the FIPS-203/Kyber forward/inverse pairing — *validated*, not assumed, by the round-trip test.
- **Compile-time ζ table** keeps the crypto path allocation-free and no_std-compatible, consistent with the rest of the module.

---

## 3. Correctness proof (the primary gate — exceeded)

All in `#[cfg(test)] mod tests` of `pq_kem.rs`; `cargo test -p bebop2-core --lib ntt_kem` → **5 passed, 0 failed**. Full `pq_kem` suite: **11 passed** (6 pre-existing + 5 new), nothing regressed.

1. `ntt_kem_sanity` — 17¹²⁸ ≡ −1, 17²⁵⁶ ≡ 1, and 512 ∤ (q−1) (the incompleteness invariant).
2. `ntt_kem_roundtrip` — `intt(ntt(a)) == a`, 1000 random polys → the forward/inverse are a valid pair (the exact property the old NTT failed).
3. `ntt_kem_negacyclic_wrap` — x²⁵⁵·x == −1 (coeff[0]=q−1, rest 0): the negacyclic sign is correct end-to-end.
4. `ntt_kem_matches_schoolbook_random` — 300 random pairs, `poly_mul_ntt == poly_mul`.
5. **`ntt_kem_exhaustive_basis_proof` — the complete proof.** `poly_mul` and `poly_mul_ntt` are both **Z_q-bilinear** maps (NTT is Z_q-linear; basemul is Z_q-bilinear; their composition is bilinear). Two bilinear maps that agree on every basis pair agree everywhere. The test checks **all 256×256 = 65 536 monomial pairs** (xⁱ·xʲ) → **0 mismatches**. This upgrades the claim from "bit-identical on a KAT sample" to "bit-identical on the entire input space (Z_q²⁵⁶)²".

By referential transparency, since `poly_mul` is a pure function and `poly_mul_ntt` returns bit-identical output for **every** input, swapping the call sites in `keygen`/`encaps`/`decaps` would produce **byte-identical** ek/dk/ct/K. The KEM-level differential is therefore mathematically implied; it is left un-wired only to respect the sign-off gate, not for lack of proof.

**Honest scope of the proof:** it proves the NTT computes the *same ring product as the schoolbook multiply*. It does **not** claim FIPS-203 wire interop — bebop's KEM already forgoes that by storing coefficient-domain (§1.1). If wire interop with NIST ACVP is ever a goal, that is a separate, larger change (NTT-domain storage + real ACVP KEM vectors) — noted, not attempted here.

---

## 4. Broader O(n²) → NTT/FFT sweep (both repos) — clean beyond ML-KEM

Grep for schoolbook/convolution shapes (nested `a[i]*b[j]` into `c[i+j]`) across `bebop2/*` and `dowiz/kernel/src`, then read each hit. Honest result: **the only genuine NTT-beneficial polynomial-ring convolution is ML-KEM's `poly_mul`** (now addressed). The other hits are **not** NTT/FFT targets, and forcing NTT onto them would be wrong:

| Site | What it is | Why NTT/FFT does NOT apply |
|---|---|---|
| `x25519.rs:187` `mul_wide` | 8×8-limb (256-bit) field multiply, GF(2²⁵⁵−19) | n=8. FFT/NTT overhead ≫ 64 limb-mults. Also an explicitly **constant-time** multiply — NTT would add data-dependent structure to a secret-independent path. |
| `sign.rs:107` `limbs_mul` | 4×4-limb (256-bit) Ed25519 multiply | n=4. Schoolbook is optimal; CT-sensitive. |
| `sign.rs:649` `mul_be` | bytewise bignum multiply for Ed25519 mod-L scalar reduction | operands ≈32 bytes. FFT bignum (Schönhage-Strassen/NTT) only wins at *thousands* of digits; CT-adjacent. |
| `pq_dsa.rs:509,548` / kernel `dsa.rs:440,479` | `8*i+j` bit-packing (power2round/decompose) | not convolution. |
| `pq_dsa.rs` NTT | ML-DSA complete NTT | **already** verified O(N log N) (audit §2). Not a gap. |
| `fft.rs:87` | radix-2 Cooley-Tukey FFT | **already** O(N log N); `dft_oracle` is test-only. |

No additional targets are manufactured — per the brief, a clean sweep is reported as clean.

---

## 5. Handoff / next steps (all gated on sign-off — none auto-applied)

1. **Bench first (audit P3).** Add a criterion bench for `pq_kem` keygen/encaps/decaps to quantify real handshake cost. The KEM is per-handshake, so the ~100× multiply speedup only matters if handshake latency is shown to matter.
2. **Wire-in (only after review).** Replace the `poly_mul` call sites (`pq_kem.rs:~505,521,565,627`) with `poly_mul_ntt`. The exhaustive proof guarantees byte-identical KEM outputs; the existing `dual_impl_bit_exact` and `kem_soak_random_seeds` tests remain as the regression net. Keep the schoolbook `poly_mul` in-tree as the permanent ground-truth reference.
3. **Optional Montgomery layer.** Only if a bench justifies it, and only behind the same exhaustive basis-identity gate.
4. **Kernel lane (separate, red-line).** `dowiz/kernel/src/pq/kem.rs` implements the wrong ring (cyclic) and η1=3 while claiming ML-KEM-768/FIPS-203. Flagged for its own review; not fixed here.

---

## 6. Files touched

- **Code (committed, non-wired):** `bebop-repo:bebop2/core/src/pq_kem.rs` — added `ZETAS_KEM`, `brv7`, `modpow_c`, `ntt_fwd_kem`, `ntt_inv_kem`, `basemul_kem`, `poly_mul_ntt` + 5 tests. Schoolbook `poly_mul` and the live KEM path are **unchanged**.
- **Standalone verifier (scratchpad, for the record):** the same algorithm + exhaustive proof was first developed and run standalone before porting.
- **This doc:** `dowiz/docs/research/OPUS-PERF-NTT-IMPLEMENTATION-2026-07-18.md`.
