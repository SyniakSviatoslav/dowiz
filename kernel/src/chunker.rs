//! chunker.rs — deterministic content-defined chunker (Tier B4 backup core).
//!
//! GROWTH-SUBSTRATE / INFRA primitive. The native Rust backup organ (Master-
//! Integration plan B4) needs content-defined chunking so that backups DEDUP
//! across small edits: a one-byte change in a 10 MB file must re-hash only the
//! ~local block, not the whole file. That is exactly the property that makes
//! the offline-on-node backup recoverable and cheap to store (3-2-1-1-0).
//!
//! Algorithm: Buzhash rolling fingerprint over a fixed window (W=48). A cut is
//! emitted when the low `bits` of the fingerprint are zero AND the chunk is at
//! least `min` bytes (with a hard cap at `max`). Because the fingerprint is a
//! function of ONLY the local window (the outgoing byte's contribution is
//! removed on each step), cut points depend on content, not absolute position —
//! the defining CDC property.
//!
//! Deterministic: the hash table is derived from a FIXED constant seed (FNV-1a
//! over the index), never from entropy. Same input ⇒ identical blocks on any
//! machine / wasm32 / native. No deps; the block id is the kernel's existing
//! `event_log::sha3_256` (FIPS 202, pure Rust).
//!
//! Scope ceiling (honest): this crate produces the content-addressed blocks +
//! a remote-rebuildable index. The actual upload to R2/Hetzner is product/infra
//! (node binary) — out of kernel scope. The chunker is the verifiable core.

use crate::event_log::sha3_256;

const WINDOW: usize = 48;

/// A content-addressed chunk: its raw bytes + a collision-resistant id.
#[derive(Clone, Debug)]
pub struct Block {
    pub bytes: Vec<u8>,
    pub id: [u8; 32],
}

/// Content-defined chunker. Deterministic for a given (min, max, bits).
pub struct Chunker {
    min: usize,
    max: usize,
    mask: u64,
    table: [u64; 256],
}

impl Chunker {
    /// `bits` = how many low fingerprint bits must be zero to cut (avg chunk ≈
    /// 2^bits). `min`/`max` bound chunk size. Same args ⇒ identical behavior.
    pub fn new(min: usize, max: usize, bits: u32) -> Self {
        // Deterministic hash table: FNV-1a over the byte index, no entropy.
        let mut table = [0u64; 256];
        let mut h: u64 = 0xcbf29ce484222325;
        for i in 0..256 {
            h ^= i as u64;
            h = h.wrapping_mul(0x100000001b3);
            // spread the bits so low-order cuts still vary
            table[i] = h.rotate_left((i % 47) as u32).wrapping_mul(0x9e3779b97f4a7c15);
        }
        let mask = if bits == 0 { 0 } else { (1u64 << bits) - 1 };
        Chunker {
            min: min.max(1),
            max: max.max(min + 1),
            mask,
            table,
        }
    }

