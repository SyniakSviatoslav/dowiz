//! AES-256-GCM (NIST SP 800-38D), hand-rolled, `no_std`, zero external crates.
//!
//! The last external crypto crate retired from the pq subsystem: AES-256 block
//! cipher (14 rounds, table-based S-box) in CTR mode + GHASH authentication in
//! GF(2^128). Only the AES-*encrypt* direction is needed (GCM runs CTR with an
//! encrypt keystream in both seal and open), so there is no inverse S-box.
//!
//! Correctness is KAT-gated against the AES-256-GCM test vector from the GCM
//! specification (McGrew–Viega), and differential-tested against the RustCrypto
//! `aes-gcm` crate in the kernel (`kernel/src/pq/aes_gcm.rs`) on a deterministic
//! vector set before that dependency is retired from the `pq` feature.
//!
//! innovate: the S-box is a table lookup, so AES round state (which mixes the key)
//! indexes the table — this is NOT cache-timing-hardened against a co-located
//! attacker. The consumers here are at-rest/backup/transfer envelopes, not a
//! high-frequency network-exposed path; upgrade trigger is an AES-NI path
//! (`core::arch::aarch64::{vaeseq_u8, vaesmcq_u8}`) with a scalar fallback for
//! x86_64/wasm. GHASH iterates the *public* message bits, so it introduces no
//! secret-dependent branch beyond the AES table lookups.

use alloc::vec::Vec;
use core::cmp;

// ─────────────────────────────────────────────────────────────────────────────
// AES-256
// ─────────────────────────────────────────────────────────────────────────────

/// The AES S-box (FIPS-197 §5.1.1). Row-major: `SBOX[hi][lo]` = `SBOX[(hi<<4)|lo]`.
#[rustfmt::skip]
const SBOX: [u8; 256] = [
    0x63,0x7c,0x77,0x7b,0xf2,0x6b,0x6f,0xc5,0x30,0x01,0x67,0x2b,0xfe,0xd7,0xab,0x76,
    0xca,0x82,0xc9,0x7d,0xfa,0x59,0x47,0xf0,0xad,0xd4,0xa2,0xaf,0x9c,0xa4,0x72,0xc0,
    0xb7,0xfd,0x93,0x26,0x36,0x3f,0xf7,0xcc,0x34,0xa5,0xe5,0xf1,0x71,0xd8,0x31,0x15,
    0x04,0xc7,0x23,0xc3,0x18,0x96,0x05,0x9a,0x07,0x12,0x80,0xe2,0xeb,0x27,0xb2,0x75,
    0x09,0x83,0x2c,0x1a,0x1b,0x6e,0x5a,0xa0,0x52,0x3b,0xd6,0xb3,0x29,0xe3,0x2f,0x84,
    0x53,0xd1,0x00,0xed,0x20,0xfc,0xb1,0x5b,0x6a,0xcb,0xbe,0x39,0x4a,0x4c,0x58,0xcf,
    0xd0,0xef,0xaa,0xfb,0x43,0x4d,0x33,0x85,0x45,0xf9,0x02,0x7f,0x50,0x3c,0x9f,0xa8,
    0x51,0xa3,0x40,0x8f,0x92,0x9d,0x38,0xf5,0xbc,0xb6,0xda,0x21,0x10,0xff,0xf3,0xd2,
    0xcd,0x0c,0x13,0xec,0x5f,0x97,0x44,0x17,0xc4,0xa7,0x7e,0x3d,0x64,0x5d,0x19,0x73,
    0x60,0x81,0x4f,0xdc,0x22,0x2a,0x90,0x88,0x46,0xee,0xb8,0x14,0xde,0x5e,0x0b,0xdb,
    0xe0,0x32,0x3a,0x0a,0x49,0x06,0x24,0x5c,0xc2,0xd3,0xac,0x62,0x91,0x95,0xe4,0x79,
    0xe7,0xc8,0x37,0x6d,0x8d,0xd5,0x4e,0xa9,0x6c,0x56,0xf4,0xea,0x65,0x7a,0xae,0x08,
    0xba,0x78,0x25,0x2e,0x1c,0xa6,0xb4,0xc6,0xe8,0xdd,0x74,0x1f,0x4b,0xbd,0x8b,0x8a,
    0x70,0x3e,0xb5,0x66,0x48,0x03,0xf6,0x0e,0x61,0x35,0x57,0xb9,0x86,0xc1,0x1d,0x9e,
    0xe1,0xf8,0x98,0x11,0x69,0xd9,0x8e,0x94,0x9b,0x1e,0x87,0xe9,0xce,0x55,0x28,0xdf,
    0x8c,0xa1,0x89,0x0d,0xbf,0xe6,0x42,0x68,0x41,0x99,0x2d,0x0f,0xb0,0x54,0xbb,0x16,
];

