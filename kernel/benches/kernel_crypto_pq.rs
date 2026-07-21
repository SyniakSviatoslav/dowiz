//! kernel_crypto_pq — P80 (S1 §3.3-C1). The ENTIRE kernel PQ lane was previously
//! UNBENCHED. This file gates behind the `pq` feature (same discipline as the `pq`
//! module itself: the canonical order/money core stays pure-std and serde-free).
//!
//! Covers: ML-DSA-65 sign/verify, ML-KEM-768 encaps/decaps, hybrid X25519⊕ML-KEM
//! decaps, and size-swept SHAKE256 / SHA3-256 digests.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dowiz_kernel::event_log::sha3_256;
use dowiz_kernel::pq::dsa::{keygen, sign, verify, MlDsa65Pk, MlDsa65Sk, RNDBYTES, SEEDBYTES};
use dowiz_kernel::pq::hybrid::{hybrid_decaps, hybrid_encaps, hybrid_keygen, HybridKeypair};
use dowiz_kernel::pq::keccak::shake256;
use dowiz_kernel::pq::kem;

fn kernel_crypto_pq(c: &mut Criterion) {
    let mut group = c.benchmark_group("kernel_crypto_pq");

    // ── ML-DSA-65 sign / verify ──
    let seed = [7u8; SEEDBYTES];
    let rnd = [0u8; RNDBYTES];
    let (pk, sk): (MlDsa65Pk, MlDsa65Sk) = keygen(&seed);
    let msg = b"dowiz mesh handshake payload";
    let sig = sign(&sk, msg, &rnd);
    group.bench_function("dsa_sign", |b| b.iter(|| black_box(sign(&sk, msg, &rnd))));
    group.bench_function("dsa_verify", |b| {
        b.iter(|| black_box(verify(&pk, msg, &sig)))
    });

    // ── ML-KEM-768 encaps / decaps ──
    let kem_seed = [9u8; 32];
    let (kem_pk, kem_sk) = kem::keygen_internal(&kem_seed);
    let m = [3u8; 32];
    let (ct, _ss_enc) = kem::encaps_internal(&kem_pk, &m);
    group.bench_function("kem_encaps", |b| {
        b.iter(|| black_box(kem::encaps_internal(&kem_pk, &m)))
    });
    group.bench_function("kem_decaps", |b| {
        b.iter(|| black_box(kem::decaps_internal(&kem_sk, &ct)))
    });

    // ── Hybrid X25519⊕ML-KEM decaps ──
    let x_seed = [1u8; 32];
    let kp: HybridKeypair = hybrid_keygen(&x_seed, &kem_seed);
    let eph_seed = [5u8; 32];
    let (h_ct, _ss) = hybrid_encaps(&kp, &m, &eph_seed);
    group.bench_function("hybrid_decaps", |b| {
        b.iter(|| black_box(hybrid_decaps(&kp, &h_ct)))
    });

    // ── SHAKE256 / SHA3-256 size sweep (INPUT size varies; output is fixed 32 B) ──
    for &size in &[256usize, 1024, 4096, 16384] {
        let data = vec![0xABu8; size];
        group.bench_function(format!("shake256_{size}"), |b| {
            b.iter(|| {
                let mut o = [0u8; 32];
                shake256(&data, &mut o);
                black_box(o)
            })
        });
        group.bench_function(format!("sha3_256_{size}"), |b| {
            b.iter(|| black_box(sha3_256(&data)))
        });
    }

    group.finish();
}

criterion_group!(benches, kernel_crypto_pq);
criterion_main!(benches);
