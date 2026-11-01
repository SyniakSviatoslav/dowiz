//! THE DOC ROOT (W-MG-M1, R-MAINGOAL-A §1.2, phase M1): one root object over a log, a tree,
//! the table of heads, the table of keys -- and, per device commit, ONE ML-DSA-65 seal over
//! the tip. Old images are untouched: a DOC is a new root layout with its own version, and
//! the format's rule (FORMAT §9: an unknown version is a refusal, never a guess) is what
//! keeps every reader written before it honest.
//!
//! ── WHERE THE VERSION SITS, AND WHY NOT IN CELL 0 ──
//!
//! Every one of the five KV readers (bebop-kv `Kv::version`, bebop-io `snap.rs`, oracle.py,
//! kv.bp `pj_kv_known`, the C ABI through bebop-io) reads the ROOT's PAYLOAD CELL 5 as the
//! KV version and refuses `> 4` as UNKNOWN_VERSION (12). None of them checks the root's
//! digest. The log readers read cell 7 (`EvLog::version`). So the root-version number is ONE
//! namespace across layouts -- every reader reads the same slot -- and a DOC root carries
//! `ROOT_VERSION_DOC` = 5 in BOTH cells: a KV reader refuses it by name, and so does a log
//! reader that asks `EvLog::unknown_version`. The research plan's `{VERSION=1, ...}` at cell 0
//! would have been read as KV v1 (cell 5 = `ref SPEC` = 0) and DECODED.
//!
//! ```text
//! DOC  v1 (root-version 5), 10 cells, digest st_digest(LAYOUT_DOC)
//!   0 FLAGS      bit0 = the log's ids are sha256 (M0); bit1 = tree == fold(log);
//!                bit2 = self-describing (SPEC != 0, M5)
//!   1 ref LOG    the EvLog v2 root (0 = no log)       2 ref TREE   a KV v3/v4 root (0 = none)
//!   3 ref HEADS  `seal::Head`s                         4 ref KEYS   `keys::Key`s
//!   5 VERSION=5  (the KV readers' cell)                6 ref SPEC   `spec.rs` (0 in a working image)
//!   7 VERSION=5  (the log readers' cell)               8 ref ANCH   the newest anchor (M6, `anch.rs`)
//!   9 ref SEG    the newest log segment (`seg.rs`, W-MG-M3; 0 = the log was never cut)
//! ```
//!
//! Every ref points BACK (FORMAT §9.1 v4's rule): a commit allocates record -> KEY -> KEYS ->
//! SEAL -> HEADS -> LOG root -> DOC last. `write.rs` is the writer, `verify.rs` the counters,
//! `cut.rs` the one way history leaves the file (a SEG over the cut ids, declared by a signed
//! TRUNCATE record -- R-MAINGOAL-A §2b: one proof mechanism, never a second structure).
//!
//! Cell 9 was "reserved 0" in the M1 layout (digest 3669646107, 2026-10-30 morning); it is
//! `ref SEG` from W-MG-M3 the same day. No DOC image older than that exists; the readers that
//! refuse a DOC do so by the VERSION cells, which did not move.
//!
//! ── THE SEAMS ──
//!
//! This crate is MIT OR Apache-2.0 and holds NO signature or hash code. Who signs, who checks,
//! and how a record's id is derived are three traits below; dowiz-hub (AGPL, where
//! `pq::dsa` and `chain.rs` live) implements them. Nothing here names ML-DSA beyond the
//! number `keys::ALG_ML_DSA_65`.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::{Store, StoreError};
pub use bebop_image::verify::BadCrc;

pub mod anch;
mod budget;
#[cfg(test)] mod tam_tests;
mod copy;
mod count;
mod cut;
mod declare;
mod forgotten;
mod frontier;
pub mod keys;
mod merge;
pub mod merge_body;
pub mod seal;
mod seals;
pub mod seg;
mod seams;
mod settle;
pub mod spec;
mod verify;
mod write;
pub use anch::{check_anchors, Anchor, AnchorCheck, AnchorCounts, Checked, Verdict, KIND_OTS, KIND_PLATFORM_TSA, KIND_ROUGHTIME};
pub use keys::{Key, KeyOp, ALG_ML_DSA_65};
pub use merge::Union;
pub use merge_body::{merge_body, parse_merge, MergeView, Row, MERGE_MARK, STATE_COMMUTES, STATE_OBSTRUCTED};
pub use seal::{Head, Seal};
pub use seams::{id_message, Ids, Signer, Verifier, ID_TAG};
pub use seg::{forgotten_body, Erased, Segment};
pub use spec::{SpecIn, SpecView, DIGEST_BLOB, DIGEST_SPEC};
pub use verify::{verify, Counters, Report, Trust};