/// Round constants `Rcon[i]` for `i = 1..=7` (AES-256 has 14 rounds).
const RCON: [u8; 7] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40];

fn sub_word(w: u32) -> u32 {
    let b0 = SBOX[(w >> 24) as usize] as u32;
    let b1 = SBOX[((w >> 16) & 0xff) as usize] as u32;
    let b2 = SBOX[((w >> 8) & 0xff) as usize] as u32;
    let b3 = SBOX[(w & 0xff) as usize] as u32;
    (b0 << 24) | (b1 << 16) | (b2 << 8) | b3
}

/// AES-256 key expansion: 8-word key → 60 round-key words (15 × 4).
fn key_expand(key: &[u8; 32]) -> [u32; 60] {
    let mut w = [0u32; 60];
    for i in 0..8 {
        w[i] = u32::from_be_bytes([
            key[4 * i],
            key[4 * i + 1],
            key[4 * i + 2],
            key[4 * i + 3],
        ]);
    }
    for i in 8..60 {
        let mut t = w[i - 1];
        if i % 8 == 0 {
            t = sub_word(t.rotate_left(8)) ^ ((RCON[i / 8 - 1] as u32) << 24);
        } else if i % 8 == 4 {
            t = sub_word(t);
        }
        w[i] = w[i - 8] ^ t;
    }
    w
}

fn sub_bytes(s: &mut [u8; 16]) {
    for b in s.iter_mut() {
        *b = SBOX[*b as usize];
    }
}

/// ShiftRows (column-major state `s[col*4 + row]`): row `r` rotated left by `r`.
fn shift_rows(s: &mut [u8; 16]) {
    // row 1
    let t = s[1];
    s[1] = s[5];
    s[5] = s[9];
    s[9] = s[13];
    s[13] = t;
    // row 2
    let (t0, t1) = (s[2], s[6]);
    s[2] = s[10];
    s[6] = s[14];
    s[10] = t0;
    s[14] = t1;
    // row 3 (shift left by 3 == shift right by 1)
    let t = s[3];
    s[3] = s[15];
    s[15] = s[11];
    s[11] = s[7];
    s[7] = t;
}

/// Multiply by 2 in GF(2^8) with reduction polynomial x^8+x^4+x^3+x+1 (0x11b).
fn xtime(x: u8) -> u8 {
    let y = x << 1;
    if x & 0x80 != 0 {
        y ^ 0x1b
    } else {
        y
    }
}

/// MixColumns (FIPS-197 §5.1.3), column-major.
fn mix_columns(s: &mut [u8; 16]) {
    for c in 0..4 {
        let i = c * 4;
        let a0 = s[i];
        let a1 = s[i + 1];
        let a2 = s[i + 2];
        let a3 = s[i + 3];
        let t = a0 ^ a1 ^ a2 ^ a3;
        s[i] = a0 ^ t ^ xtime(a0 ^ a1);
        s[i + 1] = a1 ^ t ^ xtime(a1 ^ a2);
        s[i + 2] = a2 ^ t ^ xtime(a2 ^ a3);
        s[i + 3] = a3 ^ t ^ xtime(a3 ^ a0);
    }
}

fn add_round_key(s: &mut [u8; 16], rk: &[u32; 4]) {
    for c in 0..4 {
        let w = rk[c].to_be_bytes();
        for r in 0..4 {
            s[c * 4 + r] ^= w[r];
        }
    }
}

/// AES-256 encrypt one 16-byte block (column-major input/output).
fn aes256_encrypt_block(rk: &[u32; 60], input: &[u8; 16]) -> [u8; 16] {
    let mut s = *input;
    add_round_key(&mut s, &[rk[0], rk[1], rk[2], rk[3]]);
    for round in 1..14 {
        sub_bytes(&mut s);
        shift_rows(&mut s);
        mix_columns(&mut s);
        let off = round * 4;
        add_round_key(&mut s, &[rk[off], rk[off + 1], rk[off + 2], rk[off + 3]]);
    }
    sub_bytes(&mut s);
    shift_rows(&mut s);
    add_round_key(&mut s, &[rk[56], rk[57], rk[58], rk[59]]);
    s
}

// ─────────────────────────────────────────────────────────────────────────────
// GF(2^128) / GHASH
// ─────────────────────────────────────────────────────────────────────────────

