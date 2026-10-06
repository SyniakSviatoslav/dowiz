//! MOVING HISTORY OUT OF THE HOT IMAGE. A rotation hands the whole image back
//! for cold storage and keeps a fresh one holding a checkpoint, the audit
//! trail, and every event of every order the caller says to keep.

use bebop_store::{evlog::{EvLog, Record}, Store};

use crate::{content_id_chained, decode};
use crate::{EventKind, Hub, HubError};

impl Hub {
    /// Move old history out of the hot image and hand it back for cold storage.
    ///
    /// WHY A LOG THAT ONLY GROWS IS A PROBLEM AT ALL. Every read of this hub
    /// loads the whole image and folds it; a venue at thirty orders a day
    /// writes about six events each, so after a year the thing a console polls
    /// is mostly orders nobody will ever look at again. The fold is O(events)
    /// and the Worker has 128 MB. Growth is not waste -- the history is real --
    /// but keeping ALL of it on the hot path is.
    ///
    /// WHAT COMES BACK is the image as it stood, complete, for the caller to
    /// store somewhere cold. What stays is a fresh image holding a CHECKPOINT
    /// record and every event of every order `keep` says to keep.
    ///
    /// THE RECORDS ARE MOVED VERBATIM -- same ids, same `prev` links, same
    /// payloads -- so `chain_check` still verifies each one: an id commits to
    /// the id BEFORE it, and that id is a value in the record, not a pointer
    /// into the image. A gap in the walk is exactly what the checkpoint
    /// announces; it is not damage.
    ///
    /// `keep` is asked once per ORDER ID, not once per event, because an order
    /// half of whose events survived would fold to a lie.
    pub fn rotate<F>(&mut self, keep: F) -> Result<Vec<u8>, HubError>
    where
        F: Fn(&str) -> bool,
    {
        let archived = self.store.to_bytes_trimmed();
        // MARKED (W-CRC): a quarantined record is carried with its failed crc, never re-sealed.
        let mut records = EvLog::walk_marked(&self.store);
        records.reverse();

        let tip = EvLog::tip(&self.store).unwrap_or([0u8; 32]);
        let archived_events = records.len();

        // The descriptor is text and deliberately not JSON: this crate has no
        // parser for reading one back (see `minijson`), and what a reader needs
        // here is two numbers and a hex string.
        let descriptor = format!(
            "tip={} events={} bytes={}",
            crate::crypto::hex(&tip),
            archived_events,
            archived.len()
        );
        let mut payload = Vec::with_capacity(2 + descriptor.len());
        payload.push(EventKind::Checkpoint as u8);
        payload.push(0); // no order id: a checkpoint is about the log, not an order
        payload.extend_from_slice(descriptor.as_bytes());

        // A fresh image sized for what it will hold, never smaller than a hub's
        // birth size: the whole point is that it is not the old one.
        let mut fresh = Store::create_bytes(self.store.to_bytes().len().max(64 * 1024))?;
        EvLog::init_bytes(&mut fresh)?;
        let check_id = content_id_chained(&tip, &payload);
        EvLog::append_tip_bytes(
            &mut fresh,
            &Record { id: check_id, prev: tip, actor_pubkey: [0u8; 32], actor_seq: 0, payload },
        )?;

        let mut last = check_id;
        for (r, bad) in &records {
            let Some(ev) = decode(r) else { continue };
            // A checkpoint from an EARLIER rotation is not carried forward: the
            // new one names the image that holds it, so the chain of
            // checkpoints runs through the archives rather than piling up here.
            if ev.kind == EventKind::Checkpoint {
                continue;
            }
            // THE AUDIT TRAIL IS NOT AN ORDER AND IS NOT ROTATED OUT.
            //
            // A `Revealed` record names who looked at a customer's contact
            // details; its subject is "cust:<key>", which is not an order id,
            // so `keep` -- built from the orders -- was never going to say yes
            // to one. The first rotation would have taken the whole trail out
            // of the hot log, and the only archive reader folds orders, so it
            // would have been unreachable from every surface. For a log whose
            // header calls itself the only tamper-evident thing this hub has,
            // that is the opposite of the point.
            //
            // They are small (one short record per read of a phone number) and
            // they stay.
            if !ev.kind.is_order() {
                EvLog::append_carry_bytes(&mut fresh, r, *bad)?;
                last = r.id;
                continue;
            }
            if !keep(&ev.order_id) {
                continue;
            }
            EvLog::append_carry_bytes(&mut fresh, r, *bad)?;
            last = r.id;
        }
        EvLog::set_tip_bytes(&mut fresh, &last)?;
        self.store = fresh;
        Ok(archived)
    }
}
