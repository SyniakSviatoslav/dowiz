//! THE SEAMS (W-MG-M1): who signs, who checks, how a record's id is derived. This crate holds no
//! hash and no signature, so these are traits; dowiz-hub implements them for ML-DSA-65 and
//! sha256. `id_message` is here because every implementor must hash the SAME bytes.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::Record;

/// The domain tag in front of a chained id's message (W-MG-HOLES, 2026-10-31).
pub const ID_TAG: &[u8] = b"bebop-record/1";

/// What a chained id is the hash of: `ID_TAG ‖ flags LE8 ‖ actor 32 ‖ seq LE8 ‖ prev 32 ‖ payload`.
/// Every cell of the record is in it, because a cell outside the id is a cell a forger with no
/// key may change: M0 hashed `prev ‖ payload` alone, and clearing `FLAG_SHA256` moved where the
/// body starts while the id stayed right, so a REVOKE stopped parsing and any order left
/// `events()` (`atk_holes` hole 2), and the actor of a record under a many-record seal was free
/// (hole 9). `flags` is the record's flags, `FLAG_ACTOR` not among them (`Record::flags`).
///
/// A REDACTION KEEPS THE ID, AND THE ID THEN CHECKS NOTHING: a tombstone's payload is not the
/// payload hashed, so `chained` no longer answers its id and `verify` reads it as a redaction,
/// its link held by its neighbour or its head. Its cells are bound by the FORGOTTEN that lists it.
pub fn id_message(flags: i64, actor: &[u8; 32], seq: u64, prev: &[u8; 32], payload: &[u8]) -> Vec<u8> {
    let mut m = Vec::with_capacity(ID_TAG.len() + 80 + payload.len());
    m.extend_from_slice(ID_TAG);
    m.extend_from_slice(&flags.to_le_bytes());
    m.extend_from_slice(actor);
    m.extend_from_slice(&seq.to_le_bytes());
    m.extend_from_slice(prev);
    m.extend_from_slice(payload);
    m
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
    /// `false` for a reader that carries no signature algorithm at all: every seal is then
    /// unjudged rather than forged, and a TRUNCATE or FORGOTTEN counts on its chain alone.
    /// A cut file stays readable by such a reader; `Report::holds` stays false on it.
    fn checks_signatures(&self) -> bool {
        true
    }
    /// The actor id `pk` belongs to (dowiz-hub: sha256 over it), which binds a KEY row to the id
    /// it names: a seal needs `actor_of(key.pk) == seal.actor_id`, roots included, or the unsigned
    /// row could hand a trusted id to a forger's key (`atk_holes` hole 1). `None` = this
    /// verifier has no such rule: the toys, and a reader with no algorithm, which judges no seal.
    fn actor_of(&self, pk: &[u8]) -> Option<[u8; 32]> {
        let _ = pk;
        None
    }
}

/// How a record's id is derived (M0 owns the scheme; this crate only asks). `chained` takes the
/// whole record: the id commits to every cell a reader acts on (`id_message`). Every other
/// question is asked WITH the record's flags cell (W-MG-M1B): M0's framing -- a 16-byte nonce in front
/// of the payload -- is there exactly when `FLAG_SHA256` is set, so an implementor that was
/// handed the payload alone would have to guess where the body starts, and a reader never
/// guesses. `legacy` answers whether `id` holds under a pre-M0 scheme of old images (the
/// implementor may have several, and one of them needs `prev`; `false` = it has none);
/// `redacted` reads the in-place redaction mark; `body` strips the implementor's framing
/// (nonce, kind byte) so the key-op bodies of `keys.rs` are found where the implementor put
/// them.
pub trait Ids {
    fn chained(&self, r: &Record) -> [u8; 32];
    fn legacy(&self, flags: i64, prev: &[u8; 32], payload: &[u8], id: &[u8; 32]) -> bool {
        let _ = (flags, prev, payload, id);
        false
    }
    fn redacted(&self, flags: i64, payload: &[u8]) -> bool {
        let _ = (flags, payload);
        false
    }
    fn body<'a>(&self, flags: i64, payload: &'a [u8]) -> &'a [u8] {
        let _ = flags;
        payload
    }
    /// The hash of `bytes`; `None` = none here, and the segment chain cannot be named, so `cut`
    /// says `NoTree` and `verify` refuses a segment. dowiz-hub: sha256.
    fn hash(&self, bytes: &[u8]) -> Option<[u8; 32]> {
        let _ = bytes;
        None
    }
    /// The root a segment is named by: RFC 9162's tree over the cut records' ids in file
    /// order (dowiz-hub: `bebop_store::merkle`, leaf = sha256(0x00 ‖ id)). This crate has no
    /// hash, so the implementor's absence (`None`) is `DocError::NoTree`: nothing is cut
    /// under a root nobody can recompute.
    fn tree_root(&self, ids: &[[u8; 32]]) -> Option<[u8; 32]> {
        let _ = ids;
        None
    }
}
