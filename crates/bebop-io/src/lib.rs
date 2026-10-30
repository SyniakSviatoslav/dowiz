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
/// W-ES-P3 (R-EMBED-STORE §3.2): the library API over a device -- `Db`, `ReadTxn`, `WriteTxn`,
/// `Scan`, `Refusal`, and `Detached` for an image whose bytes the caller persists.
pub mod db;
pub mod detached;
pub mod refusal;
mod snap;
pub mod txn;

pub use bebop_kv::{Batch, Bytes, LimitError, Limits};
pub use db::{Db, Generation, OnTorn, Options, Report, Verify};
pub use detached::Detached;
pub use refusal::Refusal;
pub use snap::SnapImage;
pub use txn::{ReadTxn, Scan, WriteTxn};

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
    /// The most this device can promise (W-ES-P3). A commit that asks for more is REFUSED
    /// (`Refusal::Durability`), never silently downgraded. `Detached` says `None`.
    fn durability(&self) -> Durability {
        Durability::Full
    }
    /// An empty device of the same kind beside this one, for a compaction to write into
    /// (`FileDevice`: `<path>.compact`). Default: this device cannot compact into a new one.
    fn fresh(&self) -> io::Result<Self>
    where
        Self: Sized,
    {
        Err(io::Error::new(io::ErrorKind::Unsupported, "this device has no fresh sibling"))
    }
    /// Make `staged` -- fully written and, for a durable device, barriered by the caller --
    /// REPLACE this device atomically (`FileDevice`: fdatasync, rename, fsync of the directory;
    /// the lock stays). Readers holding the old image keep it (R-EMBED-STORE §3.3).
    fn publish(&mut self, staged: Self) -> io::Result<()>
    where
        Self: Sized,
    {
        let _ = staged;
        Err(io::Error::new(io::ErrorKind::Unsupported, "this device cannot publish a replacement"))
    }
}

/// How far a flush goes (R-EMBED-STORE §3.5). Ordered: `None < Normal < Data < Full`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Durability {
    /// The writes, no barrier: consistent under a PROCESS crash (one page cache), not a power
    /// loss. `crate::sim` shows both halves of that sentence.
    None,
    /// W-ES-P11, SQLite `synchronous=NORMAL`'s promise: a commit is STAGED and published, and
    /// NOTHING reaches the device until a checkpoint (`Db::flush`, or every
    /// `Options::checkpoint_every`-th commit) writes every staged commit as one group under two
    /// barriers. A power cut loses the commits since the last checkpoint and never tears: the
    /// device always holds exactly a checkpoint. For `Writer::flush` itself `Normal` IS `Data`
    /// (a checkpoint is a `Data` flush); the batching is `Db`'s.
    Normal,
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
    /// Another handle holds `<path>.lock` (`Refusal::Locked`): `by` is the pid the holder
    /// wrote into it, when it could be read (W-ES-P3).
    Locked { by: Option<u32> },
    /// The newest generation's last commit failed its proof and the holder asked not to fall
    /// back past it (`OnTorn::Refuse`, W-ES-P3: an image from an atomic backend).
    Torn(crate::sim::Why),
    /// An earlier write or barrier failed; nothing more is written until `reopen`.
    Poisoned,
    /// Recovery refused the image.
    Recover(crate::sim::RecoverError),
    /// Staging refused (`append_delta`'s own refusals).
    Store(crate::StoreError),
    /// The in-memory image cannot be flushed over the durable one; `what` says why.
    Plan(&'static str),
    /// A v4 tree commit was refused before anything was staged (W-ES-P8C, `Writer::commit_tree`).
    Tree(bebop_kv::tree::WriteError),
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
