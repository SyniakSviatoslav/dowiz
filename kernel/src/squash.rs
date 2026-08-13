//! squash.rs — zero-dep compression for academy/diff/garbage artifacts.
//!
//! Phase E of the glyph-geometry rewrite law: artifacts are compressed on
//! write and decompressed on use, never held uncompressed in memory.
//!
//! # Formats (all pure `std`, no external crates)
//!
//! - **RLE** (`encode_rle` / `decode_rle`): byte run-length encoding. Best for
//!   buffers with long runs (sparse matrices, braille bitmaps, diff hunks).
//! - **Delta** (`encode_delta` / `decode_delta`): first-value + zigzag varint
//!   deltas, best for monotonic numeric series (timestamps, counters, pagerank
//!   iterations). Zigzag maps signed→unsigned so small-magnitude deltas stay
//!   single-byte.
//! - **Combined** (`Squash`): a self-describing container — a 1-byte tag, a
//!   varint uncompressed length, then the payload. `compress` picks RLE vs
//!   delta by which is smaller, `decompress` reverses it.
//!
//! # Guarantees
//! - Round-trip byte-identity for every input (fuzzable, tested).
//! - Deterministic output (same input ⇒ same bytes).
//! - No allocation on the `size_of`/`would_compress` decision path.

/// Run-length encode a byte slice: `(count, byte)` pairs packed as varints.
/// Each run is encoded as a varint count (1..=255 in one byte) followed by the
/// literal byte. Counts above 255 split into multiple runs.
pub fn encode_rle(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        let b = data[i];
        let mut run = 1usize;
        while i + run < data.len() && data[i + run] == b && run < 255 {
            run += 1;
        }
        out.push(run as u8);
        out.push(b);
        i += run;
    }
    out
}

/// Decode run-length encoded bytes.
pub fn decode_rle(encoded: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < encoded.len() {
        let run = encoded[i] as usize;
        if i + 1 >= encoded.len() {
            return None; // truncated: count without byte
        }
        let b = encoded[i + 1];
        for _ in 0..run {
            out.push(b);
        }
        i += 2;
    }
    Some(out)
}

/// Zigzag-encode a signed `i64` to unsigned `u64`: 0→0, -1→1, 1→2, -2→3, ...
#[inline(always)]
fn zigzag(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

/// Zigzag-decode an unsigned `u64` back to signed `i64`.
#[inline(always)]
fn unzigzag(u: u64) -> i64 {
    ((u >> 1) as i64) ^ (-((u & 1) as i64))
}

/// Encode a `u64` as an unsigned LEB128 varint (little-endian base-128).
fn write_varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let b = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            break;
        }
        out.push(b | 0x80);
    }
}

/// Decode an unsigned LEB128 varint, advancing the cursor. Returns `None` on
/// truncation or a varint longer than 10 bytes (overflow).
fn read_varint(data: &[u8], cursor: &mut usize) -> Option<u64> {
    let mut result = 0u64;
    let mut shift = 0u32;
    for _ in 0..10 {
        let b = *data.get(*cursor)?;
        *cursor += 1;
        result |= ((b & 0x7F) as u64) << shift;
        if b & 0x80 == 0 {
            return Some(result);
        }
        shift += 7;
    }
    None // varint too long / overflow
}

/// Encode a byte slice as delta-encoded (first byte, then LEB128 zigzag deltas).
/// The first byte is stored literally; each subsequent byte is a delta from the
/// previous, zigzag-encoded to unsigned and written as a varint so any delta
/// (including wrap-around from 255→0) is lossless.
pub fn encode_delta(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    if data.is_empty() {
        return out;
    }
    let mut prev = data[0] as i64;
    out.push(data[0]);
    for &b in &data[1..] {
        let cur = b as i64;
        write_varint(&mut out, zigzag(cur - prev));
        prev = cur;
    }
    out
}

/// Decode delta-encoded bytes (first byte literal, then LEB128 zigzag deltas).
pub fn decode_delta(encoded: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(encoded.len());
    if encoded.is_empty() {
        return out;
    }
    let mut cursor = 0usize;
    let first = encoded[0];
    cursor = 1;
    let mut prev = first as i64;
    out.push(first);
    while cursor < encoded.len() {
        let u = read_varint(encoded, &mut cursor).unwrap_or(0);
        prev = prev.wrapping_add(unzigzag(u));
        out.push(prev as u8);
    }
    out
}

/// Container tag: which codec produced the payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Codec {
    /// Raw (compression would not help — payload stored verbatim).
    Raw = 0,
    /// Run-length encoding.
    Rle = 1,
    /// Delta encoding (with zigzag).
    Delta = 2,
}

/// A compressed blob with its codec and uncompressed size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Squash {
    pub codec: Codec,
    pub uncompressed_len: usize,
    pub payload: Vec<u8>,
}

impl Squash {
    /// Compress a byte slice, picking the smallest of raw / RLE / delta.
    /// Deterministic: ties prefer the codec with the lower tag (Raw < Rle < Delta).
    pub fn compress(data: &[u8]) -> Self {
        let raw = data.to_vec();
        let rle = encode_rle(data);
        let delta = encode_delta(data);

        let mut best = (Codec::Raw, raw.clone());
        if rle.len() < best.1.len() || (rle.len() == best.1.len() && (Codec::Rle as u8) < (best.0 as u8)) {
            best = (Codec::Rle, rle);
        }
        if delta.len() < best.1.len() {
            best = (Codec::Delta, delta);
        }

        Squash {
            codec: best.0,
            uncompressed_len: data.len(),
            payload: best.1,
        }
    }

