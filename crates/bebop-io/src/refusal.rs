//! `Refusal`: every reason the library says no, named, with its numbers (W-ES-P3;
//! R-EMBED-STORE §3.2, §3.9). `#[non_exhaustive]`, so a later reason (Q4's `Superseded`, P4's
//! status codes) is not a breaking change. `Clone + PartialEq`, so a test names the exact one:
//! an `io::Error` is kept as its kind and OS code.

use std::fmt;
use std::io;

use bebop_kv::verify::KvError;
use bebop_kv::tree::{TreeError, WriteError};
use bebop_kv::LimitError;

use crate::sim::{RecoverError, Why};
use crate::{BadCrc, Durability, IoError, PickError, StoreError};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Refusal {
    // ── the image (open, verify, read) ──
    /// No superblock could be picked, or the live one was refused structurally.
    Pick(PickError),
    /// The newest generation's last commit failed, and so did the fallback.
    Recover { live: Why, other: Why },
    /// The newest generation's last commit failed its proof and `OnTorn::Refuse` was asked:
    /// the anchor does not hold, or an object of the last commit is cut short or fails its crc.
    LastCommit(Why),
    /// An object whose payload does not hash to its header's crc.
    BadCrc(BadCrc),
    /// The object at this cell, met walking the whole arena (`Verify::WholeArena`), is cut short
    /// or fails its crc.
    ObjectBad { at: usize },
    /// No KV root, or a slice of it does not fit its array.
    NotKv,
    /// A KV root version this code does not know.
    UnknownVersion(i64),
    /// A v4 tree (W-ES-P8C) refused by the tree reader or writer, which names why.
    Tree(TreeError),
    /// A v4 tree commit over an image that is not v4 (`v` = its version): v1-v3 move to v4 only
    /// through the explicit `Db::migrate_to_tree`.
    NotTree(i64),
    /// A cell the write path was about to trust disagrees with the image (`StoreError::Corrupt`).
    Corrupt(&'static str),
    // ── limits ──
    KeyTooLong { len: usize, max: usize },
    ValueTooLong { len: usize, max: usize },
    ArenaFull { need: i64, capacity: i64 },
    /// A format limit (`StoreError::Limit`): the generation, an object's length, ...
    Limit { what: &'static str, got: i64, max: i64 },
    TooSmall { bytes: usize, floor: usize },
    // ── concurrency and lifecycle ──
    /// A write transaction is already open on this `Db` (in this process).
    WriterBusy,
    /// Another handle (another process) holds the store's lock; `by` = its pid, when readable.
    Locked { by: Option<u32> },
    /// An earlier write or barrier failed, or a write transaction panicked: nothing more is
    /// written until `Db::reopen`.
    Poisoned,
    /// `Db::create` on a device that already holds a KV image.
    Exists,
    // ── the device ──
    Io { kind: io::ErrorKind, os: Option<i32> },
    /// The commit asked for more durability than the device can give.
    Durability { asked: Durability, device: Durability },
    /// The staged image cannot be flushed over the durable one (`IoError::Plan`).
    Plan(&'static str),
}

impl From<io::Error> for Refusal {
    fn from(e: io::Error) -> Refusal {
        Refusal::Io { kind: e.kind(), os: e.raw_os_error() }
    }
}

impl From<PickError> for Refusal {
    fn from(e: PickError) -> Refusal {
        Refusal::Pick(e)
    }
}

impl From<RecoverError> for Refusal {
    fn from(e: RecoverError) -> Refusal {
        match e {
            RecoverError::Pick(p) => Refusal::Pick(p),
            RecoverError::Both { live, other } => Refusal::Recover { live, other },
        }
    }
}

impl From<KvError> for Refusal {
    fn from(e: KvError) -> Refusal {
        match e {
            KvError::NotKv => Refusal::NotKv,
            KvError::UnknownVersion { v } => Refusal::UnknownVersion(v),
            KvError::BadCrc(b) => Refusal::BadCrc(b),
            KvError::Tree(TreeError::BadCrc(b)) => Refusal::BadCrc(b),
            KvError::Tree(t) => Refusal::Tree(t),
        }
    }
}

impl From<LimitError> for Refusal {
    fn from(e: LimitError) -> Refusal {
        match e {
            LimitError::KeyTooLong { len, max } => Refusal::KeyTooLong { len, max },
            LimitError::ValueTooLong { len, max } => Refusal::ValueTooLong { len, max },
        }
    }
}

impl From<StoreError> for Refusal {
    fn from(e: StoreError) -> Refusal {
        match e {
            StoreError::NoSuperblock => Refusal::Pick(PickError::NoValidSuperblock),
            StoreError::ArenaFull { need, capacity } => Refusal::ArenaFull { need, capacity },
            StoreError::Io(e) => Refusal::from(e),
            StoreError::Corrupt(what) => Refusal::Corrupt(what),
            StoreError::TooSmall { bytes, floor } => Refusal::TooSmall { bytes, floor },
            StoreError::Image(p) => Refusal::Pick(p),
            StoreError::Limit { what, got, max } => Refusal::Limit { what, got, max },
        }
    }
}

impl From<IoError> for Refusal {
    fn from(e: IoError) -> Refusal {
        match e {
            IoError::Io(e) => Refusal::from(e),
            IoError::Locked { by } => Refusal::Locked { by },
            IoError::Torn(why) => Refusal::LastCommit(why),
            IoError::Poisoned => Refusal::Poisoned,
            IoError::Recover(r) => Refusal::from(r),
            IoError::Store(s) => Refusal::from(s),
            IoError::Plan(what) => Refusal::Plan(what),
            IoError::Tree(w) => Refusal::from(w),
        }
    }
}

impl From<WriteError> for Refusal {
    fn from(e: WriteError) -> Refusal {
        match e {
            WriteError::Store(s) => Refusal::from(s),
            WriteError::Tree(TreeError::BadCrc(b)) => Refusal::BadCrc(b),
            WriteError::Tree(t) => Refusal::Tree(t),
            WriteError::NotTree { v } => Refusal::NotTree(v),
            WriteError::Kv(k) => Refusal::from(k),
            // The writer's own preconditions: a caller here broke them, never the image.
            WriteError::Unsorted => Refusal::Plan("tree ops not strictly ascending"),
            WriteError::Fanout(_) => Refusal::Plan("tree fanout outside MIN_FANOUT..=MAX_FANOUT"),
            WriteError::NotFresh => Refusal::Plan("a bulk load into a store that has a root"),
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for Refusal {}
