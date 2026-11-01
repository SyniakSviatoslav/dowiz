//! THE DOC WRITER (W-MG-M1): `Doc::init_bytes` makes a DOC over an empty log with its
//! genesis keys; `Doc::commit_bytes` is ONE device commit -- the record appended and named
//! as the log's tip, an optional key op (ADMIT / REVOKE) carried by that record, one SEAL
//! over the tip, the HEADS row moved, a new DOC root -- all in one transaction, so there is
//! no generation in which a record exists without its seal.
//!
//! The writer polices what it can see (the actor's key exists and is not revoked at this
//! seq, the device's seq is the next one) and leaves AUTHORITY to `verify`: whether the key
//! was admitted by an admitted key, or revoked by one, is a question about the signed records,
//! not the tables. A key op is the record's AND the table's: `op` moves the table, `rec`
//! carries the body, and a caller who lets them differ gets `keys_disagree` from `verify`.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::{EvLog, Record, Store, StoreError, Tx};
use super::keys::{key_rows, narrower, read_keys, stage_key, stage_keys, Key, KeyOp};
use super::seal::{message, read_heads, stage_heads, stage_seal, Head};
use super::seg::{frontier_of, read_segs};
use super::{Doc, DocError, Signer, C_ANCH, C_FLAGS, C_HEADS, C_KEYS, C_LOG, C_SEG, C_SPEC, C_TREE, C_VERSION, C_VERSION_LOG, DIGEST_DOC, DOC_CELLS, ROOT_VERSION_DOC};

/// The refs a DOC root names (absolute offsets; `None` = 0).
#[derive(Clone, Copy, Default)]
pub(crate) struct Refs {
    pub log: Option<usize>,
    pub tree: Option<usize>,
    pub heads: Option<usize>,
    pub keys: Option<usize>,
    pub spec: Option<usize>,
    pub anch: Option<usize>,
    pub seg: Option<usize>,
}

pub(crate) fn stage_doc(st: &mut Store, tx: &mut Tx, flags: i64, r: Refs) -> Result<usize, StoreError> {
    let d = st.alloc(tx, DOC_CELLS, DIGEST_DOC)?;
    for i in 0..DOC_CELLS as usize {
        st.put_cell(d, i, 0);
    }
    st.put_cell(d, C_FLAGS, flags);
    st.put_cell(d, C_VERSION, ROOT_VERSION_DOC);
    st.put_cell(d, C_VERSION_LOG, ROOT_VERSION_DOC);
    for (cell, target) in [(C_LOG, r.log), (C_TREE, r.tree), (C_HEADS, r.heads), (C_KEYS, r.keys), (C_SPEC, r.spec), (C_ANCH, r.anch), (C_SEG, r.seg)] {
        if let Some(t) = target {
            st.link(d, cell, t);
        }
    }
    st.seal(d);
    Ok(d)
}

impl Doc {
    /// A fresh DOC: an empty v2 log, no tree, an empty HEADS, and `genesis` as the KEYS
    /// table (each with `admit_rec` = none -- the keys the file itself vouches for).
    pub fn init_bytes(st: &mut Store, flags: i64, genesis: &[Key]) -> Result<i64, DocError> {
        let mut tx = st.begin()?;
        let log = EvLog::stage_init_in(st, &mut tx)?;
        let mut rows = Vec::with_capacity(genesis.len());
        for k in genesis {
            let at = stage_key(st, &mut tx, &Key { admit_rec: None, ..k.clone() })?;
            rows.push((k.actor_id, at));
        }
        let keys = stage_keys(st, &mut tx, &mut rows)?;
        let heads = stage_heads(st, &mut tx, &mut Vec::new())?;
        let doc = stage_doc(st, &mut tx, flags, Refs { log: Some(log), heads: Some(heads), keys: Some(keys), ..Refs::default() })?;
        Ok(st.commit_bytes(&tx, doc))
    }

