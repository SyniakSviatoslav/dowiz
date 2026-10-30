//! W-ES-P2: the SIMULATED DISK, recovery, and the ENUMERATED crash harness
//! (R-EMBED-STORE §3.6, §4.3; operator Q1 2026-10-27: every crash point is named and
//! enumerated, nothing is drawn at random -- this is not a fuzzer).
//!
//! `SimDisk` is a `Device` that keeps the durable image it started from and RECORDS every
//! write and barrier the writer issues, in order. `crash` turns that record into every disk
//! state a crash could leave; `harness` recovers each state and checks the invariants;
//! `recover` is the open-time rule the library reader applies (§3.6).
//!
//! THE FAILURE MODEL AND THE LEAN ONE (bebop-lang formal/Bebop/Disk.lean, W-ES-P5). Lean's
//! `Crash`: every CELL independently keeps its durable value or takes a pending one. The
//! harness enumerates a SUBSET of those states -- 512-byte sectors (64 cells), each written
//! whole or not, plus at most one torn sector per state (half or, in `Tear::Cells`, any cut
//! at a cell) -- so every state it checks is a state the theorem covers. What differs:
//! (1) Lean quantifies over all per-cell mixes, the harness enumerates the sector-shaped ones
//! (and named classes above 6 sectors per window); (2) Lean's recovery is `pick` alone, the
//! harness checks `recover` (pick + anchor + fallback) and, with `Opts::pick_only`, pick alone;
//! (3) Lean ASSUMES crc15 rejects every torn superblock (`htorn`), the harness computes real
//! CRC-32 and would report a mix that passed; (4) the harness also covers group flushes and
//! `Durability::None`, which Lean does not model; (5) Lean proves the 3-barrier `rustOps` too,
//! the harness checks the 2-barrier v2 order the writer uses (`io::plan`).

use std::io;

use crate::Device;

pub mod crash;
pub mod harness;
pub mod recover;

pub use recover::{recover, walk, Hide, RecoverError, Recovered, Via, Why};

/// One operation the writer issued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimOp {
    Write { off: usize, cells: Vec<i64> },
    Barrier,
}

/// A disk that records what is written to it.
#[derive(Debug, Clone)]
pub struct SimDisk {
    base: Vec<i64>,
    ops: Vec<SimOp>,
    fail_barrier: Option<usize>,
    barriers: usize,
}

impl SimDisk {
    /// A disk whose durable contents are `base`.
    pub fn new(base: Vec<i64>) -> SimDisk {
        SimDisk { base, ops: Vec::new(), fail_barrier: None, barriers: 0 }
    }

    /// The `n`-th barrier (0-based) returns EIO, once (`FaultDevice` of T-ES-14). The writes
    /// before it stay in the record: after a failed fdatasync nobody knows what landed.
    pub fn failing_barrier(mut self, n: usize) -> SimDisk {
        self.fail_barrier = Some(n);
        self
    }

    /// The durable contents the record starts from.
    pub fn base(&self) -> &[i64] {
        &self.base
    }

    /// Every write and barrier since the disk was made (or last `settle`d).
    pub fn ops(&self) -> &[SimOp] {
        &self.ops
    }

    /// What a process reads: the base with every recorded write applied, in order.
    pub fn cache(&self) -> Vec<i64> {
        let mut c = self.base.clone();
        for op in &self.ops {
            if let SimOp::Write { off, cells } = op {
                apply(&mut c, *off, cells);
            }
        }
        c
    }

    /// Make the cache the new base and forget the record: what came before is durable.
    pub fn settle(&mut self) {
        self.base = self.cache();
        self.ops.clear();
    }
}

/// Write `cells` into `img` at `off`, growing it with zeros.
pub(crate) fn apply(img: &mut Vec<i64>, off: usize, cells: &[i64]) {
    if img.len() < off + cells.len() {
        img.resize(off + cells.len(), 0);
    }
    img[off..off + cells.len()].copy_from_slice(cells);
}

impl Device for SimDisk {
    fn len_cells(&self) -> io::Result<usize> {
        Ok(self.cache().len())
    }
    fn read_cells(&self, off: usize, n: usize) -> io::Result<Vec<i64>> {
        let c = self.cache();
        Ok((off..off + n).map(|i| c.get(i).copied().unwrap_or(0)).collect())
    }
    fn write_cells(&mut self, off: usize, cells: &[i64]) -> io::Result<()> {
        self.ops.push(SimOp::Write { off, cells: cells.to_vec() });
        Ok(())
    }
    fn barrier(&mut self) -> io::Result<()> {
        let n = self.barriers;
        self.barriers += 1;
        if self.fail_barrier == Some(n) {
            return Err(io::Error::from_raw_os_error(5)); // EIO
        }
        self.ops.push(SimOp::Barrier);
        Ok(())
    }
    /// An empty disk (W-ES-P3: a compaction is written into one).
    fn fresh(&self) -> io::Result<SimDisk> {
        Ok(SimDisk::new(Vec::new()))
    }
    /// A rename: the staged disk's contents become this disk's durable contents at once. The
    /// crash model of the rename itself (the old file or the new, never a mix) is the
    /// filesystem's; this disk takes it as given.
    fn publish(&mut self, staged: SimDisk) -> io::Result<()> {
        self.base = staged.cache();
        self.ops.clear();
        Ok(())
    }
}

/// A device that stops after `budget` operations (writes and barriers): every later one
/// fails. A PROCESS crash at a named point over a real device (T-ES-24); proves nothing
/// about power loss, which is `crash`'s job.
pub struct Cut<D: Device> {
    pub inner: D,
    pub budget: usize,
}

impl<D: Device> Cut<D> {
    fn spend(&mut self) -> io::Result<()> {
        if self.budget == 0 {
            return Err(io::Error::other("cut: the simulated process died here"));
        }
        self.budget -= 1;
        Ok(())
    }
}

impl<D: Device> Device for Cut<D> {
    fn len_cells(&self) -> io::Result<usize> {
        self.inner.len_cells()
    }
    fn read_cells(&self, off: usize, n: usize) -> io::Result<Vec<i64>> {
        self.inner.read_cells(off, n)
    }
    fn write_cells(&mut self, off: usize, cells: &[i64]) -> io::Result<()> {
        self.spend()?;
        self.inner.write_cells(off, cells)
    }
    fn barrier(&mut self) -> io::Result<()> {
        self.spend()?;
        self.inner.barrier()
    }
}

#[cfg(test)]
pub(crate) mod testkit;
#[cfg(test)]
mod tests;
#[cfg(all(test, feature = "tree"))]
mod tree_tests;
