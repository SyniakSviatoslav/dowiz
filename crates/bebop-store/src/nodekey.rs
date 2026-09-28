//! The node key: one byte frame, two hashes (SPEC-BEBOP-DAG-RUNTIME-2026-09-28 §2).
//!
//! ```text
//! frame := tag(1 byte) ‖ field*            (fields in the fixed order of RT §2.3)
//! field := len(8 bytes, u64 LE) ‖ bytes(len)
//! ```
//! A number is an 8-byte little-endian field (`len = 8`); a digest is a field of
//! its own length (8 or 32); a LIST is one field whose bytes are its elements'
//! fields, so lengths nest and no count is needed. No separator, no padding.
//!
//! `K64 = (crc32(frame) << 32) | (frame_len & 0xffffffff)` is the in-process
//! INDEX (RT K-1: never a proof). The crc is zlib's, the same function as bebop's
//! `crc32x`/`crc32` builtins (`crate::crc32`), and the same `k64` as
//! `dowiz_hub::block::schema::k64` -- one bit pattern, read here as an i64.
//! `K256 = sha256(frame)` is the cross-machine name (RT §2.2). SHA-256 is written
//! out below because this crate has zero dependencies and must stay so (it is
//! also the wasm32 module's only dependency, `crates/bebop-wasm/bytes.baseline`).
//!
//! Readers of the same frames: `bebop-lang/selfhost/prelude/nodekey.bp`,
//! `crates/bebop-wasm/src/nodekey.rs` (its own frame builder, run as wasm32),
//! `crates/bebop-wasm/oracle.py --key`; `crates/bebop-wasm/gate.sh` holds all four
//! to `fixtures/key.expected`.

/// Node-kind tags (RT §2.3).
pub const TAG_COMPILE: u8 = b'C';
pub const TAG_OBJECT: u8 = b'O';
pub const TAG_PROJECTION: u8 = b'P';
pub const TAG_EFFECT: u8 = b'E';

/// A key frame under construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame(Vec<u8>);

impl Frame {
    pub fn new(tag: u8) -> Frame {
        Frame(vec![tag])
    }

    /// One length-prefixed field; `b` may be empty.
    pub fn bytes(&mut self, b: &[u8]) -> &mut Frame {
        self.0.extend_from_slice(&(b.len() as u64).to_le_bytes());
        self.0.extend_from_slice(b);
        self
    }

    /// A number: an 8-byte little-endian field.
    pub fn i64(&mut self, v: i64) -> &mut Frame {
        self.bytes(&v.to_le_bytes())
    }

    /// Open a list field: reserve its length, return where it sits.
    pub fn open(&mut self) -> usize {
        let at = self.0.len();
        self.0.extend_from_slice(&[0; 8]);
        at
    }

    /// Close the list opened at `at`: its length is everything written since.
    pub fn close(&mut self, at: usize) -> &mut Frame {
        let n = (self.0.len() - at - 8) as u64;
        self.0[at..at + 8].copy_from_slice(&n.to_le_bytes());
        self
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        false // a frame always holds its tag
    }

    pub fn k64(&self) -> i64 {
        k64(&self.0)
    }

    pub fn k256(&self) -> [u8; 32] {
        sha256(&self.0)
    }
}

/// `K64 = (crc32(frame) << 32) | (len & 0xffffffff)`, as the i64 a bebop cell holds.
pub fn k64(frame: &[u8]) -> i64 {
    (((crate::crc32(frame) as u64) << 32) | (frame.len() as u64 & 0xffff_ffff)) as i64
}

/// `K256` as lowercase hex, the form `key.expected` and every reader prints.
pub fn hex(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// SHA-256 (FIPS 180-4), scalar, std only.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64).wrapping_mul(8)).to_be_bytes());
    let mut w = [0u32; 64];
    for block in msg.chunks_exact(64) {
        for (i, c) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(y);
        }
    }
    let mut out = [0u8; 32];
    for (i, x) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&x.to_be_bytes());
    }
    out
}

/// The three frames of `crates/bebop-wasm/fixtures/key.expected`, built from the
/// field values that file lists. Public so `tests/parity.rs` asks THIS reader.
pub mod fixtures {
    use super::*;

    pub const COMPILER_DIGEST: i64 = -7046029254386353131;
    pub const FN_SOURCE: &[u8] = b"fn add(a: i64, b: i64) -> i64 { a + b }";

    /// A compile node: compiler_digest, fn_source, sig, lit, use (RT §2.3).
    pub fn compile() -> Frame {
        let mut f = Frame::new(TAG_COMPILE);
        f.i64(COMPILER_DIGEST).bytes(FN_SOURCE).i64(81985529216486895).i64(-2).i64(1311768467294899695);
        f
    }

    /// A projection with two inputs (an 8-byte key and a 32-byte key) and two params.
    pub fn proj() -> Frame {
        let digest: Vec<u8> = (0u8..32).collect();
        let mut f = Frame::new(TAG_PROJECTION);
        f.i64(COMPILER_DIGEST).i64(compile().k64());
        let inputs = f.open();
        f.i64(1234605616436508552).i64(7).bytes(&digest).i64(1).close(inputs);
        let params = f.open();
        f.i64(1790000000000).i64(3).close(params);
        f
    }

    /// A projection whose `inputs` and `params` are empty lists: two len-0 fields.
    pub fn empty() -> Frame {
        let mut f = Frame::new(TAG_PROJECTION);
        f.i64(COMPILER_DIGEST).i64(compile().k64());
        let inputs = f.open();
        f.close(inputs);
        let params = f.open();
        f.close(params);
        f
    }

    /// By the name `key.expected` and `gate.sh` use.
    pub fn named(name: &str) -> Option<Frame> {
        match name {
            "compile" => Some(compile()),
            "proj" => Some(proj()),
            "empty" => Some(empty()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
