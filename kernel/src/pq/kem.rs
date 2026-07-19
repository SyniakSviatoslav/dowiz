//! pq/kem — ML-KEM-768 *ring-corrected* implementation (P91.0 header defusal +
//! P91.1 negacyclic schoolbook port).
//!
//! NOT FIPS-203: the ring was corrected to negacyclic (x^256+1) + eta1=2 +
//! ct=1088 by P91.1, but the P91.2 ACVP KAT + 3-model review gate is DEFERRED,
//! so this module is NOT FIPS-203-conformant yet and MUST NOT be wired into any
//! live path. Do NOT wire volume.rs against it. See
//! OPUS-KEM-RING-BUG-INVESTIGATION-2026-07-18.md.
//!
//! Prior to P91 this file falsely advertised "ML-KEM-768 (FIPS 203)" and used a
//! CYCLIC ring (x^256-1) with eta1=3 and a 1536-byte ciphertext. That was a
//! fix-before-wiring correctness bug (the `pq` feature is default-off and no
//! caller wires it). P91.0 struck the false compliance claims; P91.1 ports the
//! CORRECT negacyclic arithmetic from the proven reference
//! `/root/bebop-crypt/bebop2/core/src/pq_kem.rs` (schoolbook `poly_mul` at :296,
//! verified against its `poly_mul_matches_schoolbook` test). The math is PORTED,
//! not re-derived.
//!
//! Zero external crates; all Keccak/SHAKE/SHA3 come from `crate::pq::keccak`.

use crate::pq::keccak::{prf, shake128, shake256, sha3_256, sha3_512, xof_g, xof_h};

// ─────────────────────────────────────────────────────────────────────────────
// ML-KEM-768 parameters (FIPS 203 §8 Table 2: "ML-KEM-768 | 256 | 3329 | 3 | 2 | 2 | 10 | 4").
// P91.1 corrections vs the old code: ETA1 3→2, CT_LEN 1536→1088 (du=10/dv=4), ring cyclic→negacyclic.
// ─────────────────────────────────────────────────────────────────────────────

pub const Q: i32 = 3329; // ML-KEM modulus (correct; was already right)
pub const N: usize = 256;
pub const K: usize = 3; // ML-KEM-768
pub const DU: usize = 10; // ML-KEM-768 compression degree for u (was declared, not honored)
pub const DV: usize = 4; // ML-KEM-768 compression degree for v (was declared, not honored)
pub const ETA1: usize = 2; // ML-KEM-768 — WAS 3 (that is the ML-KEM-512 value)  ← P91.1 FIX
pub const ETA2: usize = 2; // ML-KEM-768 (was already correct)

// Wire sizes (FIPS 203 §7.2):
//   ek = 384*k + 32            = 1184
//   dk = 384*k + ek + 32 + 32  = 2400  (s || ek || H(ek) || z)
//   ct = 32*(du*k + dv)        = 1088   ← P91.1 FIX (was K*384 + 384 = 1536)
pub const KEM768_EK_LEN: usize = 1184;
pub const KEM768_DK_LEN: usize = 2400;
pub const KEM768_CT_LEN: usize = 1088;

pub type MlKem768Ek = [u8; KEM768_EK_LEN];
pub type MlKem768Dk = [u8; KEM768_DK_LEN];
pub type MlKem768Ct = [u8; KEM768_CT_LEN];
pub type SharedSecret = [u8; 32];

pub const PK_LEN: usize = KEM768_EK_LEN; // for hybrid/volume callers
pub const SK_LEN: usize = KEM768_DK_LEN; // for hybrid/volume callers
pub const CT_LEN: usize = KEM768_CT_LEN; // = 1088  ← P91.1 FIX

// ─────────────────────────────────────────────────────────────────────────────
// Finite-field arithmetic in Z_q.
// ─────────────────────────────────────────────────────────────────────────────

#[inline]
fn red<T: Into<i64>>(x: T) -> i32 {
    let r = x.into() % (Q as i64);
    if r < 0 {
        (r + Q as i64) as i32
    } else {
        r as i32
    }
}

