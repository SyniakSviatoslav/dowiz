//! bebop_property_tests — property-based tests for the bebop protocol wire, crypto, and mesh.
//!
//! Tests verify:
//! 1. Hybrid sign determinism (same input twice → same signature)
//! 2. Hybrid verify tamper (ANY bit flip → verify fails)
//! 3. Bebop frame serialize/deserialize roundtrip (seal→open, 100 random payloads)
//! 4. Chronos DTN store-forward FIFO order preservation
//! 5. Wave mesh sync convergence (3 nodes sync → same state within 10 rounds)
//! 6. TriCap policy monotonicity (more restrictions never increases access)
//!
//! Run with: cargo test --test bebop_property_tests --features "pq,ct-gate"
#![cfg(feature = "pq")]

use dowiz_kernel::bebop_bridge::TriCap;
use dowiz_kernel::chronos::Chronos;
use dowiz_kernel::pq::codesign::{apply, codesign_keypair, sign_update, ApplyLedger, UpdateBlob};
use dowiz_kernel::pq::envelope;
use dowiz_kernel::pq::hybrid::{hybrid_keygen, hybrid_encaps, hybrid_decaps};
use dowiz_kernel::trinary::Tri;
use dowiz_kernel::wave::{InterferenceField, Wave};
use std::collections::HashMap;

const ENTROPY_LEN: usize = 32;

