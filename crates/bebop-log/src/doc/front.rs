//! THE SENDER'S HALF OF A TAIL (W-MG-P1, docs/design/MESH-TRANSPORT-OVER-EINFILE.md §3): where a file
//! stands (`Front`) and what another file standing at some `Front` lacks (`Tail`). Nothing here goes
//! on a wire: the bytes are `einfile-sync`'s, and no format changes. `tail.rs` is the receiver.
//!
//! A `Front` is a file's position and nothing a reader must believe: the receiver re-judges all of it
//! through `Doc::tail_into`. What each field is for:
//!
//!   * `places`: per HEADS row, the row and a FOLD OVER THAT DEVICE'S SEAL CHAIN. Not in the design's
//!     72-byte row: two chains of seals over one record history are `SealFork` to `merge_into`, and a
//!     row alone cannot see them, because equal rows are equal whatever was signed on the way.
//!   * `doc`: sha256 of the flags and the genesis actors, what `merge_into` compares for `ForeignRoot`.
//!     It is a name for a route, not a proof. Unlike the design's `doc_id`, it does not take the first
//!     record: after a cut no file has one, and a name that moves at a cut names nothing.
//!   * `seg_root`, `seg_hash`: the design's. A `Front` with another `seg_hash` is `CutOneSide`.
//!   * `forgotten_hash`: the design's. An erasure keeps the id, so no row shows it, and a sender
//!     never sends past a difference here: ERASURE DOMINATES (§12).
//!   * `anchor_hash`: the design has a count. Two files with one different anchor each have equal
//!     counts and would read as converged with an anchor missing on each side.
//!
//! WHAT A TAIL CARRIES. A device's records past the peer's row, in this file's order; its seals past
//! that row; every anchor, when the anchors differ (an anchor has no chain to diff, and the receiver
//! dedupes them as `merge_into` does). No KEY: the receiver builds it from the ADMIT in the tail,
//! so there is one unsigned input fewer.
//!
//! PRE-SEAL RECORDS DO NOT TRAVEL. A device with no HEADS row (`seal_hub`'s records, whose actor
//! signed nothing and may carry any seq) has no place in a `Front` and is not sent. `commit_bytes`
//! writes no such record after sealing, so both copies share them from the split. A file that grew
//! one anyway is forged: what stands on it arrives with a dangling `prev` and the union refuses.
//!
//! THE COST: every `Front` hashes every seal. On a hub's 10 ms that is P6's question, not this one's.

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use crate::{Record, Store};
use super::anch::Anchor;
use super::budget::Budget;
use super::cut::{read_old, seals_of, Old};
use super::merge::{anchor_key, chains, check_tail, genesis, Actor};
use super::merge_body::Row;
use super::seal::Seal;
use super::seg::{chain_hash, parse_forgotten};
use super::{Doc, DocError, Ids};

pub use super::clip::Limits;

const DOC_TAG: &[u8] = b"ein-doc/1";
const FORGOTTEN_TAG: &[u8] = b"ein-forgotten/1";
const SEALS_TAG: &[u8] = b"ein-seals/1";
const ANCHORS_TAG: &[u8] = b"ein-anchors/1";

/// One device's position: its HEADS row, and the fold of its seal chain up to that row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place {
    pub row: Row,
    pub seals: [u8; 32],
}

/// Where a file stands. Two equal `Front`s are one history (header).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Front {
    pub doc: [u8; 32],
    pub places: Vec<Place>,
    pub seg_root: [u8; 32],
    pub seg_hash: [u8; 32],
    pub forgotten_hash: [u8; 32],
    pub anchor_hash: [u8; 32],
}

/// What one file sends another: the sender's `Front`, and what the peer it was built for lacks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tail {
    pub front: Front,
    /// In the sender's file order, oldest first.
    pub recs: Vec<Record>,
    /// Per device oldest first; only `actor_id`, `seq`, `tip`, `claimed_ms`, `alg`, `sig` count.
    pub seals: Vec<Seal>,
    /// Newest first, as `Old::anchors`; only `kind`, `over`, `time_ms`, `proof` count.
    pub anchors: Vec<Anchor>,
}

impl Tail {
    /// Nothing but a position.
    pub fn is_empty(&self) -> bool {
        self.recs.is_empty() && self.seals.is_empty() && self.anchors.is_empty()
    }
}

