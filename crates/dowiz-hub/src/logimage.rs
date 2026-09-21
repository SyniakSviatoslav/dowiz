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
        let mut store = Store::create_bytes(bytes.max(MIN_LOG_BYTES));
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
        crate::chain_is_whole(&store, |r| decode(&r.payload, 0).is_some())?;
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

    /// Every record, NEWEST FIRST, which is the order every one of the tables
    /// this replaces was indexed in.
    pub fn entries(&self) -> Vec<Entry> {
        let walked = EvLog::walk(&self.store);
        let n = walked.len();
        walked
            .into_iter()
            .enumerate()
            .filter_map(|(i, r)| decode(&r.payload, (n - 1 - i) as u64))
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
        let mut fresh = Store::create_bytes(self.store.to_bytes().len().max(MIN_LOG_BYTES));
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
        let mut records = EvLog::walk(&self.store);
        records.reverse();
        // DOUBLE, do not jump to the default: a log born small that went
        // straight to 64 KiB on its first overflow would cost that on every
        // read for the rest of its life. The floor is only there so a corrupt
        // zero-length image cannot produce another one.
        let bigger = self.store.to_bytes().len().saturating_mul(2).max(MIN_LOG_BYTES);
        let mut fresh = Store::create_bytes(bigger);
        EvLog::init_bytes(&mut fresh)?;
        let mut last = None;
        for r in &records {
            EvLog::append_bytes(&mut fresh, r)?;
            last = Some(r.id);
        }
        if let Some(id) = last {
            EvLog::set_tip_bytes(&mut fresh, &id)?;
        }
        // Swapped in only once the whole copy succeeded. A partial grow that
        // replaced the store would lose history to save space.
        self.store = fresh;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mistake this floor exists for, pinned so it cannot come back.
    #[test]
    fn an_image_below_the_arena_is_raised_rather_than_born_useless() {
        let mut tiny = LogImage::create_sized(1).unwrap();
        tiny.append("e", "v", "{}").expect("an image with no room refused its first record");
        assert!(MIN_LOG_BYTES > bebop_store::ARENA * 8);
    }

    /// A TRUNCATED IMAGE IS NOT A SHORTER LOG, and for two days it was read as
    /// one. The superblock is fifteen cells at the FRONT of the image, so it
    /// survives a cut that takes half the records with it: the image loaded,
    /// `len()` answered 40, and `entries()` handed back two. A short read on a
    /// chunked fetch from a Durable Object is the realistic failure here, not a
    /// theoretical one -- and a venue cannot notice thirty-eight missing
    /// orders in a reply that looks exactly like a correct one.
    ///
    /// The cuts are FIXED fractions, not random ones: the property is a law
    /// about every prefix, and a fixed set of them says so without a generator.
    #[test]
    fn a_truncated_image_is_refused_rather_than_read_short() {
        let mut l = LogImage::create().unwrap();
        for i in 0..40 {
            l.append("e", &format!("s{}", i % 7), &format!(r#"{{"n":{i}}}"#)).unwrap();
        }
        let bytes = l.to_bytes();
        let mut refused = 0;
        for tenth in 1..10usize {
            let keep = bytes.len() * tenth / 10;
            match LogImage::load(&bytes[..keep]) {
                Err(_) => refused += 1,
                Ok(short) => assert_eq!(
                    short.len(),
                    short.entries().len(),
                    "a log cut to {keep} of {} bytes loaded and then disagreed with itself",
                    bytes.len()
                ),
            }
        }
        assert!(refused > 0, "no prefix of a 40-record log was refused");
        // And the whole image still loads whole -- a check that refuses
        // everything is not a check.
        assert_eq!(LogImage::load(&bytes).unwrap().entries().len(), 40);
    }

    /// The other half: the image is all there, and one `next` ref is not.
    #[test]
    fn a_chain_ref_that_leaves_the_image_is_refused() {
        let mut l = LogImage::create().unwrap();
        for i in 0..6 {
            l.append("e", "s", &format!(r#"{{"n":{i}}}"#)).unwrap();
        }
        let mut st = bebop_store::Store::from_bytes(&l.to_bytes());
        let root = st.root().expect("root");
        let newest = st.follow(root, 1).expect("newest");
        // Payload cell 2 of a record is the ref to the next (older) one.
        st.cells[newest + 2 + 2] = 1 << 40;
        let broken = st.to_bytes();
        assert!(
            matches!(LogImage::load(&broken), Err(HubError::Corrupt { claimed: 6, .. })),
            "a chain that stops early must be refused, not read short"
        );
    }

    #[test]
    fn a_record_comes_back_newest_first() {
        let mut l = LogImage::create().unwrap();
        l.append("err", "dubin", r#"{"m":"one"}"#).unwrap();
        l.append("err", "sushi", r#"{"m":"two"}"#).unwrap();
        let e = l.entries();
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].json, r#"{"m":"two"}"#);
        assert_eq!(e[0].subject, "sushi");
        assert_eq!(e[0].seq, 1);
        assert_eq!(e[1].seq, 0);
    }

    #[test]
    fn it_survives_a_round_trip_through_bytes() {
        let mut l = LogImage::create().unwrap();
        for i in 0..20 {
            l.append("msg", "t1", &format!(r#"{{"n":{i}}}"#)).unwrap();
        }
        let back = LogImage::load(&l.to_bytes()).unwrap();
        assert_eq!(back.len(), 20);
        assert_eq!(back.entries()[0].json, r#"{"n":19}"#);
        assert!(back.chain_check().intact());
    }

    #[test]
    fn about_filters_by_kind_and_subject() {
        let mut l = LogImage::create().unwrap();
        l.append("msg", "t1", "{}").unwrap();
        l.append("msg", "t2", "{}").unwrap();
        l.append("err", "t1", "{}").unwrap();
        assert_eq!(l.about("msg", None, 10).len(), 2);
        assert_eq!(l.about("msg", Some("t1"), 10).len(), 1);
        assert_eq!(l.about("err", Some("t2"), 10).len(), 0);
        assert_eq!(l.about("msg", None, 1).len(), 1, "the limit is the limit");
    }

    /// The order log's expensive lesson, inherited: it GROWS.
    #[test]
    fn it_grows_rather_than_refusing() {
        let mut l = LogImage::create_sized(MIN_LOG_BYTES).unwrap();
        for i in 0..400 {
            l.append("err", "v", &format!(r#"{{"n":{i},"pad":"{}"}}"#, "x".repeat(80)))
                .unwrap_or_else(|e| panic!("refused at {i}: {e:?}"));
        }
        assert_eq!(l.len(), 400);
        assert!(l.chain_check().intact(), "growing broke the chain");
        assert_eq!(l.entries()[0].json.contains("\"n\":399"), true);
    }

    #[test]
    fn the_chain_survives_a_round_trip_after_growing() {
        let mut l = LogImage::create_sized(MIN_LOG_BYTES).unwrap();
        for i in 0..200 {
            l.append("e", "v", &format!(r#"{{"n":{i}}}"#)).unwrap();
        }
        let back = LogImage::load(&l.to_bytes()).unwrap();
        let c = back.chain_check();
        assert_eq!(c.records, 200);
        assert_eq!(c.broken, 0);
    }

    #[test]
    fn keep_drops_the_oldest_and_leaves_a_valid_chain() {
        let mut l = LogImage::create().unwrap();
        for i in 0..50 {
            l.append("e", "v", &format!(r#"{{"n":{i}}}"#)).unwrap();
        }
        assert_eq!(l.keep(10).unwrap(), 40);
        assert_eq!(l.len(), 10);
        let e = l.entries();
        assert_eq!(e[0].json, r#"{"n":49}"#, "the newest survives");
        assert_eq!(e[9].json, r#"{"n":40}"#, "and the tenth-newest is the oldest left");
        assert!(l.chain_check().intact(), "the rebuilt chain does not verify");
        // And it keeps working afterwards.
        l.append("e", "v", r#"{"n":50}"#).unwrap();
        assert_eq!(l.len(), 11);
        assert!(l.chain_check().intact());
    }

    #[test]
    fn keeping_more_than_there_are_drops_nothing() {
        let mut l = LogImage::create().unwrap();
        l.append("e", "v", "{}").unwrap();
        assert_eq!(l.keep(10).unwrap(), 0);
        assert_eq!(l.len(), 1);
    }

    /// An EDITED record is what a chained id is for.
    #[test]
    fn chain_check_finds_an_edit() {
        let mut l = LogImage::create().unwrap();
        for i in 0..5 {
            l.append("e", "v", &format!(r#"{{"n":{i}}}"#)).unwrap();
        }
        assert!(l.chain_check().intact());
        // Rebuild the image with one payload changed but the ORIGINAL ids kept,
        // which is what editing the stored bytes would look like.
        let mut records = EvLog::walk(&l.store);
        records.reverse();
        let mut fresh = Store::create_bytes(64 * 1024);
        EvLog::init_bytes(&mut fresh).unwrap();
        let mut last = None;
        for (i, r) in records.iter().enumerate() {
            let mut r = r.clone();
            if i == 2 {
                r.payload = encode("e", "v", r#"{"n":999}"#).unwrap();
            }
            EvLog::append_bytes(&mut fresh, &r).unwrap();
            last = Some(r.id);
        }
        EvLog::set_tip_bytes(&mut fresh, &last.unwrap()).unwrap();
        let tampered = LogImage { store: fresh };
        let c = tampered.chain_check();
        assert_eq!(c.records, 5);
        assert_eq!(c.broken, 1, "an edited payload did not break its id");
    }

    #[test]
    fn a_payload_with_a_long_kind_or_subject_is_refused_rather_than_truncated() {
        let mut l = LogImage::create().unwrap();
        let long = "x".repeat(256);
        assert!(l.append(&long, "v", "{}").is_err());
        assert!(l.append("e", &long, "{}").is_err());
        assert_eq!(l.len(), 0, "a refused append left a record behind");
    }

    #[test]
    fn json_with_the_separator_bytes_in_it_still_decodes() {
        let mut l = LogImage::create().unwrap();
        // A payload whose text contains what could be read as a length prefix.
        let tricky = r#"{"m":"\u0001a\u0002bb","n":1}"#;
        l.append("e", "subj", tricky).unwrap();
        assert_eq!(l.entries()[0].json, tricky);
    }
}
