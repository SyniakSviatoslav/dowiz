//! ONE HUB'S ORDER LOG AS AN IMAGE: born, loaded, saved, appended to, grown.
//!
//! The `Hub` type itself is declared in `lib.rs` so that every module of this
//! crate keeps the access to its store it had before the split (`forget.rs`
//! rewrites the image in place). Reading the log back is `read.rs`; moving old
//! history out of it is `rotate.rs`; checking its chain ids is `chain.rs`.

use bebop_store::{evlog::{EvLog, Record}, Store};

use crate::{content_id_chained, e_is_full, usage_of_kind};
use crate::{EventKind, Hub, HubError, LogMark, Usage, DEFAULT_IMAGE_BYTES};

impl Hub {
    /// A brand-new hub: a fresh image with the log schema created.
    pub fn create() -> Result<Self, HubError> {
        Self::create_sized(DEFAULT_IMAGE_BYTES)
    }

    pub fn create_sized(bytes: usize) -> Result<Self, HubError> {
        let mut store = Store::create_bytes(bytes)?;
        EvLog::init_bytes(&mut store)?;
        Ok(Hub { store, seen: Some(crate::Seen::default()) })
    }

    /// Load an existing image. Refuses one with no valid superblock rather than
    /// carrying on against a store that will answer nonsense.
    pub fn load(bytes: &[u8]) -> Result<Self, HubError> {
        Self::load_since(bytes, None).map(|(hub, _)| hub)
    }

    /// `load`, not re-hashing the records `mark` covers -- for a holder that keeps the
    /// bytes in its own memory between turns (the DO, `workers/api/src/hubdo/logmark.rs`).
    /// `None` is `load` exactly: every record hashed. Hands back the mark for these bytes,
    /// to pass on the next load of them or of what this hub appends to them (none for a
    /// log with a quarantined record, `quarantine::chain_is_whole_since`).
    /// `bebop_store::verify::since` says what a mark trusts.
    pub fn load_since(bytes: &[u8], mark: Option<&LogMark>) -> Result<(Self, Option<LogMark>), HubError> {
        let store = Store::from_bytes(bytes);
        if store.pick().is_none() {
            return Err(HubError::NotAHub);
        }
        // AND REFUSES ONE THAT LOST RECORDS ON THE WAY HERE. A superblock
        // survives a truncation -- it is fifteen cells at the front of the
        // image -- so "the superblock is valid" was never the same statement
        // as "the log is all here". See `chain_is_whole`.
        // A record whose crc fails is QUARANTINED, served around and counted, not
        // refused (operator 2026-10-05); a broken link still refuses. `crate::quarantine`.
        // The scan's findings are kept (`crate::Seen`): `events()` reads them instead of hashing again.
        let (seen, next) = crate::chain_is_whole_seen(&store, mark)?;
        Ok((Hub { store, seen: Some(seen) }, next))
    }

