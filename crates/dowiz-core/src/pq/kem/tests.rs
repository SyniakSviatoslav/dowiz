//! kem's own tests. These are self-consistency, bounds and oracle checks; the
//! FIPS 203 conformance evidence is the ACVP gate in `acvp_tests.rs`, not these.

use super::ntt::{multiply_ntts, ntt, ntt_inv, GAMMAS, ZETAS};
use super::*;
use alloc::string::String;

/// TEST ORACLE ONLY: schoolbook multiplication in R_q = Z_q[x]/(x^256 + 1)
/// (the pre-P91.2 production rule, kept only to cross-check the NTT path).
fn poly_mul(a: &[i32; N], b: &[i32; N]) -> [i32; N] {
    let mut r = [0i64; N];
    for i in 0..N {
        for j in 0..N {
            let term = a[i] as i64 * b[j] as i64 % Q as i64;
            // NEGACYCLIC: x^256 = -1, so a wrapped term is SUBTRACTED.
            if i + j < N { r[i + j] += term } else { r[i + j - N] -= term }
        }
    }
    core::array::from_fn(|k| r[k].rem_euclid(Q as i64) as i32)
}

/// Deterministic pseudo-random polynomial with coefficients in [0, q).
fn lcg_poly(seed: u64) -> [i32; N] {
    let mut s = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    core::array::from_fn(|_| {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((s >> 33) % Q as u64) as i32
    })
}

// P91.0/P91.2 — header gate. Before P91.2 any FIPS-203 claim needed the
// "NOT FIPS-203" marker. After P91.2 the claim is allowed ONLY while the
// module still declares the ACVP gate and the header names it; the old false
// "Upgrade path: none needed" line may never come back.
#[test]
fn kem_header_no_false_fips_claim() {
    let src = include_str!("../kem.rs");
    let header: String = src.lines().take_while(|l| l.starts_with("//!")).collect();
    assert!(!src.contains("Upgrade path: none needed"));
    if header.contains("FIPS 203") || header.contains("FIPS-203") {
        assert!(
            src.contains("mod acvp_tests;") && header.contains("ACVP"),
            "kem.rs claims FIPS 203 without declaring and naming the ACVP gate"
        );
    }
}

// The two §4.3 tables, pinned to the first entries printed in FIPS 203 Appendix A.
#[test]
fn kem_ntt_tables_match_appendix_a() {
    assert_eq!(&ZETAS[..8], &[1, 1729, 2580, 3289, 2642, 630, 1897, 848]);
    assert_eq!(&GAMMAS[..4], &[17, Q - 17, 2761, Q - 2761]);
}

// NTT⁻¹(NTT(a)) = a, and NTT⁻¹(MultiplyNTTs(NTT a, NTT b)) equals the schoolbook
// negacyclic product: the independent oracle for Algorithms 9–12.
#[test]
fn kem_ntt_matches_schoolbook() {
    for s in 0..8u64 {
        let (a, b) = (lcg_poly(2 * s + 1), lcg_poly(2 * s + 2));
        assert_eq!(ntt_inv(&ntt(&a)), a, "NTT round-trip broke at seed {s}");
        let fast = ntt_inv(&multiply_ntts(&ntt(&a), &ntt(&b)));
        assert_eq!(fast, poly_mul(&a, &b), "NTT product != schoolbook at seed {s}");
    }
}

// ek/dk input checks (§7.2/§7.3): positive twin on a fresh key, refusal on one
// out-of-range coefficient / one flipped hash byte.
#[test]
fn kem_input_checks_positive_and_refusal() {
    let (ek, dk) = keygen_internal(&[3u8; 32], &[4u8; 32]);
    assert!(ek_check(&ek) && dk_check(&dk));
    let mut bad_ek = ek.clone();
    bad_ek[0] = 0xff;
    bad_ek[1] |= 0x0f; // first 12-bit coefficient = 4095 >= q
    assert!(!ek_check(&bad_ek));
    let mut bad_dk = dk.clone();
    bad_dk[768 * K + 32] ^= 1;
    assert!(!dk_check(&bad_dk));
}