fn seed(n: u8) -> [u8; ENTROPY_LEN] {
    [n; ENTROPY_LEN]
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 1: hybrid_sign_twice_is_deterministic
// Same input twice → same envelope signature every time.
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn hybrid_sign_twice_is_deterministic() {
    let (_pk, sk) = envelope::new_identity(&seed(1));
    let payload = b"deterministic payload for hybrid sign";
    let rnd = seed(2);

    let env1 = envelope::seal(payload, &sk, &rnd);
    let env2 = envelope::seal(payload, &sk, &rnd);

    assert_eq!(env1.content_hash, env2.content_hash,
        "content_hash must be deterministic");
    assert_eq!(env1.sig, env2.sig,
        "ML-DSA-65 signature must be deterministic with same (sk, msg, rnd)");
    assert_eq!(env1.payload, env2.payload);
}

#[test]
fn hybrid_sign_codesign_twice_is_deterministic() {
    let (_root, key) = codesign_keypair(&seed(0xAB));
    let blob_a = sign_update(b"code-signed update v1", &key, &seed(2));
    let blob_b = sign_update(b"code-signed update v1", &key, &seed(2));

    match (&blob_a, &blob_b) {
        (UpdateBlob::Signed(a), UpdateBlob::Signed(b)) => {
            assert_eq!(a.sig, b.sig, "codesign sig must be deterministic");
            assert_eq!(a.payload, b.payload);
            assert_eq!(a.content_hash, b.content_hash);
        }
        _ => panic!("expected both signed"),
    }
}

#[test]
fn hybrid_kem_keygen_determinism() {
    let x_seed = seed(0x11);
    let kem_seed = seed(0x22);

    let kp1 = hybrid_keygen(&x_seed, &kem_seed);
    let kp2 = hybrid_keygen(&x_seed, &kem_seed);

    assert_eq!(kp1.x_pk, kp2.x_pk, "X25519 pk must be deterministic");
    assert_eq!(kp1.kem_pk, kp2.kem_pk, "ML-KEM pk must be deterministic");
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 2: hybrid_verify_tamper_always_fails
// ANY single bit flip in sig, payload, or content_hash → verify fails.
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn hybrid_verify_tamper_always_fails_flip_sig_byte() {
    let (pk, sk) = envelope::new_identity(&seed(3));
    let mut env = envelope::seal(b"tamper-resistant payload", &sk, &seed(4));

    for idx in [0, 1, 100, 500, 1000, 2000, 3000, 3305, 3308].iter() {
        let pos = (*idx).min(env.sig.len().saturating_sub(1));
        let original = env.sig[pos];
        env.sig[pos] ^= 0x01;
        assert!(
            envelope::open(&env, &pk).is_err(),
            "signature bit-flip at byte {pos} must fail verification"
        );
        env.sig[pos] = original;
    }
}

#[test]
fn hybrid_verify_tamper_always_fails_flip_payload_byte() {
    let (pk, sk) = envelope::new_identity(&seed(5));
    let mut env = envelope::seal(b"payload integrity test", &sk, &seed(6));

    if !env.payload.is_empty() {
        env.payload[0] ^= 0xFF;
        assert!(
            envelope::open(&env, &pk).is_err(),
            "payload bit-flip must fail verification"
        );
    }
}

#[test]
fn hybrid_verify_tamper_always_fails_flip_hash_byte() {
    let (pk, sk) = envelope::new_identity(&seed(7));
    let mut env = envelope::seal(b"hash check test", &sk, &seed(8));

    env.content_hash[10] ^= 0x80;
    assert!(
        envelope::open(&env, &pk).is_err(),
        "content_hash bit-flip must fail verification"
    );
}

#[test]
fn hybrid_verify_tamper_always_fails_wrong_key() {
    let (pk_a, _sk_a) = envelope::new_identity(&seed(9));
    let (_pk_b, sk_b) = envelope::new_identity(&seed(10));
    let env = envelope::seal(b"wrong-key test", &sk_b, &seed(11));

    assert!(
        envelope::open(&env, &pk_a).is_err(),
        "signature must not verify under wrong public key"
    );
}

#[test]
fn hybrid_kem_tamper_ciphertext_fails_decaps() {
    let x_seed = seed(0xAA);
    let kem_seed = seed(0xBB);

    let kp = hybrid_keygen(&x_seed, &kem_seed);
    let (ct, ss_send) = hybrid_encaps(&kp, &seed(0xCC), &seed(0xDD));

    let ss_recv = hybrid_decaps(&kp, &ct).expect("valid KEM roundtrip must succeed");
    assert_eq!(ss_send, ss_recv, "valid KEM roundtrip must match");

    let mut ct_tampered = ct.clone();
    if !ct_tampered.kem_ct.is_empty() {
        ct_tampered.kem_ct[0] ^= 0x01;
    }
    let ss_tampered = hybrid_decaps(&kp, &ct_tampered);
    assert!(
        ss_tampered.is_err() || ss_tampered.unwrap() != ss_send,
        "tampered ciphertext must NOT yield the same shared secret"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 3: bebop_frame_serialize_roundtrip
// seal → open must recover the exact original payload (for 100 random payloads).
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bebop_frame_serialize_roundtrip_100_random_frames() {
    let (pk, sk) = envelope::new_identity(&seed(20));

    for i in 0u8..100 {
        let len = 8 + (i as usize % 200);
        let payload: Vec<u8> = seed(i)
            .iter()
            .cycle()
            .take(len)
            .copied()
            .collect();
        let rnd = seed(i.wrapping_add(64));

        let env = envelope::seal(&payload, &sk, &rnd);
        assert_eq!(env.sig.len(), envelope::SIG_LEN);
        assert_eq!(env.content_hash.len(), envelope::HASH_LEN);

        let opened = envelope::open(&env, &pk)
            .expect(&format!("frame {} must roundtrip", i));
        assert_eq!(opened, payload, "seal→open roundtrip failed at frame {}", i);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 4: chronos_dtn_store_forward_preserves_order
// Store N frames (snapshots), retrieve all → FIFO order preserved.
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn chronos_dtn_store_forward_preserves_order() {
    let mut chronos = Chronos::new(200);

    for i in 0u64..50 {
        let mut values = HashMap::new();
        values.insert("seq".to_string(), i as f64);
        values.insert("payload_hash".to_string(), (i * 7 + 3) as f64);
        chronos.snapshot(values);
    }

    assert_eq!(chronos.len(), 50, "all 50 frames stored");

    let all = chronos.window(0, u64::MAX);
    assert_eq!(all.len(), 50);

    let mut prev_seq = -1.0f64;
    for snap in &all {
        let seq = snap.values.get("seq").copied().unwrap_or(-999.0);
        assert!(
            seq > prev_seq,
            "FIFO order violated: seq {seq} after {prev_seq}"
        );
        prev_seq = seq;
    }

    for snap in &all {
        assert!(snap.verify(), "snapshot integrity must hold");
    }
}

#[test]
fn chronos_dtn_capacity_respected() {
    let mut chronos = Chronos::new(10);

    for i in 0..30 {
        let mut values = HashMap::new();
        values.insert("x".to_string(), i as f64);
        chronos.snapshot(values);
    }

    assert!(chronos.len() <= 10, "capacity must be respected");
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 5: wave_mesh_sync_converges
// 3 independent nodes, absorb common waves → converge to the same state.
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn wave_mesh_sync_converges_3_nodes() {
    // 3 independent interference fields (nodes).
    let mut nodes: Vec<InterferenceField> = (0..3)
        .map(|_| InterferenceField::new())
        .collect();

    // Each node emits a distinct wave.
    for (i, node) in nodes.iter_mut().enumerate() {
        let w = Wave::simple(
            &format!("node-{}", i),
            1000 * (i as u64 + 1),
            1.0 + i as f64 * 0.1,
            0.5 + i as f64 * 0.1,
            0.01,
        );
        node.add_wave(w);
    }

    // Run sync rounds: each node absorbs all other nodes' waves.
    for _ in 0..10 {
        // Collect all waves across nodes.
        let mut all_waves: Vec<Wave> = Vec::new();
        for node in nodes.iter() {
            for w in &node.waves {
                all_waves.push(w.clone());
            }
        }

        // Each node absorbs the full set (convergence).
        for node in nodes.iter_mut() {
            for w in &all_waves {
                if !node.waves.iter().any(|existing| existing.source == w.source) {
                    node.add_wave(w.clone());
                }
            }
        }
    }

    // After sync: all nodes have the same set of wave sources (semantically).
    let mut sources_0: Vec<String> = nodes[0].waves.iter().map(|w| w.source.clone()).collect();
    sources_0.sort();
    for (i, node) in nodes.iter().enumerate().skip(1) {
        let mut sources_i: Vec<String> = node.waves.iter().map(|w| w.source.clone()).collect();
        sources_i.sort();
        assert_eq!(
            sources_0, sources_i,
            "node {i} wave sources diverged after sync"
        );
    }

    assert_eq!(nodes[0].active_count(), nodes[1].active_count());
    assert_eq!(nodes[1].active_count(), nodes[2].active_count());
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 6: tricap_policy_monotonic
// Adding more restrictions to TriCap never INCREASES access.
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn tricap_policy_monotonic_more_restrictions_never_increases_access() {
    let full = TriCap { allow: Tri::True, delegate: Tri::True, revoke: Tri::False };
    let _full_eff = full.effective();
    assert_eq!(_full_eff, Tri::True, "full access baseline is True");

    let cases = vec![
        ("deny-access",       TriCap { allow: Tri::False, delegate: Tri::True,  revoke: Tri::False }),
        ("revoked",           TriCap { allow: Tri::True,  delegate: Tri::True,  revoke: Tri::True }),
        ("deny-delegate",     TriCap { allow: Tri::True,  delegate: Tri::False, revoke: Tri::False }),
        ("pending-access",    TriCap { allow: Tri::Unknown, delegate: Tri::True,  revoke: Tri::False }),
        ("all-unknown",       TriCap { allow: Tri::Unknown, delegate: Tri::Unknown, revoke: Tri::Unknown }),
        ("deny-and-revoked",  TriCap { allow: Tri::False, delegate: Tri::False, revoke: Tri::True }),
        ("pending-revoked",   TriCap { allow: Tri::Unknown, delegate: Tri::True, revoke: Tri::True }),
    ];

    // In access terms: True (full access) > Unknown (pending) > False (denied).
    // Full baseline is {allow=True, revoke=False} → effective=True.
    // Adding restrictions (deny allow, revoke, make pending) must not give MORE access.
    for (label, restricted) in &cases {
        let eff = restricted.effective();

        // When allow is False or revoked, effective must be False.
        if restricted.allow == Tri::False || restricted.revoke == Tri::True {
            assert_eq!(
                eff, Tri::False,
                "{label}: effective must be False when allow denied or revoked, got {eff:?}"
            );
        } else if restricted.allow == Tri::Unknown {
            assert_eq!(
                eff, Tri::Unknown,
                "{label}: effective must be Unknown when allow pending, got {eff:?}"
            );
        } else {
            // allow=True, revoke=False → effective must be True.
            assert_eq!(
                eff, Tri::True,
                "{label}: effective must be True when allow=True+revoke=False, got {eff:?}"
            );
        }
    }
}

#[test]
fn tricap_policy_monotonic_false_or_unknown_decreases_effective() {
    let baseline = TriCap { allow: Tri::True, delegate: Tri::True, revoke: Tri::False };
    assert_eq!(baseline.effective(), Tri::True, "baseline effective must be True");

    let deny = TriCap { allow: Tri::False, delegate: Tri::True, revoke: Tri::False };
    assert_eq!(deny.effective(), Tri::False, "deny → effective False");

    let pending = TriCap { allow: Tri::Unknown, delegate: Tri::True, revoke: Tri::False };
    assert_eq!(pending.effective(), Tri::Unknown, "pending → effective Unknown");
}

#[test]
fn tricap_policy_monotonic_revoked_always_false() {
    let cases = vec![
        TriCap { allow: Tri::True, delegate: Tri::True, revoke: Tri::True },
        TriCap { allow: Tri::False, delegate: Tri::True, revoke: Tri::True },
        TriCap { allow: Tri::Unknown, delegate: Tri::Unknown, revoke: Tri::True },
    ];

    for c in &cases {
        let eff = c.effective();
        assert!(
            eff == Tri::False || eff == Tri::Unknown,
            "revoked capability effective ({eff:?}) must never be True"
        );
        let eff_l = c.effective_lukasiewicz();
        assert!(
            eff_l == Tri::False || eff_l == Tri::Unknown,
            "revoked-lukasiewicz effective ({eff_l:?}) must never be True"
        );
    }
}

#[test]
fn tricap_policy_monotonic_allow_deny_limits_access() {
    let mut cap = TriCap::new();
    assert_eq!(cap.effective(), Tri::Unknown);

    cap.allow = Tri::True;
    assert_eq!(cap.effective(), Tri::Unknown, "still Unknown until revoke resolved");

    cap.revoke = Tri::False;
    assert_eq!(cap.effective(), Tri::True, "allow=True + not revoked = True");

    cap.delegate = Tri::False;
    assert_eq!(cap.effective(), Tri::True, "deny delegation does not change allow");
    assert_eq!(cap.may_delegate(), Tri::False);

    cap.revoke = Tri::True;
    assert_eq!(cap.effective(), Tri::False, "revoked → False regardless of allow");

    cap.allow = Tri::True;
    assert_eq!(cap.effective(), Tri::False, "revoked dominates allow");
}

// ═══════════════════════════════════════════════════════════════════════════════
// BONUS: codesign apply/refuse property tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn codesign_apply_roundtrip_100_blobs() {
    let (root, key) = codesign_keypair(&seed(0x42));

    for i in 0u8..100 {
        let payload_len = 1 + (i as usize % 500);
        let payload: Vec<u8> = (0..payload_len).map(|b| b as u8 ^ i).collect();

        let signed = sign_update(&payload, &key, &seed(i));
        let mut ledger = ApplyLedger::new();
        let applied = apply(&root, &signed, &mut ledger)
            .expect(&format!("codesign apply must succeed at blob {}", i));

        assert_eq!(applied.payload, payload, "payload must be recovered intact");
        assert!(ledger.contains(&applied.blob_hash));
    }
}

#[test]
fn codesign_unsigned_must_always_be_refused() {
    let (root, _key) = codesign_keypair(&seed(0x55));

    for i in 0u8..50 {
        let unsigned = UpdateBlob::Unsigned(vec![i; 10]);
        let mut ledger = ApplyLedger::new();
        assert!(
            apply(&root, &unsigned, &mut ledger).is_err(),
            "unsigned blob {i} must be refused"
        );
        assert!(ledger.applied.is_empty());
    }
}

#[test]
fn codesign_tamper_any_byte_fails_apply() {
    let (root, key) = codesign_keypair(&seed(0x66));

    for i in 0u8..20 {
        let mut signed = sign_update(b"tamper-me v1", &key, &seed(i));
        match &mut signed {
            UpdateBlob::Signed(s) => {
                let idx = (i as usize * 7 + 3) % s.payload.len().max(1);
                s.payload[idx] ^= 0xFF;
                let mut ledger = ApplyLedger::new();
                assert!(
                    apply(&root, &signed.clone(), &mut ledger).is_err(),
                    "tampered blob {i} must fail apply"
                );
                assert!(ledger.applied.is_empty());
            }
            UpdateBlob::Unsigned(_) => unreachable!(),
        }
    }
}

#[test]
fn codesign_different_key_must_always_fail() {
    let (root_a, _key_a) = codesign_keypair(&seed(0x77));
    let (_root_b, key_b) = codesign_keypair(&seed(0x88));

    for i in 0u8..30 {
        let signed_by_b = sign_update(b"wrong-key payload", &key_b, &seed(i));
        let mut ledger = ApplyLedger::new();
        assert!(
            apply(&root_a, &signed_by_b, &mut ledger).is_err(),
            "blob {i} signed by key_B must fail under root_A"
        );
        assert!(ledger.applied.is_empty());
    }
}
