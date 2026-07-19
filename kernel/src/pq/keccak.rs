//! Zero-dependency Keccak-f[1600] + SHAKE128/256 (FIPS 202).
//!
//! Inlined on purpose (no `sha2`/`tiny-keccak`/`pqc` crates allowed). Every other
//! module in this directory depends on `shake128`/`shake256`; this file is the one
//! and only digest primitive. A FIPS 202 KAT is included so a single bit error in
//! the sponge is caught before any scheme math touches it.

// ── Keccak-f[1600] ────────────────────────────────────────────────────────────
// State = 25 lanes of u64. Round constants (RC) and rotation offsets (RHO) per
// FIPS 202 §3.2.1 / §3.2.2.

const RC: [u64; 24] = [
    0x0000000000000001,
    0x0000000000008082,
    0x800000000000808a,
    0x8000000080008000,
    0x000000000000808b,
    0x0000000080000001,
    0x8000000080008081,
    0x8000000000008009,
    0x000000000000008a,
    0x0000000000000088,
    0x0000000080008009,
    0x000000008000000a,
    0x000000008000808b,
    0x800000000000008b,
    0x8000000000008089,
    0x8000000000008003,
    0x8000000000008002,
    0x8000000000000080,
    0x000000000000800a,
    0x800000008000000a,
    0x8000000080008081,
    0x8000000000008080,
    0x0000000080000001,
    0x8000000080008008,
];

// Rotation offsets r[x][y] (FIPS 202 §3.2.2), flat-indexed by x + 5*y.
const RHO: [u32; 25] = [
    0, 1, 62, 28, 27, 36, 44, 6, 55, 20, 3, 10, 43, 25, 39, 41, 45, 15, 21, 8, 18, 2, 61, 56, 14,
];

#[inline]
fn rotl(x: u64, n: u32) -> u64 {
    if n == 0 {
        x
    } else {
        (x << n) | (x >> (64 - n))
    }
}

/// One KECCAK-f[1600] permutation in place.
fn keccak_f(state: &mut [u64; 25]) {
    for round in 0..24 {
        // Theta
        let mut c = [0u64; 5];
        for x in 0..5 {
            c[x] = state[x] ^ state[x + 5] ^ state[x + 10] ^ state[x + 15] ^ state[x + 20];
        }
        let mut d = [0u64; 5];
        for x in 0..5 {
            d[x] = c[(x + 4) % 5] ^ rotl(c[(x + 1) % 5], 1);
        }
        for x in 0..5 {
            for y in 0..5 {
                state[x + 5 * y] ^= d[x];
            }
        }
        // Rho + Pi (matches the canonical Keccak reference C): the rotated lane
        // A[col=x][row=y] lands at B[col=y][row=(2x+3y)%5], i.e. index
        // `y + 5*((2*x+3*y)%5)`.
        let mut b = [0u64; 25];
        for x in 0..5 {
            for y in 0..5 {
                b[y + 5 * ((2 * x + 3 * y) % 5)] = rotl(state[x + 5 * y], RHO[x + 5 * y]);
            }
        }
        // Chi
        for y in 0..5 {
            let row = &b[5 * y..5 * y + 5];
            for x in 0..5 {
                state[x + 5 * y] = row[x] ^ ((!row[(x + 1) % 5]) & row[(x + 2) % 5]);
            }
        }
        // Iota
        state[0] ^= RC[round];
    }
}

/// Sponge over Keccak-f[1600]. `rate` is the block size in bytes (168 for SHAKE128,
/// 136 for SHAKE256). `pad` is the domain suffix byte (0x1f for SHAKE, 0x06 for SHA-3).
/// Squeezes exactly `out.len()` bytes.
fn sponge(rate: usize, pad: u8, input: &[u8], out: &mut [u8]) {
    let mut state = [0u64; 25];
    // pad10*1 with the SHAKE/SHA-3 domain suffix. The suffix byte (0x1f for SHAKE,
    // 0x06 for SHA-3) already contains the pad10*1 leading `1` as its LSB; we then
    // zero-fill to the last byte of the rate block and set the trailing `1` (0x80).
    let mut data = input.to_vec();
    data.push(pad);
    while data.len() % rate != rate - 1 {
        data.push(0x00);
    }
    data.push(0x80);
    // Absorb (rate is a multiple of 8, so lane-aligned).
    for chunk in data.chunks(rate) {
        for (j, byte) in chunk.iter().enumerate() {
            let lane = j / 8;
            let shift = (j % 8) * 8;
            state[lane] ^= u64::from(*byte) << shift;
        }
        keccak_f(&mut state);
    }
    // Squeeze
    let mut produced = 0;
    while produced < out.len() {
        for lane in 0..rate / 8 {
            let bytes = state[lane].to_le_bytes();
            for (k, b) in bytes.iter().enumerate() {
                let idx = produced + lane * 8 + k;
                if idx < out.len() {
                    out[idx] = *b;
                }
            }
        }
        produced += rate;
        if produced < out.len() {
            keccak_f(&mut state);
        }
    }
}

/// SHAKE128 (rate 168, pad 0x1f). Squeezes `out.len()` bytes.
pub fn shake128(input: &[u8], out: &mut [u8]) {
    sponge(168, 0x1f, input, out);
}

/// SHAKE256 (rate 136, pad 0x1f). Squeezes `out.len()` bytes.
pub fn shake256(input: &[u8], out: &mut [u8]) {
    sponge(136, 0x1f, input, out);
}

