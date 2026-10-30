//! `FileDevice`: a store file written with positioned writes (`pwrite`) and `fdatasync`
//! barriers, held under an exclusive lock on `<path>.lock` (R-EMBED-STORE §3.3: the lock is
//! NOT on the store file itself, because a compaction publishes by `rename`, which changes
//! the inode a lock on the file would be attached to).
//!
//! The lock is `flock(LOCK_EX | LOCK_NB)` through `std::fs::File::try_lock` (std, no libc
//! crate: this crate keeps zero dependencies). flock locks belong to the OPEN FILE
//! DESCRIPTION, so a second `FileDevice::open` of the same path is refused even inside one
//! process -- which is what lets the test prove it without a fork.
//!
//! A PUBLISH (W-ES-HARD). A compaction is written into a sibling (`fresh`: `<path>.compact`).
//! Published SYNCED (the caller barriered the sibling: every durability but `None`) it is
//! renamed over the store and the directory fsynced: a crash leaves the old file whole or the
//! new one, which was durable before it got the name. Published DEFERRED (`Durability::None`:
//! the fdatasync was 80% of a 1e4 put pause) NOTHING is renamed: the old file keeps the name and
//! gets no more writes, the device goes on reading and writing the sibling, and the next
//! `barrier` syncs the sibling's data, renames it, fsyncs the directory. So a crash before that
//! barrier leaves exactly the last durable generation at the name -- the tail since it is lost,
//! never anything that was durable -- and `open`, which reads only `path`, ignores whatever
//! sibling is left (`fresh` truncates it). A clean `drop` finishes a pending publish.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::{self, Read, Seek, SeekFrom};
use std::os::unix::fs::FileExt;

use super::{Device, IoError};

/// A store file and its lock.
pub struct FileDevice {
    file: File,
    /// The store's name.
    path: String,
    /// The name `file` has now: `path`, or a sibling while a deferred publish is pending.
    cur: String,
    /// `None` for a `fresh` sibling: it lives under the lock of the device it will replace.
    _lock: Option<File>,
    /// A barrier was issued after the last write (`publish` reads it off the staged sibling).
    synced: bool,
    /// A deferred publish is waiting for the next `barrier` to rename `cur` over `path`.
    pending: bool,
}

/// `<path>.lock`.
pub fn lock_path(path: &str) -> String {
    format!("{path}.lock")
}

/// `<path>.compact`: where a compaction is written before it is published by rename.
pub fn compact_path(path: &str) -> String {
    format!("{path}.compact")
}

/// The sibling to write the next compaction into: `<path>.compact`, or `<path>.compact2` while
/// a deferred publish still lives in the first (two pending publishes never share a name).
fn sibling_path(path: &str, cur: &str) -> String {
    if cur == compact_path(path) {
        format!("{path}.compact2")
    } else {
        compact_path(path)
    }
}

/// The pid the holder wrote into the lock file, if it reads as one (W-ES-P3, `Locked{by}`).
fn holder_of(lock: &mut File) -> Option<u32> {
    let mut s = String::new();
    lock.seek(SeekFrom::Start(0)).ok()?;
    lock.read_to_string(&mut s).ok()?;
    s.trim().parse().ok()
}

