use super::*;

/// RFC 4231 test case 1. The published vector, not my own output -- that is
/// the whole difference between a standard construction and a bespoke one.
#[test]
fn hmac_matches_rfc4231_case_1() {
    let key = [0x0b; 20];
    let mac = hmac_sha256(&key, b"Hi There");
    assert_eq!(
        hex(&mac),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    );
}

/// RFC 4231 test case 2 — a short ASCII key.
#[test]
fn hmac_matches_rfc4231_case_2() {
    let mac = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
    assert_eq!(
        hex(&mac),
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
}

/// RFC 4231 test case 6 — a key LONGER than the 64-byte block, which is the
/// branch that silently diverges from the standard if the hash-the-key step
/// is left out.
#[test]
fn hmac_matches_rfc4231_case_6_long_key() {
    let key = [0xaa; 131];
    let mac = hmac_sha256(&key, b"Test Using Larger Than Block-Size Key - Hash Key First");
    assert_eq!(
        hex(&mac),
        "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
    );
}

/// RFC 6070 gives PBKDF2 vectors for HMAC-SHA1; RFC 8018's SHA-256 variant
/// is checked here against the widely published cross-implementation vectors
/// for the same inputs.
#[test]
fn pbkdf2_matches_published_vectors() {
    let mut out = [0u8; 32];
    pbkdf2_sha256(b"password", b"salt", 1, &mut out);
    assert_eq!(
        hex(&out),
        "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b"
    );

    pbkdf2_sha256(b"password", b"salt", 2, &mut out);
    assert_eq!(
        hex(&out),
        "ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43"
    );

    pbkdf2_sha256(b"password", b"salt", 4096, &mut out);
    assert_eq!(
        hex(&out),
        "c5e478d59288c841aa530db6845c4c8d962893a001ce4e11a4963873aa98134a"
    );
}

/// A longer output than one hash block exercises the multi-block path, which
/// is where an off-by-one in `block_index` hides.
#[test]
fn pbkdf2_produces_more_than_one_block() {
    let mut out = [0u8; 40];
    pbkdf2_sha256(b"passwordPASSWORDpassword", b"saltSALTsaltSALTsaltSALTsaltSALTsalt", 4096, &mut out);
    assert_eq!(
        hex(&out),
        "348c89dbcbd32b2f32d814b8116e84cf2b17347ebc1800181c4e2a1fb8dd53e1c635518c7dac47e9"
    );
}

#[test]
fn base64url_round_trips_and_rejects_junk() {
    for case in [b"".as_slice(), b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar"] {
        let e = b64url_encode(case);
        assert_eq!(b64url_decode(&e).as_deref(), Some(case), "round trip {e}");
    }
    assert_eq!(b64url_encode(b"foobar"), "Zm9vYmFy");
    // Standard-alphabet and padded forms must NOT decode, or one token has
    // several spellings that all verify.
    assert!(b64url_decode("Zm9vYmFy=").is_none());
    assert!(b64url_decode("a+b/c").is_none());
}

#[test]
fn hex_round_trips() {
    let b = [0u8, 1, 15, 16, 127, 128, 255];
    assert_eq!(hex(&b), "00010f107f80ff");
    assert_eq!(unhex("00010f107f80ff").as_deref(), Some(b.as_slice()));
    assert!(unhex("abc").is_none(), "odd length is not hex");
    assert!(unhex("zz").is_none(), "non-hex digits are not hex");
}

#[test]
fn constant_time_eq_is_correct() {
    assert!(constant_time_eq(b"abc", b"abc"));
    assert!(!constant_time_eq(b"abc", b"abd"));
    assert!(!constant_time_eq(b"abc", b"ab"));
    assert!(constant_time_eq(b"", b""));
}

/// The CSPRNG must actually produce different bytes. A source stuck at zero
/// would make every salt and session id identical, and nothing else in the
/// system would notice.
#[test]
fn random_bytes_are_random() {
    let a = random_bytes(32).expect("urandom");
    let b = random_bytes(32).expect("urandom");
    assert_eq!(a.len(), 32);
    assert_ne!(a, b, "two draws must differ");
    assert!(a.iter().any(|&x| x != 0), "all-zero is not random");
}
