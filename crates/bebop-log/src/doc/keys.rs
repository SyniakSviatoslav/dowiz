//! KEY and KEYS (R-MAINGOAL-A §1.2, §2e): one KEY object per admitted device key, a sorted
//! table naming them, and the two key-op bodies a device writes INTO ITS OWN SEALED CHAIN --
//! ADMIT (a new key) and REVOKE (after a seq). The table is an index; the records are the
//! signed truth: `settle.rs` admits a key only through its ADMIT record and revokes it only
//! through its REVOKE records.
//!
//! ```text
//! KEY  v1  {actor_id 4, alg, valid_from, revoked_at, ref ADMIT_REC, pk_len, pk packed}
//!          9 + ceil(pk_len / 8) cells (ML-DSA-65: 9 + 244 = 253). revoked_at = 0: not
//!          revoked; k > 0: seals with seq > k are invalid; -1: revoked after 0, no seal is
//!          valid. An INDEX of the REVOKE records, signed by nobody (`settle.rs`). ADMIT_REC
//!          = 0: a genesis key (written by `Doc::init_bytes`, trusted by `Trust::Genesis`).
//! KEYS     {m, (actor_id 4, ref KEY) × m}   sorted by actor_id, 1 + 5m cells
//! ```
//!
//! Key-op bodies, found in a record's payload through `Ids::body` (the implementor's framing
//! -- M0's nonce, the kind byte -- sits in front of them):
//!
//! ```text
//! ADMIT   "ein-admit/1"  ‖ actor_id 32 ‖ alg LE8 ‖ valid_from LE8 ‖ pk_len LE8 ‖ pk
//! REVOKE  "ein-revoke/2" ‖ actor_id 32 ‖ after_seq LE8 ‖ tip 32
//! ```
//!
//! `tip` (W-MG-MERGE2, after W-MG-TAM's trace H2) is the id of the target's record at `after_seq`,
//! 0^32 when `after_seq` <= 0. A seq alone let the holder of a revoked key write a new record AT OR
//! BELOW it after the fact, and nothing the revoker signed moved. With the tip, the chain at
//! `after_seq` is what the revoker saw, and `prev` binds all of it below. Where that seq left under
//! a cut, only the seq binds (FORMAT §14). A key revoking itself after its own REVOKE writes 0^32:
//! that record's id covers the body, so it cannot name itself.
//!
//! Rotation = ADMIT of the new key + REVOKE of the old, two records of one chain; this crate
//! offers the format, no lane runs it (memory `no-secret-rotation.md`). A revoked key stays
//! revoked: ADMIT does not bring it back (`DocError::Readmit`), a new device takes a new key.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::{Store, StoreError, Tx};
use bebop_image::{idx, idx_end};
use super::budget::Budget;
use super::{bytes_to_cells, cells_of, cells_to_bytes, cells_to_id, id_to_cells, DocError};

/// ML-DSA-65 (FIPS 204), the only algorithm the plan names. pk 1 952 B, sig 3 309 B.
pub const ALG_ML_DSA_65: i64 = 65;
pub const LAYOUT_KEY: &str = "KEY{i64,i64,i64,i64,i64,i64,i64,ref EV,i64,[i64]}";
pub const DIGEST_KEY: i64 = 2540354000;
pub const LAYOUT_KEYS: &str = "KEYS{i64,[i64,i64,i64,i64,ref KEY]}";
pub const DIGEST_KEYS: i64 = 1448248784;
const KEY_HEAD: usize = 9;
const KEYS_ROW: usize = 5;
pub const ADMIT_MARK: &[u8] = b"ein-admit/1";
pub const REVOKE_MARK: &[u8] = b"ein-revoke/2";
/// The longest public key a KEY may carry: 64 KiB, far above any FIPS 204 size, so a damaged
/// `pk_len` never becomes an allocation.
pub const MAX_PK_BYTES: usize = 65536;
/// `revoked_at` for "after 0": 0 is taken by "not revoked", and a key that never sealed must
/// be revocable. `Key::valid_at` already reads it right: no seq is at or below it.
pub const REVOKED_ALL: i64 = -1;

/// One admitted key as read out of (or staged into) an image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    pub actor_id: [u8; 32],
    pub alg: i64,
    pub valid_from: i64,
    /// 0 = not revoked; k > 0: seals with `seq > k` are invalid; `REVOKED_ALL`: none is valid.
    pub revoked_at: i64,
    /// The ADMIT record (absolute offset) that admitted it; `None` = genesis.
    pub admit_rec: Option<usize>,
    pub pk: Vec<u8>,
}

