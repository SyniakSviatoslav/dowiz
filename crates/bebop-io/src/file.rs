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
use std::io;
use std::os::unix::fs::FileExt;

use super::{Device, IoError};

/// A store file and its lock.
pub struct FileDevice {
    file: File,
    _lock: File,
}

/// `<path>.lock`.
pub fn lock_path(path: &str) -> String {
    format!("{path}.lock")
}

impl FileDevice {
    /// Open an existing store file for writing, taking its lock first.
    /// `IoError::Locked` when another handle (any process) holds it.
    pub fn open(path: &str) -> Result<FileDevice, IoError> {
        let lock = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(lock_path(path))?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(IoError::Locked),
            Err(TryLockError::Error(e)) => return Err(IoError::Io(e)),
        }
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        Ok(FileDevice { file, _lock: lock })
    }

    /// Create a fresh store file (`Store::create`: written, the file fsynced, then its
    /// directory) and open it.
    pub fn create(path: &str, size_bytes: usize) -> Result<FileDevice, IoError> {
        crate::Store::create(path, size_bytes)?;
        FileDevice::open(path)
    }
}

impl Device for FileDevice {
    fn len_cells(&self) -> io::Result<usize> {
        Ok((self.file.metadata()?.len() / 8) as usize)
    }

    fn read_cells(&self, off: usize, n: usize) -> io::Result<Vec<i64>> {
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
        Ok(buf.chunks_exact(8).map(|w| i64::from_le_bytes([w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]])).collect())
    }

    fn write_cells(&mut self, off: usize, cells: &[i64]) -> io::Result<()> {
        self.file.write_all_at(&super::cells_to_bytes(cells), (off * 8) as u64)
    }

    fn barrier(&mut self) -> io::Result<()> {
        self.file.sync_data()
    }
}

#[cfg(test)]
mod tests;
