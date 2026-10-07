//! An append-only image of records that are not orders.
//!
//! WHY A SECOND LOG TYPE. `Hub` is the ORDER log and its fold is an order's
//! life; putting an error, a chat message or a ledger posting into it would
//! make every one of those a thing the order fold has to skip, and the fold is
//! memoised per generation on the hot path of every poll. So: the same
//! machinery, a separate image, and a record shape that says what it is.
//!
//! WHAT IT REPLACES. Five D1 tables whose rows only ever arrive and are only
//! ever read back in time order — `worker_errors`, `thread_messages`,
//! `channel_messages`, `reservation_events`, `courier_audit_log` — plus the
//! postings half of the ledger. Every one of them had an index on
//! `(something, created_at_ms DESC)`, which is what an append-only log IS.
//!
//! IT INHERITS WHAT THE ORDER LOG LEARNED, and those lessons were expensive:
//!
//!   * **The record and the tip are ONE commit.** They were two, and the order
//!     log once refused order 2450 with fifty-six cells free: the record fitted
//!     and the tip update after it did not. `append_tip_bytes` makes "the
//!     record is in but the tip is not" unreachable.
//!   * **The image grows rather than refusing.** A full arena doubles and the
//!     chain is copied across verbatim — ids and `prev` links included, since
//!     both are content addresses.
//!   * **The copy goes oldest first.** `walk` is newest-first; appending in
//!     that order leaves every `prev` pointing at a record that does not exist
//!     yet, and the result is a chain of orphans that still LOOKS like a log.
//!   * **The ids are chained**, so an edit anywhere breaks every id after it
//!     and `chain_check` finds it.
//!
//! PAYLOAD: `[kind_len][kind][subject_len][subject][json]`. Two length-prefixed
//! short strings and then the record, so a reader never has to guess where one
//! ends — the order log's own `[kind][id_len][id][json]`, generalised.

use crate::{content_id_chained, ChainCheck, HubError};
use bebop_store::evlog::{EvLog, Record};
use bebop_store::Store;

/// Small: these images hold short records and are pruned. It doubles.
pub const DEFAULT_LOG_BYTES: usize = 64 * 1024;

/// The smallest image with any room in it at all.
///
/// DERIVED, NOT WRITTEN AS A NUMBER, because getting it wrong looks like a
/// corrupt store rather than a size mistake: the arena starts at cell
/// `bebop_store::ARENA` and a cell is eight bytes, so an image of exactly that
/// many bytes has a capacity of ZERO and refuses its first record with
/// `ArenaFull { capacity: 0 }`. `create_sized(8 * 1024)` did exactly that while
/// this file was being written. The floor is the arena plus a thousand cells of
/// actual room.
pub const MIN_LOG_BYTES: usize = (bebop_store::ARENA + 1024) * 8;

/// One appended record, decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub kind: String,
    pub subject: String,
    pub json: String,
    /// Position in the chain, oldest = 0. NOT a timestamp: the record's own
    /// JSON carries the time, because a position is not a clock and treating
    /// one as the other is how a fold starts disagreeing with a stored status.
    pub seq: u64,
}

pub struct LogImage {
    store: Store,
}

fn encode(kind: &str, subject: &str, json: &str) -> Result<Vec<u8>, HubError> {
    let (k, s) = (kind.as_bytes(), subject.as_bytes());
    if k.len() > 255 || s.len() > 255 {
        return Err(HubError::OrderIdTooLong);
    }
    let mut p = Vec::with_capacity(2 + k.len() + s.len() + json.len());
    p.push(k.len() as u8);
    p.extend_from_slice(k);
    p.push(s.len() as u8);
    p.extend_from_slice(s);
    p.extend_from_slice(json.as_bytes());
    Ok(p)
}

/// The named refusals, for the quarantine list. See `crate::decode_or_reason`:
/// "it did not decode" is not evidence, and evidence is what a quarantined
/// record is for.
fn decode_or_reason(p: &[u8], seq: u64) -> Result<Entry, &'static str> {
    decode(p, seq).ok_or_else(|| {
        let Some(&kl) = p.first() else { return "empty" };
        match p.get(1..1 + kl as usize) {
            None => "kind-framing",
            Some(_) => "subject-framing",
        }
    })
}

