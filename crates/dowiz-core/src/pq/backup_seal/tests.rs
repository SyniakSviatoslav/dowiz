use super::*;
use alloc::vec;

fn ent(b: u8) -> SealEntropy {
    SealEntropy { m: [b; 32], eph: [b.wrapping_add(1); 32], nonce: [b.wrapping_add(2); 12] }
}

fn pair() -> (HybridKeypair, SealPublic) {
    let (file, pk) = keygen(&[11u8; 32], &[22u8; 32]);
    (secret_from_file(&file).unwrap(), pk)
}

fn plaintext() -> Vec<u8> {
    (0..5000u32).map(|i| (i * 31 % 251) as u8).collect()
}

#[test]
fn seal_then_open_roundtrips_byte_equal() {
    let (sk, pk) = pair();
    let sealed = seal(&pk, 1, &plaintext(), &ent(3)).unwrap();
    assert_eq!(sealed.len(), HEADER_LEN + plaintext().len() + 16);
    assert_eq!(&sealed[..7], MAGIC);
    assert_eq!(open(&sk, &sealed), Ok((1, plaintext())));
    // Empty plaintext is a legal seal too.
    let e = seal(&pk, 0, &[], &ent(4)).unwrap();
    assert_eq!(open(&sk, &e), Ok((0, vec![])));
}

#[test]
fn the_body_is_not_the_plaintext() {
    let (_, pk) = pair();
    let p = plaintext();
    let sealed = seal(&pk, 1, &p, &ent(3)).unwrap();
    assert!(!sealed.windows(64).any(|w| w == &p[..64]), "plaintext visible in the sealed file");
}

/// Every header field and the body/tag: one flipped bit refuses.
#[test]
fn a_flip_anywhere_refuses() {
    let (sk, pk) = pair();
    let sealed = seal(&pk, 1, &plaintext(), &ent(3)).unwrap();
    let at = 9 + KEM768_CT_LEN;
    let spots = [
        (0, SealError::NotASeal),                          // magic
        (7, SealError::NotASeal),                          // version
        (8, SealError::Decrypt),                           // kind (bound by the KDF)
        (9 + 500, SealError::KemRejected("key-confirmation-failed")), // KEM ct
        (at + 3, SealError::KemRejected("key-confirmation-failed")),  // x25519 eph
        (at + 40, SealError::KemRejected("key-confirmation-failed")), // confirm tag
        (at + 64, SealError::Decrypt),                     // nonce
        (HEADER_LEN + 7, SealError::Decrypt),              // body
        (sealed.len() - 1, SealError::Decrypt),            // GCM tag
    ];
    for (i, want) in spots {
        let mut t = sealed.clone();
        t[i] ^= 0x01;
        assert_eq!(open(&sk, &t), Err(want), "flip at byte {i}");
    }
}

#[test]
fn truncation_refuses_at_every_boundary() {
    let (sk, pk) = pair();
    let sealed = seal(&pk, 1, &plaintext(), &ent(3)).unwrap();
    for n in [0, 6, 9, HEADER_LEN, HEADER_LEN + 15] {
        assert_eq!(open(&sk, &sealed[..n]), Err(SealError::Truncated(n)), "cut at {n}");
    }
    // Cut inside the body: well-formed length, wrong tag.
    assert_eq!(open(&sk, &sealed[..sealed.len() - 100]), Err(SealError::Decrypt));
}

#[test]
fn the_wrong_secret_key_refuses() {
    let (_, pk) = pair();
    let (other_file, _) = keygen(&[33u8; 32], &[44u8; 32]);
    let other = secret_from_file(&other_file).unwrap();
    let sealed = seal(&pk, 1, &plaintext(), &ent(3)).unwrap();
    assert_eq!(open(&other, &sealed), Err(SealError::KemRejected("key-confirmation-failed")));
}

#[test]
fn public_key_text_roundtrips_and_malformed_text_refuses() {
    let (_, pk) = pair();
    let text = pk.encode();
    assert_eq!(SealPublic::parse(&text), Ok(pk.clone()));
    assert_eq!(SealPublic::parse(&format!("  {text}\n")), Ok(pk.clone()), "whitespace is trimmed");
    let hex = &text[PK_PREFIX.len()..];
    assert_eq!(SealPublic::parse(hex), Err(SealError::PublicKeyFormat), "no prefix");
    assert_eq!(SealPublic::parse(&text[..text.len() - 1]), Err(SealError::PublicKeyFormat), "odd hex");
    assert_eq!(SealPublic::parse(&text[..text.len() - 2]), Err(SealError::PublicKeyFormat), "short");
    assert_eq!(SealPublic::parse(&text.replace('a', "g")), Err(SealError::PublicKeyFormat), "not hex");
    // Well-formed hex, bad KEM key: first coefficient 0xfff >= q.
    let mut bad = pk.clone();
    bad.kem_pk[0] = 0xff;
    bad.kem_pk[1] |= 0x0f;
    assert_eq!(
        SealPublic::parse(&bad.encode()),
        Err(SealError::PublicKeyRefused("kem-public-key-malformed"))
    );
    let zero_x = SealPublic { x_pk: [0u8; 32], kem_pk: pk.kem_pk.clone() };
    assert_eq!(
        SealPublic::parse(&zero_x.encode()),
        Err(SealError::PublicKeyRefused("x25519-public-key-low-order"))
    );
}

#[test]
fn secret_key_file_format_is_checked() {
    let (file, pk) = keygen(&[11u8; 32], &[22u8; 32]);
    assert_eq!(file.len(), SK_FILE_LEN);
    assert_eq!(secret_from_file(&file).unwrap().kem_pk, pk.kem_pk, "positive twin");
    assert_eq!(secret_from_file(&file[..71]).err(), Some(SealError::SecretKeyFormat));
    let mut bad = file.clone();
    bad[0] = b'X';
    assert_eq!(secret_from_file(&bad).err(), Some(SealError::SecretKeyFormat));
}
