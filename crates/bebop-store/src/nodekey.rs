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

/// SHA-256 (FIPS 180-4): allocation-free, on the CPU's SHA instructions when present (P16,
/// `sha_fast.rs`).
pub fn sha256(data: &[u8]) -> [u8; 32] {
    crate::sha_fast::sha256(data)
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
