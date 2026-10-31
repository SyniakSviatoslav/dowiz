//! MOVING HISTORY OUT OF THE HOT IMAGE. A rotation hands the whole image back
//! for cold storage and keeps a fresh one holding a checkpoint, the audit
//! trail, and every event of every order the caller says to keep.
//!
//! THE CHECKPOINT NAMES THE ARCHIVE'S ROOT (W-MG-M3, R-MAINGOAL-A §2b): `root=` is the RFC
//! 9162 tree over the archived records' ids in file order -- the same `segment_root` a DOC
//! segment carries (`bebop-log/src/doc/seg.rs`) -- so a witness or a seal cites one value
//! for the whole archive and an auditor with the archive recomputes it. The hub's log is
//! not a DOC yet (W-MG-M1B's `doc_seal.rs` is where its commits will be sealed); until then
//! the checkpoint record is the hub's TRUNCATE, chained and nonced like every M0 record.

use bebop_store::{evlog::{EvLog, Record}, merkle, Store};

use crate::decode;
use crate::{EventKind, Hub, HubError};

/// The RFC 9162 root over record ids in file order (leaf = sha256(0x00 ‖ id)): what a
/// rotation's checkpoint and a DOC segment name. `sha256("")` for no records.
pub fn segment_root(ids: &[[u8; 32]]) -> [u8; 32] {
    let leaves: Vec<[u8; 32]> = ids.iter().map(|id| merkle::leaf_hash(id)).collect();
    merkle::root(&leaves)
}

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
        let ids: Vec<[u8; 32]> = records.iter().map(|(r, _)| r.id).collect();

        // The descriptor is text and deliberately not JSON: this crate has no
        // parser for reading one back (see `minijson`), and what a reader needs
        // here is two numbers and two hex strings.
        let descriptor = format!(
            "tip={} events={} bytes={} root={}",
            crate::crypto::hex(&tip),
            archived_events,
            archived.len(),
            crate::crypto::hex(&segment_root(&ids))
        );
        let mut body = Vec::with_capacity(2 + descriptor.len());
        body.push(EventKind::Checkpoint as u8);
        body.push(0); // no order id: a checkpoint is about the log, not an order
        body.extend_from_slice(descriptor.as_bytes());

        // A fresh image sized for what it will hold, never smaller than a hub's
        // birth size: the whole point is that it is not the old one.
        let mut fresh = Store::create_bytes(self.store.to_bytes().len().max(64 * 1024))?;
        EvLog::init_bytes(&mut fresh)?;
        // The checkpoint is a NEW record and continues the log's own scheme: a rotation must not
        // be what flips an unflipped venue (docs/design/MIGRATION-sha256-ids.md).
        let sha = self.entropy.writes_sha256(&self.store);
        let (payload, check_id, flags) = self.entropy.stamp(sha, &tip, body);
        EvLog::append_tip_bytes(
            &mut fresh,
            &Record { id: check_id, prev: tip, actor_pubkey: [0u8; 32], actor_seq: 0, flags, payload },
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
        (self.store, self.seen) = (fresh, None); // re-laid: the load-time scan no longer describes it
        Ok(archived)
    }
}

#[cfg(test)]
mod tests;
