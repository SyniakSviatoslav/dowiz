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
//!   5 VERSION=5  (the KV readers' cell)                6 ref SPEC   M5 (0)
//!   7 VERSION=5  (the log readers' cell)               8 ref ANCH   M6 (0)
//!   9 reserved 0
//! ```
//!
//! Every ref points BACK (FORMAT §9.1 v4's rule): a commit allocates record -> KEY -> KEYS ->
//! SEAL -> HEADS -> LOG root -> DOC last. `write.rs` is the writer, `verify.rs` the counters.
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

pub mod keys;
pub mod seal;
mod verify;
mod write;
pub use keys::{Key, KeyOp, ALG_ML_DSA_65};
pub use seal::{Head, Seal};
pub use verify::{verify, Counters, Report, Trust};

/// The root-version a DOC root carries in cells 5 and 7. KV 1-4 are taken (FORMAT §9.1).
pub const ROOT_VERSION_DOC: i64 = 5;
pub const DOC_CELLS: i64 = 10;
pub const LAYOUT_DOC: &str = "DOC{i64,ref EVLOG,ref KV,ref HEADS,ref KEYS,i64,ref SPEC,i64,ref ANCH,i64}";
/// `st_digest(LAYOUT_DOC)`: the low 32 bits of sha256, big-endian (`doc/tests.rs` re-derives).
pub const DIGEST_DOC: i64 = 3669646107;

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

/// The writer's half of a device key: WHO signs. `actor_id` is `sha256(public_key)` by the
/// implementor's convention (32 bytes, the record's `actor_pubkey` slot); the crate never
/// derives it.
pub trait Signer {
    fn actor_id(&self) -> [u8; 32];
    fn alg(&self) -> i64;
    fn public_key(&self) -> Vec<u8>;
    fn sign(&self, msg: &[u8]) -> Vec<u8>;
}

/// The reader's half: does `sig` over `msg` verify under `pk` for algorithm `alg`? An
/// algorithm the implementor does not know is `false`, never a guess.
pub trait Verifier {
    fn verify(&self, alg: i64, pk: &[u8], msg: &[u8], sig: &[u8]) -> bool;
}

/// How a record's id is derived (M0 owns the scheme; this crate only asks). `legacy` is the
/// pre-cascade scheme of old images (`None` = the implementor has none); `redacted` reads the
/// in-place redaction mark; `body` strips the implementor's framing (nonce, kind byte) so the
/// key-op bodies of `keys.rs` are found where the implementor put them.
pub trait Ids {
    fn chained(&self, prev: &[u8; 32], payload: &[u8]) -> [u8; 32];
    fn legacy(&self, payload: &[u8]) -> Option<[u8; 32]> {
        let _ = payload;
        None
    }
    fn redacted(&self, payload: &[u8]) -> bool {
        let _ = payload;
        false
    }
    fn body<'a>(&self, payload: &'a [u8]) -> &'a [u8] {
        payload
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
