//! The PATH API of `Store` (feature `std`): create a store file durably, read one, and commit
//! to one in protocol order. Moved verbatim out of lib.rs by W-ES-P1; the order of the writes in
//! `commit` is the one formal/Bebop/Disk.lean models (`rustOps`). bebop-io's `Writer` is the
//! device-generic writer; this is the original one, kept for the callers that name a path.

#[allow(unused_imports)]
use crate::prelude::*;
use std::io;
use std::io::{Seek, SeekFrom, Write};

use crate::{crc32_cells, Store, StoreError, Tx, ARENA, MAGIC, MIN_BYTES, PARTTAB_CELLS, SB_A};

impl From<io::Error> for StoreError {
    fn from(e: io::Error) -> Self { StoreError::Io(e) }
}

impl Store {
    /// Create a FRESH store file of `size_bytes`, initialised exactly as `st_open` does on a
    /// file with no valid superblock: superblock A only, generation 0, root 0, cursor at the
    /// first arena cell, capacity = size/8 - 1024.
    #[allow(clippy::indexing_slicing)] // T-ES-06 inventory (W-ES-P1): a pre-split index site; this list may only shrink
    pub fn create(path: &str, size_bytes: usize) -> io::Result<Self> {
        if size_bytes < MIN_BYTES {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("a store needs {MIN_BYTES} bytes, not {size_bytes}")));
        }
        let n = size_bytes / 8;
        let mut st = Store { cells: vec![0i64; n] };
        let capacity = (n - ARENA) as i64;
        st.cells[SB_A] = MAGIC;
        st.cells[SB_A + 1] = 2;
        st.cells[SB_A + 2] = 0;
        st.cells[SB_A + 3] = 0;
        st.cells[SB_A + 4] = ARENA as i64;
        st.cells[SB_A + 12] = capacity;
        st.cells[SB_A + 15] = crc32_cells(&st.cells, SB_A, 15) as i64;
        let mut buf = Vec::with_capacity(size_bytes);
        for c in &st.cells { buf.extend_from_slice(&c.to_le_bytes()); }
        // DURABLY (W-ES-P0, D4): `std::fs::write` alone left the file's bytes and its
        // directory entry in the page cache, so after a power loss the file could be empty or
        // absent while the caller had been told it exists. The file is synced, then its
        // directory (a new name is durable only once the directory is).
        let mut f = std::fs::File::create(path)?;
        f.write_all(&buf)?;
        f.sync_all()?;
        sync_parent_dir(path)?;
        Ok(st)
    }

    /// Read a store file from disk.
    #[allow(clippy::indexing_slicing)] // T-ES-06 inventory (W-ES-P1): a pre-split index site; this list may only shrink
    pub fn open(path: &str) -> io::Result<Self> {
        let bytes = std::fs::read(path)?;
        let n = bytes.len() / 8;
        let mut cells = Vec::with_capacity(n);
        for i in 0..n {
            let mut w = [0u8; 8];
            w.copy_from_slice(&bytes[i * 8..i * 8 + 8]);
            cells.push(i64::from_le_bytes(w));
        }
        Ok(Store { cells })
    }
    /// Commit to disk in protocol order: new objects, then the PartTab page, then the
    /// superblock LAST. A crash between any two leaves generation k-1 intact.
    pub fn commit(&mut self, tx: &Tx, root: usize, path: &str) -> Result<i64, StoreError> {
        let (pt, osb) = self.stage_commit(tx, root);
        let mut f = std::fs::OpenOptions::new().write(true).open(path)?;
        self.write_cells(&mut f, tx.mark as usize, (tx.cursor - tx.mark) as usize)?;
        f.sync_data()?;
        self.write_cells(&mut f, pt, PARTTAB_CELLS as usize + 2)?;
        f.sync_data()?;
        self.write_cells(&mut f, osb, 16)?;
        f.sync_data()?;
        Ok(tx.next_gen)
    }

    #[allow(clippy::indexing_slicing)] // T-ES-06 inventory (W-ES-P1): a pre-split index site; this list may only shrink
    fn write_cells(&self, f: &mut std::fs::File, off: usize, n: usize) -> io::Result<()> {
        if n == 0 { return Ok(()); }
        let mut buf = Vec::with_capacity(n * 8);
        for i in 0..n {
            buf.extend_from_slice(&self.cells[off + i].to_le_bytes());
        }
        f.seek(SeekFrom::Start((off * 8) as u64))?;
        f.write_all(&buf)
    }
}
/// fsync the directory holding `path`, so a file just created there survives a power loss.
pub fn sync_parent_dir(path: &str) -> io::Result<()> {
    let dir = match std::path::Path::new(path).parent() {
        Some(d) if !d.as_os_str().is_empty() => d,
        _ => std::path::Path::new("."),
    };
    std::fs::File::open(dir)?.sync_all()
}