impl FileDevice {
    /// Open an existing store file for writing, taking its lock first.
    /// `IoError::Locked { by }` when another handle (any process) holds it; `by` is the pid the
    /// holder wrote into the lock file. A sibling left by a crashed deferred publish is ignored.
    pub fn open(path: &str) -> Result<FileDevice, IoError> {
        let mut lock = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(lock_path(path))?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(IoError::Locked { by: holder_of(&mut lock) }),
            Err(TryLockError::Error(e)) => return Err(IoError::Io(e)),
        }
        // Who holds it, for the next opener's refusal. Not a lock by itself (flock is), and not
        // synced: a stale pid after a crash names a dead process, which is still the truth.
        lock.set_len(0)?;
        lock.write_all_at(std::process::id().to_string().as_bytes(), 0)?;
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        Ok(FileDevice { file, path: path.to_string(), cur: path.to_string(), _lock: Some(lock), synced: false, pending: false })
    }

    /// Create a fresh store file (`Store::create`: written, the file fsynced, then its
    /// directory) and open it.
    pub fn create(path: &str, size_bytes: usize) -> Result<FileDevice, IoError> {
        crate::Store::create(path, size_bytes)?;
        FileDevice::open(path)
    }

    /// The store file's path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Is a deferred publish waiting for the next barrier?
    pub fn pending_publish(&self) -> bool {
        self.pending
    }

    /// The deferred rename, after the sibling's data was synced: the name moves, the directory
    /// is fsynced.
    fn finish_publish(&mut self) -> io::Result<()> {
        std::fs::rename(&self.cur, &self.path)?;
        crate::sync_parent_dir(&self.path)?;
        self.cur = self.path.clone();
        self.pending = false;
        Ok(())
    }
}

impl Device for FileDevice {
    fn len_cells(&self) -> io::Result<usize> {
        Ok((self.file.metadata()?.len() / 8) as usize)
    }

    fn read_cells(&self, off: usize, n: usize) -> io::Result<Vec<i64>> {
        let buf = self.read_bytes(off, n)?;
        Ok(buf.chunks_exact(8).map(|w| i64::from_le_bytes([w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]])).collect())
    }

    fn read_bytes(&self, off: usize, n: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0u8; n * 8];
        let mut got = 0;
        // A short read is the end of the file: the rest stays zero, the format's null.
        while got < buf.len() {
            let r = self.file.read_at(&mut buf[got..], (off * 8 + got) as u64)?;
            if r == 0 {
                break;
            }
            got += r;
        }
        Ok(buf)
    }

    fn write_cells(&mut self, off: usize, cells: &[i64]) -> io::Result<()> {
        self.synced = false;
        self.file.write_all_at(&super::cells_to_bytes(cells), (off * 8) as u64)
    }

    /// fdatasync; then, for a deferred publish, the rename and the directory fsync -- data first.
    fn barrier(&mut self) -> io::Result<()> {
        self.file.sync_data()?;
        if self.pending {
            self.finish_publish()?;
        }
        self.synced = true;
        Ok(())
    }

    fn fresh(&self) -> io::Result<FileDevice> {
        let path = sibling_path(&self.path, &self.cur);
        let file = OpenOptions::new().read(true).write(true).create(true).truncate(true).open(&path)?;
        Ok(FileDevice { file, cur: path.clone(), path, _lock: None, synced: false, pending: false })
    }

    /// Synced sibling: rename it over the store, fsync the directory. Unsynced: rename nothing;
    /// read and write the sibling from now on and let the next `barrier` name it. A sibling
    /// still pending from an earlier deferred publish is superseded and removed.
    fn publish(&mut self, staged: FileDevice) -> io::Result<()> {
        // A duplicate of the sibling's handle (the same open file description; positions are
        // unused: every access is positioned), because `FileDevice` has a `Drop`.
        let file = staged.file.try_clone()?;
        if staged.synced {
            std::fs::rename(&staged.path, &self.path)?;
            crate::sync_parent_dir(&self.path)?;
        }
        if self.cur != self.path {
            std::fs::remove_file(&self.cur)?;
        }
        self.pending = !staged.synced;
        self.cur = if staged.synced { self.path.clone() } else { staged.path.clone() };
        self.synced = staged.synced;
        self.file = file;
        Ok(())
    }
}

impl Drop for FileDevice {
    /// A clean close finishes a deferred publish (data, rename, directory). A failure here
    /// cannot be reported and leaves the old file at the name: the last durable generation,
    /// whole; the tail is lost, as after a crash.
    fn drop(&mut self) {
        if self.pending && self.file.sync_data().is_ok() {
            let _ = self.finish_publish();
        }
    }
}

#[cfg(test)]
mod tests;
