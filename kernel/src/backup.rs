//! backup.rs — std shim over the pure no_std core (`dowiz_core::backup`).
//!
//! The content-addressed backup organ (Tier B4) lives in `dowiz_core::backup`:
//! the [`BlockStore`] trait, [`MemStore`], [`Manifest`], [`BackupStats`],
//! [`BackupOrgan`], [`RestoreError`], `hex_encode`/`parse_hex32`, and
//! `snapshot_and_restore_local`. Those are re-exported here so the kernel's
//! existing `dowiz_kernel::backup::*` call sites compile unchanged.
//!
//! This shim adds the one std-dependent piece: [`FileBlockStore`], the
//! disk-backed, content-addressed store (P12 §2).

pub use dowiz_core::backup::*;

use crate::event_log::sha3_256;
use crate::vfs as fs;
use crate::vfs::VfsFile;
use alloc::collections::BTreeMap;
use std::path::PathBuf;

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
/// without interior mutability), an in-memory `cache: BTreeMap<Hash, Vec<u8>>`
/// mirrors the bytes; `get_owned` always re-reads the on-disk file and
/// re-hashes it against the filename — a mismatch (on-disk bit-rot /
/// corruption) yields fail-closed `None`, never unverified bytes.
///
/// No new dependency: `std::fs` only. M6/V2 zero-dep at the storage boundary.
pub struct FileBlockStore {
    root: PathBuf,
    cache: BTreeMap<Hash, Vec<u8>>,
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
        let mut cache = BTreeMap::new();
        Self::load(&root, &mut cache)?;
        Ok(FileBlockStore { root, cache })
    }

    /// Recursively walk `blocks/` and read every `<hex>` file's bytes into the
    /// cache. Files whose name is not a valid 32-byte hex id are skipped.
    fn load(root: &PathBuf, cache: &mut BTreeMap<Hash, Vec<u8>>) -> std::io::Result<()> {
        let blocks_dir = root.join("blocks");
        if !blocks_dir.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&blocks_dir)? {
            if !entry.is_dir() {
                continue;
            }
            for entry2 in fs::read_dir(&entry.path)? {
                if !entry2.is_dir() {
                    continue;
                }
                for entry3 in fs::read_dir(&entry2.path)? {
                    if !entry3.is_file() {
                        continue;
                    }
                    let id = match parse_hex32(&entry3.name) {
                        Ok(id) => id,
                        Err(_) => continue,
                    };
                    let bytes = match fs::read(&entry3.path) {
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
        if let Ok(mut f) = fs::open_file(&partial, fs::OpenMode::Read) {
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
    fn remove(&mut self, id: &Hash) -> Option<Vec<u8>> {
        let path = self.block_path(id);
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
        self.cache.remove(id)
    }
}

impl FileBlockStore {
    /// Total bytes physically retained (sum of unique block sizes on disk) —
    /// the real on-disk cost after dedup.
    pub fn stored_bytes(&self) -> u64 {
        self.cache.values().map(|v| v.len() as u64).sum()
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
                x = x
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (x >> 33) as u8
            })
            .collect()
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
        let _ = crate::vfs::remove_dir_all(&tmp);
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
        let _ = crate::vfs::remove_dir_all(&tmp);
    }

    /// DEDUP across a 1-byte edit, disk-backed, mirrors the MemStore property.
    #[test]
    fn fileblockstore_one_byte_edit_dedups_over_90pct() {
        let tmp = std::env::temp_dir().join(format!("fbs_dedup_{}", std::process::id()));
        let _ = crate::vfs::remove_dir_all(&tmp);
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
        let _ = crate::vfs::remove_dir_all(&tmp);
    }

    /// Re-backing identical content is 100% dedup, disk-backed.
    #[test]
    fn fileblockstore_identical_rebackup_fully_dedups() {
        let tmp = std::env::temp_dir().join(format!("fbs_reback_{}", std::process::id()));
        let _ = crate::vfs::remove_dir_all(&tmp);
        let data = sample(60_000);
        let mut organ = fbs_organ(&tmp);
        let (_m1, _s1) = organ.backup(&data);
        let store_len_after_first = organ.store().len();
        let (_m2, s2) = organ.backup(&data);
        assert_eq!(s2.new_blocks, 0, "re-backup must write no new blocks");
        assert_eq!(s2.dedup_ratio(), 1.0);
        assert_eq!(organ.store().len(), store_len_after_first);
        let _ = crate::vfs::remove_dir_all(&tmp);
    }

    /// Fail-closed restore: a missing block yields Err, disk-backed.
    #[test]
    fn fileblockstore_missing_block_fails_closed() {
        let tmp = std::env::temp_dir().join(format!("fbs_missing_{}", std::process::id()));
        let _ = crate::vfs::remove_dir_all(&tmp);
        let data = sample(40_000);
        let mut organ = fbs_organ(&tmp);
        let (mut manifest, _s) = organ.backup(&data);
        manifest.blocks.push([0xAB; 32]);
        manifest.total_len += 1;
        let err = organ.restore(&manifest).unwrap_err();
        assert_eq!(err, RestoreError::MissingBlock([0xAB; 32]));
        let _ = crate::vfs::remove_dir_all(&tmp);
    }

    /// Content-address integrity: a 1-bit on-disk corruption makes `get_owned`
    /// fail-closed (None), never return unverified bytes.
    #[test]
    fn fileblockstore_corrupt_block_rejected() {
        let tmp = std::env::temp_dir().join(format!("fbs_corrupt_{}", std::process::id()));
        let _ = crate::vfs::remove_dir_all(&tmp);
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
        let mut raw = crate::vfs::read(&path).expect("read block file");
        raw[0] ^= 0x01; // 1-bit flip
        crate::vfs::write(&path, &raw).expect("rewrite corrupted");

        // Corrupted block is rejected fail-closed.
        assert!(
            store.get_owned(&id).is_none(),
            "corrupted block must be rejected"
        );
        let _ = crate::vfs::remove_dir_all(&tmp);
    }

    /// Crash-atomicity invariant: a `.partial` left behind (simulating a kill-9
    /// between the partial write and the rename) is NOT visible as a block.
    /// `get_owned` must ignore the temp file and return None for that id.
    #[test]
    fn fileblockstore_partial_write_invisible() {
        let tmp = std::env::temp_dir().join(format!("fbs_partial_{}", std::process::id()));
        let _ = crate::vfs::remove_dir_all(&tmp);
        let block = vec![0x7u8; 2048];
        let id = crate::event_log::sha3_256(&block);
        let hex = hex_encode(&id);
        // Simulate an interrupted write: leave only a .partial, no final file.
        let tmp_dir = tmp.join("tmp");
        crate::vfs::create_dir_all(&tmp_dir).expect("create tmp dir");
        crate::vfs::write(tmp_dir.join(format!("{hex}.partial")), &block).expect("write partial");
        let store = FileBlockStore::open(&tmp).expect("open store");
        // The block must not be readable; get_owned sees only the final path.
        assert!(
            store.get_owned(&id).is_none(),
            "a .partial must never be visible as a stored block"
        );
        // And the blocks/ tree stays empty (no half-written file leaked).
        assert_eq!(store.len(), 0);
        let _ = crate::vfs::remove_dir_all(&tmp);
    }

    /// TORVALDS-14: the backup store must NOT panic when the underlying filesystem
    /// write fails (full disk / permission denied). It signals failure via the
    /// `bool` return so the caller can degrade instead of crashing the process.
    /// Prior to the fix, `put` `panic!`-ed on `create_dir_all`/`write`/`rename`
    /// failure — i.e. it died exactly when the disk was most likely to be full.
    #[test]
    fn fileblockstore_put_fails_without_panic_on_io_error() {
        let tmp = std::env::temp_dir().join(format!("fbs_ro_{}", std::process::id()));
        let _ = crate::vfs::remove_dir_all(&tmp);
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
        crate::vfs::write(&blocked, b"not-a-dir").expect("plant blocking file");
        let _guard = scopeguard_remove_all(&tmp);
        // Must return false (failure signalled), NOT panic.
        let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| store.put(id, &block)));
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
                let _ = crate::vfs::remove_dir_all(self.0);
            }
        }
        G(path)
    }
}
