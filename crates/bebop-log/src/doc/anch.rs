//! ANCH (W-MG-M1B, R-MAINGOAL-A §3 and §7 phase M6): the level-1 TIME ANCHORS of a DOC --
//! evidence, kept IN THE FILE as an opaque proof blob, that a record id existed no later
//! than some moment, from a source outside the file. Three kinds are named; the proofs are
//! blobs this crate never interprets, the adapters that make and check them live outside the
//! core (dowiz-hub `doc_seal/{tsa,ots,roughtime}.rs`), and a verifier checks an anchor from
//! the file plus PUBLIC KEYS or public block headers, never from a live service (§2d: QLDB's
//! proofs died with its service; Ceramic's anchor provider could censor).
//!
//! ```text
//! ANCH  v1  {VERSION=1, kind, over 4, time_ms, ref PREV_ANCH, proof_len, proof packed}
//!           9 + ceil(proof_len / 8) cells; DOC cell 8 names the NEWEST, each names the one
//!           before it (0 for the first) -- a chain like the seals', so an anchor is never
//!           lost by a later one.
//! ```
//!
//! `over` is the 32-byte thing anchored: a record id of this DOC's log (its tip when the
//! anchor was taken). An anchor whose `over` names no record of the log binds nothing and
//! `check_anchors` counts it bad before any adapter sees it -- UNLESS the log has been cut
//! (W-MG-M3b, `seg.rs`): then `over` may name a record that left under a segment, which the
//! file can no longer show. Such an anchor is `under_cut`: it is carried by every cut and
//! copy, listed and counted, its proof still run (a forged proof is still `Bad`), but it is
//! never `Ok` here -- the verdict is `Unchecked` naming the segments, because the one thing
//! it would prove ("this id was in the log") needs the archive: the id's membership under a
//! segment's root (`Ids::tree_root` over the archived ids) is what places it. `time_ms` is
//! what the WRITER says the anchor's moment was -- a claim, repeated; the proof is what the
//! adapter checks.
//!
//! The kinds, in the order the card asked for them:
//!   1 PLATFORM TSA MARK   ML-DSA-65 over `"bebop-anchor/1" ‖ kind ‖ over ‖ time_ms` by the
//!                         platform's key; proves "no later than time_ms, by dowiz's word";
//!                         checked against the platform keys the AUDITOR brings.
//!   2 OPENTIMESTAMPS      the `.ots` file for digest = `over`; proves "no later than Bitcoin
//!                         block N" once the ops are evaluated to the block's Merkle root and
//!                         that root is compared with a block header the auditor brings.
//!   3 ROUGHTIME           the signed response to nonce = `over`; "no EARLIER than MIDP −
//!                         RADI" (the only lower bound the file can carry).
//!
//! What a `Verdict::Ok` says, and does not: the holder of a key, or the Bitcoin chain, saw
//! `over` by then. NOT that the event happened, not that `claimed_ms` or `time_ms` is right.
//! A kind this verifier has no adapter for is `Unchecked`, with the reason -- never `Ok`
//! (card: no anchor verdict without a real check).

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::collections::BTreeSet;
use crate::{EvLog, Store, StoreError, Tx};
use super::seg::read_segs;
use super::write::{stage_doc, Refs};
use super::{bytes_to_cells, cells_of, cells_to_bytes, cells_to_id, id_to_cells, Doc, DocError};

pub const ANCH_VERSION: i64 = 1;
pub const LAYOUT_ANCH: &str = "ANCH{i64,i64,i64,i64,i64,i64,i64,ref ANCH,i64,[i64]}";
/// `st_digest(LAYOUT_ANCH)` (`doc/dsa_tests.rs` re-derives it beside the other five).
pub const DIGEST_ANCH: i64 = 180762881;
pub const KIND_PLATFORM_TSA: i64 = 1;
pub const KIND_OTS: i64 = 2;
pub const KIND_ROUGHTIME: i64 = 3;
const ANCH_HEAD: usize = 9;
/// The longest proof an ANCH may carry: 1 MiB. A TSA mark is 5 261 B, an `.ots` under 10 KB,
/// a Roughtime response about 1 KB; the cap keeps a damaged `proof_len` from becoming an
/// allocation.
pub const MAX_PROOF_BYTES: usize = 1 << 20;