/// The root-version a DOC root carries in cells 5 and 7. KV 1-4 are taken (FORMAT §9.1).
pub const ROOT_VERSION_DOC: i64 = 5;
pub const DOC_CELLS: i64 = 10;
pub const LAYOUT_DOC: &str = "DOC{i64,ref EVLOG,ref KV,ref HEADS,ref KEYS,i64,ref SPEC,i64,ref ANCH,ref SEG}";
/// `st_digest(LAYOUT_DOC)`: the low 32 bits of sha256, big-endian (`doc/dsa_tests.rs`
/// re-derives). 3669646107 was the M1 layout with cell 9 reserved.
pub const DIGEST_DOC: i64 = 2619226223;

pub const FLAG_SHA256_IDS: i64 = 1;
pub const FLAG_TREE_IS_FOLD: i64 = 2;
pub const FLAG_SELF_DESCRIBING: i64 = 4;

pub(crate) const C_FLAGS: usize = 0;
pub(crate) const C_LOG: usize = 1;
pub(crate) const C_TREE: usize = 2;
pub(crate) const C_HEADS: usize = 3;
pub(crate) const C_KEYS: usize = 4;
pub(crate) const C_VERSION: usize = 5;
pub(crate) const C_SPEC: usize = 6;
pub(crate) const C_VERSION_LOG: usize = 7;
pub(crate) const C_ANCH: usize = 8;
pub(crate) const C_SEG: usize = 9;

