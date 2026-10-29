//! `Db`'s options and the small types its API names (W-ES-P3): `Generation`, `Verify`, `OnTorn`,
//! `Options`, `Report`. Split out of `db.rs` (the 300-line rule).

use bebop_kv::delta::{DEAD_DIV, MAX_DELTAS};
use bebop_kv::Limits;

use crate::sim::Via;
use crate::MAX_PAD_CELLS;

/// A generation number. Monotonic across commits AND compactions (design D3b).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Generation(pub i64);

impl Generation {
    pub fn get(self) -> i64 {
        self.0
    }
}

/// How much of an image is proved (R-EMBED-STORE §12 P10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verify {
    /// Pick, the PartTab's crc, and the anchor over the last commit (recovery): O(last commit).
    /// Older commits are trusted -- a damaged old object reads as a refusal or a wrong answer,
    /// never a panic -- until `Db::verify` asks for more.
    Anchor,
    /// `Anchor` + the crc of every object the KV root reaches (root, four arrays, every delta
    /// record) + the key order: what `Kv::load_checked` / `KvIn::open` prove.
    Reachable,
    /// `Reachable` + every object of the arena walked and its crc checked.
    WholeArena,
}

/// What to do when the newest generation's last commit fails its proof (design D3a).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnTorn {
    /// A file: a power cut tore it; open the previous generation (FORMAT §3.1, §3.6).
    Fallback,
    /// An atomic backend (dowiz's Durable Object): a torn image is corruption; refuse it by name.
    Refuse,
}

/// How a `Db` opens and writes. Built from `Options::default()` / `Options::detached()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Options {
    /// What `open` proves (default `Anchor`).
    pub verify: Verify,
    /// Default `Fallback`; `Options::detached()` says `Refuse`.
    pub on_torn: OnTorn,
    /// Key and value limits, never above the format's.
    pub limits: Limits,
    /// The largest image a compaction may grow to (doubling from 16 KiB).
    pub max_image_bytes: usize,
    /// The image `create` starts with.
    pub initial_bytes: usize,
    /// The compaction trigger: the longest delta chain, and dead cells over live / `dead_div`.
    pub max_deltas: usize,
    pub dead_div: i64,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            verify: Verify::Anchor,
            on_torn: OnTorn::Fallback,
            limits: Limits::FORMAT,
            max_image_bytes: MAX_PAD_CELLS * 8,
            initial_bytes: 16 * 1024,
            max_deltas: MAX_DELTAS,
            dead_div: DEAD_DIV,
        }
    }
}

impl Options {
    /// For an image whose bytes the caller persists atomically: a torn last commit is refused.
    pub fn detached() -> Options {
        Options { on_torn: OnTorn::Refuse, ..Options::default() }
    }
    pub fn verify(mut self, v: Verify) -> Options {
        self.verify = v;
        self
    }
    pub fn on_torn(mut self, t: OnTorn) -> Options {
        self.on_torn = t;
        self
    }
    pub fn limits(mut self, l: Limits) -> Options {
        self.limits = l.lowered();
        self
    }
    pub fn max_image_bytes(mut self, n: usize) -> Options {
        self.max_image_bytes = n;
        self
    }
    pub fn initial_bytes(mut self, n: usize) -> Options {
        self.initial_bytes = n;
        self
    }
    pub fn compaction(mut self, max_deltas: usize, dead_div: i64) -> Options {
        self.max_deltas = max_deltas;
        self.dead_div = dead_div;
        self
    }
}

/// What `Db::verify` proved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    pub generation: Generation,
    pub depth: Verify,
    /// How recovery proved the last commit.
    pub via: Via,
    /// Objects whose crc was checked beyond recovery's (`WholeArena`: the whole arena).
    pub objects: usize,
}
