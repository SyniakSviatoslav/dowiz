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
    /// v4 (W-ES-P8C): the tree writer's fanout, and the ONLY automatic compaction trigger of a
    /// v4 image -- superseded cells over live / `tree_dead_div` (default 1: arena <= ~2x live).
    pub tree_fanout: usize,
    pub tree_dead_div: i64,
    /// `create` writes an empty v4 map instead of the empty v2 schema (default false).
    pub create_tree: bool,
    /// W-ES-P11, `Durability::Normal`: the n-th commit staged since the last checkpoint
    /// flushes them all as one group (two barriers). Default 64; 0 = never automatically
    /// (`Db::flush` only).
    pub checkpoint_every: usize,
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
            tree_fanout: bebop_kv::tree::FANOUT,
            tree_dead_div: 1,
            create_tree: false,
            checkpoint_every: 64,
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
    /// The v4 writer's fanout and dead-cell compaction ratio.
    pub fn tree(mut self, fanout: usize, dead_div: i64) -> Options {
        self.tree_fanout = fanout;
        self.tree_dead_div = dead_div;
        self
    }
    pub fn create_tree(mut self, yes: bool) -> Options {
        self.create_tree = yes;
        self
    }
    pub fn compaction(mut self, max_deltas: usize, dead_div: i64) -> Options {
        self.max_deltas = max_deltas;
        self.dead_div = dead_div;
        self
    }
    /// `Durability::Normal`'s checkpoint trigger, in commits (0: only `Db::flush`).
    pub fn checkpoint_every(mut self, n: usize) -> Options {
        self.checkpoint_every = n;
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
