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
mod tests;
