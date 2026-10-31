//! THE CUT (W-MG-M3): the one way records leave a DOC file. `Doc::cut_into` rebuilds the
//! DOC into a FRESH store without its oldest records -- this container has no compaction,
//! so a cut is a rebuild, exactly as `dowiz-hub`'s `rotate.rs` and `forget.rs` rebuild --
//! and leaves a SEG over the cut ids at the DOC's `ref SEG`. What it writes is UNDECLARED
//! until the caller commits a TRUNCATE record carrying `Segment::truncate_body` through
//! `Doc::commit_bytes` (signed, sealed); `verify` refuses the file by name until then
//! (`DocError::CutUndeclared`). The old store is the archive: the caller keeps it, and the
//! segment's root recomputes from its ids (`cut/tests.rs`).
//!
//! What is carried, verbatim: every kept record (same id, prev, actor, seq, flags, payload;
//! new offsets), every seal that names a kept record (its `ref PREV_SEAL` rewired; the
//! oldest kept seal of a device points at nothing and `verify` reads its `prev_tip` from
//! the SEG's frontier row), every KEY (its `ref ADMIT_REC` remapped), HEADS as they stood,
//! every older SEG (rewired), and EVERY ANCHOR (W-MG-M3b: a file anchored once must still
//! be cuttable, or the two main-goal properties -- kept history, external time -- would
//! exclude each other; an anchor whose `over` left under the segment is not dropped and not
//! judged here, see `anch.rs` `under_cut`). What is refused: a cut through the ADMIT record
//! of a key still in KEYS (`CutsAdmit`: the proof of the key is that record), a DOC that
//! names a TREE or SPEC (`Carries`: the layer owning it moves it), a log with nothing to cut.
//!
//! The counts under the cut (`redacted`, `declared`) are counted by mark, not by seal: a
//! writer has no `Verifier`. They are the TRUNCATE signer's claim -- verify a file before
//! cutting it. The frontier row's `scheme` is the same kind of claim: whether a chained
//! record of the device had left by the cut (M0's forward-once rule, `verify.rs`).

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::collections::BTreeMap;
use crate::{EvLog, Record, Store};
use super::anch::{read_anchors, stage_anchor, Anchor};
use super::keys::{read_keys, stage_key, stage_keys, Key};
use super::seal::{read_heads, read_seal, stage_heads, stage_seal, Head, Seal};
use super::seg::{frontier_of, parse_forgotten, read_segs, stage_seg, Frontier, Segment, SCHEME_CHAINED, SCHEME_LEGACY};
use super::write::{stage_doc, Refs};
use super::{Doc, DocError, Ids};

type Actor = [u8; 32];

/// The old file, read once: records oldest first with their offsets, the heads, the keys,
/// the segments and the anchors newest first.
struct Old {
    flags: i64,
    recs: Vec<(usize, Record)>,
    heads: Vec<Head>,
    keys: Vec<Key>,
    segs: Vec<Segment>,
    anchors: Vec<Anchor>,
}

fn read_old(st: &Store) -> Result<Old, DocError> {
    let view = Doc::open(st)?;
    for (what, r) in [("TREE", view.tree), ("SPEC", view.spec)] {
        if r.is_some() {
            return Err(DocError::Carries(what));
        }
    }
    if EvLog::chain_crc_at(st, view.log)?.is_none() {
        return Err(DocError::Corrupt("the log chain never ends"));
    }
    for t in [view.heads, view.keys].into_iter().flatten() {
        st.check_obj(t)?;
    }
    let anchors = read_anchors(st, view.anch);
    for a in &anchors {
        st.check_obj(a.at)?;
    }
    let objs = EvLog::objs_at(st, view.log);
    let newest_first = EvLog::walk_until_at(st, view.log, |_| false);
    let mut recs: Vec<(usize, Record)> = objs.into_iter().zip(newest_first).collect();
    recs.reverse();
    Ok(Old { flags: view.flags, recs, heads: read_heads(st, view.heads), keys: read_keys(st, view.keys), segs: read_segs(st, view.seg, EvLog::step_cap(st))?, anchors })
}

/// Every seal reachable from `head`, newest first, crc checked.
fn seals_of(st: &Store, head: &Head, cap: usize) -> Result<Vec<Seal>, DocError> {
    let mut out = Vec::new();
    let mut cur = head.seal;
    while let Some(at) = cur {
        if out.len() > cap {
            return Err(DocError::Corrupt("a seal chain never ends"));
        }
        st.check_obj(at)?;
        let s = read_seal(st, at).ok_or(DocError::Corrupt("a HEADS row names an object that is not a SEAL"))?;
        cur = s.prev;
        out.push(s);
    }
    Ok(out)
}