/// One anchor as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    pub at: usize,
    pub kind: i64,
    pub over: [u8; 32],
    pub time_ms: i64,
    pub prev: Option<usize>,
    pub proof: Vec<u8>,
}

/// What an adapter found in one anchor's proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The proof checks. `not_after_ms` / `not_before_ms` are the bounds it gives on when
    /// `over` existed (an upper bound from a TSA mark or a block, a lower bound from
    /// Roughtime); `what` names what was checked and against which key or block.
    Ok { not_after_ms: Option<i64>, not_before_ms: Option<i64>, what: String },
    /// The proof was checked and does NOT hold: a forged or damaged anchor.
    Bad(String),
    /// Nothing was checked, and here is why (no adapter, no key given, the proof is still
    /// pending at its calendar, a hash this build does not have). Never a pass.
    Unchecked(String),
}

/// The reader's half of an anchor kind. The DOC never asks a live service; what the
/// implementor needs beyond the file (public keys, block headers) it holds itself.
pub trait AnchorCheck {
    fn check(&self, a: &Anchor) -> Verdict;
}

/// No adapters at all: every anchor is `Unchecked`. What a reader with no keys prints.
pub struct NoAdapters;
impl AnchorCheck for NoAdapters {
    fn check(&self, a: &Anchor) -> Verdict {
        Verdict::Unchecked(format!("no adapter for anchor kind {} in this verifier", a.kind))
    }
}

/// One anchor, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    pub anchor: Anchor,
    /// Does `over` name a LIVE record of this log? `false` = the anchor binds nothing the
    /// file can show: bad, unless `under_cut`.
    pub names_record: bool,
    /// `over` names no live record and the log has segments: only the archive can place
    /// it (W-MG-M3b). Counted `unchecked`, never `ok`; `bad` only when its proof fails.
    pub under_cut: bool,
    pub verdict: Verdict,
}

/// The anchor counters, beside `verify`'s eight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AnchorCounts {
    pub anchors: usize,
    pub ok: usize,
    pub bad: usize,
    pub unchecked: usize,
    /// Of the `unchecked` (or `bad`): anchors over an id that left the log under a segment.
    pub cut: usize,
}

impl AnchorCounts {
    pub fn of(checked: &[Checked]) -> Self {
        let mut c = AnchorCounts { anchors: checked.len(), ..Default::default() };
        for x in checked {
            match x.verdict {
                Verdict::Ok { .. } => c.ok += 1,
                Verdict::Bad(_) => c.bad += 1,
                Verdict::Unchecked(_) => c.unchecked += 1,
            }
            if x.under_cut {
                c.cut += 1;
            }
        }
        c
    }
}

/// One ANCH object; `None` for a wrong digest, version or length. The crc is the caller's
/// question (`check_anchors` refuses the chain on one).
pub fn read_anchor(st: &Store, at: usize) -> Option<Anchor> {
    if st.obj_digest(at) != DIGEST_ANCH || st.obj_cells(at) < ANCH_HEAD || st.get(at, 0) != ANCH_VERSION {
        return None;
    }
    let proof_len = st.get(at, 8);
    if !(0..=MAX_PROOF_BYTES as i64).contains(&proof_len) {
        return None;
    }
    let n_cells = (proof_len as usize).div_ceil(8);
    if st.obj_cells(at) != ANCH_HEAD + n_cells {
        return None;
    }
    Some(Anchor {
        at,
        kind: st.get(at, 1),
        over: cells_to_id(&cells_of(st, at, 2, 4)),
        time_ms: st.get(at, 6),
        prev: st.follow(at, 7),
        proof: cells_to_bytes(&cells_of(st, at, ANCH_HEAD, n_cells), proof_len as usize),
    })
}

/// How many anchors a chain may have before it is "never ends": one per possible object.
fn chain_cap(st: &Store) -> usize {
    st.cells.len() / (ANCH_HEAD + 2) + 1
}

/// The anchors from `newest` back, NEWEST FIRST, stopping at a damaged object or the cap.
/// No crc here; `check_anchors` is the checked read.
pub fn read_anchors(st: &Store, newest: Option<usize>) -> Vec<Anchor> {
    let mut out = Vec::new();
    let cap = chain_cap(st);
    let mut cur = newest;
    while let Some(at) = cur {
        if out.len() >= cap {
            break;
        }
        let Some(a) = read_anchor(st, at) else { break };
        cur = a.prev;
        out.push(a);
    }
    out
}