    /// The image to persist. The caller writes this wherever the hub lives.
    ///
    /// FULL CAPACITY, zeros included. `grow()` reads its own length to choose
    /// the next size, so this one must keep saying how big the arena is.
    /// Callers that only store and reload the image want
    /// `to_bytes_trimmed` instead.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.store.to_bytes()
    }

    /// The image without the unused tail of its arena -- what a hub that lives
    /// in a Durable Object or a backup should actually write.
    ///
    /// The tail is zeros the reader re-creates from the capacity in the
    /// superblock, so `Hub::load` on this is the same hub, with the same
    /// capacity, and the next `grow()` doubles from the same number. A fresh
    /// 64 KiB hub is a few hundred bytes; a 4 MiB one that has taken ten
    /// orders is about 50 KB rather than 4 MB.
    pub fn to_bytes_trimmed(&self) -> Vec<u8> {
        self.store.to_bytes_trimmed()
    }

    /// What this image has spent. See `Usage`.
    ///
    /// THE LOG GROWS RATHER THAN REFUSING, so its reading is a sawtooth that
    /// predicts a doubling and not a failure. An earlier version of this called
    /// the log fixed-size because `to_bytes` preserves its capacity — which is
    /// true of a SAVE and says nothing about an APPEND, and `append` doubles
    /// the image when the arena is full.
    pub fn usage(&self) -> Usage {
        let cap = self.store.capacity_cells();
        usage_of_kind(&self.store, cap, true)
    }

    pub fn len(&self) -> usize {
        EvLog::len(&self.store)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Append an event. `order_id` and `order_json` are opaque here: this module
    /// never interprets an order, and in particular never decides a status. The
    /// kernel does that, and only its answer is recorded.
    pub fn append(
        &mut self,
        kind: EventKind,
        order_id: &str,
        order_json: &str,
        seq: u64,
        actor_pubkey: [u8; 32],
    ) -> Result<i64, HubError> {
        let idb = order_id.as_bytes();
        if idb.len() > 255 {
            return Err(HubError::OrderIdTooLong);
        }
        // payload = [kind][id_len][id bytes][order json]
        let mut payload = Vec::with_capacity(2 + idb.len() + order_json.len());
        payload.push(kind as u8);
        payload.push(idb.len() as u8);
        payload.extend_from_slice(idb);
        payload.extend_from_slice(order_json.as_bytes());

        let prev = EvLog::tip(&self.store).unwrap_or([0u8; 32]);
        // CHAINED: the id commits to the record BEFORE it, so an edit anywhere
        // in the log breaks every id after it. See `content_id_chained`.
        let id = content_id_chained(&prev, &payload);
        let rec = Record { id, prev, actor_pubkey, actor_seq: seq, payload };

        // ── THE IMAGE GROWS RATHER THAN REFUSING ──
        //
        // Measured, not assumed: a 4 MiB image holds 1440 order events. A venue
        // doing fifty orders a day writes three or four events each, so it
        // fills in about a WEEK -- and then the hub stops accepting orders,
        // during service, with a message about an arena.
        //
        // Unlike the KV stores this growth is not waste: the log is append-only
        // because it is a log, and every record in it is history somebody may
        // need. So the answer is not to reclaim, it is to make room. On a full
        // arena the image doubles and the existing chain is copied across
        // VERBATIM -- ids and prev links included, since both are content
        // addresses and rewriting either would break the chain.
        //
        // It costs one O(n) copy per doubling, which is a handful of
        // milliseconds a few times in a hub's life, and it happens under the
        // same write lock that serialises every other append.
        // ── THE RECORD AND THE TIP ARE ONE COMMIT ──
        //
        // They were two, and the gap between them was a live defect. MEASURED:
        // the order log refused order 2450 with FIFTY-SIX CELLS still free.
        // The record fitted; the tip update after it did not, and only the
        // record's failure was handled -- so a hub with room to grow answered
        // `arena_full` and a venue stopped taking orders mid-service. The fix
        // then was to handle the second failure too, in four places, in two
        // files, each remembering that re-appending the record would count it
        // twice (that is how the stock ledger once recorded 3002 deliveries
        // for 3000 made).
        //
        // `append_tip_bytes` removes the gap instead of guarding it. The record
        // and the new root are allocated in ONE transaction, so if either does
        // not fit, NOTHING is committed and the arena cursor has not moved --
        // "the record is in but the tip is not" is no longer a state this log
        // can be in. Growing and trying again is then the whole recovery, and
        // it cannot double-count because the failed attempt left no record.
        match EvLog::append_tip_bytes(&mut self.store, &rec) {
            Ok(gen) => Ok(gen),
            Err(e) if e_is_full(&e) => {
                self.grow()?;
                Ok(EvLog::append_tip_bytes(&mut self.store, &rec)?)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Double the image and copy the chain into it, oldest first.
    ///
    /// `walk` is newest-first, so it is reversed: appending in the wrong order
    /// would leave every `prev` pointing at a record that does not exist yet,
    /// and the copy would be a chain of orphans that still LOOKS like a log.
    fn grow(&mut self) -> Result<(), HubError> {
        // DOUBLE, DO NOT JUMP TO THE DEFAULT. `.max(DEFAULT_IMAGE_BYTES)` was
        // here and it meant a hub born at 64 KiB went straight to 4 MiB on its
        // first overflow -- which on a Worker is five D1 rows read and written
        // on every request, and the resource limit a few orders later. The
        // floor exists so a corrupt zero-length image cannot produce a
        // zero-length one; it is not a target.
        let bigger = self.store.to_bytes().len().saturating_mul(2).max(64 * 1024);
        let mut fresh = Store::create_bytes(bigger)?;
        EvLog::init_bytes(&mut fresh)?;
        // VERBATIM INCLUDING A FAILED CRC (W-CRC): re-sealing a quarantined record
        // would launder its changed byte into a record that verifies.
        EvLog::copy_chain_bytes(&self.store, &mut fresh)?;
        // Swapped in only once the whole copy succeeded. A partial grow that
        // replaced the store would lose history to save space.
        self.store = fresh;
        Ok(())
    }
}
