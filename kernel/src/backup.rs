//! backup.rs — native content-addressed backup organ (Tier B4).
//!
//! GROWTH-SUBSTRATE / INFRA primitive. This is the storage half of the offline-
//! on-node backup: the `chunker` (Buzhash CDC) splits a byte stream into
//! content-defined blocks; this organ stores each unique block once, keyed by
//! its `sha3_256` id (content-addressed), and can reconstruct the original bytes
//! byte-for-byte from the ordered list of block ids (a *manifest*).
//!
//! Verified-by-Math property (the whole point of B4):
//!   1. DEDUP — two near-identical files (a 1-byte edit) share ~all blocks, so
//!      the second backup adds only the local region, not the whole file.
//!   2. EXACT RESTORE — restoring a manifest yields the original bytes bit-for-
//!      bit (round-trip identity), which is the recoverability guarantee.
//!
//! No new deps: reuses `crate::chunker::Chunker` for CDC and the kernel's
//! existing `crate::event_log::sha3_256` (via the chunker's `Block.id`). The
//! store is an in-memory `HashMap<Hash, Vec<u8>>` behind a `BlockStore` trait so
//! a real append-log / R2 backend can drop in later (product/infra scope).

use crate::chunker::Chunker;
use std::collections::HashMap;

/// Content-address of a block: the chunker's sha3_256 id.
pub type Hash = [u8; 32];

/// Pluggable content-addressed block store. Put is idempotent: storing a block
/// whose id already exists is a no-op (that IS the dedup). Get returns the raw
/// bytes for a previously-stored id.
pub trait BlockStore {
    /// Store `bytes` under `id`. Returns `true` iff this was a NEW block
    /// (i.e. it deduped away when `false`).
    fn put(&mut self, id: Hash, bytes: &[u8]) -> bool;
    /// Fetch bytes for `id`, if present.
    fn get(&self, id: &Hash) -> Option<&[u8]>;
    /// Number of distinct blocks held.
    fn len(&self) -> usize;
    /// True when no blocks are held.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// In-memory content-addressed store (for tests / single-node local-first use).
#[derive(Default, Debug)]
pub struct MemStore {
    map: HashMap<Hash, Vec<u8>>,
}

impl MemStore {
    pub fn new() -> Self {
        MemStore {
            map: HashMap::new(),
        }
    }
    /// Total bytes physically retained (sum of unique block sizes) — the real
    /// on-disk cost after dedup.
    pub fn stored_bytes(&self) -> usize {
        self.map.values().map(|v| v.len()).sum()
    }
}

impl BlockStore for MemStore {
    fn put(&mut self, id: Hash, bytes: &[u8]) -> bool {
        if self.map.contains_key(&id) {
            false
        } else {
            self.map.insert(id, bytes.to_vec());
            true
        }
    }
    fn get(&self, id: &Hash) -> Option<&[u8]> {
        self.map.get(id).map(|v| v.as_slice())
    }
    fn len(&self) -> usize {
        self.map.len()
    }
}

/// An ordered list of block ids that reconstructs one file. Restoring it
/// concatenates `store.get(id)` for each id in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub blocks: Vec<Hash>,
    /// Total logical size (bytes) of the original stream — a restore check.
    pub total_len: usize,
}

/// Result of a backup: the manifest plus dedup accounting for THIS backup.
#[derive(Clone, Debug)]
pub struct BackupStats {
    /// Blocks the chunker produced for this stream (with repeats).
    pub total_blocks: usize,
    /// Blocks that were NEW to the store (physically written).
    pub new_blocks: usize,
    /// Blocks that already existed (deduped away).
    pub deduped_blocks: usize,
}

impl BackupStats {
    /// Fraction of this backup's blocks that deduped (0.0..=1.0).
    pub fn dedup_ratio(&self) -> f64 {
        if self.total_blocks == 0 {
            0.0
        } else {
            self.deduped_blocks as f64 / self.total_blocks as f64
        }
    }
}

/// The backup organ: owns a chunker config + a content-addressed store.
pub struct BackupOrgan<S: BlockStore> {
    chunker: Chunker,
    store: S,
}

impl<S: BlockStore> BackupOrgan<S> {
    /// Build with an explicit chunker + store. `min`/`max`/`bits` tune CDC.
    pub fn new(store: S, min: usize, max: usize, bits: u32) -> Self {
        BackupOrgan {
            chunker: Chunker::new(min, max, bits),
            store,
        }
    }

    /// Back up `data`: chunk → store unique blocks → return manifest + stats.
    /// Idempotent at the block level, so re-backing identical content is free.
    pub fn backup(&mut self, data: &[u8]) -> (Manifest, BackupStats) {
        let blocks = self.chunker.chunk(data);
        let mut manifest = Vec::with_capacity(blocks.len());
        let mut new_blocks = 0usize;
        let mut deduped = 0usize;
        // Guard against intra-stream repeats double-counting a physical write.
        let mut written_this_call: HashMap<Hash, ()> = HashMap::new();
        for blk in &blocks {
            let already = self.store.get(&blk.id).is_some();
            if already {
                deduped += 1;
            } else if written_this_call.contains_key(&blk.id) {
                // repeated block within THIS same stream: not a fresh write
                deduped += 1;
            } else {
                let is_new = self.store.put(blk.id, &blk.bytes);
                if is_new {
                    new_blocks += 1;
                    written_this_call.insert(blk.id, ());
                } else {
                    deduped += 1;
                }
            }
            manifest.push(blk.id);
        }
        let stats = BackupStats {
            total_blocks: blocks.len(),
            new_blocks,
            deduped_blocks: deduped,
        };
        (
            Manifest {
                blocks: manifest,
                total_len: data.len(),
            },
            stats,
        )
    }

