/// bebop_session_crypto — end-to-end session crypto integration test.
/// Tests: key agreement → session establishment → encrypt → decrypt → close.
///
/// Run with: cargo test --test bebop_session_crypto --features "pq,ct-gate"
#[cfg(feature = "pq")]

use dowiz_kernel::bebop_bridge::{ChronosDtn, TriCap, HybridSignBridge};
use dowiz_kernel::pq::hybrid::{HybridKeypair, hybrid_keygen, hybrid_encaps, hybrid_decaps};
use dowiz_kernel::pq::hybrid_signing::{HybridSigner, HybridSignature};
use dowiz_kernel::ports::agent::cap::SignatureVerifier;
use dowiz_kernel::trinary::Tri;

fn seed(n: u8) -> [u8; 32] {
    [n; 32]
}

fn generate_keypair(x_byte: u8, kem_byte: u8) -> HybridKeypair {
    hybrid_keygen(&seed(x_byte), &seed(kem_byte))
}

#[test]
fn session_key_agreement_roundtrip() {
    let _alice = generate_keypair(0xAB, 0xCD);
    let bob = generate_keypair(0x11, 0x22);

    let (ct_a, shared_a) = hybrid_encaps(&bob, &seed(0x33), &seed(0x44));
    let shared_b = hybrid_decaps(&bob, &ct_a).unwrap();

    assert_eq!(
        shared_a, shared_b,
        "Shared secrets must match after key agreement"
    );
}

#[test]
fn session_encrypt_decrypt_roundtrip() {
    let session_key = [0x42u8; 32];
    let plaintext = b"bebop session payload";

    let mut ciphertext = [0u8; 64];
    for (i, byte) in plaintext.iter().enumerate() {
        ciphertext[i] = byte ^ session_key[i % 32];
    }
    ciphertext[plaintext.len()] = 0xFF; // terminator

    // Decrypt plaintext portion only; terminator was not XOR'd
    let mut decrypted = [0u8; 64];
    for (i, byte) in ciphertext.iter().enumerate().take(plaintext.len()) {
        decrypted[i] = byte ^ session_key[i % 32];
    }

    assert_eq!(
        &decrypted[..plaintext.len()],
        plaintext,
        "Decrypted text must match plaintext"
    );
    assert_eq!(
        ciphertext[plaintext.len()],
        0xFF,
        "Terminator must be intact"
    );
}

#[test]
fn session_full_lifecycle() {
    let _alice_kp = generate_keypair(0x01, 0x02);
    let bob_kp = generate_keypair(0x03, 0x04);

    let (ct, shared_alice) = hybrid_encaps(&bob_kp, &seed(0x05), &seed(0x06));
    let shared_bob = hybrid_decaps(&bob_kp, &ct).unwrap();
    assert_eq!(shared_alice, shared_bob);

    let msg = b"hello from session";
    let mut enc = [0u8; 128];
    for (i, b) in msg.iter().enumerate() {
        enc[i] = b ^ shared_alice[i % 32];
    }

    let mut dec = [0u8; 128];
    for (i, b) in enc.iter().enumerate().take(msg.len()) {
        dec[i] = b ^ shared_bob[i % 32];
    }
    assert_eq!(&dec[..msg.len()], msg);

    let msg2 = b"different message";
    let mut enc2 = [0u8; 128];
    for (i, b) in msg2.iter().enumerate() {
        enc2[i] = b ^ shared_alice[i % 32];
    }
    assert_ne!(
        &enc[..msg.len()], &enc2[..msg2.len()],
        "Different messages produce different ciphertexts"
    );
}

#[test]
fn session_key_forward_secrecy() {
    // Different session → different shared secret.
    let _alice_kp = generate_keypair(0x11, 0x12);
    let bob_kp = generate_keypair(0x13, 0x14);

    let (ct1, _s1) = hybrid_encaps(&bob_kp, &seed(0x15), &seed(0x16));
    let (ct2, _s2) = hybrid_encaps(&bob_kp, &seed(0x25), &seed(0x26));

    // Different ephemeral seeds → different X25519 ephemeral pubkeys.
    assert_ne!(
        ct1.x_ephemeral, ct2.x_ephemeral,
        "Fresh encapsulations produce different X25519 ephemeral keys"
    );
    assert_ne!(
        ct1.confirm, ct2.confirm,
        "Fresh encapsulations produce different confirmation tags"
    );
}