fn decode(p: &[u8], seq: u64) -> Option<Entry> {
    let kl = *p.first()? as usize;
    let k = p.get(1..1 + kl)?;
    let sl = *p.get(1 + kl)? as usize;
    let s = p.get(2 + kl..2 + kl + sl)?;
    let j = p.get(2 + kl + sl..)?;
    Some(Entry {
        kind: String::from_utf8_lossy(k).into_owned(),
        subject: String::from_utf8_lossy(s).into_owned(),
        json: String::from_utf8_lossy(j).into_owned(),
        seq,
    })
}

impl LogImage {
    pub fn create() -> Result<Self, HubError> {
        Self::create_sized(DEFAULT_LOG_BYTES)
    }

    pub fn create_sized(bytes: usize) -> Result<Self, HubError> {
        let mut store = Store::create_bytes(bytes.max(MIN_LOG_BYTES))?;
        EvLog::init_bytes(&mut store)?;
        Ok(LogImage { store })
    }

    pub fn load(bytes: &[u8]) -> Result<Self, HubError> {
        let store = Store::from_bytes(bytes);
        if store.pick().is_none() {
            return Err(HubError::NotAHub);
        }
        // A store with no log root is not this kind of image. Refusing is the
        // point: reading it as empty is a failure dressed as an absence, which
        // is the `.ok().flatten()` defect that would have orphaned a venue's
        // whole order log.
        if EvLog::len(&store) == 0 && store.root().is_none() {
            return Err(HubError::NotAHub);
        }
        // AND IT IS NOT THIS IMAGE EITHER IF IT LOST RECORDS ON THE WAY HERE.
        // The superblock is fifteen cells at the front, so it survives a
        // truncation that takes half the records with it -- this image loaded
        // clean and then said `len() == 40` while `entries()` gave two. That
        // is a venue quietly losing thirty-eight of its errors, messages or
        // ledger postings, and the one place it can still be said out loud is
        // here. See `crate::chain_is_whole`.
        // A record whose crc fails is QUARANTINED, not refused (`crate::quarantine`).
        crate::chain_is_whole_quarantining(&store)?;
        Ok(LogImage { store })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.store.to_bytes_trimmed()
    }

