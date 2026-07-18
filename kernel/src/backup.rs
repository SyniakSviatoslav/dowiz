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
//! store is behind a `BlockStore` trait so a real append-log / R2 backend can
//! drop in later (product/infra scope). Two implementations ship today:
//!   - `MemStore`            — in-memory `HashMap` (tests / single-node local).
//!   - `FileBlockStore`      — disk-backed, content-addressed (P12 §2): one file
//!     per unique block under `<root>/blocks/<xx>/<yy>/<hex>`, crash-atomic
//!     writes via `tmp/<id>.partial` + POSIX-rename, and an in-memory index so
//!     the existing `get`/`len` borrow contract is preserved (no new dep, std
//!     only). `get_owned` re-reads the on-disk bytes and re-hashes them against
//!     the filename — a content-address integrity check (fail-closed None).

use crate::chunker::Chunker;
use crate::event_log::sha3_256;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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
    /// Fetch owned bytes for `id`, if present. Disk-backed stores (which cannot
    /// return a borrowed slice tied to `&self`) override this directly; the
    /// default impl just clones what `get` returns so `MemStore` is unchanged.
    fn get_owned(&self, id: &Hash) -> Option<Vec<u8>> {
        self.get(id).map(|s| s.to_vec())
    }
    /// Number of distinct blocks held.
    fn len(&self) -> usize;
    /// True when no blocks are held.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// In-memory content-addressed store (for tests / single-node local-first use).
#[derive(Default, Debug, Clone)]
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

/// Disk-backed, content-addressed block store (P12 §2). One file per unique
/// block, named by its sha3 id, under a 65536-way sharded fan-out:
///
///   `<root>/blocks/<hex[0:2]>/<hex[2:4]>/<hex>`
///
/// Writes are crash-atomic: content is written to `<root>/tmp/<id>.partial`,
/// `fsync`'d, then `rename`'d into place (POSIX rename is atomic), so a
/// kill-9 between the partial write and the rename leaves NO half-written
/// block visible — `put` is all-or-nothing. A block whose final path already
/// exists is a dedup no-op (returns `false`, mirroring `MemStore`).
///
/// The on-disk `blocks/` tree is the durable source of truth. To satisfy the
/// trait's borrowed-slice `get`/`len` contract (which a disk read cannot meet
/// without interior mutability), an in-memory `cache: HashMap<Hash, Vec<u8>>`
/// mirrors the bytes; `get_owned` always re-reads the on-disk file and
/// re-hashes it against the filename — a mismatch (on-disk bit-rot /
/// corruption) yields fail-closed `None`, never unverified bytes.
///
/// No new dependency: `std::fs` only. M6/V2 zero-dep at the storage boundary.
pub struct FileBlockStore {
    root: PathBuf,
    cache: HashMap<Hash, Vec<u8>>,
}

impl FileBlockStore {
    /// Open (creating if needed) a store rooted at `root`. Loads the existing
    /// `blocks/` tree into the in-memory cache so a store reopened across
    /// process restarts still answers `get`/`len`.
    pub fn open(root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let root = root.into();
        fs::create_dir_all(root.join("blocks"))?;
        fs::create_dir_all(root.join("manifests"))?;
        fs::create_dir_all(root.join("tmp"))?;
        let mut cache = HashMap::new();
        Self::load(&root, &mut cache)?;
        Ok(FileBlockStore { root, cache })
    }

    /// Recursively walk `blocks/` and read every `<hex>` file's bytes into the
    /// cache. Files whose name is not a valid 32-byte hex id are skipped.
    fn load(root: &PathBuf, cache: &mut HashMap<Hash, Vec<u8>>) -> std::io::Result<()> {
        let blocks_dir = root.join("blocks");
        if !blocks_dir.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&blocks_dir)? {
            let e = entry?;
            let p1 = e.path();
            if !p1.is_dir() {
                continue;
            }
            for entry2 in fs::read_dir(&p1)? {
                let e2 = entry2?;
                let p2 = e2.path();
                if !p2.is_dir() {
                    continue;
                }
                for entry3 in fs::read_dir(&p2)? {
                    let e3 = entry3?;
                    let file = e3.path();
                    if !file.is_file() {
                        continue;
                    }
                    let name = match file.file_name().and_then(|n| n.to_str()) {
                        Some(n) => n,
                        None => continue,
                    };
                    let id = match parse_hex32(name) {
                        Ok(id) => id,
                        Err(_) => continue,
                    };
                    let bytes = match fs::read(&file) {
                        Ok(b) => b,
                        Err(_) => continue,
                    };
                    cache.insert(id, bytes);
                }
            }
        }
        Ok(())
    }

    /// Path of the final on-disk block file for `id`.
    fn block_path(&self, id: &Hash) -> PathBuf {
        let hex = hex_encode(id);
        self.root
            .join("blocks")
            .join(&hex[0..2])
            .join(&hex[2..4])
            .join(&hex)
    }
}