impl Key {
    /// May this key seal `seq`? Compared unsigned: `seq` is the record's, written by whoever
    /// holds the key, and a seq past `i64::MAX` must not wrap to below every `revoked_at`.
    pub fn valid_at(&self, seq: u64) -> bool {
        valid_at(self.revoked_at, seq)
    }
}

/// `Key::valid_at` over the cell alone: `settle.rs` folds `revoked_at` and never clones a key.
pub(crate) fn valid_at(revoked_at: i64, seq: u64) -> bool {
    revoked_at == 0 || (revoked_at > 0 && seq <= revoked_at as u64)
}

/// A key operation carried by a sealed commit (`Doc::commit_bytes`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyOp {
    /// Admit `key` (its `admit_rec` is filled in by the writer).
    Admit(Key),
    /// Revoke `actor_id`'s key: seals past `after_seq` are invalid, and its record there is `tip`
    /// (`Doc::tip_at`), 0^32 when `after_seq` <= 0.
    Revoke { actor_id: [u8; 32], after_seq: i64, tip: [u8; 32] },
}

/// `revoked_at` after one more REVOKE: the earliest wins, so no REVOKE ever widens a key.
pub fn narrower(revoked_at: i64, after_seq: i64) -> i64 {
    let this = if after_seq > 0 { after_seq } else { REVOKED_ALL };
    if revoked_at == 0 {
        this
    } else {
        revoked_at.min(this)
    }
}

impl KeyOp {
    /// The body a record carries for this op (the caller frames it and derives the id).
    pub fn body(&self) -> Vec<u8> {
        match self {
            KeyOp::Admit(k) => admit_body(&k.actor_id, k.alg, k.valid_from, &k.pk),
            KeyOp::Revoke { actor_id, after_seq, tip } => revoke_body(actor_id, *after_seq, tip),
        }
    }
}

pub fn admit_body(actor_id: &[u8; 32], alg: i64, valid_from: i64, pk: &[u8]) -> Vec<u8> {
    let mut b = Vec::with_capacity(ADMIT_MARK.len() + 32 + 24 + pk.len());
    b.extend_from_slice(ADMIT_MARK);
    b.extend_from_slice(actor_id);
    b.extend_from_slice(&alg.to_le_bytes());
    b.extend_from_slice(&valid_from.to_le_bytes());
    b.extend_from_slice(&(pk.len() as i64).to_le_bytes());
    b.extend_from_slice(pk);
    b
}

pub fn revoke_body(actor_id: &[u8; 32], after_seq: i64, tip: &[u8; 32]) -> Vec<u8> {
    let mut b = Vec::with_capacity(REVOKE_MARK.len() + 72);
    b.extend_from_slice(REVOKE_MARK);
    b.extend_from_slice(actor_id);
    b.extend_from_slice(&after_seq.to_le_bytes());
    b.extend_from_slice(tip);
    b
}

fn le8(b: &[u8]) -> Option<i64> {
    b.get(..8).and_then(|w| w.try_into().ok()).map(i64::from_le_bytes)
}
fn id32(b: &[u8]) -> Option<[u8; 32]> {
    b.get(..32).and_then(|w| w.try_into().ok())
}

/// `(actor_id, alg, valid_from, pk)` of an ADMIT body, or `None` when it is not exactly one: EXACT,
/// like FORGOTTEN and MERGE (W-MG-TIDY2), a byte past `pk` makes it no key op.
pub fn parse_admit(body: &[u8]) -> Option<([u8; 32], i64, i64, Vec<u8>)> {
    admit_view(body).map(|(a, alg, from, pk)| (a, alg, from, pk.to_vec()))
}

/// `parse_admit` borrowing the key: the fixpoint asks once per key per pass.
pub(crate) fn admit_view(body: &[u8]) -> Option<([u8; 32], i64, i64, &[u8])> {
    let rest = body.strip_prefix(ADMIT_MARK)?;
    let actor = id32(rest)?;
    let alg = le8(rest.get(32..)?)?;
    let valid_from = le8(rest.get(40..)?)?;
    let n = le8(rest.get(48..)?)?;
    if !(0..=MAX_PK_BYTES as i64).contains(&n) {
        return None;
    }
    let pk = rest.get(56..)?;
    if pk.len() != idx(n)? {
        return None;
    }
    Some((actor, alg, valid_from, pk))
}

/// `(actor_id, after_seq, tip)` of a REVOKE body, or `None` when it is not exactly one.
pub fn parse_revoke(body: &[u8]) -> Option<([u8; 32], i64, [u8; 32])> {
    let rest = body.strip_prefix(REVOKE_MARK)?;
    if rest.len() != 72 {
        return None;
    }
    Some((id32(rest)?, le8(rest.get(32..)?)?, id32(rest.get(40..)?)?))
}

