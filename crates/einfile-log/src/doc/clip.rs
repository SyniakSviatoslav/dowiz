//! A TAIL THAT FITS (W-MG-P1): `Doc::tail_part` gives the longest front part of a tail that stays
//! inside a message's limits, so no carrier has to take a tail whole. It cuts where a file could
//! have stood: every device on a seal's edge, a prefix of the sender's record order, so an ADMIT
//! still comes before what it admits. The part's `Front` is that position and not the sender's
//! real one, because a MERGE names what it took in and a row past what landed is `merges_bad`.
//!
//! THE COST, AND IT IS NOT HIDDEN: each part is one meeting, so each may owe one MERGE on each side.
//! Anchors wait for the last part: a part is not a position they have anything to say about.
//! ONE SYNC PAST THE LIMITS has no edge to cut at: the whole tail comes back and the encoder refuses
//! it by name; the whole file (`merge_into`) is the way past that.

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use crate::{Record, Store};
use super::cut::read_old;
use super::front::{folds, seal_map, Front, Place, Tail};
use super::merge::Actor;
use super::merge_body::Row;
use super::seal::Seal;
use super::{Doc, DocError, Ids};

/// What one message may carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub records: usize,
    pub seals: usize,
    /// Payloads and signatures, plus each object's fixed head.
    pub bytes: usize,
}

const RECORD_HEAD: usize = 32 + 32 + 8 * 3;
const SEAL_HEAD: usize = 32 + 8 + 32 + 8 * 3;

fn fits(t: &Tail, lim: &Limits) -> bool {
    let bytes = t.recs.iter().map(|r| RECORD_HEAD + r.payload.len()).sum::<usize>() + t.seals.iter().map(|s| SEAL_HEAD + s.sig.len()).sum::<usize>() + t.anchors.iter().map(|a| 64 + a.proof.len()).sum::<usize>();
    t.recs.len() <= lim.records && t.seals.len() <= lim.seals && bytes <= lim.bytes
}

/// How many records of `t` make the longest prefix inside `lim` with every device on a seal's edge.
fn prefix(t: &Tail, lim: &Limits) -> usize {
    let sealed: BTreeMap<(Actor, u64), usize> = t.seals.iter().map(|s| ((s.actor_id, s.seq), SEAL_HEAD + s.sig.len())).collect();
    let mut open: BTreeSet<Actor> = BTreeSet::new();
    let (mut bytes, mut seals, mut best) = (0usize, 0usize, 0usize);
    for (i, r) in t.recs.iter().enumerate() {
        bytes += RECORD_HEAD + r.payload.len();
        if i + 1 > lim.records || bytes > lim.bytes {
            break;
        }
        match sealed.get(&(r.actor_pubkey, r.actor_seq)) {
            Some(cost) => {
                bytes += cost;
                seals += 1;
                if seals > lim.seals || bytes > lim.bytes {
                    break;
                }
                open.remove(&r.actor_pubkey);
            }
            None => {
                open.insert(r.actor_pubkey);
            }
        }
        if open.is_empty() {
            best = i + 1;
        }
    }
    best
}

/// One device's place after the cut: at its newest kept seal.
fn place_at(actor: &Actor, seq: u64, chain: &[Seal], ids: &dyn Ids) -> Result<Option<Place>, DocError> {
    let f = folds(actor, chain, ids)?;
    Ok(chain.iter().zip(&f).find(|(s, _)| s.seq == seq).map(|(s, h)| Place { row: Row { actor_id: *actor, seq, tip: s.tip }, seals: *h }))
}

fn cut_to(t: Tail, n: usize, map: &BTreeMap<Actor, Vec<Seal>>, peer: &Front, ids: &dyn Ids) -> Result<Tail, DocError> {
    let recs: Vec<Record> = t.recs.iter().take(n).cloned().collect();
    let mut top: BTreeMap<Actor, u64> = BTreeMap::new();
    for r in &recs {
        top.insert(r.actor_pubkey, r.actor_seq);
    }
    let seals = t.seals.iter().filter(|s| top.get(&s.actor_id).is_some_and(|seq| s.seq <= *seq)).cloned().collect();
    let none: Vec<Seal> = Vec::new();
    let mut places = Vec::with_capacity(t.front.places.len());
    for p in &t.front.places {
        let actor = p.row.actor_id;
        let moved = t.recs.iter().any(|r| r.actor_pubkey == actor) || t.seals.iter().any(|s| s.actor_id == actor);
        if !moved {
            places.push(*p);
        } else if let Some(seq) = top.get(&actor) {
            places.extend(place_at(&actor, *seq, map.get(&actor).unwrap_or(&none), ids)?);
        } else {
            places.extend(peer.places.iter().find(|q| q.row.actor_id == actor).copied());
        }
    }
    Ok(Tail { front: Front { places, ..t.front }, recs, seals, anchors: Vec::new() })
}

impl Doc {
    /// `tail_of`, inside `lim` (header).
    pub fn tail_part(st: &Store, peer: &Front, ids: &dyn Ids, lim: &Limits) -> Result<Tail, DocError> {
        let t = Doc::tail_of(st, peer, ids)?;
        if fits(&t, lim) {
            return Ok(t);
        }
        let n = prefix(&t, lim);
        if n == 0 {
            return Ok(t);
        }
        let o = read_old(st)?;
        cut_to(t, n, &seal_map(&o, st)?, peer, ids)
    }
}