    /// A DOC over an EXISTING chain (W-MG-M1B, dowiz-hub's `SealedHub::seal_hub`): `records`
    /// oldest first, appended VERBATIM -- ids, `prev`, flags, payload, actor as they were,
    /// the newest named as the log's tip -- with `genesis` as the KEYS table and an EMPTY
    /// HEADS: nothing is sealed until a device commits, and `verify` says so (`seals_ok` =
    /// 0 until then). What the records' ids hold under is the caller's `Ids` business at
    /// verify time, not this writer's.
    pub fn init_over_bytes(st: &mut Store, flags: i64, genesis: &[Key], records: &[Record]) -> Result<i64, DocError> {
        let mut tx = st.begin()?;
        let mut log = EvLog::stage_init_in(st, &mut tx)?;
        for r in records {
            log = EvLog::stage_append_in(st, &mut tx, log, r, Some(&r.id))?;
        }
        let mut rows = Vec::with_capacity(genesis.len());
        for k in genesis {
            let at = stage_key(st, &mut tx, &Key { admit_rec: None, ..k.clone() })?;
            rows.push((k.actor_id, at));
        }
        let keys = stage_keys(st, &mut tx, &mut rows)?;
        let heads = stage_heads(st, &mut tx, &mut Vec::new())?;
        let doc = stage_doc(st, &mut tx, flags, Refs { log: Some(log), heads: Some(heads), keys: Some(keys), ..Refs::default() })?;
        Ok(st.commit_bytes(&tx, doc))
    }

    /// One device commit. `rec` is the caller's (its id derived by the caller's `Ids`, its
    /// `actor_pubkey` the signer's actor id, its `actor_seq` the device's next seq); `op`
    /// is the key op its body carries, if any; `claimed_ms` is the device's own clock, a
    /// claim the seal signs and `verify` repeats without believing.
    pub fn commit_bytes(
        st: &mut Store,
        rec: &Record,
        signer: &dyn Signer,
        claimed_ms: i64,
        op: Option<&KeyOp>,
    ) -> Result<i64, DocError> {
        let view = Doc::open(st)?;
        let log = view.log.ok_or(DocError::Corrupt("a DOC without a log"))?;
        let actor = signer.actor_id();
        if rec.actor_pubkey != actor {
            return Err(DocError::Actor);
        }
        let keys = read_keys(st, view.keys);
        let key = keys.iter().find(|k| k.actor_id == actor).ok_or(DocError::UnknownActor(actor))?;
        if key.alg != signer.alg() {
            return Err(DocError::Actor);
        }
        if !key.valid_at(rec.actor_seq) {
            return Err(DocError::Revoked { after_seq: key.revoked_at, seq: rec.actor_seq });
        }
        let heads = read_heads(st, view.heads);
        let head = heads.iter().find(|h| h.actor_id == actor).copied();
        let want = head.map_or(1, |h| h.seq + 1);
        if rec.actor_seq != want {
            return Err(DocError::Seq { want, got: rec.actor_seq });
        }

        let mut tx = st.begin()?;
        let new_log = EvLog::stage_append_in(st, &mut tx, log, rec, Some(&rec.id))?;
        let ev = st.follow(new_log, 1).ok_or(DocError::Corrupt("the appended record is not reachable"))?;

        let mut keys_root = view.keys;
        if let Some(op) = op {
            let staged = match op {
                KeyOp::Admit(k) => {
                    if keys.iter().any(|old| old.actor_id == k.actor_id && old.revoked_at != 0) {
                        return Err(DocError::Readmit(k.actor_id));
                    }
                    Key { admit_rec: Some(ev), ..k.clone() }
                }
                KeyOp::Revoke { actor_id, after_seq, tip } => {
                    let old = keys.iter().find(|k| k.actor_id == *actor_id).ok_or(DocError::UnknownActor(*actor_id))?;
                    let seq = (*after_seq).max(0) as u64;
                    let itself = *actor_id == actor && seq == rec.actor_seq;
                    if seq > 0 && !itself && !Doc::tip_at(st, actor_id, seq).map_or_else(|| under_cut(st, actor_id, seq), |t| t == *tip) {
                        return Err(DocError::RevokeTip { actor: *actor_id, seq });
                    }
                    Key { revoked_at: narrower(old.revoked_at, *after_seq), ..old.clone() }
                }
            };
            let mut rows = key_rows(st, view.keys);
            let at = stage_key(st, &mut tx, &staged)?;
            rows.retain(|r| r.0 != staged.actor_id);
            rows.push((staged.actor_id, at));
            if let Some(t) = view.keys {
                tx.sup_delta += 2 + st.obj_cells(t) as i64;
            }
            keys_root = Some(stage_keys(st, &mut tx, &mut rows)?);
        }

        seal_tip(st, tx, &view, heads, new_log, rec, signer, claimed_ms, keys_root)
    }