// Implicit rejection returns exactly J(z ‖ c) on a tampered ciphertext.
#[test]
fn kem_implicit_rejection_is_j_of_z_c() {
    let z = [5u8; 32];
    let (ek, dk) = keygen_internal(&[6u8; 32], &z);
    let (mut c, _) = encaps_internal(&ek, &[7u8; 32]);
    c[0] ^= 1;
    assert_eq!(decaps_internal(&dk, &c).unwrap(), crate::pq::keccak::xof_j(&z, &c).to_vec());
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
    assert_eq!((CT_LEN, KEM768_CT_LEN), (1088, 1088), "32*(du*k+dv) = 1088 (was 1536)");
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
    assert_eq!(
        &dk[KEM768_DK_LEN - 32..],
        &z[..],
        "FO seed z must be stored in dk"
    );
    // A different z must change the keypair (z is actually consumed).
    let (_ek2, dk2) = keygen_internal(&d, &[100u8; 32]);
    assert_ne!(dk, dk2, "changing z must change the secret key");

    // Round-trip: encaps -> decaps recovers the shared secret.
    let m = [42u8; 32];
    let (c, k_send) = encaps_internal(&ek, &m);
    assert_eq!(c.len(), KEM768_CT_LEN, "ciphertext length");
    let k_recv = decaps_internal(&dk, &c).unwrap();
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
    let k_recv = decaps_internal(&sk, &c).unwrap();
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
    let k_tampered = decaps_internal(&sk, &ct).unwrap();
    let k_clean = decaps_internal(&sk, &c).unwrap();
    assert_ne!(
        k_tampered, k_clean,
        "tampered ct must not yield clean secret"
    );
}

#[test]
fn kem_soak_random_seeds() {
    for s in 0u8..50 {
        let d = [s; 32];
        let z = [s.wrapping_mul(13).wrapping_add(5); 32];
        let (pk, sk) = keygen_internal(&d, &z);
        let m = [s.wrapping_mul(7).wrapping_add(3); 32];
        let (c, k_send) = encaps_internal(&pk, &m);
        let k_recv = decaps_internal(&sk, &c).unwrap();
        assert_eq!(k_send, k_recv, "roundtrip mismatch at seed {s}");
        let mut ct = c.clone();
        let idx = (s as usize) % ct.len();
        ct[idx] ^= 0x01;
        let k_t = decaps_internal(&sk, &ct).unwrap();
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
        assert_eq!(
            (r as i128 - x as i128).rem_euclid(Q as i128),
            0,
            "red({x}) wrong residue"
        );
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

/// EXHAUSTIVE over x ∈ [0, Q) × the deployed widths d ∈ {1, 4, 10, 12} (~13.3k):
/// `compress`/`decompress` are panic-/overflow-free with in-range outputs.
#[test]
fn item7_exhaustive_compress_decompress_bounds() {
    for &d in &[1usize, 4, 10, 12] {
        let bound = 1i32 << d;
        for x in 0..Q {
            let c = compress(d, x);
            assert!(
                c >= 0 && c < bound,
                "compress({d},{x}) = {c} out of [0,2^d)"
            );
            let dec = decompress(d, c);
            assert!(
                dec >= 0 && dec < Q,
                "decompress({d},{c}) = {dec} out of [0,Q)"
            );
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

// Malformed decaps input is REFUSED, never a panic; the well-formed twin decapsulates.
#[test]
fn kem_decaps_refuses_malformed_input() {
    let (ek, dk) = keygen_internal(&[8u8; 32], &[9u8; 32]);
    let (c, k) = encaps_internal(&ek, &[1u8; 32]);
    assert_eq!(decaps_internal(&dk, &c), Ok(k));
    assert_eq!(decaps_internal(&dk[..100], &c), Err(KemError::DkLength(100)));
    assert_eq!(decaps_internal(&dk, &c[..1087]), Err(KemError::CtLength(1087)));
    assert_eq!(decaps_internal(&dk, &[c.clone(), vec![0]].concat()), Err(KemError::CtLength(1089)));
    let mut bad = dk.clone();
    bad[768 * K + 40] ^= 1;
    assert_eq!(decaps_internal(&bad, &c), Err(KemError::DkHash));
}
