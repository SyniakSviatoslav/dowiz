//! `FileDevice`: a store file written with positioned writes (`pwrite`) and `fdatasync`
//! barriers, held under an exclusive lock on `<path>.lock` (R-EMBED-STORE §3.3: the lock is
//! NOT on the store file itself, because a compaction publishes by `rename`, which changes
//! the inode a lock on the file would be attached to).
//!
//! The lock is `flock(LOCK_EX | LOCK_NB)` through `std::fs::File::try_lock` (std, no libc
//! crate: this crate keeps zero dependencies). flock locks belong to the OPEN FILE
//! DESCRIPTION, so a second `FileDevice::open` of the same path is refused even inside one
//! process -- which is what lets the test prove it without a fork.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::{self, Read, Seek, SeekFrom};
use std::os::unix::fs::FileExt;

use super::{Device, IoError};

/// A store file and its lock.
pub struct FileDevice {
    file: File,
    path: String,
    /// `None` for a `fresh` sibling: it lives under the lock of the device it will replace.
    _lock: Option<File>,
}

/// `<path>.lock`.
pub fn lock_path(path: &str) -> String {
    format!("{path}.lock")
}

/// `<path>.compact`: where a compaction is written before it is published by rename.
pub fn compact_path(path: &str) -> String {
    format!("{path}.compact")
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
    /// holder wrote into the lock file.
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
        Ok(FileDevice { file, path: path.to_string(), _lock: Some(lock) })
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
        self.file.write_all_at(&super::cells_to_bytes(cells), (off * 8) as u64)
    }

    fn barrier(&mut self) -> io::Result<()> {
        self.file.sync_data()
    }

    fn fresh(&self) -> io::Result<FileDevice> {
        let path = compact_path(&self.path);
        let file = OpenOptions::new().read(true).write(true).create(true).truncate(true).open(&path)?;
        Ok(FileDevice { file, path, _lock: None })
    }

    /// fdatasync the staged file, rename it over this one, fsync the directory, and read from
    /// the new inode from now on. A crash before the rename leaves the old file whole; after it,
    /// the new one, which was durable before it got the name (R-EMBED-STORE §3.3).
    fn publish(&mut self, staged: FileDevice) -> io::Result<()> {
        staged.file.sync_data()?;
        std::fs::rename(&staged.path, &self.path)?;
        crate::sync_parent_dir(&self.path)?;
        self.file = staged.file;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