    pub fn len(&self) -> usize {
        EvLog::len(&self.store)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn usage(&self) -> crate::Usage {
        crate::usage_of_kind(&self.store, self.store.capacity_cells(), true)
    }

    /// Append one record. The id commits to the record before it.
    pub fn append(&mut self, kind: &str, subject: &str, json: &str) -> Result<(), HubError> {
        let payload = encode(kind, subject, json)?;
        let prev = EvLog::tip(&self.store).unwrap_or([0u8; 32]);
        let id = content_id_chained(&prev, &payload);
        let seq = EvLog::len(&self.store) as u64;
        let rec = Record { id, prev, actor_pubkey: [0u8; 32], actor_seq: seq, payload };
        match EvLog::append_tip_bytes(&mut self.store, &rec) {
            Ok(_) => Ok(()),
            Err(e) if crate::e_is_full(&e) => {
                self.grow()?;
                EvLog::append_tip_bytes(&mut self.store, &rec)?;
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// The newest record alone, unpacking no other (W-PITR2: the journal's in-step check).
    pub fn newest(&self) -> Option<Entry> {
        let n = self.len() as u64;
        EvLog::walk_until(&self.store, |_| true).first().and_then(|r| decode(&r.payload, n.saturating_sub(1)))
    }

    /// Every record, NEWEST FIRST, which is the order every one of the tables
    /// this replaces was indexed in.
    pub fn entries(&self) -> Vec<Entry> {
        let walked = EvLog::walk_marked(&self.store);
        let n = walked.len();
        walked
            .into_iter()
            .enumerate()
            .filter_map(|(i, (r, bad))| bad.is_none().then(|| decode(&r.payload, (n - 1 - i) as u64)).flatten())
            .collect()
    }

    /// Every record this image holds and this build cannot read, newest first.
    /// The same law as the order log's: `len() == entries().len() +
    /// quarantined().len()`, and a non-zero count is a failing gate.
    pub fn quarantined(&self) -> Vec<crate::Quarantined> {
        let walked = EvLog::walk_marked(&self.store);
        walked
            .iter()
            .enumerate()
            .filter_map(|(at, (r, bad))| {
                let ok = if bad.is_some() { Err("crc") } else { decode_or_reason(&r.payload, 0).map(|_| ()) };
                ok.err().map(|reason| crate::Quarantined { id: crate::hex32(&r.id), at, reason })
            })
            .collect()
    }

    /// Newest first, of one kind, optionally about one subject.
    ///
    /// THE FILTER IS THE WHOLE QUERY LANGUAGE HERE and that is deliberate: a
    /// log of a few hundred short records is cheaper to walk than to index, and
    /// an index on an append-only image would have to be rebuilt on every grow.
    pub fn about(&self, kind: &str, subject: Option<&str>, limit: usize) -> Vec<Entry> {
        self.entries()
            .into_iter()
            .filter(|e| e.kind == kind)
            .filter(|e| subject.map_or(true, |s| e.subject == s))
            .take(limit)
            .collect()
    }

    /// Walk the chain and check every id against the payload it names.
    pub fn chain_check(&self) -> ChainCheck {
        let mut out = ChainCheck::default();
        for r in EvLog::walk(&self.store) {
            out.records += 1;
            if r.id == content_id_chained(&r.prev, &r.payload) {
                out.chained += 1;
            } else {
                out.broken += 1;
            }
        }
        out
    }

    /// Keep the newest `n` records and drop the rest.
    ///
    /// THE PRUNE IS A REBUILD, not a deletion: the store is append-only, so
    /// "forget the old ones" means writing a fresh image holding the ones that
    /// stay. The chain is rebuilt from the kept records, which means the ids
    /// CHANGE — and they must, because an id commits to its predecessor and the
    /// predecessor is gone. `chain_check` after a prune verifies the new chain,
    /// not the old one, and that is the honest thing for it to verify.
    ///
    /// Returns how many were dropped.
    pub fn keep(&mut self, n: usize) -> Result<usize, HubError> {
        let all = self.entries();
        if all.len() <= n {
            return Ok(0);
        }
        let dropped = all.len() - n;
        let mut keep: Vec<Entry> = all.into_iter().take(n).collect();
        keep.reverse();
        let mut fresh = Store::create_bytes(self.store.to_bytes().len().max(MIN_LOG_BYTES))?;
        EvLog::init_bytes(&mut fresh)?;
        let mut prev = [0u8; 32];
        let mut last = None;
        for (i, e) in keep.iter().enumerate() {
            let payload = encode(&e.kind, &e.subject, &e.json)?;
            let id = content_id_chained(&prev, &payload);
            let rec =
                Record { id, prev, actor_pubkey: [0u8; 32], actor_seq: i as u64, payload };
            EvLog::append_bytes(&mut fresh, &rec)?;
            prev = id;
            last = Some(id);
        }
        if let Some(id) = last {
            EvLog::set_tip_bytes(&mut fresh, &id)?;
        }
        self.store = fresh;
        Ok(dropped)
    }

    /// Double the image and copy the chain into it, oldest first.
    fn grow(&mut self) -> Result<(), HubError> {
        // DOUBLE, do not jump to the default: a log born small that went
        // straight to 64 KiB on its first overflow would cost that on every
        // read for the rest of its life. The floor is only there so a corrupt
        // zero-length image cannot produce another one.
        let bigger = self.store.to_bytes().len().saturating_mul(2).max(MIN_LOG_BYTES);
        let mut fresh = Store::create_bytes(bigger)?;
        EvLog::init_bytes(&mut fresh)?;
        // Verbatim, a failed crc CARRIED (W-CRC): a re-seal would launder it.
        EvLog::copy_chain_bytes(&self.store, &mut fresh)?;
        // Swapped in only once the whole copy succeeded. A partial grow that
        // replaced the store would lose history to save space.
        self.store = fresh;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