/// Reduction polynomial `x^128 + x^7 + x^2 + x + 1` (R = 0xe1 << 120).
const R: u128 = 0xe100_0000_0000_0000_0000_0000_0000_0000;

/// GF(2^128) multiplication (GCM bit order: blocks are big-endian u128, bit 127 =
/// x^127). Matches NIST SP 800-38D §6.3: test the multiplier bits MSB-first,
/// right-shift the multiplicand, reducing with R when a 1 shifts out the bottom.
fn gf_mul(mut x: u128, y: u128) -> u128 {
    let mut z = 0u128;
    let mut v = y;
    for _ in 0..128 {
        if x >> 127 == 1 {
            z ^= v;
        }
        x <<= 1;
        let lsb = v & 1;
        v >>= 1;
        if lsb == 1 {
            v ^= R;
        }
    }
    z
}

/// Increment the low 32 bits of the GCM counter block (SP 800-38D, 96-bit IV case).
fn inc32(counter: &mut [u8; 16]) {
    let mut n = u32::from_be_bytes([counter[12], counter[13], counter[14], counter[15]]);
    n = n.wrapping_add(1);
    let b = n.to_be_bytes();
    counter[12..16].copy_from_slice(&b);
}

// ─────────────────────────────────────────────────────────────────────────────
// AES-256-GCM AEAD
// ─────────────────────────────────────────────────────────────────────────────

/// AEAD failure (key-length misuse or authentication failure on open).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AeadError {
    /// Key was not exactly 32 bytes.
    InvalidKey,
    /// The GCM authentication tag did not verify (tamper / wrong key / wrong nonce).
    AuthFailed,
}

impl core::fmt::Display for AeadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AeadError::InvalidKey => write!(f, "AES-256-GCM key must be exactly 32 bytes"),
            AeadError::AuthFailed => write!(f, "GCM authentication failed"),
        }
    }
}

/// A keyed AES-256-GCM instance: expanded round keys + the GHASH subkey `H`.
pub struct Aes256Gcm {
    rk: [u32; 60],
    h: u128,
}

impl Aes256Gcm {
    /// Key a fresh instance from exactly 32 key bytes.
    pub fn new(key: &[u8; 32]) -> Self {
        let rk = key_expand(key);
        let h_block = aes256_encrypt_block(&rk, &[0u8; 16]);
        let h = u128::from_be_bytes(h_block);
        Aes256Gcm { rk, h }
    }

    /// Key from a slice; rejects any length other than 32.
    pub fn new_from_slice(key: &[u8]) -> Result<Self, AeadError> {
        if key.len() != 32 {
            return Err(AeadError::InvalidKey);
        }
        let mut k = [0u8; 32];
        k.copy_from_slice(key);
        Ok(Self::new(&k))
    }

    /// Encrypt + authenticate (empty AAD): returns `ciphertext || 16-byte tag`.
    pub fn encrypt(&self, nonce: &[u8; 12], plaintext: &[u8]) -> Vec<u8> {
        // J0 = nonce || 0x00000001 (the pre-counter block for the tag).
        let mut j0 = [0u8; 16];
        j0[..12].copy_from_slice(nonce);
        j0[15] = 0x01;

        // CTR keystream starts at inc32(J0).
        let mut counter = j0;
        inc32(&mut counter);

        let mut out = Vec::with_capacity(plaintext.len() + 16);
        let mut i = 0usize;
        while i < plaintext.len() {
            let ks = aes256_encrypt_block(&self.rk, &counter);
            let n = cmp::min(16, plaintext.len() - i);
            for j in 0..n {
                out.push(plaintext[i + j] ^ ks[j]);
            }
            inc32(&mut counter);
            i += n;
        }

        // Tag = GHASH(C) XOR E_K(J0). (AAD is empty.)
        let s = self.ghash(&out);
        let ek_j0 = aes256_encrypt_block(&self.rk, &j0);
        for j in 0..16 {
            out.push(s[j] ^ ek_j0[j]);
        }
        out
    }