/// `ids` with no hash cannot name a position: `NoTree`, the refusal a cut gives for the same lack.
fn hash(ids: &dyn Ids, b: &[u8]) -> Result<[u8; 32], DocError> {
    ids.hash(b).ok_or(DocError::NoTree)
}

/// Each device's seal chain, oldest first.
pub(super) fn seal_map(o: &Old, st: &Store) -> Result<BTreeMap<Actor, Vec<Seal>>, DocError> {
    let mut budget = Budget::of(st);
    let mut out = BTreeMap::new();
    for h in &o.heads {
        let mut chain = seals_of(st, h, o.recs.len() + 1, &mut budget)?;
        chain.reverse();
        out.insert(h.actor_id, chain);
    }
    Ok(out)
}

/// Per seal of a chain, oldest first, the fold of the chain up to and with it. Everything a seal
/// says is in it, and `same_seal` is what it stands in for.
pub(super) fn folds(actor: &Actor, chain: &[Seal], ids: &dyn Ids) -> Result<Vec<[u8; 32]>, DocError> {
    let mut m = SEALS_TAG.to_vec();
    m.extend_from_slice(actor);
    let mut h = hash(ids, &m)?;
    let mut out = Vec::with_capacity(chain.len());
    for s in chain {
        let mut m = Vec::with_capacity(32 + 32 + 8 * 3 + s.sig.len());
        m.extend_from_slice(&h);
        m.extend_from_slice(&s.seq.to_le_bytes());
        m.extend_from_slice(&s.tip);
        m.extend_from_slice(&s.claimed_ms.to_le_bytes());
        m.extend_from_slice(&s.alg.to_le_bytes());
        m.extend_from_slice(&s.sig);
        h = hash(ids, &m)?;
        out.push(h);
    }
    Ok(out)
}

/// Does this chain agree with a device's `Place`? A seal at that seq must fold the same; no seal at
/// that seq is fine only past this chain's end, where the other side is simply ahead.
pub(super) fn agree(chain: &[Seal], p: &Place, ids: &dyn Ids) -> Result<(), DocError> {
    let (actor, seq) = (p.row.actor_id, p.row.seq);
    let f = folds(&actor, chain, ids)?;
    match chain.iter().zip(&f).find(|(s, _)| s.seq == seq) {
        Some((_, h)) if *h == p.seals => Ok(()),
        None if chain.last().is_none_or(|top| seq > top.seq) => Ok(()),
        _ => Err(DocError::SealFork { actor, seq }),
    }
}

fn forgotten_hash(o: &Old, ids: &dyn Ids) -> Result<[u8; 32], DocError> {
    let mut list: Vec<(Actor, Actor)> = Vec::new();
    for (_, r) in &o.recs {
        if ids.redacted(r.flags, &r.payload) {
            continue;
        }
        if let Some((_, erased)) = parse_forgotten(ids.body(r.flags, &r.payload)) {
            list.extend(erased.iter().map(|e| (e.id, e.digest)));
        }
    }
    list.sort();
    let mut m = FORGOTTEN_TAG.to_vec();
    for (id, digest) in &list {
        m.extend_from_slice(id);
        m.extend_from_slice(digest);
    }
    hash(ids, &m)
}

/// Order-free: a union lays ours first then theirs, so two converged files hold them differently.
fn anchor_hash(o: &Old, ids: &dyn Ids) -> Result<[u8; 32], DocError> {
    let mut keys: Vec<_> = o.anchors.iter().map(anchor_key).collect();
    keys.sort();
    keys.dedup();
    let mut m = ANCHORS_TAG.to_vec();
    for (kind, over, time_ms, proof) in &keys {
        m.extend_from_slice(&kind.to_le_bytes());
        m.extend_from_slice(over);
        m.extend_from_slice(&time_ms.to_le_bytes());
        m.extend_from_slice(&(proof.len() as u64).to_le_bytes());
        m.extend_from_slice(proof);
    }
    hash(ids, &m)
}

fn doc_name(o: &Old, ids: &dyn Ids) -> Result<[u8; 32], DocError> {
    let mut m = DOC_TAG.to_vec();
    m.extend_from_slice(&o.flags.to_le_bytes());
    for a in genesis(o) {
        m.extend_from_slice(&a);
    }
    hash(ids, &m)
}