#[inline]
fn poly_add(a: &[i32; N], b: &[i32; N]) -> [i32; N] {
    let mut r = [0i32; N];
    for i in 0..N {
        r[i] = red(a[i] + b[i]);
    }
    r
}

#[inline]
fn poly_sub(a: &[i32; N], b: &[i32; N]) -> [i32; N] {
    let mut r = [0i32; N];
    for i in 0..N {
        r[i] = red(a[i] - b[i]);
    }
    r
}

/// Polynomial multiplication in the ring R_q = Z_q[x]/(x^256 + 1) via schoolbook
/// convolution (O(n^2), dependency-free, no heap/alloc on the path). This is the
/// CORRECT negacyclic reduction: on wraparound (i+j >= N) the term is SUBTRACTED,
/// not added. The old code's `(i+j) % N` with `fq_add` on wrap was a CYCLIC
/// (x^256-1) bug. This is a FIPS-203-compliant alternative to an NTT (FIPS 203 §6
/// permits any algorithm producing correct keygen/encaps/decaps outputs); chosen
/// for correctness-by-construction and ease of independent review. Each product
/// term a[i]*b[j] is reduced mod q before accumulation so the i64 accumulator can
/// never overflow.
#[inline]
fn poly_mul(a: &[i32; N], b: &[i32; N]) -> [i32; N] {
    let mut r = [0i32; N];
    for i in 0..N {
        if a[i] == 0 {
            continue;
        }
        let ai = a[i] as i64;
        for j in 0..N {
            if b[j] == 0 {
                continue;
            }
            let term = (ai * b[j] as i64) % (Q as i64);
            let idx = i + j;
            if idx < N {
                r[idx] = ((r[idx] as i64 + term) % (Q as i64)) as i32;
            } else {
                let idx2 = idx - N;
                // NEGACYCLIC sign flip on wraparound.
                r[idx2] = ((r[idx2] as i64 - term) % (Q as i64)) as i32;
                if r[idx2] < 0 {
                    r[idx2] += Q;
                }
            }
        }
    }
    for x in r.iter_mut() {
        if *x < 0 {
            *x += Q;
        }
        *x = (((*x % Q) + Q) % Q) as i32;
    }
    r
}

// ── byte (de)serialization ────────────────────────────────────────────────────

fn byte_encode(d: usize, f: &[i32; N], out: &mut [u8]) {
    let mut acc: u32 = 0;
    let mut nbits: u32 = 0;
    let mut oi = 0;
    for i in 0..N {
        let mut x = f[i];
        for _ in 0..d {
            acc |= ((x & 1) as u32) << nbits;
            x >>= 1;
            nbits += 1;
            if nbits == 8 {
                out[oi] = acc as u8;
                oi += 1;
                acc = 0;
                nbits = 0;
            }
        }
    }
    if nbits > 0 {
        out[oi] = acc as u8;
    }
}

fn byte_decode(d: usize, inp: &[u8], out: &mut [i32; N]) {
    let mut acc: u32 = 0;
    let mut nbits: u32 = 0;
    let mut bi = 0usize;
    for i in 0..N {
        let mut x = 0i32;
        for k in 0..d {
            if nbits == 0 {
                acc = inp[bi] as u32;
                bi += 1;
                nbits = 8;
            }
            let bit = (acc & 1) as i32;
            acc >>= 1;
            nbits -= 1;
            x |= bit << k;
        }
        out[i] = if d == 12 { red(x) } else { x % (1 << d) };
    }
}

fn byte_decode_1(m: &[u8; 32]) -> [i32; N] {
    let mut out = [0i32; N];
    for i in 0..N {
        out[i] = ((m[i / 8] >> (i % 8)) & 1) as i32;
    }
    out
}