    /// Restore the original bytes from a manifest. Returns `Err` if any block id
    /// is missing from the store (corrupt / incomplete backup) or the restored
    /// length disagrees with the manifest's recorded length.
    pub fn restore(&self, manifest: &Manifest) -> Result<Vec<u8>, RestoreError> {
        let mut out = Vec::with_capacity(manifest.total_len);
        for id in &manifest.blocks {
            match self.store.get(id) {
                Some(bytes) => out.extend_from_slice(bytes),
                None => return Err(RestoreError::MissingBlock(*id)),
            }
        }
        if out.len() != manifest.total_len {
            return Err(RestoreError::LengthMismatch {
                expected: manifest.total_len,
                got: out.len(),
            });
        }
        Ok(out)
    }

    /// Borrow the store (e.g. for physical-size accounting).
    pub fn store(&self) -> &S {
        &self.store
    }
}

/// Why a restore failed (fail-closed: never returns partial/garbage bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RestoreError {
    MissingBlock(Hash),
    LengthMismatch { expected: usize, got: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(n: usize) -> Vec<u8> {
        // deterministic pseudo-random bytes (LCG) — reproducible, no entropy
        let mut x: u64 = 0x1234_5678_9abc_def0;
        (0..n)
            .map(|_| {
                x = x
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (x >> 33) as u8
            })
            .collect()
    }

    /// Round-trip identity: back up → restore yields the EXACT original bytes.
    #[test]
    fn restore_is_byte_identical() {
        let data = sample(120_000);
        let mut organ = BackupOrgan::new(MemStore::new(), 1024, 32 * 1024, 12);
        let (manifest, _stats) = organ.backup(&data);
        let restored = organ.restore(&manifest).expect("restore ok");
        assert_eq!(restored, data, "restore must be byte-identical");
        assert_eq!(restored.len(), manifest.total_len);
    }

    /// DEDUP across a 1-byte edit: backing up file B (differs from A by one
    /// byte in the middle) reuses nearly every block from A. New physical
    /// blocks for B must be a tiny fraction; dedup ratio > 90%.
    /// AND both restore byte-identically from the SHARED store.
    #[test]
    fn one_byte_edit_dedups_over_90pct_and_both_restore() {
        let file_a = sample(200_000);
        let mut file_b = file_a.clone();
        let mid = file_b.len() / 2;
        file_b[mid] ^= 0xff; // flip a single byte in the middle

        let mut organ = BackupOrgan::new(MemStore::new(), 1024, 32 * 1024, 12);

        // Back up A (all blocks new), then B into the SAME store.
        let (man_a, stats_a) = organ.backup(&file_a);
        let (man_b, stats_b) = organ.backup(&file_b);

        // A is the first backup → nothing deduped yet.
        assert_eq!(stats_a.deduped_blocks, 0);
        assert!(stats_a.new_blocks > 3, "file A should chunk into many blocks");

        // B shares almost all of A's blocks: dedup ratio must exceed 0.90.
        let ratio = stats_b.dedup_ratio();
        assert!(
            ratio > 0.90,
            "dedup ratio too low: {ratio:.4} ({} new / {} total)",
            stats_b.new_blocks,
            stats_b.total_blocks
        );

        // Only a small local region is physically new for B.
        assert!(
            stats_b.new_blocks <= 3,
            "expected <=3 new blocks for a 1-byte edit, got {}",
            stats_b.new_blocks
        );

        // Both files restore byte-identically from the shared, deduped store.
        let restored_a = organ.restore(&man_a).expect("restore A");
        let restored_b = organ.restore(&man_b).expect("restore B");
        assert_eq!(restored_a, file_a, "A must restore exactly");
        assert_eq!(restored_b, file_b, "B must restore exactly");

        // Storage saving is real: unique bytes retained < sum of both files.
        let stored = organ.store().stored_bytes();
        assert!(
            stored < file_a.len() + file_b.len(),
            "store did not dedup: {stored} >= {}",
            file_a.len() + file_b.len()
        );
    }

    /// Re-backing up IDENTICAL content is 100% dedup (idempotent organ).
    #[test]
    fn identical_rebackup_fully_dedups() {
        let data = sample(60_000);
        let mut organ = BackupOrgan::new(MemStore::new(), 1024, 32 * 1024, 12);
        let (_m1, _s1) = organ.backup(&data);
        let store_len_after_first = organ.store().len();
        let (_m2, s2) = organ.backup(&data);
        assert_eq!(s2.new_blocks, 0, "re-backup must write no new blocks");
        assert_eq!(s2.dedup_ratio(), 1.0);
        assert_eq!(
            organ.store().len(),
            store_len_after_first,
            "store size unchanged on identical re-backup"
        );
    }

    /// Fail-closed restore: a missing block yields Err, never garbage bytes.
    #[test]
    fn missing_block_fails_closed() {
        let data = sample(40_000);
        let mut organ = BackupOrgan::new(MemStore::new(), 1024, 32 * 1024, 12);
        let (mut manifest, _s) = organ.backup(&data);
        // corrupt the manifest with a bogus id not in the store
        manifest.blocks.push([0xAB; 32]);
        manifest.total_len += 1;
        let err = organ.restore(&manifest).unwrap_err();
        assert_eq!(err, RestoreError::MissingBlock([0xAB; 32]));
    }

    /// Empty stream: valid, zero blocks, restores to empty.
    #[test]
    fn empty_stream_round_trips() {
        let mut organ = BackupOrgan::new(MemStore::new(), 1024, 32 * 1024, 12);
        let (manifest, stats) = organ.backup(&[]);
        assert_eq!(stats.total_blocks, 0);
        assert!(manifest.blocks.is_empty());
        let restored = organ.restore(&manifest).expect("empty restore");
        assert!(restored.is_empty());
    }
}
