//! KEY and KEYS (R-MAINGOAL-A §1.2, §2e): one KEY object per admitted device key, a sorted
//! table naming them, and the two key-op bodies a device writes INTO ITS OWN SEALED CHAIN --
//! ADMIT (a new key) and REVOKE (after a seq). The table is an index; the records are the
//! signed truth, and `verify.rs` admits a key only through its ADMIT record.
//!
//! ```text
//! KEY  v1  {actor_id 4, alg, valid_from, revoked_at, ref ADMIT_REC, pk_len, pk packed}
//!          9 + ceil(pk_len / 8) cells (ML-DSA-65: 9 + 244 = 253). revoked_at = 0: not
//!          revoked; k > 0: seals with seq > k are invalid. ADMIT_REC = 0: a genesis key
//!          (written by `Doc::init_bytes`, trusted by `Trust::Genesis`).
//! KEYS     {m, (actor_id 4, ref KEY) × m}   sorted by actor_id, 1 + 5m cells
//! ```
//!
//! Key-op bodies, found in a record's payload through `Ids::body` (the implementor's framing
//! -- M0's nonce, the kind byte -- sits in front of them):
//!
//! ```text
//! ADMIT   "bebop-admit/1"  ‖ actor_id 32 ‖ alg LE8 ‖ valid_from LE8 ‖ pk_len LE8 ‖ pk
//! REVOKE  "bebop-revoke/1" ‖ actor_id 32 ‖ after_seq LE8
//! ```
//!
//! Rotation = ADMIT of the new key + REVOKE of the old, two records of one chain; this crate
//! offers the format, no lane runs it (memory `no-secret-rotation.md`).

#[allow(unused_imports)]
use crate::prelude::*;
use crate::{Store, StoreError, Tx};
use super::{bytes_to_cells, cells_of, cells_to_bytes, cells_to_id, id_to_cells};

/// ML-DSA-65 (FIPS 204), the only algorithm the plan names. pk 1 952 B, sig 3 309 B.
pub const ALG_ML_DSA_65: i64 = 65;
pub const LAYOUT_KEY: &str = "KEY{i64,i64,i64,i64,i64,i64,i64,ref EV,i64,[i64]}";
pub const DIGEST_KEY: i64 = 2540354000;
pub const LAYOUT_KEYS: &str = "KEYS{i64,[i64,i64,i64,i64,ref KEY]}";
pub const DIGEST_KEYS: i64 = 1448248784;
const KEY_HEAD: usize = 9;
const KEYS_ROW: usize = 5;
pub const ADMIT_MARK: &[u8] = b"bebop-admit/1";
pub const REVOKE_MARK: &[u8] = b"bebop-revoke/1";
/// The longest public key a KEY may carry: 64 KiB, far above any FIPS 204 size, so a damaged
/// `pk_len` never becomes an allocation.
pub const MAX_PK_BYTES: usize = 65536;

/// One admitted key as read out of (or staged into) an image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    pub actor_id: [u8; 32],
    pub alg: i64,
    pub valid_from: i64,
    /// 0 = not revoked; otherwise seals with `seq > revoked_at` are invalid.
    pub revoked_at: i64,
    /// The ADMIT record (absolute offset) that admitted it; `None` = genesis.
    pub admit_rec: Option<usize>,
    pub pk: Vec<u8>,
}

impl Key {
    /// May this key seal `seq`?
    pub fn valid_at(&self, seq: u64) -> bool {
        self.revoked_at == 0 || (seq as i64) <= self.revoked_at
    }
}

/// A key operation carried by a sealed commit (`Doc::commit_bytes`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyOp {
    /// Admit `key` (its `admit_rec` is filled in by the writer).
    Admit(Key),
    /// Revoke `actor_id`'s key: seals past `after_seq` are invalid.
    Revoke { actor_id: [u8; 32], after_seq: i64 },
}

impl KeyOp {
    /// The body a record carries for this op (the caller frames it and derives the id).
    pub fn body(&self) -> Vec<u8> {
        match self {
            KeyOp::Admit(k) => admit_body(&k.actor_id, k.alg, k.valid_from, &k.pk),
            KeyOp::Revoke { actor_id, after_seq } => revoke_body(actor_id, *after_seq),
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

pub fn revoke_body(actor_id: &[u8; 32], after_seq: i64) -> Vec<u8> {
    let mut b = Vec::with_capacity(REVOKE_MARK.len() + 40);
    b.extend_from_slice(REVOKE_MARK);
    b.extend_from_slice(actor_id);
    b.extend_from_slice(&after_seq.to_le_bytes());
    b
}

fn le8(b: &[u8]) -> Option<i64> {
    b.get(..8).and_then(|w| w.try_into().ok()).map(i64::from_le_bytes)
}
fn id32(b: &[u8]) -> Option<[u8; 32]> {
    b.get(..32).and_then(|w| w.try_into().ok())
}

/// `(actor_id, alg, valid_from, pk)` of an ADMIT body, or `None` when it is not one.
pub fn parse_admit(body: &[u8]) -> Option<([u8; 32], i64, i64, Vec<u8>)> {
    let rest = body.strip_prefix(ADMIT_MARK)?;
    let actor = id32(rest)?;
    let alg = le8(rest.get(32..)?)?;
    let valid_from = le8(rest.get(40..)?)?;
    let n = le8(rest.get(48..)?)?;
    if !(0..=MAX_PK_BYTES as i64).contains(&n) {
        return None;
    }
    let pk = rest.get(56..56 + n as usize)?.to_vec();
    Some((actor, alg, valid_from, pk))
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
    let n_cells = (pk_len as usize).div_ceil(8);
    if st.obj_cells(at) != KEY_HEAD + n_cells {
        return None;
    }
    Some(Key {
        actor_id: cells_to_id(&cells_of(st, at, 0, 4)),
        alg: st.get(at, 4),
        valid_from: st.get(at, 5),
        revoked_at: st.get(at, 6),
        admit_rec: st.follow(at, 7),
        pk: cells_to_bytes(&cells_of(st, at, KEY_HEAD, n_cells), pk_len as usize),
    })
}

/// The KEYS table's rows `(actor_id, KEY offset)`, in table order, bounded by the object.
pub fn key_rows(st: &Store, keys: Option<usize>) -> Vec<([u8; 32], usize)> {
    let Some(t) = keys else { return Vec::new() };
    if st.obj_digest(t) != DIGEST_KEYS || st.obj_cells(t) < 1 {
        return Vec::new();
    }
    let m = st.get(t, 0).max(0) as usize;
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
    key_rows(st, keys).into_iter().filter_map(|(_, k)| read_key(st, k)).collect()
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