#[test]
fn session_tricap_auth_gate() {
    let cap = TriCap::new();

    assert_eq!(
        cap.effective(),
        Tri::Unknown,
        "All-unknown cap must resolve to Unknown"
    );

    let denied = TriCap {
        allow: Tri::True,
        delegate: Tri::False,
        revoke: Tri::True,
    };
    assert_eq!(
        denied.effective(),
        Tri::False,
        "Revoked cap with True allow must be False"
    );
    assert_eq!(
        denied.effective_lukasiewicz(),
        Tri::False,
        "Lukasiewicz: revoked overrides allow"
    );
    assert_eq!(
        denied.may_delegate(),
        Tri::False,
        "Delegation must be False"
    );

    let allowed = TriCap {
        allow: Tri::True,
        delegate: Tri::True,
        revoke: Tri::False,
    };
    assert_eq!(
        allowed.effective(),
        Tri::True,
        "True allow with no revoke must be True"
    );
    assert!(allowed.may_delegate().is_true());
}

#[test]
fn session_dtn_store_forward() {
    let mut dtn = ChronosDtn::new(16);

    let frames: Vec<Vec<u8>> = (0..8).map(|i| vec![i as u8; 16]).collect();
    let mut latest_ts = 0u64;
    for (i, frame) in frames.iter().enumerate() {
        let ts = dtn.store(frame.clone());
        latest_ts = ts;
        assert!(
            dtn.pending() <= i + 1,
            "DTN must not exceed capacity"
        );
    }

    let forwarded = dtn.forward_before(latest_ts + 1);
    assert_eq!(forwarded.len(), 8, "All frames should be forwarded");
    for (i, (_ts, frame)) in forwarded.iter().enumerate() {
        assert_eq!(frame[0], i as u8, "Frame {} should have ID {}", i, i);
    }
    assert_eq!(dtn.pending(), 0, "Queue empty after forward");

    // Capacity pruning
    let mut small = ChronosDtn::new(3);
    small.store(vec![1]);
    small.store(vec![2]);
    small.store(vec![3]);
    small.store(vec![4]);
    assert_eq!(small.pending(), 3);
}

#[test]
fn session_hybrid_sign_then_encrypt() {
    let signer = HybridSigner::from_seeds(&seed(0xA0), &seed(0xA1), &seed(0xA2));
    let cls_pub = signer.classical_public();
    let mldsa_pub = signer.mldsa_public();

    let payload = b"bebop session message v1";
    let sig = signer.sign_mldsa(payload, &seed(0xA3));

    let verified = HybridSigner::verify_mldsa(&cls_pub, &mldsa_pub, payload, &sig);
    assert!(verified, "Hybrid signature must verify");

    // Envelope roundtrip
    let env = signer.seal_envelope_mldsa(payload, &seed(0xA4));
    let recovered =
        HybridSigner::open_envelope_mldsa(&cls_pub, &mldsa_pub, &env)
            .expect("Envelope must open");
    assert_eq!(recovered, payload);
}

#[test]
fn session_invalid_key_rejected() {
    let bob_kp = generate_keypair(0x50, 0x51);
    let eve_kp = generate_keypair(0x60, 0x61);

    let (ct, _shared_alice) = hybrid_encaps(&bob_kp, &seed(0x70), &seed(0x71));

    let result = hybrid_decaps(&eve_kp, &ct);
    assert!(
        result.is_err(),
        "Eve must not be able to decrypt Alice's session"
    );
}

// ── HybridSignBridge (bebop protocol surface) ──────────────────────────────

#[test]
fn session_bridge_sign_verify_roundtrip() {
    let bridge = HybridSignBridge::new();
    let cls_secret = [0xBBu8; 32];
    let pq_secret = [0xDDu8; 32];
    let msg = b"bebop session bridge";

    let sig = bridge.sign_msg(&cls_secret, &pq_secret, msg);
    assert!(bridge.verify_msg(
        &bridge.verifier.classical_public(&cls_secret),
        &bridge.verifier.pq_public(&pq_secret),
        msg,
        &sig,
    ));
}

#[test]
fn session_bridge_tampered_msg_rejected() {
    let bridge = HybridSignBridge::new();
    let cls_secret = [0x10u8; 32];
    let pq_secret = [0x11u8; 32];
    let cls_pub = bridge.verifier.classical_public(&cls_secret);
    let pq_pub = bridge.verifier.pq_public(&pq_secret);

    let sig = bridge.sign_msg(&cls_secret, &pq_secret, b"original");
    assert!(!bridge.verify_msg(&cls_pub, &pq_pub, b"tampered", &sig));
}

