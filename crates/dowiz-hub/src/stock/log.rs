//! THE APPEND-ONLY STOCK LOG: open, load, save, read back, append -- and grow
//! the image instead of refusing a write when its arena fills.

use super::*;

impl StockLog {
    /// What this image has spent. See [`crate::Usage`].
    ///
    /// GROWS RATHER THAN REFUSING: `write` doubles the image and copies the
    /// chain when the arena fills, retrying up to six times. Measured at 7168
    /// cells growing to 523264 over four thousand events with no refusal, so
    /// this reading predicts a doubling rather than a failure.
    /// How many events the log holds, from the root's counter. Survives
    /// `grow()` unchanged, which the store generation does not.
    pub fn len(&self) -> usize {
        EvLog::len(&self.store)
    }

    pub fn usage(&self) -> crate::Usage {
        let cap = self.store.capacity_cells();
        crate::usage_of_kind(&self.store, cap, true)
    }

    pub fn create() -> Result<Self, crate::HubError> {
        Self::create_sized(DEFAULT_STOCK_BYTES)
    }

    /// A log that starts at a chosen size.
    ///
    /// The default is a year of a venue's movements, which is right on a disk
    /// and wrong on a Worker: 8 MiB is nine D1 chunks read and written on every
    /// order that reserves an ingredient. The log grows itself when an append
    /// does not fit, so a small start costs a few doublings and nothing else.
    pub fn create_sized(bytes: usize) -> Result<Self, crate::HubError> {
        let mut store = Store::create_bytes(bytes)?;
        EvLog::init_bytes(&mut store)?;
        Ok(StockLog { store, clock: None, every: checkpoint::CHECKPOINT_EVERY, grew: false, bad: Vec::new(), entropy: Default::default() })
    }

    pub fn load(bytes: &[u8]) -> Result<Self, crate::HubError> {
        Self::load_since(bytes, None).map(|(log, _)| log)
    }

    /// `load`, not re-hashing the records `mark` covers (W-HUBCRC, `Hub::load_since`).
    /// A MARK ONLY FOR A CLEAN LOG: `bad` must name EVERY quarantined record, and a marked
    /// scan sees only the newer ones -- so a log with any quarantined record gets no mark
    /// back, and its next load hashes everything, as this one did.
    pub fn load_since(bytes: &[u8], mark: Option<&crate::LogMark>) -> Result<(Self, Option<crate::LogMark>), crate::HubError> {
        let store = Store::from_bytes(bytes);
        if store.pick().is_none() {
            return Err(crate::HubError::NotAHub);
        }
        // AND REFUSES ONE THAT LOST RECORDS ON THE WAY HERE (W-AUDIT S3,
        // 2026-09-27), as `Hub::load` and `LogImage::load` have since the
        // short-read defect. This loader alone trusted the superblock: a stock
        // image cut short LOADED, folded a partial history, and -- because
        // `append` re-folds the whole log first -- a `Consumed` whose
        // `Reserved` was in the lost tail turned every later stock write into
        // a `Linkage` refusal. Refusing here is what a caller can act on.
        // An append log QUARANTINES a bad-crc record and serves the rest (W-CRC, operator
        // 2026-10-05); what breaks the walk itself (root, count, a link) still refuses.
        let (scan, next) = crate::quarantine::chain_is_whole_since(&store, mark)?;
        let (bad, next) = if scan.quarantined.is_empty() {
            (Vec::new(), next)
        } else {
            (EvLog::walk_marked(&store).into_iter().filter(|(_, c)| c.is_some()).map(|(r, _)| r.id).collect(), None)
        };
        Ok((StockLog { store, clock: None, every: checkpoint::CHECKPOINT_EVERY, grew: false, bad, entropy: Default::default() }, next))
    }