pub(super) fn front_of(o: &Old, map: &BTreeMap<Actor, Vec<Seal>>, ids: &dyn Ids) -> Result<Front, DocError> {
    let mut places = Vec::with_capacity(o.heads.len());
    for h in &o.heads {
        let none: Vec<Seal> = Vec::new();
        let chain = map.get(&h.actor_id).unwrap_or(&none);
        let f = folds(&h.actor_id, chain, ids)?;
        let seals = chain.iter().zip(&f).find(|(s, _)| s.seq == h.seq).map_or([0u8; 32], |(_, x)| *x);
        places.push(Place { row: Row { actor_id: h.actor_id, seq: h.seq, tip: h.tip }, seals });
    }
    Ok(Front {
        doc: doc_name(o, ids)?,
        places,
        seg_root: o.segs.first().map_or([0u8; 32], |s| s.root),
        seg_hash: chain_hash(&o.segs, ids).ok_or(DocError::NoTree)?,
        forgotten_hash: forgotten_hash(o, ids)?,
        anchor_hash: anchor_hash(o, ids)?,
    })
}

/// How many of `chain`'s records the peer at `row` holds. Its tip is ours, or its row is past our
/// end and it is ahead; anything else is two histories on one device.
fn held_by(chain: &[&Record], row: &Row) -> Result<usize, DocError> {
    if let Some(p) = chain.iter().position(|r| r.id == row.tip) {
        return Ok(p + 1);
    }
    match chain.iter().find(|r| r.actor_seq == row.seq) {
        Some(r) => Err(DocError::Fork { actor: row.actor_id, seq: row.seq, ours: r.id, theirs: row.tip }),
        None if chain.last().is_none_or(|top| row.seq > top.actor_seq) => Ok(chain.len()),
        None => Err(DocError::Corrupt("a peer's row sits under this device's oldest record")),
    }
}

impl Doc {
    /// Where the DOC at `st`'s root stands.
    pub fn front(st: &Store, ids: &dyn Ids) -> Result<Front, DocError> {
        let o = read_old(st)?;
        front_of(&o, &seal_map(&o, st)?, ids)
    }

    /// What a file standing at `peer` lacks. Another document or another cut gets a bare position:
    /// naming those is the receiver's (`ForeignRoot`, `CutOneSide`), and nothing of ours is owed to
    /// it. `ForgetOneSide` here is §12's rule: not one record leaves past an erasure difference.
    /// A tombstone, a FORGOTTEN or a legacy id in the tail is refused before a byte is built.
    pub fn tail_of(st: &Store, peer: &Front, ids: &dyn Ids) -> Result<Tail, DocError> {
        let o = read_old(st)?;
        let map = seal_map(&o, st)?;
        let front = front_of(&o, &map, ids)?;
        if front.doc != peer.doc || front.seg_hash != peer.seg_hash {
            return Ok(Tail { front, recs: Vec::new(), seals: Vec::new(), anchors: Vec::new() });
        }
        if front.forgotten_hash != peer.forgotten_hash {
            return Err(DocError::ForgetOneSide);
        }
        let place: BTreeMap<Actor, &Place> = peer.places.iter().map(|p| (p.row.actor_id, p)).collect();
        let by = chains(&o);
        let mut wanted: BTreeSet<Actor> = BTreeSet::new();
        let mut seals = Vec::new();
        for h in &o.heads {
            let row = place.get(&h.actor_id);
            if let Some(chain) = by.get(&h.actor_id) {
                let held = row.map_or(Ok(0), |p| held_by(chain, &p.row))?;
                wanted.extend(chain.iter().skip(held).map(|r| r.id));
            }
            if let Some(chain) = map.get(&h.actor_id) {
                if let Some(p) = row {
                    agree(chain, p, ids)?;
                }
                let seen = row.map_or(0, |p| p.row.seq);
                seals.extend(chain.iter().filter(|s| s.seq > seen).cloned());
            }
        }
        let recs: Vec<Record> = o.recs.iter().map(|(_, r)| r).filter(|r| wanted.contains(&r.id)).cloned().collect();
        check_tail(&recs, ids)?;
        let anchors = if front.anchor_hash == peer.anchor_hash { Vec::new() } else { o.anchors.clone() };
        Ok(Tail { front, recs, seals, anchors })
    }
}