/// Why a DOC operation refused.
#[derive(Debug)]
pub enum DocError {
    /// The root is not a DOC root (digest, length, or the version cells).
    NotDoc,
    Store(StoreError),
    BadCrc(BadCrc),
    /// The record's `actor_pubkey` is not the signer's actor id.
    Actor,
    /// The KEYS table has no key for this actor: nothing could ever check its seal.
    UnknownActor([u8; 32]),
    /// The actor's key is revoked after `after_seq`; a seal at `seq` would be invalid.
    Revoked { after_seq: i64, seq: u64 },
    /// The device chain is strictly sequential: `want` is the next seq, `got` what came.
    Seq { want: u64, got: u64 },
    /// A table the commit must carry forward is damaged; `what` names it.
    Corrupt(&'static str),
    /// A segment's records are gone from the log and no effective TRUNCATE names its root
    /// (W-MG-M3's kill: a sealed segment dropped without the signed cut).
    CutUndeclared { root: [u8; 32] },
    /// `keep` is not smaller than the log: there is nothing to cut.
    NothingToCut,
    /// The cut would take the ADMIT record of a key still in KEYS; that key's proof lives
    /// in the record, so the record stays. Cut up to it.
    CutsAdmit([u8; 32]),
    /// The cut ends at `seq` of this device, between two records one seal covers. Cut to a seal's
    /// edge: `keep` counts records, and a sync lays many under one seal.
    CutInsideSeal { actor: [u8; 32], seq: u64 },
    /// A REVOKE after this `seq` names a tip that is not this device's record there.
    RevokeTip { actor: [u8; 32], seq: u64 },
    /// The cut or the redaction would take a REVOKE record of this actor's key; the
    /// revocation is that record, so the record stays. Cut up to it.
    TakesRevoke([u8; 32]),
    /// An ADMIT of a key already revoked here: a REVOKE is final, a new device takes a new key.
    Readmit([u8; 32]),
    /// The caller's `Ids` has no `tree_root`: a segment has no root to be declared by.
    NoTree,
    /// The DOC names an object this cut does not carry (`what`: "TREE", "SPEC", "ANCH"); the
    /// layer that owns it moves it to the fresh file (R-MAINGOAL-A §2b: compaction of the
    /// tree never touches the log, and the log's cut never touches the tree).
    Carries(&'static str),
    /// One device, two histories: `(actor, seq)` holds a different id on each side. Most often a
    /// device restored from a backup; a restored device takes a new key (M2 design §5.6).
    Fork { actor: [u8; 32], seq: u64, ours: [u8; 32], theirs: [u8; 32] },
    /// The two files do not name one set of genesis keys (or one id scheme): not one document.
    ForeignRoot,
    /// One actor id, two different keys on the two sides.
    KeyClash([u8; 32]),
    /// The two files do not carry one segment chain; `root` is the newest either names. M2-min.
    CutOneSide { root: [u8; 32] },
    /// A tombstone, or a FORGOTTEN, one side does not have: a merge never brings the erased text
    /// back and never spreads an erasure unasked. M2-min (design §5.2).
    ForgetOneSide,
    /// Records of `actor` past a REVOKE the other side holds. M2-min (design §5.3).
    RevokedTail { actor: [u8; 32], after_seq: i64 },
    /// A record to take in is under a legacy id, which proves nothing (design §5.5).
    LegacyTail { actor: [u8; 32], seq: u64 },
    /// A file to merge does not hold (`Report::holds`): `ours` says which. There is nothing to
    /// glue in a file that fails on its own, and a union that fails is not written either.
    NotWhole { ours: bool },
    /// Each side holds and their union does not: what only the two together show, an actor keyed on
    /// one side whose records on the other were never judged (`merge.rs`).
    UnionNotWhole,
    /// One key, two seal chains over the same records. HEADS keeps one chain and a seal signs the
    /// tip before it, so a merge would lose a signature; a person picks. `seq`: where they part.
    SealFork { actor: [u8; 32], seq: u64 },
}

/// `StoreError` carries an `io::Error` and derives no `PartialEq`; two `Store` refusals are
/// equal when they print the same, which is what a test asserting on one can see.
impl PartialEq for DocError {
    fn eq(&self, o: &DocError) -> bool {
        match (self, o) {
            (DocError::Store(a), DocError::Store(b)) => format!("{a:?}") == format!("{b:?}"),
            (DocError::NotDoc, DocError::NotDoc) | (DocError::Actor, DocError::Actor) => true,
            (DocError::BadCrc(a), DocError::BadCrc(b)) => a == b,
            (DocError::UnknownActor(a), DocError::UnknownActor(b)) => a == b,
            (DocError::Revoked { after_seq: a, seq: s }, DocError::Revoked { after_seq: b, seq: t }) => a == b && s == t,
            (DocError::Seq { want: a, got: s }, DocError::Seq { want: b, got: t }) => a == b && s == t,
            (DocError::Corrupt(a), DocError::Corrupt(b)) => a == b,
            (DocError::CutUndeclared { root: a }, DocError::CutUndeclared { root: b }) => a == b,
            (DocError::CutsAdmit(a), DocError::CutsAdmit(b)) | (DocError::TakesRevoke(a), DocError::TakesRevoke(b)) | (DocError::Readmit(a), DocError::Readmit(b)) => a == b,
            (DocError::Carries(a), DocError::Carries(b)) => a == b,
            (DocError::Fork { actor: a, seq: s, ours: o, theirs: t }, DocError::Fork { actor: b, seq: u, ours: p, theirs: q }) => a == b && s == u && o == p && t == q,
            (DocError::KeyClash(a), DocError::KeyClash(b)) => a == b,
            (DocError::CutOneSide { root: a }, DocError::CutOneSide { root: b }) => a == b,
            (DocError::RevokedTail { actor: a, after_seq: s }, DocError::RevokedTail { actor: b, after_seq: t }) => a == b && s == t,
            (DocError::LegacyTail { actor: a, seq: s }, DocError::LegacyTail { actor: b, seq: t }) => a == b && s == t,
            (DocError::NotWhole { ours: a }, DocError::NotWhole { ours: b }) => a == b,
            (DocError::SealFork { actor: a, seq: s }, DocError::SealFork { actor: b, seq: t }) => a == b && s == t,
            (DocError::UnionNotWhole, DocError::UnionNotWhole) => true,
            (DocError::RevokeTip { actor: a, seq: s }, DocError::RevokeTip { actor: b, seq: t }) => a == b && s == t,
            (DocError::CutInsideSeal { actor: a, seq: s }, DocError::CutInsideSeal { actor: b, seq: t }) => a == b && s == t,
            (DocError::ForeignRoot, DocError::ForeignRoot) | (DocError::ForgetOneSide, DocError::ForgetOneSide) => true,
            (DocError::NothingToCut, DocError::NothingToCut) | (DocError::NoTree, DocError::NoTree) => true,
            _ => false,
        }
    }
}

impl From<StoreError> for DocError {
    fn from(e: StoreError) -> Self {
        DocError::Store(e)
    }
}
impl From<BadCrc> for DocError {
    fn from(e: BadCrc) -> Self {
        DocError::BadCrc(e)
    }
}

/// A DOC root as read: its flags and the objects it names (absolute offsets).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocView {
    pub at: usize,
    pub flags: i64,
    pub log: Option<usize>,
    pub tree: Option<usize>,
    pub heads: Option<usize>,
    pub keys: Option<usize>,
    pub spec: Option<usize>,
    pub anch: Option<usize>,
    /// The newest SEG (`seg.rs`); `None` = the log was never cut.
    pub seg: Option<usize>,
}

/// The DOC layout's handle; the writer is `write.rs`, the counters `verify.rs`.
pub struct Doc;

impl Doc {
    /// Is the object at `root` a DOC root? Digest, length and BOTH version cells must say so.
    pub fn is_doc(st: &Store, root: usize) -> bool {
        st.obj_digest(root) == DIGEST_DOC
            && st.obj_len(root) == DOC_CELLS
            && st.obj_cells(root) == DOC_CELLS as usize
            && st.get(root, C_VERSION) == ROOT_VERSION_DOC
            && st.get(root, C_VERSION_LOG) == ROOT_VERSION_DOC
    }

