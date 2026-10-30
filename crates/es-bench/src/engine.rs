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

/// How a `commit` reaches the device (R-EMBED-STORE §12.1: SQLite in the mode that fits the row).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sync {
    /// No barrier (SQLite `synchronous=OFF`): the `put` / `delete` rows.
    None,
    /// W-ES-P8E, SQLite `synchronous=NORMAL` in WAL: a commit never tears, nothing is synced
    /// until a checkpoint, a power cut loses the commits since the last one. bebop: W-ES-P11's
    /// `Durability::Normal` (checkpoint every 64th commit); SQLite: `synchronous=NORMAL`;
    /// redb: `Durability::None` with an `Immediate` every 64th commit (it has no NORMAL);
    /// LMDB: `NO_SYNC` without `WRITEMAP` keeps ACI and loses only D, which is its NORMAL --
    /// and also its `None`, so its two rows measure the same thing.
    Normal,
    /// Every commit synced before it returns (SQLite `synchronous=FULL`): `put_durable`.
    Full,
}

impl Sync {
    pub fn durable(d: bool) -> Sync {
        if d { Sync::Full } else { Sync::None }
    }
}

pub trait Engine: Sized {
    /// Short label used in every result line.
    const NAME: &'static str;
    /// A fresh store in `dir` (empty or absent) holding `entries` (sorted by key), written in
    /// one transaction and synced to disk before returning.
    fn load(dir: &Path, entries: &[(Vec<u8>, Vec<u8>)]) -> Self;
    /// Open what `load` left, with every later `commit` reaching the device as `sync` says.
    fn open(dir: &Path, sync: Sync) -> Self;
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