impl BlockStore for FileBlockStore {
    fn put(&mut self, id: Hash, bytes: &[u8]) -> bool {
        let final_path = self.block_path(&id);
        if final_path.exists() {
            // Idempotent dedup: already physically present.
            return false;
        }
        // Ensure the shard directory exists before writing.
        if let Some(parent) = final_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                // A durability primitive must NEVER panic on I/O (the disk is most
                // likely to be full exactly when the backup runs). Signal failure
                // via the trait's bool return instead — TORVALDS-14.
                eprintln!("FileBlockStore: failed to create shard dir {parent:?}: {e}");
                return false;
            }
        }
        // Crash-atomic write: <root>/tmp/<id>.partial → fsync → rename.
        let hex = hex_encode(&id);
        let partial = self.root.join("tmp").join(format!("{hex}.partial"));
        // Best-effort: drop any stale partial from a prior interrupted write
        // of the same id so the partial represents THIS write only.
        let _ = fs::remove_file(&partial);
        if let Err(e) = fs::write(&partial, bytes) {
            let _ = fs::remove_file(&partial);
            eprintln!("FileBlockStore: failed to write partial {partial:?}: {e}");
            return false;
        }
        // fsync the partial so its bytes are durable before the atomic rename.
        if let Ok(f) = fs::File::open(&partial) {
            let _ = f.sync_all();
        }
        if let Err(e) = fs::rename(&partial, &final_path) {
            let _ = fs::remove_file(&partial);
            eprintln!("FileBlockStore: failed to rename into place {final_path:?}: {e}");
            return false;
        }
        self.cache.insert(id, bytes.to_vec());
        true
    }

    fn get(&self, id: &Hash) -> Option<&[u8]> {
        // Borrowed slice comes from the in-memory cache, which mirrors disk.
        self.cache.get(id).map(|v| v.as_slice())
    }

    fn get_owned(&self, id: &Hash) -> Option<Vec<u8>> {
        let path = self.block_path(id);
        if !path.exists() {
            return None;
        }
        let bytes = fs::read(&path).ok()?;
        // Content-address integrity: the filename IS the key. Re-hash the
        // stored bytes and compare; a mismatch (corruption / bit-rot) is a
        // fail-closed None — never return unverified bytes.
        if sha3_256(&bytes) != *id {
            return None;
        }
        Some(bytes)
    }

    fn len(&self) -> usize {
        self.cache.len()
    }
}