    /// The image's root as a DOC, its crc checked. `NotDoc` for any other root (an old KV or
    /// log image), so a caller that opens both kinds asks this first.
    pub fn open(st: &Store) -> Result<DocView, DocError> {
        let root = st.root().ok_or(DocError::NotDoc)?;
        if !Self::is_doc(st, root) {
            return Err(DocError::NotDoc);
        }
        st.check_obj(root)?;
        Ok(DocView {
            at: root,
            flags: st.get(root, C_FLAGS),
            log: st.follow(root, C_LOG),
            tree: st.follow(root, C_TREE),
            heads: st.follow(root, C_HEADS),
            keys: st.follow(root, C_KEYS),
            spec: st.follow(root, C_SPEC),
            anch: st.follow(root, C_ANCH),
            seg: st.follow(root, C_SEG),
        })
    }
}

/// Little-endian bytes <-> cells, 8 to a cell (the record payload's packing, reused for keys
/// and signatures).
pub(crate) fn bytes_to_cells(b: &[u8]) -> Vec<i64> {
    crate::pack_payload(b)
}
pub(crate) fn cells_to_bytes(c: &[i64], len: usize) -> Vec<u8> {
    crate::unpack_payload(c, len)
}
/// 32 bytes as 4 cells and back (`lib.rs` keeps the one definition).
pub(crate) fn id_to_cells(b: &[u8; 32]) -> [i64; 4] {
    crate::b32_to_cells(b)
}
/// `id_to_cells` for a test outside this crate that names an id's cells (dowiz-hub's kill
/// tests rewrite a record's id the way a forger would).
pub fn id_cells(b: &[u8; 32]) -> [i64; 4] {
    id_to_cells(b)
}
/// Fewer than four cells (a cut object) read as zero cells, never as an index past the end.
pub(crate) fn cells_to_id(c: &[i64]) -> [u8; 32] {
    let mut four = [0i64; 4];
    for (d, s) in four.iter_mut().zip(c) {
        *d = *s;
    }
    crate::cells_to_b32(&four)
}
/// `n` payload cells of `obj` from `at`, bounded by what the object really holds.
pub(crate) fn cells_of(st: &Store, obj: usize, at: usize, n: usize) -> Vec<i64> {
    let have = st.obj_cells(obj).saturating_sub(at).min(n);
    (0..have).map(|i| st.get(obj, at + i)).collect()
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod dsa_tests;
#[cfg(test)]
mod lean_tests;