    /// FULL CAPACITY: `grow()` doubles from this length, so it keeps the
    /// zeros. Persist `to_bytes_trimmed`.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.store.to_bytes()
    }

    /// The image without the unused tail of its arena. Reloads identical:
    /// `Store::from_bytes` pads the zeros back from the superblock's capacity.
    pub fn to_bytes_trimmed(&self) -> Vec<u8> {
        self.store.to_bytes_trimmed()
    }

    /// Every event, OLDEST FIRST.
    ///
    /// `EvLog::walk` returns newest first -- the root points at the last record
    /// and each links back to its predecessor -- which is right for "show me
    /// what just happened" and catastrophic for a fold. Applied in that order a
    /// reservation lands before the delivery that made it possible, and the
    /// ledger refuses its own history: the first symptom was a log that
    /// replayed as `OutOfStock` for stock it plainly had.
    ///
    /// A fold is defined over time moving forwards. The reversal belongs here,
    /// once, rather than at each of the three call sites.
    /// A record quarantined at load (failed crc) is left out here, in `raw()` and in
    /// `tail()` alike, so every fold agrees and `len == events + quarantined`.
    pub fn events(&self) -> Vec<StockEvent> {
        let mut out: Vec<StockEvent> = EvLog::walk(&self.store)
            .into_iter()
            .filter(|r| !self.is_bad(&r.id))
            .filter_map(|r| decode(&String::from_utf8_lossy(&r.payload)))
            .collect();
        out.reverse();
        out
    }

    /// A record quarantined at load for a failed crc (W-CRC).
    pub(super) fn is_bad(&self, id: &[u8; 32]) -> bool {
        !self.bad.is_empty() && self.bad.contains(id)
    }

    /// How many records the load quarantined for a failed crc (owner health).
    pub fn quarantined(&self) -> usize {
        self.bad.len()
    }

    /// THE ENTROPY SEAM (M0): the nonce in every record written after this call is drawn
    /// from `seed`; without one it is derived and flagged. See `Hub::set_entropy`.
    pub fn set_entropy(&mut self, seed: [u8; 32]) {
        self.entropy.set(seed);
    }

    /// THE SCHEME SWITCH (W-MG-MIG): legacy or sha256 records from now on, one-way per log.
    /// See `Hub::set_sha256`. A record ADOPTED with a nonce (`write_payload`) keeps its own
    /// scheme whatever this says: its id is the tablet's, and the bytes decide.
    pub fn set_sha256(&mut self, on: bool) {
        self.entropy.set_sha256(on);
    }

    /// The scheme the next new record is written under (`set_sha256`, or the log's own once flipped).
    pub fn writes_sha256(&self) -> bool {
        self.entropy.writes_sha256(&self.store)
    }

    /// A NEW record: `rec` is the JSON object, and this puts the nonce in front of it
    /// before `write_payload` chains it -- while the log is on sha256; a log not yet flipped
    /// writes the bare object, which `write_bytes` chains under the legacy FNV id, byte for
    /// byte as ac5ff0c4 does. Every door that makes a record goes through here; the doors
    /// that copy one (`adopt.rs`) go to `write_payload` with the bytes they have.
    pub(super) fn write_new(&mut self, rec: String) -> Result<(), StockError> {
        if EvLog::version(&self.store) < 2 {
            // A v1 store has no flags cell: rebuilt as v2 first, the chain copied verbatim.
            self.grow().map_err(|_| StockError::Malformed)?;
        }
        if !self.entropy.writes_sha256(&self.store) {
            return self.write_bytes(rec.into_bytes(), false);
        }
        let prev = EvLog::tip(&self.store).unwrap_or([0u8; 32]);
        let (nonce, derived) = self.entropy.nonce(&prev, rec.as_bytes());
        self.write_bytes(crate::chain::with_nonce(&nonce, rec.as_bytes()), derived)
    }

    /// One record, chained to the previous, AS THESE BYTES. Which scheme chains it is
    /// what the bytes say (`codec::nonced_bytes`): a nonced payload gets `sha256(prev ‖
    /// payload)` and the sha256 flag, a bare JSON object the legacy FNV id -- so a tablet
    /// replaying a hub's pre-M0 records lands on the hub's own ids, and the hub chaining a
    /// tablet's records lands on the tablet's. Takes the bytes, so a test can lay down an
    /// OLDER encoding and prove the fold still reads them, and a checkpoint is written
    /// through the same door as an event. The chain is what makes the log tamper-evident
    /// (`chain.rs` says exactly what that means and what it does not).
    pub(super) fn write_payload(&mut self, payload: Vec<u8>) -> Result<(), StockError> {
        self.write_bytes(payload, false)
    }

    fn write_bytes(&mut self, payload: Vec<u8>, derived: bool) -> Result<(), StockError> {
        let sha = crate::stock::codec::nonced_bytes(&payload);
        let prev = EvLog::tip(&self.store).unwrap_or([0u8; 32]);
        let id = crate::chain::id_for(&prev, &payload, sha);
        let rec = Record {
            id,
            prev,
            // Stock events are the venue's own, recorded by the hub rather than
            // signed by a person; the actor slot is zero rather than borrowing
            // an identity that did not act.
            actor_pubkey: [0u8; 32],
            // The root's own counter, not a walk: `events().len()` decoded the
            // whole stock log on every single delivery just to number it.
            actor_seq: EvLog::len(&self.store) as u64,
            flags: if sha { crate::chain::flags_for(derived) } else { 0 },
            payload,
        };
        // THE IMAGE GROWS RATHER THAN REFUSING, like the order log. A shelf
        // that cannot record a delivery because its arena is full is a kitchen
        // that stops being able to sell -- the refusal path reads this ledger.
        // ── APPEND AND TIP ARE ONE COMMIT ──
        //
        // They used to be two, and they failed differently: measured, on a
        // nearly full arena the RECORD still fits while the tip update does
        // not. `walk` follows the store's object chain rather than the tip
        // hash, so a record written without its tip is already IN the chain --
        // re-appending it after growing counted it twice, which is exactly
        // what the first version of this did (3002 deliveries recorded for
        // 3000 made).
        //
        // `append_tip_bytes` allocates the record and the new root in ONE
        // transaction: either both fit or nothing is committed and the arena
        // cursor has not moved. So a failure leaves no record to double-count,
        // and growing and trying again is the whole recovery.
        let mut placed = EvLog::append_tip_bytes(&mut self.store, &rec).is_ok();
        for _ in 0..6 {
            if placed {
                break;
            }
            self.grow().map_err(|_| StockError::Malformed)?;
            placed = EvLog::append_tip_bytes(&mut self.store, &rec).is_ok();
        }
        if !placed {
            return Err(StockError::Malformed);
        }
        Ok(())
    }

    /// Double the image and copy the chain across, oldest first.
    ///
    /// `walk` is newest-first, so the copy is reversed: appending in the wrong
    /// order leaves every `prev` pointing at a record that does not exist yet,
    /// which is a heap of orphans that still looks like a log.
    fn grow(&mut self) -> Result<(), crate::HubError> {
        let bigger = self.store.to_bytes().len().saturating_mul(2).max(64 * 1024);
        let mut fresh = Store::create_bytes(bigger)?;
        EvLog::init_bytes(&mut fresh)?;
        // The chain's BYTES, failed crcs carried: a quarantined record is never re-sealed
        // into one that verifies again (W-CRC).
        EvLog::copy_chain_bytes(&self.store, &mut fresh)?;
        // Swapped in only once the whole copy succeeded: a partial grow that
        // replaced the store would lose the ledger to save space.
        self.store = fresh;
        self.grew = true;
        Ok(())
    }

    /// The shelf NOW: the newest checkpoint plus the records after it (R7),
    /// equal to `StockLedger::fold(&self.events())` by the checkpoint's law.
    pub fn ledger(&self) -> Result<StockLedger, StockError> {
        self.fold_tail(true, false).map(|f| f.0)
    }

    /// Append one event, AFTER the ledger has agreed to it.
    ///
    /// `decide` runs against the current fold and nothing is written when it
    /// refuses -- the fail-closed gate §4 specifies. An event that would break
    /// an invariant never reaches the log, so a replay of the log can never
    /// reconstruct an impossible state.
    ///
    /// A new write-off or count must be SIGNED ([`signed`]); the history it is
    /// decided against need not be.
    pub fn append(&mut self, ev: &StockEvent) -> Result<(), StockError> {
        self.append_all(std::slice::from_ref(ev))
    }

    /// Append several as ONE decision.
    ///
    /// §4's "one commit, not two": an order's reservations fold atomically. If
    /// the third dish in a basket is out of stock, the first two must not be
    /// reserved -- otherwise a refused order silently holds ingredients that
    /// nothing will ever release.
    pub fn append_all(&mut self, evs: &[StockEvent]) -> Result<(), StockError> {
        // Decided against a ledger that accumulates the batch, so two lines of
        // one order competing for the same ingredient are caught here rather
        // than by the second one failing after the first was written.
        let with: Vec<(StockEvent, meta::Meta)> = evs.iter().map(|e| (e.clone(), meta::Meta::default())).collect();
        self.commit(&with, false).map(|_| ())
    }
}
