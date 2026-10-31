// SPDX-License-Identifier: MIT OR Apache-2.0
//! The bebop image, opened: every entry of a KV (v1-v4), every record of an EvLog (v1-v2), or a
//! DOC (W-MG-M5: its log's records, its heads, its SPEC) -- checked as the library checks them,
//! held so a host can ask for them one at a time (`abi.rs`).
//!
//! THE RULES ARE THE LIBRARY'S, CALLED, NOT COPIED. `pick_checked_in` (FORMAT §3.1),
//! `last_commit_in` (FORMAT §7, the last-commit rule), `Kv::load_checked` (v1-v3: the crc of the
//! root, its four arrays and every delta record, then the decode), `TreeIn::verify` (v4: the crc
//! of every reachable node and value, rules 1-5), `EvLog::chain_crc` (the root and every record),
//! `Doc::open` + `doc::verify` (DOC, FORMAT §9.4: the root, its log chain, HEADS, KEYS, and the
//! eight counters). `crates/bebop-wasm` carries SHORT COPIES of the first two because its byte
//! ratchet may only fall; this crate has a ratchet of its own and is not under that pressure, so
//! it links the normative rules and the answers are the native conformance reader's by
//! construction (`crates/bebop-wasm/fixtures/conform/src/main.rs` `read`, held equal on every
//! fixture by `tests/conformance.rs`).
//!
//! Status vocabulary: docs/STATUS-CODES.md, through `Refusal` -> `abi::status_of`. The two folds
//! are the format's: KV = FNV-1a 64 over `len8 || key || len8 || value` in key order
//! (`Kv::snapshot_root_u64`, `TreeIn::fold`); LOG = FNV-1a 64 over `seq8 || len8 || payload`
//! oldest first (the fold `oracle.py` and bebop-wasm print for a log; FORMAT has no other). A
//! DOC's root is its LOG's fold: its records are the kept history, the tree is derived state.
//!
//! EVERYTHING REFUSES, NOTHING PANICS: every claim in the image is bounded by the library
//! before it is followed, and this layer adds no index of its own.
//!
//! WHAT A DOC READ DOES NOT SAY: this crate carries no signature algorithm (`ids.rs`), so the
//! seals are counted, never checked -- `DocCounts::checked` is 0 and the host must print that.

pub mod abi;
mod abi_doc;
mod doc;
pub mod ids;
pub mod sha256;
pub mod status;

use bebop_image::format::pick_checked_in;
use bebop_image::verify::{last_commit_in, LastCommit};
use bebop_image::{Store, Superblock, ARENA};
use bebop_kv::tree::{TreeError, TreeIn, VERSION_TREE};
use bebop_kv::verify::KvError;
use bebop_kv::{Kv, FNV_OFFSET, FNV_PRIME};
use bebop_log::doc::{Anchor, Doc as DocRoot, Head, SpecView};
pub use doc::DocCounts;
use doc::open_doc;
use bebop_log::{EvLog, Record, DIGEST_EVLOG_ROOT};

/// Why an image was not opened. Each variant is one status code (`abi::status_of`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// No valid superblock, or the live one refused structurally (FORMAT §3.1): status 1.
    NoSuperblock,
    /// A superblock but no root, or a KV claim that does not hold: status 2.
    NotAKv,
    /// Asked for the log of an image whose root is not a log root: status 3.
    NotALog,
    /// The log's root claims `claimed` records; the chain delivers `chained` (`None` = a cycle): 4.
    Truncated { claimed: usize, chained: Option<usize> },
    /// An object the read reached fails its crc: status 10, naming the object.
    BadCrc { obj: usize },
    /// The KV root names a version above 4 that is not a DOC: status 12.
    UnknownVersion,
    /// The last commit's range does not hash to its anchor and every object in it checks: 41.
    LastCommit,
    /// A DOC's segment (records that left the file) is named by no TRUNCATE on the kept chain: 75.
    CutUndeclared,
}

/// What the root turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A KV root; `version` is 1..=4 as the root says it.
    Kv { version: i64 },
    /// An EvLog root (v1 or v2).
    Log,
    /// A DOC root (root-version 5, FORMAT §9.4).
    Doc,
}

/// An opened image. The entries (KV) or records (LOG / DOC) are owned copies, so the host may
/// free the image's bytes after `open`.
pub struct Doc {
    pub kind: Kind,
    /// The family's fold, as the two's-complement i64 every reader prints.
    pub fold: i64,
    /// KV: every entry in key order (empty for a log). DOC: the TREE's entries when it has a v4 tree.
    pub entries: Vec<(Vec<u8>, Vec<u8>)>,
    /// LOG / DOC: every record OLDEST FIRST (empty for a KV).
    pub records: Vec<Record>,
    /// DOC: the HEADS rows (empty otherwise).
    pub heads: Vec<Head>,
    /// DOC: its SPEC, when the file describes itself.
    pub spec: Option<SpecView>,
    /// DOC: its time anchors, newest first (present, never verified here: `abi_doc::bw_anchors`).
    pub anchors: Vec<Anchor>,
    pub(crate) counts: Option<DocCounts>,
    pub(crate) st: Store,
    sb: Superblock,
}

fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// Open the image the bytes hold: the superblock, the last commit, then the family the root's
/// digest names. The order is the native conformance reader's, so the status is too; a DOC is
/// asked for FIRST (`Doc::is_doc`: digest, length and both version cells), because to the KV
/// path its version cell reads "above 4".
pub fn open(bytes: &[u8]) -> Result<Doc, Refusal> {
    let st = Store::from_bytes(bytes);
    let sb = pick_checked_in(&st).map_err(|_| Refusal::NoSuperblock)?;
    match last_commit_in(&st, &sb) {
        Ok(_) => {}
        Err(LastCommit::Object(b)) => return Err(Refusal::BadCrc { obj: b.obj }),
        Err(LastCommit::Anchor(_)) => return Err(Refusal::LastCommit),
    }
    let root = st.root().ok_or(Refusal::NotAKv)?;
    if DocRoot::is_doc(&st, root) {
        return open_doc(st, sb);
    }
    if st.obj_digest(root) == DIGEST_EVLOG_ROOT {
        return open_log(st, sb);
    }
    open_kv(st, sb, root)
}

pub(crate) fn blank(kind: Kind, fold: i64, st: Store, sb: Superblock) -> Doc {
    Doc { kind, fold, entries: Vec::new(), records: Vec::new(), heads: Vec::new(), spec: None, anchors: Vec::new(), counts: None, st, sb }
}

/// The KV families. The root's crc first, then its version, exactly as `Kv::load_checked`
/// orders them; v4 goes to `TreeIn` on every target (`v4_here()` is false on wasm32).
fn open_kv(st: Store, sb: Superblock, root: usize) -> Result<Doc, Refusal> {
    st.check_obj(root).map_err(|b| Refusal::BadCrc { obj: b.obj })?;
    let version = Kv::version(&st);
    let (entries, fold) = if version == VERSION_TREE {
        tree_entries(&st, root)?
    } else {
        let kv = Kv::load_checked(&st).map_err(|e| match e {
            KvError::NotKv => Refusal::NotAKv,
            KvError::BadCrc(b) => Refusal::BadCrc { obj: b.obj },
            KvError::UnknownVersion { .. } => Refusal::UnknownVersion,
            KvError::Tree(t) => tree_refusal(t),
        })?;
        let fold = kv.snapshot_root_u64();
        (kv.entries.into_iter().map(|(k, v)| (k.into_bytes(), v)).collect(), fold)
    };
    Ok(Doc { entries, ..blank(Kind::Kv { version }, fold as i64, st, sb) })
}

/// A v4 tree at `root`: every reachable node and value checked, the entries, the fold.
pub(crate) fn tree_entries(st: &Store, root: usize) -> Result<(Vec<(Vec<u8>, Vec<u8>)>, u64), Refusal> {
    let t = TreeIn::at(st, root).map_err(tree_refusal)?;
    t.verify().map_err(tree_refusal)?;
    let fold = t.fold().map_err(tree_refusal)?;
    Ok((t.entries().map_err(tree_refusal)?, fold))
}

/// A v4 refusal in the one vocabulary: a crc is 10, every structural rule is NOT_A_KV (2),
/// the word a v3 chain that does not hold gets (conform `read`, STATUS-CODES §3).
fn tree_refusal(e: TreeError) -> Refusal {
    match e {
        TreeError::BadCrc(b) => Refusal::BadCrc { obj: b.obj },
        _ => Refusal::NotAKv,
    }
}

/// The LOG fold over records oldest first.
pub(crate) fn log_fold(records: &[Record]) -> i64 {
    let mut h = FNV_OFFSET;
    for r in records {
        h = fnv1a(h, &r.actor_seq.to_le_bytes());
        h = fnv1a(h, &(r.payload.len() as u64).to_le_bytes());
        h = fnv1a(h, &r.payload);
    }
    h as i64
}

/// The records of the log rooted at `log` (`None` = the image's root): the root and every
/// record's crc in one walk, the count held to the root's claim, then oldest first.
pub(crate) fn log_records(st: &Store, log: Option<usize>) -> Result<Vec<Record>, Refusal> {
    let claimed = EvLog::len_at(st, log);
    let chained = EvLog::chain_crc_at(st, log).map_err(|b| Refusal::BadCrc { obj: b.obj })?;
    if chained != Some(claimed) {
        return Err(Refusal::Truncated { claimed, chained });
    }
    let mut records = EvLog::walk_until_at(st, log, |_| false);
    records.reverse();
    Ok(records)
}

fn open_log(st: Store, sb: Superblock) -> Result<Doc, Refusal> {
    let records = log_records(&st, st.root())?;
    Ok(Doc { fold: log_fold(&records), records, ..blank(Kind::Log, 0, st, sb) })
}

impl Doc {
    /// Entries of a KV or records of a log / DOC.
    pub fn len(&self) -> usize {
        match self.kind {
            Kind::Kv { .. } => self.entries.len(),
            Kind::Log | Kind::Doc => self.records.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The value under `key` (KV, or a DOC's v4 tree; `None` for a log too).
    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        let i = self.entries.binary_search_by(|(k, _)| k.as_slice().cmp(key)).ok()?;
        self.entries.get(i).map(|(_, v)| v.as_slice())
    }

    /// `Verify::WholeArena` over what `open` already proved: every object of `[ARENA,
    /// arena_used)`, in allocation order, hashes to its header crc. `open` checked what the
    /// root REACHES (`Verify::Reachable`); an older generation's dead object is only met here.
    pub fn verify_arena(&self) -> Result<(), Refusal> {
        let used = (self.sb.arena_used.max(0) as usize).min(self.st.cells.len());
        let mut i = ARENA;
        while i < used {
            self.st.check_obj(i).map_err(|b| Refusal::BadCrc { obj: b.obj })?;
            i = i.saturating_add(2).saturating_add(self.st.obj_len(i).max(0) as usize);
        }
        Ok(())
    }

    /// The live superblock `open` picked.
    pub fn superblock(&self) -> &Superblock {
        &self.sb
    }
}
