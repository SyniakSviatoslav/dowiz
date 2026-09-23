//! pq/kem — ML-KEM-768 (FIPS 203), byte-exact against the NIST ACVP vectors.
//!
//! VERIFIED (P91.2, 2026-09-23): every ML-KEM-768 vector of NIST ACVP-Server
//! `ML-KEM-keyGen-FIPS203` and `ML-KEM-encapDecap-FIPS203` internalProjection.json
//! (master 975de31e, fetched 2026-09-23; vendored as `kat/acvp/mlkem768-*.json`,
//! provenance in `kat/acvp/README-mlkem768.md`) passes in `kem/acvp_tests.rs`:
//! 25 keyGen (ek AND dk), 25 encapsulation (c AND k, plus decaps of the same c),
//! 10 decapsulation incl. 5 implicit-rejection cases (K̄ = J(z‖c)), 10
//! decapsulation-key checks (§7.3) and 10 encapsulation-key checks (§7.2) —
//! 80 vectors, each group asserting its count.
//!
//! The arithmetic is FIPS 203 Algorithms 5–18 as written: t̂, ŝ and Â are in the
//! NTT domain (`kem/ntt.rs`, Algorithms 9–12). Before P91.2 this file multiplied
//! schoolbook in the coefficient domain, so its keys could never match the
//! standard's bytes; that multiply survives only as the test oracle in
//! `kem/tests.rs` that cross-checks the NTT.
//!
//! NOT VERIFIED / NOT DONE:
//!   * No constant-time or side-channel review. `decaps_internal` selects K′/K̄
//!     with a mask, but `%`, the rejection sampler and the codec are unaudited,
//!     and nothing here has been measured for timing.
//!   * Only the `_internal` (seeded) algorithms exist; randomness is the
//!     caller's. `decaps_internal` REFUSES (`KemError`) a dk or c of the wrong
//!     length and a dk failing the §7.3 hash check; `ek_check` (§7.2) is run by
//!     `hybrid::hybrid_encaps_to`, the path the backup seal uses.
//!   * ML-KEM-512/1024 are not implemented.

use alloc::vec::Vec;

use crate::pq::keccak::{prf, sha3_256, sha3_512, shake128, xof_j};

mod ntt;
use ntt::{multiply_ntts, ntt, ntt_inv, poly_add, poly_sub};

// ─────────────────────────────────────────────────────────────────────────────
// ML-KEM-768 parameters (FIPS 203 §8 Table 2: "ML-KEM-768 | 256 | 3329 | 3 | 2 | 2 | 10 | 4").
// ─────────────────────────────────────────────────────────────────────────────

pub const Q: i32 = 3329;
pub const N: usize = 256;
pub const K: usize = 3;
pub const DU: usize = 10;
pub const DV: usize = 4;
pub const ETA1: usize = 2;
pub const ETA2: usize = 2;

// Wire sizes (FIPS 203 §7.2):
//   ek = 384*k + 32            = 1184
//   dk = 384*k + ek + 32 + 32  = 2400  (dk_pke || ek || H(ek) || z)
//   ct = 32*(du*k + dv)        = 1088
pub const KEM768_EK_LEN: usize = 1184;
pub const KEM768_DK_LEN: usize = 2400;
pub const KEM768_CT_LEN: usize = 1088;

pub type MlKem768Ek = [u8; KEM768_EK_LEN];
pub type MlKem768Dk = [u8; KEM768_DK_LEN];
pub type MlKem768Ct = [u8; KEM768_CT_LEN];
pub type SharedSecret = [u8; 32];

pub const PK_LEN: usize = KEM768_EK_LEN; // for hybrid/volume callers
pub const SK_LEN: usize = KEM768_DK_LEN; // for hybrid/volume callers
pub const CT_LEN: usize = KEM768_CT_LEN;

type Poly = [i32; N];
type PolyVec = [Poly; K];

/// x mod q, in [0, q).
#[inline]
fn red<T: Into<i64>>(x: T) -> i32 {
    let r = x.into() % (Q as i64);
    if r < 0 {
        (r + Q as i64) as i32
    } else {
        r as i32
    }
}

// ── ByteEncode_d / ByteDecode_d (Algorithms 5 and 6), little-endian bit order ──