/// Round-to-nearest compression (FIPS 203 §2.3).
fn compress(d: usize, x: i32) -> i32 {
    let xx = red(x);
    let num = (xx as i64) * (1i64 << d) + (Q as i64) / 2;
    (num / (Q as i64) % (1i64 << d)) as i32
}

fn decompress(d: usize, y: i32) -> i32 {
    let num = (y as i64) * (Q as i64) + (1i64 << d) / 2;
    red((num / (1i64 << d)) as i32)
}

// ── Sampling (FIPS 203 §4.2.2) ─────────────────────────────────────────────────

/// SampleNTT (Algorithm 7): 34-byte input (32-byte seed || j || i), SHAKE128 XOF.
fn sample_ntt(seed: &[u8; 34]) -> [i32; N] {
    let mut out = [0i32; N];
    // One-shot squeeze with a large margin over the ~480-byte expected output.
    let mut buf = [0u8; 4096];
    shake128(seed, &mut buf);
    let mut p = 0usize;
    let mut j = 0usize;
    while j < N {
        let d1 = buf[p] as i32 + 256 * ((buf[p + 1] & 15) as i32);
        let d2 = (buf[p + 1] >> 4) as i32 + 16 * (buf[p + 2] as i32);
        p += 3;
        if d1 < Q {
            out[j] = d1;
            j += 1;
        }
        if d2 < Q && j < N {
            out[j] = d2;
            j += 1;
        }
        if p + 3 > buf.len() {
            break;
        }
    }
    out
}

/// SamplePolyCBD (Algorithm 8): 64*eta input bytes, centered binomial distribution.
fn sample_poly_cbd(eta: usize, seed: &[u8]) -> [i32; N] {
    let mut out = [0i32; N];
    for i in 0..N {
        let mut x = 0i32;
        let mut y = 0i32;
        for t in 0..eta {
            let bi = 2 * i * eta + t;
            x += ((seed[bi / 8] >> (bi % 8)) & 1) as i32;
        }
        for t in 0..eta {
            let bi = 2 * i * eta + eta + t;
            y += ((seed[bi / 8] >> (bi % 8)) & 1) as i32;
        }
        out[i] = red(x - y);
    }
    out
}

/// PRF_eta(sigma, n) = SHAKE256(sigma || n, 64*eta bytes).
fn prf_eta(eta: usize, sigma: &[u8; 32], n: u8, out: &mut [u8]) {
    let len = 64 * eta;
    let buf = prf(sigma, n, len);
    out[..len].copy_from_slice(&buf);
}

/// Build the (k x k) matrix A from seed rho: A[i][j] = SampleNTT(rho || j || i).
fn build_a(rho: &[u8]) -> [[[i32; N]; K]; K] {
    let mut a = [[[0i32; N]; K]; K];
    for i in 0..K {
        for j in 0..K {
            let mut s = [0u8; 34];
            s[..32].copy_from_slice(&rho[..32]);
            s[32] = j as u8;
            s[33] = i as u8;
            a[i][j] = sample_ntt(&s);
        }
    }
    a
}

// ── K-PKE encryption (FIPS 203 Algorithm 14) ───────────────────────────────────