impl FileBlockStore {
    /// Total bytes physically retained (sum of unique block sizes on disk) —
    /// the real on-disk cost after dedup.
    pub fn stored_bytes(&self) -> u64 {
        self.cache.values().map(|v| v.len() as u64).sum()
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

/// Lowercase hex encode of a 32-byte hash (no dependency, std only).
fn hex_encode(id: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(64);
    for &b in id {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

/// Parse a 64-char lowercase-hex string into a `[u8; 32]`. Errors on wrong
/// length or non-hex characters (used when scanning the on-disk `blocks/` tree
/// so stray files are ignored rather than crashing the loader).
fn parse_hex32(s: &str) -> Result<[u8; 32], ()> {
    if s.len() != 64 {
        return Err(());
    }
    let bytes = s.as_bytes();
    let mut out = [0u8; 32];
    for i in 0..32 {
        let hi = match val(bytes[i * 2]) {
            Some(v) => v,
            None => return Err(()),
        };
        let lo = match val(bytes[i * 2 + 1]) {
            Some(v) => v,
            None => return Err(()),
        };
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

fn val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
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
            let already = self.store.get_owned(&blk.id).is_some();
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
            // Prefer `get_owned` so disk-backed stores return their own bytes
            // (FileBlockStore additionally re-hashes for content-address
            // integrity). Falls back to the borrowed `get` otherwise.
            match self.store.get_owned(id) {
                Some(bytes) => out.extend_from_slice(&bytes),
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
        assert!(
            stats_a.new_blocks > 3,
            "file A should chunk into many blocks"
        );

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

    // ---- FileBlockStore (P12 §2) tests: same properties, disk-backed ----

    fn fbs_organ(root: &std::path::Path) -> BackupOrgan<FileBlockStore> {
        let store = FileBlockStore::open(root).expect("open store");
        BackupOrgan::new(store, 1024, 32 * 1024, 12)
    }

    /// Round-trip identity for the disk-backed store.
    #[test]
    fn fileblockstore_restore_is_byte_identical() {
        let tmp = std::env::temp_dir().join(format!("fbs_rid_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let data = sample(120_000);
        let mut organ = fbs_organ(&tmp);
        let (manifest, _stats) = organ.backup(&data);
        let restored = organ.restore(&manifest).expect("restore ok");
        assert_eq!(
            restored, data,
            "FileBlockStore restore must be byte-identical"
        );
        assert_eq!(restored.len(), manifest.total_len);
        // Reopen the store from disk and confirm the manifest still restores
        // (durability: bytes live on disk, not only in RAM).
        drop(organ);
        let organ2 = fbs_organ(&tmp);
        let restored2 = organ2.restore(&manifest).expect("restore from disk");
        assert_eq!(restored2, data, "FileBlockStore must restore after reopen");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// DEDUP across a 1-byte edit, disk-backed, mirrors the MemStore property.
    #[test]
    fn fileblockstore_one_byte_edit_dedups_over_90pct() {
        let tmp = std::env::temp_dir().join(format!("fbs_dedup_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let file_a = sample(200_000);
        let mut file_b = file_a.clone();
        let mid = file_b.len() / 2;
        file_b[mid] ^= 0xff;

        let mut organ = fbs_organ(&tmp);
        let (man_a, stats_a) = organ.backup(&file_a);
        let (man_b, stats_b) = organ.backup(&file_b);

        assert_eq!(stats_a.deduped_blocks, 0);
        assert!(stats_a.new_blocks > 3);
        let ratio = stats_b.dedup_ratio();
        assert!(ratio > 0.90, "dedup ratio too low: {ratio:.4}");
        assert!(
            stats_b.new_blocks <= 3,
            "expected <=3 new blocks, got {}",
            stats_b.new_blocks
        );

        let restored_a = organ.restore(&man_a).expect("restore A");
        let restored_b = organ.restore(&man_b).expect("restore B");
        assert_eq!(restored_a, file_a);
        assert_eq!(restored_b, file_b);

        let stored = organ.store().stored_bytes();
        assert!(
            stored < (file_a.len() + file_b.len()) as u64,
            "store did not dedup"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Re-backing identical content is 100% dedup, disk-backed.
    #[test]
    fn fileblockstore_identical_rebackup_fully_dedups() {
        let tmp = std::env::temp_dir().join(format!("fbs_reback_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let data = sample(60_000);
        let mut organ = fbs_organ(&tmp);
        let (_m1, _s1) = organ.backup(&data);
        let store_len_after_first = organ.store().len();
        let (_m2, s2) = organ.backup(&data);
        assert_eq!(s2.new_blocks, 0, "re-backup must write no new blocks");
        assert_eq!(s2.dedup_ratio(), 1.0);
        assert_eq!(organ.store().len(), store_len_after_first);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Fail-closed restore: a missing block yields Err, disk-backed.
    #[test]
    fn fileblockstore_missing_block_fails_closed() {
        let tmp = std::env::temp_dir().join(format!("fbs_missing_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let data = sample(40_000);
        let mut organ = fbs_organ(&tmp);
        let (mut manifest, _s) = organ.backup(&data);
        manifest.blocks.push([0xAB; 32]);
        manifest.total_len += 1;
        let err = organ.restore(&manifest).unwrap_err();
        assert_eq!(err, RestoreError::MissingBlock([0xAB; 32]));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Content-address integrity: a 1-bit on-disk corruption makes `get_owned`
    /// fail-closed (None), never return unverified bytes.
    #[test]
    fn fileblockstore_corrupt_block_rejected() {
        let tmp = std::env::temp_dir().join(format!("fbs_corrupt_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        // One fixed block so the id is deterministic and known.
        let block = vec![0x42u8; 4096];
        let id = crate::event_log::sha3_256(&block);
        let mut store = FileBlockStore::open(&tmp).expect("open store");
        assert!(store.put(id, &block), "first put is new");

        // get_owned returns the bytes, and they verify.
        let got = store.get_owned(&id).expect("clean block readable");
        assert_eq!(got, block);

        // Flip one byte of the on-disk file.
        let hex = hex_encode(&id);
        let path = tmp
            .join("blocks")
            .join(&hex[0..2])
            .join(&hex[2..4])
            .join(&hex);
        let mut raw = std::fs::read(&path).expect("read block file");
        raw[0] ^= 0x01; // 1-bit flip
        std::fs::write(&path, &raw).expect("rewrite corrupted");

        // Corrupted block is rejected fail-closed.
        assert!(
            store.get_owned(&id).is_none(),
            "corrupted block must be rejected"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Crash-atomicity invariant: a `.partial` left behind (simulating a kill-9
    /// between the partial write and the rename) is NOT visible as a block.
    /// `get_owned` must ignore the temp file and return None for that id.
    #[test]
    fn fileblockstore_partial_write_invisible() {
        let tmp = std::env::temp_dir().join(format!("fbs_partial_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let block = vec![0x7u8; 2048];
        let id = crate::event_log::sha3_256(&block);
        let hex = hex_encode(&id);
        // Simulate an interrupted write: leave only a .partial, no final file.
        let tmp_dir = tmp.join("tmp");
        std::fs::create_dir_all(&tmp_dir).expect("create tmp dir");
        std::fs::write(tmp_dir.join(format!("{hex}.partial")), &block).expect("write partial");
        let store = FileBlockStore::open(&tmp).expect("open store");
        // The block must not be readable; get_owned sees only the final path.
        assert!(
            store.get_owned(&id).is_none(),
            "a .partial must never be visible as a stored block"
        );
        // And the blocks/ tree stays empty (no half-written file leaked).
        assert_eq!(store.len(), 0);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// TORVALDS-14: the backup store must NOT panic when the underlying filesystem
    /// write fails (full disk / permission denied). It signals failure via the
    /// `bool` return so the caller can degrade instead of crashing the process.
    /// Prior to the fix, `put` `panic!`-ed on `create_dir_all`/`write`/`rename`
    /// failure — i.e. it died exactly when the disk was most likely to be full.
    #[test]
    fn fileblockstore_put_fails_without_panic_on_io_error() {
        let tmp = std::env::temp_dir().join(format!("fbs_ro_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        // A valid store.
        let mut store = FileBlockStore::open(&tmp).expect("open store");
        let block = vec![0x9u8; 512];
        let id = crate::event_log::sha3_256(&block);
        // Block the shard directory's *parent* by placing a regular file where
        // `put` will try to `create_dir_all` the shard path. `create_dir_all`
        // then fails with ENOTDIR — an error even root cannot bypass (unlike a
        // read-only bit, which root ignores). This simulates the realistic
        // "filesystem write failed" path without depending on permissions.
        let hex = hex_encode(&id);
        let blocked = tmp.join("blocks").join(&hex[0..2]);
        std::fs::write(&blocked, b"not-a-dir").expect("plant blocking file");
        let _guard = scopeguard_remove_all(&tmp);
        // Must return false (failure signalled), NOT panic.
        let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            store.put(id, &block)
        }));
        match ok {
            Ok(ret) => assert!(!ret, "put must return false on I/O failure, not succeed"),
            Err(_) => panic!("FileBlockStore::put panicked on I/O error — TORVALDS-14 regression"),
        }
    }

    /// Best-effort removal of the temp dir (so the test doesn't leak).
    fn scopeguard_remove_all(path: &std::path::Path) -> impl Drop + '_ {
        struct G<'a>(&'a std::path::Path);
        impl Drop for G<'_> {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(self.0);
            }
        }
        G(path)
    }
}
