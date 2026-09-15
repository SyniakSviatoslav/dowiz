//! The primitives the hub's identity rests on, built from `sha2` alone.
//!
//! WHY THESE ARE HERE RATHER THAN PULLED IN. `native-spa-server` carries a
//! ZERO-DEP-ALLOWLIST whose CI gate permits the list to SHRINK and never to
//! grow. `sha2`, `subtle` and `base64` are in it; `hmac`, `argon2`,
//! `password-hash` and every RNG crate are not. So the choice was not
//! "hand-roll or use a library" — it was "build these two standard
//! constructions on an allowed hash, or have no authentication on the hub at
//! all".
//!
//! WHAT IS AND IS NOT HAND-ROLLED, precisely. No primitive is invented here.
//! SHA-256 comes from `sha2`. HMAC is RFC 2104 and PBKDF2 is RFC 8018 — both
//! are short, fully specified constructions ON TOP of a hash, and both ship
//! with official test vectors, which is why the tests below check against
//! RFC 4231 and RFC 6070 rather than against my own output. A construction
//! verified against the standard's own vectors is a different thing from a
//! bespoke scheme.
//!
//! WHAT THIS COSTS, stated plainly. PBKDF2 is WEAKER THAN argon2id: it is not
//! memory-hard, so it buys less against an attacker with GPUs. argon2id is what
//! the Worker's `auth.rs` uses and what this would use if the allowlist allowed
//! it. The mitigation is iteration count (`PBKDF2_ITERATIONS` below, at OWASP's
//! 2023 figure for this exact primitive), and the honest summary is: this is a
//! respectable password hash, not the best one available in the abstract.

use sha2::{Digest, Sha256};

/// SHA-256 block size in bytes — HMAC's key padding width.
const BLOCK: usize = 64;
/// SHA-256 output size in bytes.
pub const HASH_LEN: usize = 32;

/// OWASP's 2023 recommendation for PBKDF2-HMAC-SHA256. Slowness IS the feature
/// here; this is the one place in the hub where a fast answer is a worse one.
pub const PBKDF2_ITERATIONS: u32 = 600_000;

/// HMAC-SHA256 (RFC 2104).
pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; HASH_LEN] {
    // A key longer than the block is replaced by its hash, per the RFC. Omitting
    // this makes long keys silently behave as different keys than the standard
    // says, which only shows up when interoperating.
    let mut k = [0u8; BLOCK];
    if key.len() > BLOCK {
        let d = Sha256::digest(key);
        k[..HASH_LEN].copy_from_slice(&d);
    } else {
        k[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(msg);
    let inner = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner);
    let mut out = [0u8; HASH_LEN];
    out.copy_from_slice(&outer.finalize());
    out
}

/// PBKDF2-HMAC-SHA256 (RFC 8018 §5.2), producing `out.len()` bytes.
pub fn pbkdf2_sha256(password: &[u8], salt: &[u8], iterations: u32, out: &mut [u8]) {
    debug_assert!(iterations > 0, "zero iterations is not a KDF");
    let mut block_index: u32 = 1;
    let mut written = 0usize;
    while written < out.len() {
        // U1 = PRF(P, S || INT_BE32(i))
        let mut salted = Vec::with_capacity(salt.len() + 4);
        salted.extend_from_slice(salt);
        salted.extend_from_slice(&block_index.to_be_bytes());
        let mut u = hmac_sha256(password, &salted);
        let mut t = u;
        // T_i = U1 xor U2 xor ... xor Uc
        for _ in 1..iterations {
            u = hmac_sha256(password, &u);
            for (a, b) in t.iter_mut().zip(u.iter()) {
                *a ^= *b;
            }
        }
        let n = core::cmp::min(HASH_LEN, out.len() - written);
        out[written..written + n].copy_from_slice(&t[..n]);
        written += n;
        block_index += 1;
    }
}

/// Compare without leaking the position of the first difference.
///
/// Every comparison of a MAC, a token or a password hash goes through this. A
/// plain `==` short-circuits, which over enough requests reveals the secret one
/// byte at a time.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Random bytes from the operating system's CSPRNG.
///
/// Reads `/dev/urandom` directly rather than through an RNG crate, for the same
/// allowlist reason as above — and this is the source those crates read anyway.
/// It RETURNS AN ERROR rather than falling back to a clock or a counter: a salt
/// or a session id that is predictable is worse than no session at all, and a
/// silent fallback is exactly how that ships unnoticed.
pub fn random_bytes(n: usize) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut f = std::fs::File::open("/dev/urandom")?;
    let mut buf = vec![0u8; n];
    f.read_exact(&mut buf)?;
    Ok(buf)
}

const B64URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// base64url, no padding (RFC 4648 §5). Used for token parts, which travel in
/// an HTTP header and so must survive without `+`, `/` or `=`.
pub fn b64url_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        out.push(B64URL[(n >> 18 & 63) as usize] as char);
        out.push(B64URL[(n >> 12 & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64URL[(n >> 6 & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(B64URL[(n & 63) as usize] as char);
        }
    }
    out
}

pub fn b64url_decode(s: &str) -> Option<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            // Padding is not produced by the encoder above, and accepting it
            // would make two spellings of one token both verify.
            _ => return None,
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    // Leftover bits must be zero, or the input was not produced by a correct
    // encoder and two distinct strings could decode to the same bytes.
    if bits > 0 && (acc & ((1 << bits) - 1)) != 0 {
        return None;
    }
    Some(out)
}

/// Lowercase hex, for storing a digest as text.
pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap_or('0'));
        s.push(char::from_digit((b & 15) as u32, 16).unwrap_or('0'));
    }
    s
}

pub fn unhex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    let b = s.as_bytes();
    (0..s.len() / 2)
        .map(|i| {
            let hi = (b[2 * i] as char).to_digit(16)?;
            let lo = (b[2 * i + 1] as char).to_digit(16)?;
            Some((hi * 16 + lo) as u8)
        })
        .collect()
}

#[cfg(test)]
mod tests {
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
}