/// One KEY object, bounded by what it really holds; `None` for a wrong digest or length.
pub fn read_key(st: &Store, at: usize) -> Option<Key> {
    if st.obj_digest(at) != DIGEST_KEY || st.obj_cells(at) < KEY_HEAD {
        return None;
    }
    let pk_len = st.get(at, 8);
    if !(0..=MAX_PK_BYTES as i64).contains(&pk_len) {
        return None;
    }
    let n_cells = idx_end(pk_len).div_ceil(8);
    if st.obj_cells(at) != KEY_HEAD + n_cells {
        return None;
    }
    Some(Key {
        actor_id: cells_to_id(&cells_of(st, at, 0, 4)),
        alg: st.get(at, 4),
        valid_from: st.get(at, 5),
        revoked_at: st.get(at, 6),
        admit_rec: st.follow(at, 7),
        pk: cells_to_bytes(&cells_of(st, at, KEY_HEAD, n_cells), idx_end(pk_len)),
    })
}

/// The KEYS table's rows `(actor_id, KEY offset)`, in table order, bounded by the object.
pub fn key_rows(st: &Store, keys: Option<usize>) -> Vec<([u8; 32], usize)> {
    let Some(t) = keys else { return Vec::new() };
    if st.obj_digest(t) != DIGEST_KEYS || st.obj_cells(t) < 1 {
        return Vec::new();
    }
    let m = idx_end(st.get(t, 0).max(0));
    let m = m.min(st.obj_cells(t).saturating_sub(1) / KEYS_ROW);
    (0..m)
        .filter_map(|i| {
            let b = 1 + i * KEYS_ROW;
            st.follow(t, b + 4).map(|k| (cells_to_id(&cells_of(st, t, b, 4)), k))
        })
        .collect()
}

/// The KEYS table's keys, in table order. A row whose KEY is unreadable is skipped: the
/// seals it would have covered then count as bad, which is the refusal-biased answer.
pub fn read_keys(st: &Store, keys: Option<usize>) -> Vec<Key> {
    let mut budget = Budget::of(st);
    key_rows(st, keys).into_iter().map_while(|(_, k)| budget.spend(st, k).ok().map(|_| k)).filter_map(|k| read_key(st, k)).collect()
}

/// `read_keys` for `verify`: one row's KEY is paid for before its key is copied, and a table that
/// runs the budget out is refused, never read short.
pub(crate) fn read_keys_in(st: &Store, keys: Option<usize>, budget: &mut Budget) -> Result<Vec<Key>, DocError> {
    let mut out = Vec::new();
    for (_, k) in key_rows(st, keys) {
        budget.spend(st, k)?;
        out.extend(read_key(st, k));
    }
    Ok(out)
}

pub(crate) fn stage_key(st: &mut Store, tx: &mut Tx, key: &Key) -> Result<usize, StoreError> {
    let pkc = bytes_to_cells(&key.pk);
    let o = st.alloc(tx, (KEY_HEAD + pkc.len()) as i64, DIGEST_KEY)?;
    for (i, c) in id_to_cells(&key.actor_id).iter().enumerate() {
        st.put_cell(o, i, *c);
    }
    st.put_cell(o, 4, key.alg);
    st.put_cell(o, 5, key.valid_from);
    st.put_cell(o, 6, key.revoked_at);
    match key.admit_rec {
        Some(r) => st.link(o, 7, r),
        None => st.put_cell(o, 7, 0),
    }
    st.put_cell(o, 8, key.pk.len() as i64);
    for (j, c) in pkc.iter().enumerate() {
        st.put_cell(o, KEY_HEAD + j, *c);
    }
    st.seal(o);
    Ok(o)
}

/// The table over `(actor_id, KEY offset)` rows, sorted by actor id.
pub(crate) fn stage_keys(st: &mut Store, tx: &mut Tx, rows: &mut Vec<([u8; 32], usize)>) -> Result<usize, StoreError> {
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let t = st.alloc(tx, (1 + rows.len() * KEYS_ROW) as i64, DIGEST_KEYS)?;
    st.put_cell(t, 0, rows.len() as i64);
    for (i, (actor, at)) in rows.iter().enumerate() {
        let base = 1 + i * KEYS_ROW;
        for (j, c) in id_to_cells(actor).iter().enumerate() {
            st.put_cell(t, base + j, *c);
        }
        st.link(t, base + 4, *at);
    }
    st.seal(t);
    Ok(t)
}
