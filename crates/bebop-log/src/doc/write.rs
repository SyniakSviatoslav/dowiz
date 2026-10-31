//! THE DOC WRITER (W-MG-M1): `Doc::init_bytes` makes a DOC over an empty log with its
//! genesis keys; `Doc::commit_bytes` is ONE device commit -- the record appended and named
//! as the log's tip, an optional key op (ADMIT / REVOKE) carried by that record, one SEAL
//! over the tip, the HEADS row moved, a new DOC root -- all in one transaction, so there is
//! no generation in which a record exists without its seal.
//!
//! The writer polices what it can see (the actor's key exists and is not revoked at this
//! seq, the device's seq is the next one) and leaves AUTHORITY to `verify`: whether the key
//! was admitted by an admitted key is a question about the signed records, not the tables.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::{EvLog, Record, Store, StoreError, Tx};
use super::keys::{key_rows, read_keys, stage_key, stage_keys, Key, KeyOp};
use super::seal::{message, read_heads, stage_heads, stage_seal, Head};
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
        let mut heads = read_heads(st, view.heads);
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
                KeyOp::Admit(k) => Key { admit_rec: Some(ev), ..k.clone() },
                KeyOp::Revoke { actor_id, after_seq } => {
                    let old = keys.iter().find(|k| k.actor_id == *actor_id).ok_or(DocError::UnknownActor(*actor_id))?;
                    Key { revoked_at: *after_seq, ..old.clone() }
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
}