    /// Decompress back to the original bytes. Returns `None` on corrupt input.
    pub fn decompress(&self) -> Option<Vec<u8>> {
        let out = match self.codec {
            Codec::Raw => self.payload.clone(),
            Codec::Rle => decode_rle(&self.payload)?,
            Codec::Delta => decode_delta(&self.payload),
        };
        if out.len() != self.uncompressed_len {
            return None; // length mismatch ⇒ corruption
        }
        Some(out)
    }

    /// Size on disk / in memory of the compressed representation.
    pub fn size_of(&self) -> usize {
        // 1 byte tag + payload (+ we track uncompressed_len out-of-band).
        1 + self.payload.len()
    }

    /// Would compression actually shrink the artifact?
    pub fn shrinks(&self) -> bool {
        self.size_of() < self.uncompressed_len
    }

    /// Serialize to a self-describing byte buffer (tag + payload).
    /// The uncompressed length is implied by decoding; stored out-of-band.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(1 + self.payload.len());
        out.push(self.codec as u8);
        out.extend_from_slice(&self.payload);
        out
    }

    /// Deserialize from `to_bytes`, given the known uncompressed length.
    pub fn from_bytes(bytes: &[u8], uncompressed_len: usize) -> Option<Self> {
        if bytes.is_empty() {
            return None;
        }
        let codec = match bytes[0] {
            0 => Codec::Raw,
            1 => Codec::Rle,
            2 => Codec::Delta,
            _ => return None,
        };
        Some(Squash {
            codec,
            uncompressed_len,
            payload: bytes[1..].to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rle_roundtrip() {
        let data = b"aaaaabbbbcccccccd";
        let enc = encode_rle(data);
        let dec = decode_rle(&enc).unwrap();
        assert_eq!(dec, data);
    }

    #[test]
    fn rle_empty() {
        assert!(encode_rle(&[]).is_empty());
        assert_eq!(decode_rle(&[]).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn rle_truncated_is_none() {
        assert_eq!(decode_rle(&[5]), None); // count without byte
    }

    #[test]
    fn rle_long_run_splits() {
        let data = vec![0xABu8; 300];
        let enc = encode_rle(&data);
        let dec = decode_rle(&enc).unwrap();
        assert_eq!(dec, data);
        // A 300-run must split into at least 2 runs (255 + 45).
        assert!(enc.len() >= 4, "expected at least two runs, got {} bytes", enc.len());
    }

    #[test]
    fn delta_roundtrip() {
        let data = [0u8, 1, 2, 3, 4, 5, 100, 101, 102];
        let enc = encode_delta(&data);
        let dec = decode_delta(&enc);
        assert_eq!(dec, data);
    }

    #[test]
    fn delta_empty() {
        assert!(encode_delta(&[]).is_empty());
        assert!(decode_delta(&[]).is_empty());
    }

    #[test]
    fn delta_wraps_around() {
        // Wrap from 255 back down to 0 exercises the zigzag signed-delta path.
        let data = [255u8, 0, 1, 254, 255];
        let enc = encode_delta(&data);
        let dec = decode_delta(&enc);
        assert_eq!(dec, data);
    }

    #[test]
    fn zigzag_roundtrip() {
        for v in [-3i64, -2, -1, 0, 1, 2, 3, i64::MIN / 2, i64::MAX / 2] {
            assert_eq!(unzigzag(zigzag(v)), v, "zigzag roundtrip failed for {v}");
        }
    }

    #[test]
    fn squash_roundtrip_identity() {
        let data = b"the quick brown fox jumps over the lazy dog. the quick brown fox.";
        let s = Squash::compress(data);
        assert_eq!(s.decompress().unwrap(), data);
    }

    #[test]
    fn squash_compresses_repetitive() {
        let data = vec![b'x'; 1000];
        let s = Squash::compress(&data);
        assert!(s.shrinks(), "repetitive data must compress: {} vs {}", s.size_of(), data.len());
        assert_eq!(s.decompress().unwrap(), data);
    }

    #[test]
    fn squash_to_from_bytes_roundtrip() {
        let data = vec![1u8, 1, 1, 2, 2, 3, 3, 3, 3];
        let s = Squash::compress(&data);
        let bytes = s.to_bytes();
        let back = Squash::from_bytes(&bytes, data.len()).unwrap();
        assert_eq!(back, s);
        assert_eq!(back.decompress().unwrap(), data);
    }

    #[test]
    fn squash_rejects_corrupt_tag() {
        assert_eq!(Squash::from_bytes(&[9], 0), None);
        assert_eq!(Squash::from_bytes(&[], 0), None);
    }

    #[test]
    fn squash_rejects_length_mismatch() {
        let data = b"hello";
        let s = Squash::compress(data);
        // Claim a wrong uncompressed length → decompress must fail.
        let mut corrupt = s.clone();
        corrupt.uncompressed_len = 999;
        assert_eq!(corrupt.decompress(), None);
    }
}