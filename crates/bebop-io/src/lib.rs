//! W-ES-P2 (R-EMBED-STORE §3.5, §3.9, §5 row P2): the DEVICE a store is written through, the
//! commit protocol v2 (`plan`), the writer that owns the device (`writer`: recovery on open,
//! `Poisoned` after a failed barrier) and the file backend (`file`: pwrite + fdatasync and a
//! lock file). The crash harness that checks all of it is `crate::sim`.
//!
//! Declared in lib.rs as `iodev`, not `io`: lib.rs already binds `io` to `std::io`. W-ES-P1
//! moves this tree into `bebop-io`, where it is the crate root.
//!
//! WHAT DOES NOT CHANGE: the bytes. Every commit is still staged by `Store::stage_commit`
//! (via `commit_bytes`, `Kv::compacted_bytes`, `append_delta`); this module only decides the
//! ORDER the staged cells reach the device and where the barriers go. The goldens that pin
//! the image bytes are untouched by construction.

#![forbid(unsafe_code)]

#[allow(unused_imports)]
use bebop_image::*;
/// `crate::kv::...` keeps meaning bebop-kv (Writer::commit_delta, the harness).
use bebop_kv as kv;
/// The enumerated crash harness (W-ES-P2).
pub mod sim;
use std::io;

#[cfg(all(unix, not(target_arch = "wasm32")))]
pub mod file;
pub mod plan;
pub mod writer;

#[cfg(all(unix, not(target_arch = "wasm32")))]
pub use file::FileDevice;
pub use plan::{plan, Durable, Step};
pub use writer::Writer;

/// Where a store's cells live. A cell is an `i64`, little-endian on any byte device.
pub trait Device {
    /// Whole cells the device holds now.
    fn len_cells(&self) -> io::Result<usize>;
    /// `n` cells from `off`. A cell past the end reads 0, the format's null (a trimmed image).
    fn read_cells(&self, off: usize, n: usize) -> io::Result<Vec<i64>>;
    /// Write `cells` at `off`, growing the device if it is shorter. NOT durable until `barrier`.
    fn write_cells(&mut self, off: usize, cells: &[i64]) -> io::Result<()>;
    /// fdatasync: when this returns Ok, every write issued before it is durable. An Err means
    /// NOTHING is known about those writes (fsyncgate, ATC'20): the writer poisons itself.
    fn barrier(&mut self) -> io::Result<()>;
}

/// How far a flush goes (R-EMBED-STORE §3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durability {
    /// The writes, no barrier: consistent under a PROCESS crash (one page cache), not a power
    /// loss. `crate::sim` shows both halves of that sentence.
    None,
    /// The two-barrier protocol: power-loss safe.
    Data,
    /// `Data`; for a commit there is nothing more to sync. It differs from `Data` only at
    /// create/compact, where `Store::create` already fsyncs the file and its directory.
    Full,
}

/// Why the writer refused (the I/O half of §3.2's `Refusal`).
#[derive(Debug)]
pub enum IoError {
    /// The device said no.
    Io(io::Error),
    /// Another handle holds `<path>.lock` (`Refusal::Locked`).
    Locked,
    /// An earlier write or barrier failed; nothing more is written until `reopen`.
    Poisoned,
    /// Recovery refused the image.
    Recover(crate::sim::RecoverError),
    /// Staging refused (`append_delta`'s own refusals).
    Store(crate::StoreError),
    /// The in-memory image cannot be flushed over the durable one; `what` says why.
    Plan(&'static str),
}

impl From<io::Error> for IoError {
    fn from(e: io::Error) -> Self {
        IoError::Io(e)
    }
}

/// The image as a byte vector (little-endian cells), for `Store::from_bytes`.
pub(crate) fn cells_to_bytes(cells: &[i64]) -> Vec<u8> {
    cells.iter().flat_map(|c| c.to_le_bytes()).collect()
}

#[cfg(test)]
mod tests;
