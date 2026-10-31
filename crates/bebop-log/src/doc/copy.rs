//! `Doc::copy_bytes` (W-MG-M1B): a DOC carried whole into another store, in ONE transaction
//! -- what a grow does (the arena is full, the image doubles) and what a compaction will do.
//! Records are copied VERBATIM, oldest first (ids, `prev`, flags, payload untouched -- all of
//! them content addresses or signed); KEY objects with their `ref ADMIT_REC` re-pointed at
//! the copied record; every seal chain oldest first with `ref PREV_SEAL` re-pointed; the
//! anchor chain likewise; the DOC's flags as they were. Nothing is re-derived and nothing is
//! re-signed, so `verify` answers the same eight counters on the copy -- the test
//! `a_copied_doc_verifies_the_same` holds it to that.
//!
//! WHAT IS REFUSED, BY NAME: a record whose crc fails (a copy would re-seal it -- the laundering
//! `verify::copy_chain_bytes` carries a failed crc to avoid -- and a DOC is refused whole on one
//! anyway, so there is nothing to carry); a DOC with a TREE or a SPEC (M5's objects; copying a
//! KV is `bebop-kv`'s business, and this crate does not guess at it).

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::collections::BTreeMap;
use crate::{EvLog, Record, Store, Tx};
use super::anch::{read_anchors, stage_anchor};
use super::keys::{key_rows, read_key, stage_key, stage_keys, Key};
use super::seal::{read_heads, read_seal, stage_heads, stage_seal, Head};
use super::seg::{read_segs, stage_seg, Segment};
use super::write::{stage_doc, Refs};
use super::{Doc, DocError, FLAG_SELF_DESCRIBING};

/// What a redacting copy asks per record: `Some(payload)` replaces the payload, id and
/// `prev` kept (`forget.rs`'s rule: the tombstone keeps its place and stops vouching for
/// its content); `None` carries the record verbatim.
pub type Redact<'a> = &'a dyn Fn(&Record) -> Option<Vec<u8>>;

impl Doc {
    /// Copy the DOC at `src`'s root into `dst` (a fresh store with no root). Returns the
    /// new generation. `dst` must have room for everything: on `ArenaFull` nothing is
    /// committed and the caller tries a bigger store.
    pub fn copy_bytes(src: &Store, dst: &mut Store) -> Result<i64, DocError> {
        let mut tx = dst.begin()?;
        let (doc, _) = stage_copy_with(src, dst, &mut tx, None, None)?;
        Ok(dst.commit_bytes(&tx, doc))
    }

    /// REDACTION IN PLACE, for a DOC (W-MG-M3b, R-MAINGOAL-A §2c): the copy with the payloads
    /// `redact` answers for replaced, everything else -- ids, `prev`, seals, keys, anchors,
    /// segments -- verbatim, so the tips the seals signed do not move and the tombstones
    /// are declared, not hidden. Returns how many records were redacted; the caller then
    /// commits the signed FORGOTTEN (`seg::forgotten_body`) through `commit_bytes`, or
    /// `verify` says `redacted != declared`. Nothing is committed when the count is 0.
    pub fn redact_into(src: &Store, dst: &mut Store, redact: Redact<'_>) -> Result<usize, DocError> {
        let mut tx = dst.begin()?;
        let (doc, n) = stage_copy_with(src, dst, &mut tx, None, Some(redact))?;
        if n > 0 {
            dst.commit_bytes(&tx, doc);
        }
        Ok(n)
    }
}

/// The body of `copy_bytes`, one transaction the caller owns. `spec` names a SPEC already
/// staged in `dst`: the copy then IS an archive (`Doc::archive_bytes`, `spec.rs`).
pub(crate) fn stage_copy(src: &Store, dst: &mut Store, tx: &mut Tx, spec: Option<usize>) -> Result<usize, DocError> {
    stage_copy_with(src, dst, tx, spec, None).map(|(doc, _)| doc)
}