    /// ONE SEAL PER SYNC (W-MG-M2, K2: 3 448 B a seal): the device's `recs`, oldest first, each its
    /// next seq, appended under ONE seal over the newest, in one transaction. No key op rides here:
    /// an ADMIT must be `commit_bytes`, because its KEY row names its record. `verify` needs nothing
    /// new: the tip reaches the records under it by `prev`, and `covered` takes them by seq.
    /// The id of `actor`'s record at `seq`: what a REVOKE after `seq` names. A frontier row answers
    /// under a cut. `None` when this file has no such record.
    pub fn tip_at(st: &Store, actor: &[u8; 32], seq: u64) -> Option<[u8; 32]> {
        let view = Doc::open(st).ok()?;
        let held = EvLog::walk_until_at(st, view.log, |_| false).iter().find(|r| r.actor_pubkey == *actor && r.actor_seq == seq).map(|r| r.id);
        held.or_else(|| frontier_of(&read_segs(st, view.seg, EvLog::step_cap(st)).ok()?, actor).filter(|f| f.seq == seq).map(|f| f.tip))
    }

    pub fn commit_many_bytes(st: &mut Store, recs: &[Record], signer: &dyn Signer, claimed_ms: i64) -> Result<i64, DocError> {
        let view = Doc::open(st)?;
        let mut log = view.log.ok_or(DocError::Corrupt("a DOC without a log"))?;
        let actor = signer.actor_id();
        let Some(last) = recs.last() else { return Err(DocError::Corrupt("a sync with no record")) };
        let keys = read_keys(st, view.keys);
        let key = keys.iter().find(|k| k.actor_id == actor).ok_or(DocError::UnknownActor(actor))?;
        if key.alg != signer.alg() {
            return Err(DocError::Actor);
        }
        let heads = read_heads(st, view.heads);
        let mut want = heads.iter().find(|h| h.actor_id == actor).map_or(1, |h| h.seq + 1);
        for r in recs {
            if r.actor_pubkey != actor {
                return Err(DocError::Actor);
            }
            if !key.valid_at(r.actor_seq) {
                return Err(DocError::Revoked { after_seq: key.revoked_at, seq: r.actor_seq });
            }
            if r.actor_seq != want {
                return Err(DocError::Seq { want, got: r.actor_seq });
            }
            want += 1;
        }
        let mut tx = st.begin()?;
        for r in recs {
            log = EvLog::stage_append_in(st, &mut tx, log, r, Some(&r.id))?;
        }
        seal_tip(st, tx, &view, heads, log, last, signer, claimed_ms, view.keys)
    }
}

/// Is `seq` of `actor` under a cut, with no frontier row at it? Then no file can name its tip.
fn under_cut(st: &Store, actor: &[u8; 32], seq: u64) -> bool {
    let view = Doc::open(st).ok();
    let segs = view.and_then(|v| read_segs(st, v.seg, EvLog::step_cap(st)).ok()).unwrap_or_default();
    frontier_of(&segs, actor).is_some_and(|f| seq < f.seq)
}

/// The end of a device commit: one SEAL over `rec`, its HEADS row moved, a new DOC, committed. A
/// `StoreError` here is `Err` to the caller as it always was.
#[allow(clippy::too_many_arguments)]
fn seal_tip(st: &mut Store, mut tx: Tx, view: &super::DocView, mut heads: Vec<Head>, new_log: usize, rec: &Record, signer: &dyn Signer, claimed_ms: i64, keys_root: Option<usize>) -> Result<i64, DocError> {
    let actor = signer.actor_id();
    let head = heads.iter().find(|h| h.actor_id == actor).copied();
    let prev_tip = head.map_or([0u8; 32], |h| h.tip);
    let msg = message(&actor, rec.actor_seq, &rec.id, claimed_ms, &prev_tip);
    let sig = signer.sign(&msg);
    let seal = stage_seal(st, &mut tx, &actor, rec.actor_seq, &rec.id, claimed_ms, head.and_then(|h| h.seal), signer.alg(), &sig)?;
    heads.retain(|h| h.actor_id != actor);
    heads.push(Head { actor_id: actor, seq: rec.actor_seq, tip: rec.id, seal: Some(seal) });
    if let Some(t) = view.heads {
        tx.sup_delta += 2 + st.obj_cells(t) as i64;
    }
    let heads_root = stage_heads(st, &mut tx, &mut heads)?;
    tx.sup_delta += 2 + st.obj_cells(view.at) as i64;
    let doc = stage_doc(
        st,
        &mut tx,
        view.flags,
        Refs { log: Some(new_log), tree: view.tree, heads: Some(heads_root), keys: keys_root, spec: view.spec, anch: view.anch, seg: view.seg },
    )?;
    Ok(st.commit_bytes(&tx, doc))
}
