//! The one surface every engine is driven through. Each method is ONE transaction where the
//! engine has transactions: `get_many` and `prefix_many` are one read transaction around the
//! whole batch, `commit` is one write transaction. That is deliberate and stated in the report:
//! under this box's proot every fcntl/fstat costs ~140-273 us, so a per-get transaction would
//! measure the box's syscall tax, not the engine (R-STORE-VS-SQL-2026-10-27 §2).
use std::path::Path;

/// One change inside a commit.
#[derive(Clone, Copy, Debug)]
pub enum W<'a> {
    Put(&'a [u8], &'a [u8]),
    Del(&'a [u8]),
}

pub trait Engine: Sized {
    /// Short label used in every result line.
    const NAME: &'static str;
    /// A fresh store in `dir` (empty or absent) holding `entries` (sorted by key), written in
    /// one transaction and synced to disk before returning.
    fn load(dir: &Path, entries: &[(Vec<u8>, Vec<u8>)]) -> Self;
    /// Open what `load` left. `durable`: every later `commit` is synced before it returns.
    fn open(dir: &Path, durable: bool) -> Self;
    /// Sum of `acc_of(value)` over `keys` (all present), in one read transaction.
    fn get_many(&self, keys: &[Vec<u8>]) -> u64;
    /// Sum over every key/value in each prefix range of `value.len() + key[14]`, one read txn.
    fn prefix_many(&self, prefixes: &[Vec<u8>]) -> u64;
    /// One write transaction.
    fn commit(&mut self, ops: &[W]);
    /// The engine's own way of giving space back. Returns a note on what it did.
    fn compact(&mut self) -> String;
    /// The engine's full integrity check, if it has one: `None` = it offers none.
    fn verify(&mut self) -> Option<bool>;
    /// Counters worth printing next to a write row (bebop: deltas/compactions).
    fn note(&self) -> String {
        String::new()
    }
}