/// `stage_copy`, with an optional redaction; returns the DOC and how many payloads moved.
fn stage_copy_with(src: &Store, dst: &mut Store, tx: &mut Tx, spec: Option<usize>, redact: Option<Redact<'_>>) -> Result<(usize, usize), DocError> {
    let mut redacted = 0usize;
    {
        let view = Doc::open(src)?;
        if view.tree.is_some() || view.spec.is_some() {
            return Err(DocError::Corrupt("a DOC with a TREE or SPEC is not copied here"));
        }
        if EvLog::chain_crc_at(src, view.log)?.is_none() {
            return Err(DocError::Corrupt("the log chain never ends"));
        }
        for t in [view.heads, view.keys].into_iter().flatten() {
            src.check_obj(t)?;
        }
        // The records, oldest first, with where each one lands.
        let mut objs = EvLog::objs_at(src, view.log);
        objs.reverse();
        let mut recs = EvLog::walk_until_at(src, view.log, |_| false);
        recs.reverse();
        if objs.len() != recs.len() {
            return Err(DocError::Corrupt("the log's objects and records disagree"));
        }
        let mut log = EvLog::stage_init_in(dst, tx)?;
        let mut moved: BTreeMap<usize, usize> = BTreeMap::new();
        for (old, r) in objs.iter().zip(&recs) {
            let tomb;
            let r = match redact.and_then(|f| f(r)) {
                Some(payload) => {
                    redacted += 1;
                    tomb = Record { payload, ..r.clone() };
                    &tomb
                }
                None => r,
            };
            log = EvLog::stage_append_in(dst, tx, log, r, Some(&r.id))?;
            let new = dst.follow(log, 1).ok_or(DocError::Corrupt("the copied record is not reachable"))?;
            moved.insert(*old, new);
        }
        // Keys: each KEY re-staged with its ADMIT record re-pointed.
        let mut rows = Vec::new();
        for (actor, at) in key_rows(src, view.keys) {
            src.check_obj(at)?;
            let k = read_key(src, at).ok_or(DocError::Corrupt("a KEYS row names no KEY"))?;
            let admit_rec = match k.admit_rec {
                Some(a) => Some(*moved.get(&a).ok_or(DocError::Corrupt("a KEY's ADMIT record is not in the log"))?),
                None => None,
            };
            let new = stage_key(dst, tx, &Key { admit_rec, ..k })?;
            rows.push((actor, new));
        }
        let keys = stage_keys(dst, tx, &mut rows)?;
        // Heads: each device's seal chain, oldest first.
        let mut heads: Vec<Head> = Vec::new();
        let cap = recs.len() + 1;
        for h in read_heads(src, view.heads) {
            let mut chain = Vec::new();
            let mut cur = h.seal;
            while let Some(at) = cur {
                if chain.len() >= cap {
                    return Err(DocError::Corrupt("a seal chain never ends"));
                }
                src.check_obj(at)?;
                let s = read_seal(src, at).ok_or(DocError::Corrupt("a HEADS row names no SEAL"))?;
                cur = s.prev;
                chain.push(s);
            }
            let mut prev = None;
            for s in chain.iter().rev() {
                prev = Some(stage_seal(dst, tx, &s.actor_id, s.seq, &s.tip, s.claimed_ms, prev, s.alg, &s.sig)?);
            }
            heads.push(Head { seal: prev, ..h });
        }
        let heads = stage_heads(dst, tx, &mut heads)?;
        // Anchors, oldest first.
        let anchors = read_anchors(src, view.anch);
        let mut anch = None;
        for a in anchors.iter().rev() {
            src.check_obj(a.at)?;
            anch = Some(stage_anchor(dst, tx, a.kind, &a.over, a.time_ms, anch, &a.proof)?);
        }
        // Segments, oldest first: the cuts this log already went through are its history.
        let mut seg = None;
        for s in read_segs(src, view.seg, EvLog::step_cap(src))?.iter().rev() {
            seg = Some(stage_seg(dst, tx, &Segment { prev: seg, ..s.clone() })?);
        }
        let flags = if spec.is_some() { view.flags | FLAG_SELF_DESCRIBING } else { view.flags };
        Ok((stage_doc(dst, tx, flags, Refs { log: Some(log), tree: None, heads: Some(heads), keys: Some(keys), spec, anch, seg })?, redacted))
    }
}