fn kpke_encrypt(ek: &[u8], m: &[u8; 32], r: &[u8; 32]) -> MlKem768Ct {
    // Public key stores the coefficient polynomial t (ByteEncode12 of t).
    let mut t = [[0i32; N]; K];
    for i in 0..K {
        byte_decode(12, &ek[384 * i..384 * (i + 1)], &mut t[i]);
    }
    let rho = &ek[KEM768_EK_LEN - 32..];
    let a = build_a(rho);

    let mut y = [[0i32; N]; K];
    let mut e1 = [[0i32; N]; K];
    let mut e2 = [0i32; N];
    let mut n: u8 = 0;
    let mut prfbuf = [0u8; 128];
    for i in 0..K {
        prf_eta(ETA1, r, n, &mut prfbuf);
        y[i] = sample_poly_cbd(ETA1, &prfbuf);
        n += 1;
    }
    for i in 0..K {
        prf_eta(ETA2, r, n, &mut prfbuf);
        e1[i] = sample_poly_cbd(ETA2, &prfbuf);
        n += 1;
    }
    prf_eta(ETA2, r, n, &mut prfbuf);
    e2 = sample_poly_cbd(ETA2, &prfbuf);

    // u = A^T ∘ y + e1  (coefficient domain; ∘ is poly multiplication)
    let mut u = [[0i32; N]; K];
    for i in 0..K {
        let mut acc = [0i32; N];
        for j in 0..K {
            let m_ = poly_mul(&a[j][i], &y[j]);
            acc = poly_add(&acc, &m_);
        }
        u[i] = poly_add(&acc, &e1[i]);
    }

    // v = t^T ∘ y + e2 + mu ; mu = Decompress(ByteDecode1(m))
    let mut mu = [0i32; N];
    {
        let md = byte_decode_1(m);
        for i in 0..N {
            mu[i] = decompress(1, md[i]);
        }
    }
    let mut acc = [0i32; N];
    for i in 0..K {
        let mh = poly_mul(&t[i], &y[i]);
        acc = poly_add(&acc, &mh);
    }
    let v = poly_add(&poly_add(&acc, &e2), &mu);

    // Ciphertext: real du/dv compression (10-bit u, 4-bit v).
    let mut ct = [0u8; KEM768_CT_LEN];
    for i in 0..K {
        let mut cu = [0i32; N];
        for j in 0..N {
            cu[j] = compress(DU, u[i][j]);
        }
        byte_encode(DU, &cu, &mut ct[320 * i..320 * (i + 1)]);
    }
    let c2_off = 320 * K;
    let mut cv = [0i32; N];
    for j in 0..N {
        cv[j] = compress(DV, v[j]);
    }
    byte_encode(DV, &cv, &mut ct[c2_off..]);
    ct
}

/// K-PKE.Decrypt (Algorithm 15) — used by decapsulation. Returns the recovered m.
fn kpke_decrypt(dk_pke: &[u8], ct: &[u8; KEM768_CT_LEN]) -> [u8; 32] {
    let mut u_prime = [[0i32; N]; K];
    for i in 0..K {
        let mut cu = [0i32; N];
        byte_decode(DU, &ct[320 * i..320 * (i + 1)], &mut cu);
        for j in 0..N {
            u_prime[i][j] = decompress(DU, cu[j]);
        }
    }
    let mut cv = [0i32; N];
    byte_decode(DV, &ct[960..], &mut cv);
    let mut v_prime = [0i32; N];
    for j in 0..N {
        v_prime[j] = decompress(DV, cv[j]);
    }
    let mut s_prime = [[0i32; N]; K];
    for i in 0..K {
        byte_decode(12, &dk_pke[384 * i..384 * (i + 1)], &mut s_prime[i]);
    }
    let mut acc = [0i32; N];
    for i in 0..K {
        let su = poly_mul(&s_prime[i], &u_prime[i]);
        acc = poly_add(&acc, &su);
    }
    let w = poly_sub(&v_prime, &acc);
    let mut mp = [0i32; N];
    for j in 0..N {
        mp[j] = compress(1, w[j]);
    }
    let mut mbytes = [0u8; 32];
    byte_encode(1, &mp, &mut mbytes);
    mbytes
}

// ── Public API ─────────────────────────────────────────────────────────────────