/// SHAKE256 XOF: absorbs `seed || i || j`, squeezes exactly `len` bytes.
/// Used by ML-KEM uniform sampling (FIPS 203 §2 / Algorithm 10).
pub fn shake256_xof(seed: &[u8; 32], i: u8, j: u8, len: usize) -> Vec<u8> {
    let mut input = Vec::with_capacity(34);
    input.extend_from_slice(seed);
    input.push(i);
    input.push(j);
    let mut out = vec![0u8; len];
    sponge(136, 0x1f, &input, &mut out);
    out
}

/// SHA3-256 (rate 136, pad 0x06). Squeezes 32 bytes.
pub fn sha3_256(input: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    sponge(136, 0x06, input, &mut out);
    out
}

/// SHA3-512 (rate 72, pad 0x06). Squeezes 64 bytes.
pub fn sha3_512(input: &[u8]) -> [u8; 64] {
    let mut out = [0u8; 64];
    sponge(72, 0x06, input, &mut out);
    out
}

/// G(X) = SHAKE256(X, 64) (FIPS 203 §2).
pub fn xof_g(input: &[u8]) -> [u8; 64] {
    let mut out = [0u8; 64];
    sponge(136, 0x1f, input, &mut out);
    out
}

/// H(X) = SHA3-256(X) (FIPS 203 §2).
pub fn xof_h(input: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    sponge(136, 0x06, input, &mut out);
    out
}

/// J(z, c) = SHAKE256(z || c, 32) — placeholder for future signing schemes.
pub fn xof_j(z: &[u8], c: &[u8]) -> [u8; 32] {
    let mut input = Vec::with_capacity(z.len() + c.len());
    input.extend_from_slice(z);
    input.extend_from_slice(c);
    let mut out = [0u8; 32];
    sponge(136, 0x1f, &input, &mut out);
    out
}

/// PRF(s, b, len) = SHAKE256(s || b, len) (FIPS 203 §2, seed-expansion).
pub fn prf(s: &[u8; 32], b: u8, len: usize) -> Vec<u8> {
    let mut input = Vec::with_capacity(33);
    input.extend_from_slice(s);
    input.push(b);
    let mut out = vec![0u8; len];
    sponge(136, 0x1f, &input, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Parse a hex string literal into a fixed array (test helper).
    fn hex<const L: usize>(s: &str) -> [u8; L] {
        let s = s.trim();
        assert_eq!(s.len(), L * 2, "hex length mismatch");
        let mut out = [0u8; L];
        let bytes = s.as_bytes();
        for i in 0..L {
            let hi = (bytes[2 * i] as char).to_digit(16).unwrap();
            let lo = (bytes[2 * i + 1] as char).to_digit(16).unwrap();
            out[i] = ((hi << 4) | lo) as u8;
        }
        out
    }

    // FIPS 202 known-answer tests (empty-string anchors are unambiguous).
    // SHAKE256("") = 46b9dd2b0ba88d13233b3feb743eeb243fcd52ea62b81b82b50c27646ed5762f
    #[test]
    fn kat_shake256_empty() {
        let mut out = [0u8; 32];
        shake256(&[], &mut out);
        assert_eq!(
            out,
            [
                0x46, 0xb9, 0xdd, 0x2b, 0x0b, 0xa8, 0x8d, 0x13, 0x23, 0x3b, 0x3f, 0xeb, 0x74, 0x3e,
                0xeb, 0x24, 0x3f, 0xcd, 0x52, 0xea, 0x62, 0xb8, 0x1b, 0x82, 0xb5, 0x0c, 0x27, 0x64,
                0x6e, 0xd5, 0x76, 0x2f
            ]
        );
    }

    // SHAKE128("") first 16 bytes = 7f9c2ba4e88f827d616045507605853e
    #[test]
    fn kat_shake128_empty() {
        let mut out = [0u8; 16];
        shake128(&[], &mut out);
        assert_eq!(
            out,
            [
                0x7f, 0x9c, 0x2b, 0xa4, 0xe8, 0x8f, 0x82, 0x7d, 0x61, 0x60, 0x45, 0x50, 0x76, 0x05,
                0x85, 0x3e
            ]
        );
    }

    // SHAKE256("abc") = 483366601360a8771c6863080cc4114d8db44530f8f1e1ee4f94ea37e78b5739
    #[test]
    fn kat_shake256_abc() {
        let mut out = [0u8; 32];
        shake256(b"abc", &mut out);
        assert_eq!(
            out,
            [
                0x48, 0x33, 0x66, 0x60, 0x13, 0x60, 0xa8, 0x77, 0x1c, 0x68, 0x63, 0x08, 0x0c, 0xc4,
                0x11, 0x4d, 0x8d, 0xb4, 0x45, 0x30, 0xf8, 0xf1, 0xe1, 0xee, 0x4f, 0x94, 0xea, 0x37,
                0xe7, 0x8b, 0x57, 0x39
            ]
        );
    }

    // SHA3 KAT (FIPS 202) — anchors the newly-added SHA3-256/512; the old KEM
    // only used SHAKE, so a SHA3 bug would have been invisible until now.
    #[test]
    fn kem_debug_sha3_kat() {
        let s3_256_empty = sha3_256(&[]);
        assert_eq!(
            s3_256_empty,
            hex::<32>("a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a")
        );
        let s3_512_empty = sha3_512(&[]);
        assert_eq!(
            s3_512_empty,
            hex::<64>("a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26")
        );
    }
}