    /// Split `data` into content-defined, content-addressed blocks.
    pub fn chunk(&self, data: &[u8]) -> Vec<Block> {
        let n = data.len();
        if n == 0 {
            return Vec::new();
        }
        let mut blocks = Vec::new();
        let mut start = 0usize;
        let mut fp: u64 = 0;
        // ring of the last WINDOW bytes' table values, and the raw bytes
        let mut ring: [u64; WINDOW] = [0; WINDOW];
        let mut raw: [u8; WINDOW] = [0; WINDOW];
        let mut filled = 0usize;
        let mut pos = 0usize; // index into ring/raw (mod WINDOW)
        let mut i = 0usize;
        while i < n {
            let b = data[i];
            let hb = self.table[b as usize];
            // roll: rotate-left the fingerprint by 1, XOR in the new byte
            fp = fp.rotate_left(1) ^ hb;
            // remove the contribution of the byte leaving the window
            if filled == WINDOW {
                let old = raw[pos];
                // the old contribution was rotated left WINDOW times since it
                // entered; subtract it back out (XOR is its own inverse)
                fp ^= self.table[old as usize].rotate_left(WINDOW as u32);
            } else {
                filled += 1;
            }
            ring[pos] = hb;
            raw[pos] = b;
            pos = (pos + 1) % WINDOW;

            let len = i - start + 1;
            let cut = len >= self.min && (self.mask == 0 || (fp & self.mask) == 0);
            let hard = len >= self.max;
            if (cut || hard) && i < n - 1 {
                // emit [start..=i]
                let chunk = &data[start..=i];
                let id = sha3_256(chunk);
                blocks.push(Block {
                    bytes: chunk.to_vec(),
                    id,
                });
                start = i + 1;
                fp = 0;
                filled = 0;
                pos = 0;
                ring = [0; WINDOW];
                raw = [0; WINDOW];
            }
            i += 1;
        }
        // tail (everything after the last cut, or the whole input if no cut)
        if start < n {
            let chunk = &data[start..n];
            let id = sha3_256(chunk);
            blocks.push(Block {
                bytes: chunk.to_vec(),
                id,
            });
        }
        blocks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(n: usize) -> Vec<u8> {
        // deterministic pseudo-random bytes (LCG) — reproducible, no entropy
        let mut x: u64 = 0x1234_5678_9abc_def0;
        (0..n)
            .map(|_| {
                x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                (x >> 33) as u8
            })
            .collect()
    }

    /// Deterministic: same input ⇒ identical block ids, on every run.
    #[test]
    fn deterministic() {
        let data = sample(20_000);
        let c = Chunker::new(1024, 32 * 1024, 12);
        let a = c.chunk(&data);
        let b = c.chunk(&data);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.id, y.id);
            assert_eq!(x.bytes, y.bytes);
        }
    }

    /// Dedup: identical content ⇒ identical block ids (a re-backup shares
    /// every block). This is the property that makes the organ storage-cheap.
    #[test]
    fn dedup_same_content() {
        let data = sample(50_000);
        let c = Chunker::new(1024, 32 * 1024, 12);
        let blocks = c.chunk(&data);
        // every block id unique within one file
        let mut seen = std::collections::HashSet::new();
        for blk in &blocks {
            assert!(seen.insert(blk.id), "duplicate block id within a file");
        }
        // re-chunk the same data → identical ids (rebuildable / shared)
        let again = c.chunk(&data);
        assert_eq!(blocks.len(), again.len());
        for (x, y) in blocks.iter().zip(again.iter()) {
            assert_eq!(x.id, y.id);
        }
    }

    /// LOCALITY (the defining CDC property): flip one byte in the MIDDLE and
    /// only the blocks straddling the change differ; the prefix and suffix
    /// blocks keep their ids. That is why a 1-byte edit re-hashes ~one block,
    /// not the file.
    #[test]
    fn locality_one_byte_change() {
        let mut data = sample(200_000);
        let c = Chunker::new(1024, 32 * 1024, 12);
        let before = c.chunk(&data);
        // change a single byte in the middle
        let mid = data.len() / 2;
        data[mid] ^= 0xff;
        let after = c.chunk(&data);

        assert_eq!(before.len(), after.len(), "same block count expected");
        let mut changed = 0;
        for (x, y) in before.iter().zip(after.iter()) {
            if x.id != y.id {
                changed += 1;
            }
        }
        // only a small local region changes, not the whole file
        assert!(changed <= 3, "too many blocks changed: {changed}");
        assert!(changed >= 1, "expected at least the edited block to change");
    }

    /// REBUILD: concatenating the block bytes in order reproduces the input.
    #[test]
    fn rebuild_from_blocks() {
        let data = sample(80_000);
        let c = Chunker::new(1024, 32 * 1024, 12);
        let blocks = c.chunk(&data);
        let mut rebuilt = Vec::new();
        for blk in &blocks {
            rebuilt.extend_from_slice(&blk.bytes);
        }
        assert_eq!(rebuilt, data);
    }
}