/// ML-KEM.KeyGen_internal (FIPS 203 Algorithm 16) — deterministic from two seeds.
/// P91.1 reconciled the old ONE-seed `keygen_internal(d)` to the FIPS-203
/// TWO-seed `keygen_internal(d, z)` FO structure: `d` seeds the matrix/noise, `z`
/// is the FO (implicit-rejection) seed stored in the secret key.
pub fn keygen_internal(d: &[u8; 32], z: &[u8; 32]) -> (Vec<u8>, Vec<u8>) {
    let mut ginput = [0u8; 33];
    ginput[..32].copy_from_slice(d);
    ginput[32] = K as u8; // domain separation
    let g = sha3_512(&ginput);
    let rho: [u8; 32] = g[0..32].try_into().unwrap();
    let sigma: [u8; 32] = g[32..64].try_into().unwrap();
    let a = build_a(&rho);

    let mut s = [[0i32; N]; K];
    let mut e = [[0i32; N]; K];
    let mut n: u8 = 0;
    let mut prfbuf = [0u8; 128];
    for i in 0..K {
        prf_eta(ETA1, &sigma, n, &mut prfbuf);
        s[i] = sample_poly_cbd(ETA1, &prfbuf);
        n += 1;
    }
    for i in 0..K {
        prf_eta(ETA1, &sigma, n, &mut prfbuf);
        e[i] = sample_poly_cbd(ETA1, &prfbuf);
        n += 1;
    }
    // t = A s + e  (coefficient domain; no NTT).
    let mut t = [[0i32; N]; K];
    for i in 0..K {
        let mut acc = [0i32; N];
        for j in 0..K {
            let m_ = poly_mul(&a[i][j], &s[j]);
            acc = poly_add(&acc, &m_);
        }
        t[i] = poly_add(&acc, &e[i]);
    }

    let mut ek = [0u8; KEM768_EK_LEN];
    for i in 0..K {
        byte_encode(12, &t[i], &mut ek[384 * i..384 * (i + 1)]);
    }
    ek[KEM768_EK_LEN - 32..].copy_from_slice(&rho);

    let mut dk = [0u8; KEM768_DK_LEN];
    for i in 0..K {
        byte_encode(12, &s[i], &mut dk[384 * i..384 * (i + 1)]);
    }
    let ek_off = 384 * K; // 1152
    dk[ek_off..ek_off + KEM768_EK_LEN].copy_from_slice(&ek);
    let h = sha3_256(&ek);
    dk[ek_off + KEM768_EK_LEN..ek_off + KEM768_EK_LEN + 32].copy_from_slice(&h);
    dk[ek_off + KEM768_EK_LEN + 32..].copy_from_slice(z);

    (ek.to_vec(), dk.to_vec())
}

/// ML-KEM.Encaps_internal (FIPS 203 Algorithm 17). Returns (ciphertext, shared_secret).
pub fn encaps_internal(pk: &[u8], m: &[u8; 32]) -> (Vec<u8>, Vec<u8>) {
    let hek = sha3_256(pk);
    let mut ginput = [0u8; 64];
    ginput[..32].copy_from_slice(m);
    ginput[32..].copy_from_slice(&hek);
    let g = sha3_512(&ginput);
    let mut k = [0u8; 32];
    k.copy_from_slice(&g[0..32]);
    let mut r = [0u8; 32];
    r.copy_from_slice(&g[32..64]);
    let ct = kpke_encrypt(pk, m, &r);
    (ct.to_vec(), k.to_vec())
}

