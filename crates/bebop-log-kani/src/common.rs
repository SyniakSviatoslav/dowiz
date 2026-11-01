//! Symbolic inputs, and the harnesses' OWN readers of the wire format. The readers are deliberately
//! not the crate's `le8`/`id32`: a property checked through the code under check says little.

/// Any byte string of length `0..=N`: a symbolic array cut at a symbolic length. No allocation in
/// the harness, so the length is the only thing that varies.
pub struct Bytes<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> Bytes<N> {
    pub fn any() -> Self {
        Bytes { buf: kani::any(), len: kani::any_where(|l: &usize| *l <= N) }
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}

/// The little-endian u64 at `at`. The caller has checked the length.
pub fn u64_at(b: &[u8], at: usize) -> u64 {
    let mut w = [0u8; 8];
    w.copy_from_slice(&b[at..at + 8]);
    u64::from_le_bytes(w)
}

pub fn i64_at(b: &[u8], at: usize) -> i64 {
    u64_at(b, at) as i64
}

/// The 32 bytes at `at`. The caller has checked the length.
pub fn id_at(b: &[u8], at: usize) -> [u8; 32] {
    let mut w = [0u8; 32];
    w.copy_from_slice(&b[at..at + 32]);
    w
}