    /// Decrypt + verify (empty AAD): returns plaintext, or [`AeadError::AuthFailed`].
    pub fn decrypt(&self, nonce: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>, AeadError> {
        if ciphertext.len() < 16 {
            return Err(AeadError::AuthFailed);
        }
        let (ct, tag) = ciphertext.split_at(ciphertext.len() - 16);

        // Recompute the tag over the ciphertext.
        let mut j0 = [0u8; 16];
        j0[..12].copy_from_slice(nonce);
        j0[15] = 0x01;
        let s = self.ghash(ct);
        let ek_j0 = aes256_encrypt_block(&self.rk, &j0);
        let mut expected = [0u8; 16];
        for j in 0..16 {
            expected[j] = s[j] ^ ek_j0[j];
        }
        // Constant-time tag compare (public data, but cheap insurance against
        // length/early-exit timing).
        let mut diff = 0u8;
        for j in 0..16 {
            diff |= expected[j] ^ tag[j];
        }
        if diff != 0 {
            return Err(AeadError::AuthFailed);
        }

        // CTR keystream.
        let mut counter = j0;
        inc32(&mut counter);
        let mut out = Vec::with_capacity(ct.len());
        let mut i = 0usize;
        while i < ct.len() {
            let ks = aes256_encrypt_block(&self.rk, &counter);
            let n = cmp::min(16, ct.len() - i);
            for j in 0..n {
                out.push(ct[i + j] ^ ks[j]);
            }
            inc32(&mut counter);
            i += n;
        }
        Ok(out)
    }

    /// GHASH over `ciphertext` (empty AAD), including the 16-byte length block.
    fn ghash(&self, ciphertext: &[u8]) -> [u8; 16] {
        let mut y = 0u128;
        let mut i = 0usize;
        let mut rem = ciphertext.len();
        while rem >= 16 {
            let mut block = [0u8; 16];
            block.copy_from_slice(&ciphertext[i..i + 16]);
            y = gf_mul(y ^ u128::from_be_bytes(block), self.h);
            i += 16;
            rem -= 16;
        }
        if rem > 0 {
            let mut block = [0u8; 16];
            block[..rem].copy_from_slice(&ciphertext[i..]);
            y = gf_mul(y ^ u128::from_be_bytes(block), self.h);
        }
        // Length block: [len(AAD)=0]_64 || [len(C) in bits]_64.
        let mut len_block = [0u8; 16];
        len_block[8..16].copy_from_slice(&((ciphertext.len() as u64) * 8).to_be_bytes());
        y = gf_mul(y ^ u128::from_be_bytes(len_block), self.h);
        y.to_be_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(s.len() % 2 == 0, "hex string must be even length");
        (0..s.len() / 2)
            .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap())
            .collect()
    }

    /// The AES-256-GCM test vector from the GCM specification (McGrew–Viega,
    /// "The Galois/Counter Mode of Operation", Test Case 3), empty AAD.
    #[test]
    fn kat_gcm_spec_tc3() {
        let key: [u8; 32] = hex("feffe9928665731c6d6a8f9467308308 feffe9928665731c6d6a8f9467308308")
            .try_into()
            .unwrap();
        let nonce: [u8; 12] = hex("cafebabefacedbaddecaf888").try_into().unwrap();
        let pt = hex(
            "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72\
             1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b391aafd255",
        );
        let ct_expected = hex(
            "522dc1f099567d07f47f37a32a84427d643a8cdcbfe5c0c97598a2bd2555d1aa\
             8cb08e48590dbb3da7b08b1056828838c5f61e6393ba7a0abcc9f662898015ad",
        );
        let tag_expected = hex("b094dac5d93471bdec1a502270e3cc6c");

        let cipher = Aes256Gcm::new(&key);
        let sealed = cipher.encrypt(&nonce, &pt);
        assert_eq!(sealed.len(), pt.len() + 16);
        assert_eq!(&sealed[..pt.len()], &ct_expected[..], "ciphertext mismatch");
        assert_eq!(&sealed[pt.len()..], &tag_expected[..], "tag mismatch");

        // Round-trip.
        let opened = cipher.decrypt(&nonce, &sealed).expect("decrypt must succeed");
        assert_eq!(opened, pt);
    }

    /// Tampering any ciphertext byte must fail authentication.
    #[test]
    fn red_tamper_rejected() {
        let key = [7u8; 32];
        let nonce = [3u8; 12];
        let pt = b"the quick brown fox jumps over the lazy dog".to_vec();
        let cipher = Aes256Gcm::new(&key);
        let mut sealed = cipher.encrypt(&nonce, &pt);
        sealed[0] ^= 0x80;
        assert!(cipher.decrypt(&nonce, &sealed).is_err());
    }

    /// Empty plaintext still produces a valid 16-byte tag.
    #[test]
    fn green_empty_plaintext() {
        let cipher = Aes256Gcm::new(&[1u8; 32]);
        let sealed = cipher.encrypt(&[2u8; 12], b"");
        assert_eq!(sealed.len(), 16);
        assert_eq!(cipher.decrypt(&[2u8; 12], &sealed).unwrap(), b"");
    }
}