pub(crate) fn stage_anchor(
    st: &mut Store,
    tx: &mut Tx,
    kind: i64,
    over: &[u8; 32],
    time_ms: i64,
    prev: Option<usize>,
    proof: &[u8],
) -> Result<usize, StoreError> {
    let pc = bytes_to_cells(proof);
    let o = st.alloc(tx, (ANCH_HEAD + pc.len()) as i64, DIGEST_ANCH)?;
    st.put_cell(o, 0, ANCH_VERSION);
    st.put_cell(o, 1, kind);
    for (i, c) in id_to_cells(over).iter().enumerate() {
        st.put_cell(o, 2 + i, *c);
    }
    st.put_cell(o, 6, time_ms);
    match prev {
        Some(p) => st.link(o, 7, p),
        None => st.put_cell(o, 7, 0),
    }
    st.put_cell(o, 8, proof.len() as i64);
    for (j, c) in pc.iter().enumerate() {
        st.put_cell(o, ANCH_HEAD + j, *c);
    }
    st.seal(o);
    Ok(o)
}

impl Doc {
    /// Record one anchor: a new ANCH in front of the chain and a new DOC root naming it, in
    /// ONE transaction; the log, keys, heads and tree are carried as they are. The event
    /// is the file's -- a device with no network simply never calls this, and the file is
    /// as valid without it (M6's kill test: "DO without network -> no ANCHORED, file valid").
    pub fn anchor_bytes(st: &mut Store, kind: i64, over: &[u8; 32], time_ms: i64, proof: &[u8]) -> Result<i64, DocError> {
        if proof.len() > MAX_PROOF_BYTES {
            return Err(DocError::Corrupt("an anchor proof past MAX_PROOF_BYTES"));
        }
        let view = Doc::open(st)?;
        let mut tx = st.begin()?;
        let a = stage_anchor(st, &mut tx, kind, over, time_ms, view.anch, proof)?;
        tx.sup_delta += 2 + st.obj_cells(view.at) as i64;
        let doc = stage_doc(
            st,
            &mut tx,
            view.flags,
            Refs { log: view.log, tree: view.tree, heads: view.heads, keys: view.keys, spec: view.spec, anch: Some(a), seg: view.seg },
        )?;
        Ok(st.commit_bytes(&tx, doc))
    }
}

/// Every anchor of the DOC at the image's root, checked: the chain's crcs (a failure is a
/// refusal, as for the log), `over` against the log's record ids, then the adapter. `Err` =
/// not a DOC, a crc failure, an object on the chain that is not an ANCH, or a chain that
/// never ends.
pub fn check_anchors(st: &Store, ac: &dyn AnchorCheck) -> Result<Vec<Checked>, DocError> {
    let view = Doc::open(st)?;
    if EvLog::chain_crc_at(st, view.log)?.is_none() {
        return Err(DocError::Corrupt("the log chain never ends"));
    }
    let ids: BTreeSet<[u8; 32]> = EvLog::walk_until_at(st, view.log, |_| false).into_iter().map(|r| r.id).collect();
    let segments = read_segs(st, view.seg, EvLog::step_cap(st))?.len();
    let cap = chain_cap(st);
    let mut out = Vec::new();
    let mut cur = view.anch;
    while let Some(at) = cur {
        if out.len() >= cap {
            return Err(DocError::Corrupt("the anchor chain never ends"));
        }
        st.check_obj(at)?;
        let a = read_anchor(st, at).ok_or(DocError::Corrupt("an object on the anchor chain is not an ANCH"))?;
        cur = a.prev;
        let names_record = ids.contains(&a.over);
        let under_cut = !names_record && segments > 0;
        let verdict = if names_record {
            ac.check(&a)
        } else if !under_cut {
            Verdict::Bad("`over` names no record of this log".to_string())
        } else {
            // The proof is still run (a forgery is still bad); a passing proof binds an id
            // the file cannot show, so it is a question for the archive, never a pass here.
            match ac.check(&a) {
                Verdict::Bad(why) => Verdict::Bad(why),
                _ => Verdict::Unchecked(format!("`over` names no live record and the log was cut ({segments} segment(s)): place the id in the archive under a segment root")),
            }
        };
        out.push(Checked { anchor: a, names_record, under_cut, verdict });
    }
    Ok(out)
}
