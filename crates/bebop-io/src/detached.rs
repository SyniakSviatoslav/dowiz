//! `Detached`: a device with NO storage (W-ES-P3).
//!
//! For an image whose bytes the CALLER persists -- dowiz's Durable Object, which diffs the image
//! against its 96 KiB chunks and writes the changed ones in one atomic `put` (workers/api
//! `hubdo/atomic.rs`, W-ATOMIC). The image lives in the `Db`; `Db::snapshot_bytes` /
//! `ReadTxn::image` hand it out. Writes and barriers go nowhere, so the device promises
//! `Durability::None` and a commit asking for more is refused, never quietly accepted.
//! It holds nothing to read: a `Db` over it is made from bytes (`Db::from_bytes`) or fresh
//! (`Db::create_detached`), never by reading it.

use std::io;

use crate::{Device, Durability};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Detached;

impl Device for Detached {
    fn len_cells(&self) -> io::Result<usize> {
        Ok(0)
    }
    fn read_cells(&self, _off: usize, n: usize) -> io::Result<Vec<i64>> {
        Ok(vec![0; n])
    }
    fn write_cells(&mut self, _off: usize, _cells: &[i64]) -> io::Result<()> {
        Ok(())
    }
    fn barrier(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn durability(&self) -> Durability {
        Durability::None
    }
    fn fresh(&self) -> io::Result<Detached> {
        Ok(Detached)
    }
    fn publish(&mut self, _staged: Detached) -> io::Result<()> {
        Ok(())
    }
}