fn byte_encode(d: usize, f: &Poly, out: &mut [u8]) {
    let mut acc: u32 = 0;
    let mut nbits: u32 = 0;
    let mut oi = 0;
    for &c in f.iter() {
        let mut x = c;
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
}

/// ByteDecode_d. For d = 12 the value is reduced mod q (Algorithm 6, m = q);
/// for d < 12 it is taken mod 2^d.
fn byte_decode(d: usize, inp: &[u8], out: &mut Poly) {
    let mut acc: u32 = 0;
    let mut nbits: u32 = 0;
    let mut bi = 0usize;
    for o in out.iter_mut() {
        let mut x = 0i32;
        for k in 0..d {
            if nbits == 0 {
                acc = inp[bi] as u32;
                bi += 1;
                nbits = 8;
            }
            x |= ((acc & 1) as i32) << k;
            acc >>= 1;
            nbits -= 1;
        }
        *o = if d == 12 { red(x) } else { x };
    }
}

/// Compress_d(x) = ⌈(2^d/q)·x⌋ mod 2^d, rounding half up (FIPS 203 §4.2.1).
fn compress(d: usize, x: i32) -> i32 {
    let num = (red(x) as i64) * (1i64 << d) + (Q as i64) / 2;
    (num / (Q as i64) % (1i64 << d)) as i32
}

/// Decompress_d(y) = ⌈(q/2^d)·y⌋ (FIPS 203 §4.2.1).
fn decompress(d: usize, y: i32) -> i32 {
    let num = (y as i64) * (Q as i64) + (1i64 << d) / 2;
    red((num >> d) as i32)
}

// ── Sampling (FIPS 203 §4.2.2) ─────────────────────────────────────────────────

/// SampleNTT (Algorithm 7): rejection-sample an NTT-domain polynomial from
/// SHAKE128(ρ || j || i). 4096 squeezed bytes = 2730 12-bit candidates; running out
/// needs > 2474 of them rejected (p ≈ 0.19 each); the assert refuses it loudly.
fn sample_ntt(seed: &[u8; 34]) -> Poly {
    let mut out = [0i32; N];
    let mut buf = [0u8; 4096];
    shake128(seed, &mut buf);
    let mut p = 0usize;
    let mut j = 0usize;
    while j < N {
        assert!(p + 3 <= buf.len(), "SampleNTT: XOF buffer exhausted");
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
    }
    out
}

/// SamplePolyCBD_eta (Algorithm 8) over PRF_eta(s, n) = SHAKE256(s || n, 64·eta).
fn cbd_prf(eta: usize, s: &[u8; 32], n: u8) -> Poly {
    let seed = prf(s, n, 64 * eta);
    let bit = |bi: usize| ((seed[bi / 8] >> (bi % 8)) & 1) as i32;
    let mut out = [0i32; N];
    for (i, o) in out.iter_mut().enumerate() {
        let (mut x, mut y) = (0i32, 0i32);
        for t in 0..eta {
            x += bit(2 * i * eta + t);
            y += bit(2 * i * eta + eta + t);
        }
        *o = red(x - y);
    }
    out
}

/// Â[i][j] = SampleNTT(ρ || j || i) — note the index order (Algorithm 13 line 6).
fn build_a(rho: &[u8]) -> [PolyVec; K] {
    let mut a = [[[0i32; N]; K]; K];
    for (i, row) in a.iter_mut().enumerate() {
        for (j, aij) in row.iter_mut().enumerate() {
            let mut s = [0u8; 34];
            s[..32].copy_from_slice(&rho[..32]);
            s[32] = j as u8;
            s[33] = i as u8;
            *aij = sample_ntt(&s);
        }
    }
    a
}

/// Σ_j f[j] ∘ g[j] in the NTT domain.
fn dot_ntt(f: &PolyVec, g: &PolyVec) -> Poly {
    let mut acc = [0i32; N];
    for j in 0..K {
        acc = poly_add(&acc, &multiply_ntts(&f[j], &g[j]));
    }
    acc
}

// ── K-PKE (Algorithms 13–15) ───────────────────────────────────────────────────

/// K-PKE.KeyGen (Algorithm 13): returns (ek_pke, dk_pke) = (ByteEncode12(t̂)‖ρ, ByteEncode12(ŝ)).
fn kpke_keygen(d: &[u8; 32]) -> (MlKem768Ek, [u8; 384 * K]) {
    let mut ginput = [0u8; 33];
    ginput[..32].copy_from_slice(d);
    ginput[32] = K as u8; // FIPS 203 final: G(d || k)
    let g = sha3_512(&ginput);
    let rho = &g[0..32];
    let sigma: [u8; 32] = g[32..64].try_into().unwrap();
    let a = build_a(rho);
    let mut s_hat = [[0i32; N]; K];
    let mut e_hat = [[0i32; N]; K];
    for i in 0..K {
        s_hat[i] = ntt(&cbd_prf(ETA1, &sigma, i as u8));
    }
    for i in 0..K {
        e_hat[i] = ntt(&cbd_prf(ETA1, &sigma, (K + i) as u8));
    }
    let mut ek = [0u8; KEM768_EK_LEN];
    let mut dk = [0u8; 384 * K];
    for i in 0..K {
        let t_hat = poly_add(&dot_ntt(&a[i], &s_hat), &e_hat[i]);
        byte_encode(12, &t_hat, &mut ek[384 * i..384 * (i + 1)]);
        byte_encode(12, &s_hat[i], &mut dk[384 * i..384 * (i + 1)]);
    }
    ek[384 * K..].copy_from_slice(rho);
    (ek, dk)
}

/// K-PKE.Encrypt (Algorithm 14).
fn kpke_encrypt(ek: &[u8], m: &[u8; 32], r: &[u8; 32]) -> MlKem768Ct {
    let mut t_hat = [[0i32; N]; K];
    for i in 0..K {
        byte_decode(12, &ek[384 * i..384 * (i + 1)], &mut t_hat[i]);
    }
    let a = build_a(&ek[384 * K..384 * K + 32]);
    let mut y_hat = [[0i32; N]; K];
    for i in 0..K {
        y_hat[i] = ntt(&cbd_prf(ETA1, r, i as u8));
    }
    let mut ct = [0u8; KEM768_CT_LEN];
    for i in 0..K {
        // (Âᵀ ∘ ŷ)[i] = Σ_j Â[j][i] ∘ ŷ[j]
        let col: PolyVec = core::array::from_fn(|j| a[j][i]);
        let e1 = cbd_prf(ETA2, r, (K + i) as u8);
        let u = poly_add(&ntt_inv(&dot_ntt(&col, &y_hat)), &e1);
        let cu: Poly = core::array::from_fn(|c| compress(DU, u[c]));
        byte_encode(DU, &cu, &mut ct[32 * DU * i..32 * DU * (i + 1)]);
    }
    let e2 = cbd_prf(ETA2, r, (2 * K) as u8);
    let mu: Poly = core::array::from_fn(|i| decompress(1, ((m[i / 8] >> (i % 8)) & 1) as i32));
    let v = poly_add(&poly_add(&ntt_inv(&dot_ntt(&t_hat, &y_hat)), &e2), &mu);
    let cv: Poly = core::array::from_fn(|c| compress(DV, v[c]));
    byte_encode(DV, &cv, &mut ct[32 * DU * K..]);
    ct
}

/// K-PKE.Decrypt (Algorithm 15): m = ByteEncode1(Compress1(v′ − NTT⁻¹(ŝᵀ ∘ NTT(u′)))).
fn kpke_decrypt(dk_pke: &[u8], ct: &[u8]) -> [u8; 32] {
    let mut s_hat = [[0i32; N]; K];
    let mut u_hat = [[0i32; N]; K];
    for i in 0..K {
        byte_decode(12, &dk_pke[384 * i..384 * (i + 1)], &mut s_hat[i]);
        let mut cu = [0i32; N];
        byte_decode(DU, &ct[32 * DU * i..32 * DU * (i + 1)], &mut cu);
        u_hat[i] = ntt(&core::array::from_fn(|c| decompress(DU, cu[c])));
    }
    let mut cv = [0i32; N];
    byte_decode(DV, &ct[32 * DU * K..], &mut cv);
    let v: Poly = core::array::from_fn(|c| decompress(DV, cv[c]));
    let w = poly_sub(&v, &ntt_inv(&dot_ntt(&s_hat, &u_hat)));
    let mp: Poly = core::array::from_fn(|c| compress(1, w[c]));
    let mut m = [0u8; 32];
    byte_encode(1, &mp, &mut m);
    m
}

// ── ML-KEM internal algorithms (Algorithms 16–18) ──────────────────────────────

/// ML-KEM.KeyGen_internal (Algorithm 16): dk = dk_pke ‖ ek ‖ H(ek) ‖ z.
pub fn keygen_internal(d: &[u8; 32], z: &[u8; 32]) -> (Vec<u8>, Vec<u8>) {
    let (ek, dk_pke) = kpke_keygen(d);
    let mut dk = Vec::with_capacity(KEM768_DK_LEN);
    dk.extend_from_slice(&dk_pke);
    dk.extend_from_slice(&ek);
    dk.extend_from_slice(&sha3_256(&ek));
    dk.extend_from_slice(z);
    (ek.to_vec(), dk)
}

/// ML-KEM.Encaps_internal (Algorithm 17): (K, r) = G(m ‖ H(ek)); c = Encrypt(ek, m, r).
/// Returns (ciphertext, shared_secret).
pub fn encaps_internal(ek: &[u8], m: &[u8; 32]) -> (Vec<u8>, Vec<u8>) {
    let mut ginput = [0u8; 64];
    ginput[..32].copy_from_slice(m);
    ginput[32..].copy_from_slice(&sha3_256(ek));
    let g = sha3_512(&ginput);
    let r: [u8; 32] = g[32..64].try_into().unwrap();
    let ct = kpke_encrypt(ek, m, &r);
    (ct.to_vec(), g[..32].to_vec())
}

/// ML-KEM.Decaps_internal (Algorithm 18), with implicit rejection: if the
/// re-encryption of the decrypted m′ does not reproduce c, the result is
/// K̄ = J(z ‖ c) = SHAKE256(z ‖ c, 32), never an error and never K′.
///
/// Malformed INPUT is different from a forged ciphertext and is refused: a dk
/// or c of the wrong length, or a dk failing the §7.3 hash check, is `Err`,
/// never a panic — a sealed file read from disk is untrusted input.
pub fn decaps_internal(dk: &[u8], c: &[u8]) -> Result<Vec<u8>, KemError> {
    if dk.len() != KEM768_DK_LEN {
        return Err(KemError::DkLength(dk.len()));
    }
    if c.len() != KEM768_CT_LEN {
        return Err(KemError::CtLength(c.len()));
    }
    if !dk_check(dk) {
        return Err(KemError::DkHash);
    }
    let dk_pke = &dk[..384 * K];
    let ek = &dk[384 * K..768 * K + 32];
    let h = &dk[768 * K + 32..768 * K + 64];
    let z = &dk[768 * K + 64..768 * K + 96];
    let m_prime = kpke_decrypt(dk_pke, c);
    let mut ginput = [0u8; 64];
    ginput[..32].copy_from_slice(&m_prime);
    ginput[32..].copy_from_slice(h);
    let g = sha3_512(&ginput);
    let r_prime: [u8; 32] = g[32..64].try_into().unwrap();
    let k_bar = xof_j(z, c);
    let c_prime = kpke_encrypt(ek, &m_prime, &r_prime);
    // Constant-time select of K′ vs K̄ on the full-ciphertext comparison.
    let mut diff = 0u8;
    for (a, b) in c_prime.iter().zip(c.iter()) {
        diff |= a ^ b;
    }
    let mask = ((diff as u16).wrapping_sub(1) >> 8) as u8; // 0xff iff equal
    Ok((0..32).map(|i| (g[i] & mask) | (k_bar[i] & !mask)).collect())
}

/// Why a decapsulation input was refused (never raised for a forged-but-
/// well-formed ciphertext: that one gets implicit rejection instead).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KemError {
    /// dk is not 2400 bytes (the value is the length given).
    DkLength(usize),
    /// c is not 1088 bytes (the value is the length given).
    CtLength(usize),
    /// dk fails the §7.3 hash check H(ek) == dk[768k+32 .. 768k+64].
    DkHash,
}

// ── Input checks (FIPS 203 §7.2 and §7.3) ──────────────────────────────────────

/// Encapsulation-key check (§7.2): correct length, and the modulus check
/// ByteEncode12(ByteDecode12(ek[0:384k])) == ek[0:384k] (every coefficient < q).
pub fn ek_check(ek: &[u8]) -> bool {
    if ek.len() != KEM768_EK_LEN {
        return false;
    }
    let mut ok = true;
    for i in 0..K {
        let chunk = &ek[384 * i..384 * (i + 1)];
        let mut f = [0i32; N];
        byte_decode(12, chunk, &mut f);
        let mut back = [0u8; 384];
        byte_encode(12, &f, &mut back);
        ok &= back[..] == chunk[..];
    }
    ok
}

/// Decapsulation-key check (§7.3): correct length, and the hash check
/// H(dk[384k : 768k+32]) == dk[768k+32 : 768k+64].
pub fn dk_check(dk: &[u8]) -> bool {
    dk.len() == KEM768_DK_LEN && sha3_256(&dk[384 * K..768 * K + 32])[..] == dk[768 * K + 32..768 * K + 64]
}

#[cfg(test)]
mod acvp_tests;
#[cfg(test)]
mod tests;