impl Doc {
    /// Cut every record but the newest `keep` out of the DOC at `st`'s root into `fresh`
    /// (an empty store the caller sized; a `Store` refusal for room comes back as
    /// `DocError::Store` for the caller to retry larger, as `forget.rs` does). On `Ok` the
    /// fresh store holds the cut DOC and the returned segment is what the caller must now
    /// declare: commit a record whose body is `Segment::truncate_body` through
    /// `commit_bytes`. The old store is untouched: it is the archive.
    pub fn cut_into(st: &Store, fresh: &mut Store, keep: usize, ids: &dyn Ids) -> Result<Segment, DocError> {
        let old = read_old(st)?;
        let n = old.recs.len();
        if keep >= n {
            return Err(DocError::NothingToCut);
        }
        let cut = n - keep;
        let (gone, kept) = old.recs.split_at(cut);
        for k in &old.keys {
            if let Some(at) = k.admit_rec {
                if gone.iter().any(|(o, _)| *o == at) {
                    return Err(DocError::CutsAdmit(k.actor_id));
                }
                if !kept.iter().any(|(o, _)| *o == at) {
                    return Err(DocError::Corrupt("a KEY names an ADMIT record the log does not hold"));
                }
            }
        }
        let cut_ids: Vec<[u8; 32]> = gone.iter().map(|(_, r)| r.id).collect();
        let root = ids.tree_root(&cut_ids).ok_or(DocError::NoTree)?;
        let newest = old.segs.first();
        let first = newest.map_or(0, |s| s.last + 1);
        let mut frontier: BTreeMap<Actor, Frontier> = BTreeMap::new();
        let (mut redacted, mut declared) = (0u64, 0u64);
        for (_, r) in gone {
            // M0's rule carried: the device has moved to chained ids once any record of its,
            // under an older segment or in this cut, chained.
            let moved_before = match frontier.get(&r.actor_pubkey) {
                Some(f) => f.scheme == SCHEME_CHAINED,
                None => frontier_of(&old.segs, &r.actor_pubkey).is_some_and(|f| f.scheme == SCHEME_CHAINED),
            };
            let scheme = if moved_before || r.id == ids.chained(&r.prev, &r.payload) { SCHEME_CHAINED } else { SCHEME_LEGACY };
            frontier.insert(r.actor_pubkey, Frontier { actor_id: r.actor_pubkey, seq: r.actor_seq, tip: r.id, scheme });
            if ids.redacted(r.flags, &r.payload) {
                redacted += 1;
            } else if let Some((k, _)) = parse_forgotten(ids.body(r.flags, &r.payload)) {
                declared += k;
            }
        }
        let seg = Segment {
            first,
            last: first + cut as u64 - 1,
            root,
            prev: None,
            prev_root: newest.map_or([0u8; 32], |s| s.root),
            blob: None,
            redacted,
            declared,
            frontier: frontier.into_values().collect(),
        };

        let mut tx = fresh.begin()?;
        let mut log = EvLog::stage_init_in(fresh, &mut tx)?;
        let mut moved: BTreeMap<usize, usize> = BTreeMap::new();
        let last_id = kept.last().map(|(_, r)| r.id);
        for (o, r) in kept {
            let tip = (Some(r.id) == last_id).then_some(&r.id);
            log = EvLog::stage_append_in(fresh, &mut tx, log, r, tip)?;
            let ev = fresh.follow(log, 1).ok_or(DocError::Corrupt("the carried record is not reachable"))?;
            moved.insert(*o, ev);
        }
        let mut key_rows = Vec::with_capacity(old.keys.len());
        for k in &old.keys {
            let admit_rec = match k.admit_rec {
                Some(at) => Some(*moved.get(&at).ok_or(DocError::Corrupt("an ADMIT record was not carried"))?),
                None => None,
            };
            key_rows.push((k.actor_id, stage_key(fresh, &mut tx, &Key { admit_rec, ..k.clone() })?));
        }
        let keys = stage_keys(fresh, &mut tx, &mut key_rows)?;
        let cap = n + 1;
        let mut heads = Vec::with_capacity(old.heads.len());
        for h in &old.heads {
            let cut_seq = seg.frontier.iter().find(|f| f.actor_id == h.actor_id).map_or(0, |f| f.seq);
            let mut chain = seals_of(st, h, cap)?;
            chain.retain(|s| s.seq > cut_seq);
            chain.reverse();
            let mut prev = None;
            for s in &chain {
                prev = Some(stage_seal(fresh, &mut tx, &s.actor_id, s.seq, &s.tip, s.claimed_ms, prev, s.alg, &s.sig)?);
            }
            heads.push(Head { seal: prev, ..*h });
        }
        let heads = stage_heads(fresh, &mut tx, &mut heads)?;
        let mut prev_seg = None;
        for s in old.segs.iter().rev() {
            prev_seg = Some(stage_seg(fresh, &mut tx, &Segment { prev: prev_seg, ..s.clone() })?);
        }
        let seg_at = stage_seg(fresh, &mut tx, &Segment { prev: prev_seg, ..seg.clone() })?;
        // The anchors, oldest first, every one of them: an anchor over a record that just
        // left is history too (`anch.rs`: `under_cut`, never dropped, never a verdict here).
        let mut anch = None;
        for a in old.anchors.iter().rev() {
            anch = Some(stage_anchor(fresh, &mut tx, a.kind, &a.over, a.time_ms, anch, &a.proof)?);
        }
        let doc = stage_doc(fresh, &mut tx, old.flags, Refs { log: Some(log), heads: Some(heads), keys: Some(keys), seg: Some(seg_at), anch, ..Refs::default() })?;
        fresh.commit_bytes(&tx, doc);
        Ok(seg)
    }
}

#[cfg(test)]
mod tests;
