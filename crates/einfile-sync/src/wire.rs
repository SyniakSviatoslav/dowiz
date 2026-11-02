//! THE MESSAGES (§3): `merge_body.rs`'s coding, LE8, explicit lengths, an exact parser. Nothing here
//! is an image format: FORMAT §9 and the five readers do not move.
//!
//! ```text
//! MESSAGE   "ein-sync/1" ‖ kind u8 ‖ body
//! FRONTIER  1  doc 32 ‖ n LE8 ‖ (actor 32 ‖ seq LE8 ‖ tip 32 ‖ seals 32) × n
//!              ‖ seg_root 32 ‖ seg_hash 32 ‖ forgotten_hash 32 ‖ anchor_hash 32
//! TAIL      2  FRONTIER body
//!              ‖ n LE8 ‖ (prev 32 ‖ actor 32 ‖ seq LE8 ‖ flags LE8 ‖ len LE8 ‖ payload) × n
//!              ‖ n LE8 ‖ (actor 32 ‖ seq LE8 ‖ tip 32 ‖ claimed_ms LE8 ‖ alg LE8 ‖ len LE8 ‖ sig) × n
//!              ‖ n LE8 ‖ (kind LE8 ‖ over 32 ‖ time_ms LE8 ‖ len LE8 ‖ proof) × n
//! ACK       3  FRONTIER body
//! NEED      4  1 ‖ seg_root 32   (archive)    |    2 ‖ len LE8 ‖ name   (refused)
//! ```
//!
//! A record's id is not sent: the receiver derives it, so an id on the wire can never disagree with
//! what it names. A row is 104 bytes, not the design's 72: `Place` says why.
//!
//! THE ORDER OF THE WORK (§11): the length before the first read, each count against what is left
//! before anything is allocated, and each object's cap before its bytes are copied.

#[allow(unused_imports)]
use alloc::{string::String, string::ToString, vec, vec::Vec};
use einfile_log::doc::keys::parse_admit;
use einfile_log::doc::seal::{Seal, MAX_SIG_BYTES};
use einfile_log::doc::{Anchor, DocError, Front, Ids, Place, Row, Tail};
use einfile_log::Record;
use crate::{Budget, SyncError};

pub const MARK: &[u8] = b"ein-sync/1";
const K_FRONTIER: u8 = 1;
const K_TAIL: u8 = 2;
const K_ACK: u8 = 3;
const K_NEED: u8 = 4;
const NEED_ARCHIVE: u8 = 1;
const NEED_REFUSED: u8 = 2;
const PLACE_BYTES: usize = 104;
const RECORD_MIN: usize = 88;
const SEAL_MIN: usize = 96;
const ANCHOR_MIN: usize = 56;
const NAME_MAX: u64 = 64;

/// What the other side needs before a tail can mean anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Need {
    /// Another segment chain: the whole file, `seg_root` naming the newest we hold (§10).
    Archive { seg_root: [u8; 32] },
    /// The library's refusal, by name: nothing was written.
    Refused { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    Frontier(Front),
    Tail(Tail),
    Ack(Front),
    Need(Need),
}

/// A refusal's name on the wire. Only names: a peer learns no more than which rule said no.
pub fn refusal_name(e: &DocError) -> &'static str {
    match e {
        DocError::Fork { .. } => "Fork",
        DocError::SealFork { .. } => "SealFork",
        DocError::KeyClash(_) => "KeyClash",
        DocError::ForeignRoot => "ForeignRoot",
        DocError::CutOneSide { .. } => "CutOneSide",
        DocError::ForgetOneSide => "ForgetOneSide",
        DocError::RevokedTail { .. } => "RevokedTail",
        DocError::LegacyTail { .. } => "LegacyTail",
        DocError::NotWhole { .. } => "NotWhole",
        DocError::UnionNotWhole => "UnionNotWhole",
        DocError::Seq { .. } => "Seq",
        _ => "Refused",
    }
}

fn head(kind: u8) -> Vec<u8> {
    let mut b = MARK.to_vec();
    b.push(kind);
    b
}

fn put_n(b: &mut Vec<u8>, n: usize) {
    b.extend_from_slice(&(n as u64).to_le_bytes());
}

fn put_front(b: &mut Vec<u8>, f: &Front) {
    b.extend_from_slice(&f.doc);
    put_n(b, f.places.len());
    for p in &f.places {
        b.extend_from_slice(&p.row.actor_id);
        b.extend_from_slice(&p.row.seq.to_le_bytes());
        b.extend_from_slice(&p.row.tip);
        b.extend_from_slice(&p.seals);
    }
    for h in [&f.seg_root, &f.seg_hash, &f.forgotten_hash, &f.anchor_hash] {
        b.extend_from_slice(h);
    }
}