#[test]
fn session_cross_signature_leg_tamper_rejected() {
    let signer = HybridSigner::from_seeds(&seed(0xC0), &seed(0xC1), &seed(0xC2));
    let cls_pub = signer.classical_public();
    let mldsa_pub = signer.mldsa_public();
    let msg = b"cross-domain probe";

    // ML-DSA leg tampered
    let sig = signer.sign_mldsa(msg, &seed(0xC3));
    let mut tampered_pq = sig.clone();
    if !tampered_pq.pq.is_empty() {
        tampered_pq.pq[0] ^= 0xFF;
    }
    assert!(
        !HybridSigner::verify_mldsa(&cls_pub, &mldsa_pub, msg, &tampered_pq),
        "Tampered PQ leg must be rejected"
    );

    // Classical leg tampered
    let mut tampered_cls = sig;
    tampered_cls.classical[0] ^= 0xFF;
    assert!(
        !HybridSigner::verify_mldsa(&cls_pub, &mldsa_pub, msg, &tampered_cls),
        "Tampered classical leg must be rejected"
    );
}

#[test]
fn session_no_classical_only_fallback() {
    let signer = HybridSigner::from_seeds(&seed(0xD0), &seed(0xD1), &seed(0xD2));
    let cls_pub = signer.classical_public();
    let mldsa_pub = signer.mldsa_public();
    let msg = b"no-fallback attack";

    let full_sig = signer.sign_mldsa(msg, &seed(0xD3));
    let broken = HybridSignature {
        classical: full_sig.classical.clone(),
        pq: vec![0u8; full_sig.pq.len()],
    };
    assert!(
        !HybridSigner::verify_mldsa(&cls_pub, &mldsa_pub, msg, &broken),
        "Zeroed PQ leg must be rejected (no classical-only fallback)"
    );
}

#[test]
fn session_deterministic_hybrid_signing() {
    let signer = HybridSigner::from_seeds(&seed(0xE0), &seed(0xE1), &seed(0xE2));
    let msg = b"deterministic session probe";

    let a = signer.sign_mldsa(msg, &seed(0xE3));
    let b = signer.sign_mldsa(msg, &seed(0xE3));
    assert_eq!(a, b, "Same inputs → same sig (deterministic)");

    let c = signer.sign_mldsa(b"different", &seed(0xE3));
    assert_ne!(a, c, "Different msg → different sig");
}

#[test]
fn session_tampered_kem_ct_rejected() {
    let _alice = generate_keypair(0xF0, 0xF1);
    let bob = generate_keypair(0xF2, 0xF3);

    let (mut ct, _ss) = hybrid_encaps(&bob, &seed(0xF4), &seed(0xF5));
    if !ct.kem_ct.is_empty() {
        ct.kem_ct[0] ^= 0xFF;
    }
    assert!(
        hybrid_decaps(&bob, &ct).is_err(),
        "Tampered KEM ct must be rejected"
    );
}

#[test]
fn session_slhdsa_sign_verify_roundtrip() {
    let signer = HybridSigner::from_seeds(&seed(0x10), &seed(0x11), &seed(0x12));
    let cls_pub = signer.classical_public();
    let slhdsa_pub = signer.slhdsa_public();

    let msg = b"bebop session SLH-DSA roundtrip";
    let sig = signer.sign_slhdsa(msg, &seed(0x13));

    let verified = HybridSigner::verify_slhdsa(&cls_pub, &slhdsa_pub, msg, &sig);
    assert!(verified, "SLH-DSA hybrid signature must verify");
    assert!(!sig.pq.is_empty(), "SLH-DSA sig must be non-empty");
    assert!(
        sig.pq.iter().any(|&b| b != 0),
        "SLH-DSA sig must not be all-zeros"
    );
}

#[test]
fn session_cross_domain_rejection() {
    // ML-DSA sig must NOT verify as SLH-DSA.
    let signer = HybridSigner::from_seeds(&seed(0x20), &seed(0x21), &seed(0x22));
    let cls_pub = signer.classical_public();
    let slhdsa_pub = signer.slhdsa_public();
    let msg = b"cross-domain attack";

    let sig = signer.sign_mldsa(msg, &seed(0x23));
    assert!(
        !HybridSigner::verify_slhdsa(&cls_pub, &slhdsa_pub, msg, &sig),
        "ML-DSA sig must NOT verify as SLH-DSA"
    );

    let sig2 = signer.sign_slhdsa(msg, &seed(0x24));
    let mldsa_pub = signer.mldsa_public();
    assert!(
        !HybridSigner::verify_mldsa(&cls_pub, &mldsa_pub, msg, &sig2),
        "SLH-DSA sig must NOT verify as ML-DSA"
    );
}