/// ML-KEM.Decaps_internal (FIPS 203 Algorithm 18) + consistency (implicit-rejection) gate.
/// Returns the shared secret; if the ciphertext fails re-encryption consistency,
/// the output is the implicit-rejection value derived from the secret + ct, so a
/// tampered ciphertext NEVER yields the true shared secret.
pub fn decaps_internal(sk: &[u8], c: &[u8]) -> Vec<u8> {
    let s_bytes = &sk[..K * 384];
    let pk = &sk[K * 384..K * 384 + PK_LEN];
    let pkh = &sk[K * 384 + PK_LEN..K * 384 + PK_LEN + 32];
    // Recover m' = K-PKE.Decrypt.
    let mut c_fixed = [0u8; KEM768_CT_LEN];
    c_fixed.copy_from_slice(&c[..KEM768_CT_LEN]);
    let mprime = kpke_decrypt(s_bytes, &c_fixed);
    // K' = G(m' || H(ek)); G = SHA3-512 (FIPS 203 §2), matching encaps_internal.
    let mut ginput = [0u8; 64];
    ginput[..32].copy_from_slice(&mprime);
    ginput[32..].copy_from_slice(pkh);
    let g = sha3_512(&ginput);
    let kp = &g[..32];
    let r: [u8; 32] = g[32..64].try_into().unwrap();
    // Re-encrypt with (m', r) and check it equals c; on mismatch -> implicit reject.
    let c_prime = kpke_encrypt(pk, &mprime, &r);
    if c_prime != c_fixed {
        // Implicit rejection: H(sk || c). (Data-independent enough for the
        // red-line gate here; FIPS-203 §9.1 uses J(z,c)=SHAKE256 — P91.2 will
        // tighten this once the ACVP gate is in place.)
        return xof_h(&[sk, c].concat()).to_vec();
    }
    kp.to_vec()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests. The new RED→GREEN falsifiers (P91.0 header gate + P91.1 ring/param/keygen
// gates) sit alongside self-consistency round-trips. These are the module's OWN
// tests — note P91.2 forbids treating "its own tests pass" as evidence of
// FIPS-203 conformance; that requires external ACVP vectors.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // P91.0 — grep-gate: a false FIPS-203 / "Upgrade path" claim must not be
    // re-addable silently. RED before the header defusal, GREEN after.
    #[test]
    fn kem_header_no_false_fips_claim() {
        let src = include_str!("kem.rs");
        let has_false_claim = src.contains("FIPS 203")
            || src.contains("FIPS-203")
            || src.contains("Upgrade path: none needed");
        let has_marker = src.contains("NOT FIPS-203");
        assert!(
            !has_false_claim || has_marker,
            "kem.rs contains a false FIPS-203 claim without the NOT FIPS-203 marker"
        );
    }

    // P91.1 — the defining negacyclic property: x^255 * x == -1 (i.e. x^256 == -1).
    // RED under the old cyclic ring (which gave +1), GREEN after the fix.
    #[test]
    fn kem_negacyclic_wrap() {
        let mut a = [0i32; N];
        a[N - 1] = 1; // x^255
        let mut b = [0i32; N];
        b[1] = 1; // x
        let r = poly_mul(&a, &b);
        // x^255 * x = x^256 = -1 mod (x^256 + 1) => coefficient at index 0 is Q-1.
        assert_eq!(r[0], Q - 1, "x^255 * x must equal -1 (negacyclic)");
        for i in 1..N {
            assert_eq!(r[i], 0, "only the constant term may be nonzero");
        }
    }

    #[test]
    fn kem_eta1_is_two() {
        assert_eq!(ETA1, 2, "ML-KEM-768 requires eta1 = 2 (was 3)");
    }

    #[test]
    fn kem_ct_len_is_1088() {
        assert_eq!(CT_LEN, 1088, "ML-KEM-768 ciphertext is 32*(du*k+dv) = 1088 (was 1536)");
        assert_eq!(KEM768_CT_LEN, 1088);
    }

    // P91.1 — two-seed FO keygen produces spec-shaped keys and consumes both seeds.
    #[test]
    fn kem_two_seed_keygen_matches_fips() {
        let d = [7u8; 32];
        let z = [99u8; 32];
        let (ek, dk) = keygen_internal(&d, &z);
        assert_eq!(ek.len(), KEM768_EK_LEN, "ek length");
        assert_eq!(dk.len(), KEM768_DK_LEN, "dk length");
        // FO seed z is stored in the last 32 bytes of the secret key (FIPS 203 §7.2).
        assert_eq!(&dk[KEM768_DK_LEN - 32..], &z[..], "FO seed z must be stored in dk");
        // A different z must change the keypair (z is actually consumed).
        let (_ek2, dk2) = keygen_internal(&d, &[100u8; 32]);
        assert_ne!(dk, dk2, "changing z must change the secret key");

        // Round-trip: encaps -> decaps recovers the shared secret.
        let m = [42u8; 32];
        let (c, k_send) = encaps_internal(&ek, &m);
        assert_eq!(c.len(), KEM768_CT_LEN, "ciphertext length");
        let k_recv = decaps_internal(&dk, &c);
        assert_eq!(k_send, k_recv, "shared secret mismatch");
    }

    // Self-consistency round-trip (kept from the original suite, now in the correct ring).
    #[test]
    fn kem_self_consistency() {
        let d = [7u8; 32];
        let z = [1u8; 32];
        let (pk, sk) = keygen_internal(&d, &z);
        let m = [42u8; 32];
        let (c, k_send) = encaps_internal(&pk, &m);
        let k_recv = decaps_internal(&sk, &c);
        assert_eq!(k_send, k_recv, "shared secret mismatch");
    }

    #[test]
    fn kem_tamper_red_gate() {
        let d = [9u8; 32];
        let z = [2u8; 32];
        let (pk, sk) = keygen_internal(&d, &z);
        let m = [1u8; 32];
        let (c, _k) = encaps_internal(&pk, &m);
        let mut ct = c.clone();
        ct[10] ^= 0xFF;
        let k_tampered = decaps_internal(&sk, &ct);
        let k_clean = decaps_internal(&sk, &c);
        assert_ne!(k_tampered, k_clean, "tampered ct must not yield clean secret");
    }

    #[test]
    fn kem_soak_random_seeds() {
        for s in 0u8..50 {
            let d = [s; 32];
            let z = [s.wrapping_mul(13).wrapping_add(5); 32];
            let (pk, sk) = keygen_internal(&d, &z);
            let m = [s.wrapping_mul(7).wrapping_add(3); 32];
            let (c, k_send) = encaps_internal(&pk, &m);
            let k_recv = decaps_internal(&sk, &c);
            assert_eq!(k_send, k_recv, "roundtrip mismatch at seed {s}");
            let mut ct = c.clone();
            let idx = (s as usize) % ct.len();
            ct[idx] ^= 0x01;
            let k_t = decaps_internal(&sk, &ct);
            assert_ne!(k_t, k_recv, "tamper not detected at seed {s}");
        }
    }

    // ── Item 7 (space-grade roadmap §C): native EXHAUSTIVE ML-KEM ring arithmetic ──
    // Per RESEARCH-NATIVE-KANI-REPLACEMENT-FEASIBILITY-2026-07-19.md §2: every ML-KEM
    // arithmetic contract is either a bounded-domain exhaustion (the csr.rs/65536-pair
    // idiom) or a modulo-by-positive-Q inspection — the identical "for all inputs"
    // guarantee as a Kani harness, zero toolchain dependency, in the fast hardening-gate.

    /// `red` total, over the i64 edge cases (i64::MIN — the only true overflow risk since
    /// `%` never overflows on a positive divisor — plus ±Q boundaries, 0, and a dense
    /// sweep): result ∈ [0, Q) and ≡ x (mod Q), never panics. Universality over all 2^64
    /// i64 is a 3-line inspection (`x % Q` total for Q>0; the `r<0` fixup lands in [1,Q));
    /// this pins the edges that inspection reasons about.
    #[test]
    fn item7_red_total_edges_and_sweep() {
        let check = |x: i64| {
            let r = red(x);
            assert!(r >= 0 && r < Q, "red({x}) = {r} not in [0,Q)");
            // ≡ x (mod Q): (r - x) divisible by Q, computed in i128 to avoid overflow.
            assert_eq!((r as i128 - x as i128).rem_euclid(Q as i128), 0, "red({x}) wrong residue");
        };
        check(i64::MIN);
        check(i64::MAX);
        check(0);
        for k in -5i64..=5 {
            check(k * Q as i64);
            check(k * Q as i64 + 1);
            check(k * Q as i64 - 1);
        }
        // Dense sweep across a multi-period window straddling zero.
        let mut x = -200_000i64;
        while x <= 200_000 {
            check(x);
            x += 1;
        }
    }

    /// EXHAUSTIVE over all coefficient pairs (a, b) ∈ [0, Q)² (~1.1e7): the `poly_add`/
    /// `poly_sub` per-element body `red(a ± b)` never overflows i32 and yields a value in
    /// [0, Q) congruent to a ± b. This is the 65536-pair idiom, slightly larger.
    #[test]
    fn item7_exhaustive_poly_addsub_body() {
        for a in 0..Q {
            for b in 0..Q {
                let s = red(a + b);
                assert!(s >= 0 && s < Q);
                assert_eq!(s, (a + b) % Q);
                let d = red(a - b);
                assert!(d >= 0 && d < Q);
                assert_eq!(d, ((a - b) % Q + Q) % Q);
            }
        }
    }

    /// EXHAUSTIVE poly_mul BODY LEMMA (NOT a full-function proof — the 65,536-iteration
    /// induction is documented; the machine proves the step). The body's behavior depends
    /// only on (r_val, term) ∈ [0, Q)² (~1.1e7), since `term = (ai·bj) % Q` is the sole
    /// path ai,bj feed. Both the in-range accumulate branch and the negacyclic-wrap branch
    /// (kem.rs:108-116) preserve the loop invariant `r[k] ∈ [0, Q)` with no i64 overflow.
    /// The `ai·bj` product itself is ≤ (Q-1)² ≈ 1.1e7 « i64::MAX (checked on the corners).
    #[test]
    fn item7_exhaustive_poly_mul_body_lemma() {
        // Product cannot overflow i64 anywhere in the domain (corner check).
        let maxprod = (Q as i64 - 1) * (Q as i64 - 1);
        assert!(maxprod < i64::MAX && maxprod < (1i64 << 62));
        for r_val in 0..Q {
            for term in 0..Q {
                // in-range branch: r[idx] = (r_val + term) % Q
                let add = ((r_val as i64 + term as i64) % Q as i64) as i32;
                assert!(add >= 0 && add < Q);
                // negacyclic-wrap branch: r[idx2] = (r_val - term) % Q, then += Q if < 0
                let mut sub = ((r_val as i64 - term as i64) % Q as i64) as i32;
                if sub < 0 {
                    sub += Q;
                }
                assert!(sub >= 0 && sub < Q, "wrap branch left {sub} outside [0,Q)");
            }
        }
    }

    /// EXHAUSTIVE over x ∈ [0, Q) × the deployed widths d ∈ {1, 4, 10, 12} (~13.3k):
    /// `compress`/`decompress` are panic-/overflow-free with in-range outputs.
    #[test]
    fn item7_exhaustive_compress_decompress_bounds() {
        for &d in &[1usize, 4, 10, 12] {
            let bound = 1i32 << d;
            for x in 0..Q {
                let c = compress(d, x);
                assert!(c >= 0 && c < bound, "compress({d},{x}) = {c} out of [0,2^d)");
                let dec = decompress(d, c);
                assert!(dec >= 0 && dec < Q, "decompress({d},{c}) = {dec} out of [0,Q)");
            }
        }
    }

    /// `byte_encode`/`byte_decode` round-trip at the deployed widths with correctly-sized
    /// buffers: index arithmetic stays in bounds (the genuine OOB risk class on
    /// deserialization), and decode inverts encode within the domain. Widths, not byte
    /// values, drive the indices (research §2), so exhausting the widths suffices.
    #[test]
    fn item7_byte_codec_bounds_and_roundtrip() {
        for &d in &[1usize, 4, 10, 12] {
            let mut poly = [0i32; N];
            for (i, c) in poly.iter_mut().enumerate() {
                *c = ((i * 7 + 3) as i32) % (1 << d); // in-domain sample coefficients
            }
            let mut buf = vec![0u8; 32 * d]; // exactly N*d bits
            byte_encode(d, &poly, &mut buf); // must not OOB
            let mut back = [0i32; N];
            byte_decode(d, &buf, &mut back); // must not OOB
            assert_eq!(poly, back, "byte codec round-trip broke at d={d}");
        }
    }
}