pub fn encode_frontier(f: &Front) -> Vec<u8> {
    let mut b = head(K_FRONTIER);
    put_front(&mut b, f);
    b
}

pub fn encode_ack(f: &Front) -> Vec<u8> {
    let mut b = head(K_ACK);
    put_front(&mut b, f);
    b
}

pub fn encode_need(n: &Need) -> Vec<u8> {
    let mut b = head(K_NEED);
    match n {
        Need::Archive { seg_root } => {
            b.push(NEED_ARCHIVE);
            b.extend_from_slice(seg_root);
        }
        Need::Refused { name } => {
            b.push(NEED_REFUSED);
            put_n(&mut b, name.len());
            b.extend_from_slice(name.as_bytes());
        }
    }
    b
}

/// A tail as a message, or `Over` when it is not one this budget's peer would read.
pub fn encode_tail(t: &Tail, budget: &Budget) -> Result<Vec<u8>, SyncError> {
    for (what, got, max) in [("records", t.recs.len() as u64, budget.records), ("seals", t.seals.len() as u64, budget.seals), ("anchors", t.anchors.len() as u64, budget.anchors)] {
        if got > max {
            return Err(SyncError::Over { what, got, max });
        }
    }
    let mut b = head(K_TAIL);
    put_front(&mut b, &t.front);
    put_n(&mut b, t.recs.len());
    for r in &t.recs {
        b.extend_from_slice(&r.prev);
        b.extend_from_slice(&r.actor_pubkey);
        b.extend_from_slice(&r.actor_seq.to_le_bytes());
        b.extend_from_slice(&r.flags.to_le_bytes());
        put_n(&mut b, r.payload.len());
        b.extend_from_slice(&r.payload);
    }
    put_n(&mut b, t.seals.len());
    for s in &t.seals {
        b.extend_from_slice(&s.actor_id);
        b.extend_from_slice(&s.seq.to_le_bytes());
        b.extend_from_slice(&s.tip);
        b.extend_from_slice(&s.claimed_ms.to_le_bytes());
        b.extend_from_slice(&s.alg.to_le_bytes());
        put_n(&mut b, s.sig.len());
        b.extend_from_slice(&s.sig);
    }
    put_n(&mut b, t.anchors.len());
    for a in &t.anchors {
        b.extend_from_slice(&a.kind.to_le_bytes());
        b.extend_from_slice(&a.over);
        b.extend_from_slice(&a.time_ms.to_le_bytes());
        put_n(&mut b, a.proof.len());
        b.extend_from_slice(&a.proof);
    }
    if b.len() > budget.message {
        return Err(SyncError::Over { what: "message", got: b.len() as u64, max: budget.message as u64 });
    }
    Ok(b)
}

struct Cur<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Cur<'a> {
    fn left(&self) -> usize {
        self.b.len().saturating_sub(self.at)
    }
    fn take(&mut self, n: usize, what: &'static str) -> Result<&'a [u8], SyncError> {
        let end = self.at.checked_add(n).ok_or(SyncError::Short(what))?;
        let s = self.b.get(self.at..end).ok_or(SyncError::Short(what))?;
        self.at = end;
        Ok(s)
    }
    fn id(&mut self, what: &'static str) -> Result<[u8; 32], SyncError> {
        self.take(32, what)?.try_into().map_err(|_| SyncError::Short(what))
    }
    fn u(&mut self, what: &'static str) -> Result<u64, SyncError> {
        let w: [u8; 8] = self.take(8, what)?.try_into().map_err(|_| SyncError::Short(what))?;
        Ok(u64::from_le_bytes(w))
    }
    fn i(&mut self, what: &'static str) -> Result<i64, SyncError> {
        Ok(self.u(what)? as i64)
    }
    /// A count, under its budget and no larger than the bytes left can hold.
    fn count(&mut self, what: &'static str, max: u64, unit: usize) -> Result<usize, SyncError> {
        let n = self.u(what)?;
        if n > max {
            return Err(SyncError::Over { what, got: n, max });
        }
        if n > (self.left() / unit) as u64 {
            return Err(SyncError::Short(what));
        }
        Ok(n as usize)
    }
    /// A length, under its cap, and the bytes behind it.
    fn blob(&mut self, what: &'static str, max: u64) -> Result<&'a [u8], SyncError> {
        let n = self.u(what)?;
        if n > max {
            return Err(SyncError::Over { what, got: n, max });
        }
        self.take(usize::try_from(n).map_err(|_| SyncError::Short(what))?, what)
    }
}

fn get_front(c: &mut Cur<'_>, budget: &Budget) -> Result<Front, SyncError> {
    let doc = c.id("doc")?;
    let n = c.count("places", budget.places, PLACE_BYTES)?;
    let mut places = Vec::with_capacity(n);
    for _ in 0..n {
        let row = Row { actor_id: c.id("place")?, seq: c.u("place")?, tip: c.id("place")? };
        places.push(Place { row, seals: c.id("place")? });
    }
    Ok(Front { doc, places, seg_root: c.id("seg_root")?, seg_hash: c.id("seg_hash")?, forgotten_hash: c.id("forgotten_hash")?, anchor_hash: c.id("anchor_hash")? })
}

fn get_tail(c: &mut Cur<'_>, budget: &Budget, ids: &dyn Ids) -> Result<Tail, SyncError> {
    let front = get_front(c, budget)?;
    let n = c.count("records", budget.records, RECORD_MIN)?;
    let mut recs = Vec::with_capacity(n);
    let mut keys = 0u64;
    for _ in 0..n {
        let (prev, actor_pubkey) = (c.id("record")?, c.id("record")?);
        let (actor_seq, flags) = (c.u("record")?, c.i("record")?);
        let payload = c.blob("record", budget.record_bytes)?.to_vec();
        let mut r = Record { id: [0u8; 32], prev, actor_pubkey, actor_seq, flags, payload };
        r.id = ids.chained(&r);
        keys += u64::from(parse_admit(ids.body(r.flags, &r.payload)).is_some());
        if keys > budget.keys {
            return Err(SyncError::Over { what: "keys", got: keys, max: budget.keys });
        }
        recs.push(r);
    }
    let n = c.count("seals", budget.seals, SEAL_MIN)?;
    let mut seals = Vec::with_capacity(n);
    for _ in 0..n {
        let (actor_id, seq, tip) = (c.id("seal")?, c.u("seal")?, c.id("seal")?);
        let (claimed_ms, alg) = (c.i("seal")?, c.i("seal")?);
        let sig = c.blob("seal", MAX_SIG_BYTES as u64)?.to_vec();
        seals.push(Seal { at: 0, actor_id, seq, tip, claimed_ms, prev: None, time: None, alg, sig });
    }
    let n = c.count("anchors", budget.anchors, ANCHOR_MIN)?;
    let mut anchors = Vec::with_capacity(n);
    for _ in 0..n {
        let (kind, over, time_ms) = (c.i("anchor")?, c.id("anchor")?, c.i("anchor")?);
        let proof = c.blob("anchor", budget.proof_bytes)?.to_vec();
        anchors.push(Anchor { at: 0, kind, over, time_ms, prev: None, proof });
    }
    Ok(Tail { front, recs, seals, anchors })
}

fn get_need(c: &mut Cur<'_>) -> Result<Need, SyncError> {
    match c.take(1, "need")?.first() {
        Some(&NEED_ARCHIVE) => Ok(Need::Archive { seg_root: c.id("need")? }),
        Some(&NEED_REFUSED) => {
            let name = c.blob("need", NAME_MAX)?;
            Ok(Need::Refused { name: core::str::from_utf8(name).map_err(|_| SyncError::Mark("need"))?.to_string() })
        }
        _ => Err(SyncError::Mark("need")),
    }
}

/// One message, exactly. `Over` before a byte of a message past the budget is read.
pub fn decode(bytes: &[u8], budget: &Budget, ids: &dyn Ids) -> Result<Msg, SyncError> {
    if bytes.len() > budget.message {
        return Err(SyncError::Over { what: "message", got: bytes.len() as u64, max: budget.message as u64 });
    }
    let body = bytes.strip_prefix(MARK).ok_or(SyncError::Mark("message"))?;
    let mut c = Cur { b: body, at: 0 };
    let msg = match c.take(1, "kind")?.first() {
        Some(&K_FRONTIER) => Msg::Frontier(get_front(&mut c, budget)?),
        Some(&K_TAIL) => Msg::Tail(get_tail(&mut c, budget, ids)?),
        Some(&K_ACK) => Msg::Ack(get_front(&mut c, budget)?),
        Some(&K_NEED) => Msg::Need(get_need(&mut c)?),
        _ => return Err(SyncError::Mark("kind")),
    };
    if c.left() != 0 {
        return Err(SyncError::Trailing);
    }
    Ok(msg)
}

#[cfg(test)]
mod tests;
