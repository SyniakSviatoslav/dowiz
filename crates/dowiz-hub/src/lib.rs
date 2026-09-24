//! One tenant's hub: the order log, over a bebop store, as PURE logic.
//!
//! Bytes in, bytes out. No filesystem, no network, no clock, no randomness — the
//! caller supplies the store image, the time and the ids, and gets a new image
//! back. That is what makes this testable natively today while the Worker shell
//! around it (a Durable Object for single-writer serialisation, object storage
//! for the image) is still unbuilt: the part that holds the data is proven
//! before the part that moves it exists.
//!
//! WHY AN EVENT LOG AND NOT A TABLE. An order's state is a FOLD over what
//! happened to it, not a mutable row — the kernel's own decide/fold law. The
//! bebop event log appends ONE object per event and relinks the root, which is
//! O(1); the KV layout would rewrite the world on every put. The log is the
//! truth and any projection is rebuilt from it.
//!
//! HONEST COST. Reading one order walks the chain, which is O(n) in the log.
//! At one restaurant's volume — a few hundred orders a day — that is nothing,
//! and the walk happens in memory over an image already loaded. It is stated
//! here rather than discovered later: at platform volume this wants an index,
//! and the index would be a projection rebuilt from the log, never a second
//! source of truth.

#![forbid(unsafe_code)]

pub mod caps;
pub mod catalog;
pub mod consent;
pub mod crypto;
pub mod hours;
pub mod import;
pub mod media;
pub mod minijson;
pub mod modifiers;
pub mod palette;
pub mod activation;
pub mod allergens;
pub mod brand;
pub mod features;
pub mod graph;
pub mod post;
pub mod promo;
pub mod redact;
pub mod roster;
pub mod room;
pub mod settings;
pub mod stock;
pub mod subs;
pub mod voice;
pub mod zone;
pub mod logimage;
pub mod table;
pub mod tables;
pub mod tz;
pub mod token;
pub mod forget;

use bebop_store::evlog::{EvLog, Record};
use bebop_store::{Store, StoreError};

/// Default image size for a fresh hub. 4 MiB holds a few thousand orders at the
/// measured 47 cells per append, and the image is only as large as it is written.
pub const DEFAULT_IMAGE_BYTES: usize = 4 * 1024 * 1024;

/// What happened. The kind is the first payload byte so a record can be routed
/// without parsing the JSON behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// A new order, payload = the kernel's serialized order.
    Placed = 1,
    /// A status transition the KERNEL allowed, payload = the updated order.
    Advanced = 2,
    /// Money arrived. NOT a status transition: an order can be paid while still
    /// PENDING, and paying is not something the order FSM has an edge for. It is
    /// its own fact, recorded as its own event rather than smuggled through a
    /// transition the kernel would rightly refuse.
    Paid = 3,
    /// Somebody LOOKED at a customer's contact details.
    ///
    /// A read, in a log of writes, and deliberately so. The order log is the
    /// only append-only, tamper-evident thing this hub has, and an audit trail
    /// kept anywhere softer is an audit trail that can be tidied up. The
    /// payload names who looked, at whom, and when; it carries no contact
    /// details itself, because a log of who read a phone number that also
    /// contains the phone number has doubled the exposure it exists to record.
    Revealed = 4,
    /// A fact was ADDED to an order without its status moving: the customer's
    /// note, a courier's proof of delivery.
    ///
    /// Not `Advanced`, which means specifically a transition the KERNEL
    /// allowed. Writing an annotation as a transition would put events in the
    /// log that the order machine never decided, and the first person to audit
    /// the lifecycle would find a status change with no edge behind it.
    Noted = 5,
    /// A MARK IN THE LOG WHERE HISTORY WAS MOVED OUT OF IT.
    ///
    /// Written by `rotate` as the first record of a fresh hot image, naming the
    /// archived image's tip and how many events it holds. It is not an order
    /// and never folds into one; it exists so that a log can be read and the
    /// reader can tell the difference between "this venue has never taken an
    /// order" and "the older ones are somewhere else, and here is where".
    Checkpoint = 6,
    /// THE MONEY ON A ROUND CHANGED, and the kitchen had not taken it yet — or
    /// a person holding `void` took a line off after it had.
    ///
    /// NOT `Noted`, which promises that no money moved: every reader that asks
    /// "did this order's total change?" would otherwise have to open every
    /// note and look. A delta that may change `items`, `subtotal`, `discount`,
    /// `total`, `fulfilment.table`, `adjustments` and `amended`, and nothing
    /// else; written only by the object's `amend` and `transfer` commands, and
    /// signed by the person who made it. Not a status: the order FSM is never
    /// asked, and its golden signature does not move.
    /// (docs/design/BLUEPRINT-POS-THE-ROOM-2026-09-22.md §3.)
    Amended = 7,
    /// A PERSON WAS FORGOTTEN: the declaration that names how many records
    /// were redacted in place (`forget.rs`). Not an order; no contact details.
    Forgotten = 8,
}

impl EventKind {
    /// Does this event describe an ORDER? Everything that folds the log into
    /// orders asks this first.
    pub fn is_order(self) -> bool {
        matches!(
            self,
            EventKind::Placed
                | EventKind::Advanced
                | EventKind::Paid
                | EventKind::Noted
                | EventKind::Amended
        )
    }

    /// The kind a stored byte names, for a caller that carries events across
    /// a boundary the enum cannot cross -- a Durable Object's JSON body, say.
    pub fn from_u8(b: u8) -> Option<Self> {
        Self::from_byte(b)
    }

    fn from_byte(b: u8) -> Option<Self> {
        match b {
            1 => Some(EventKind::Placed),
            2 => Some(EventKind::Advanced),
            3 => Some(EventKind::Paid),
            4 => Some(EventKind::Revealed),
            5 => Some(EventKind::Noted),
            6 => Some(EventKind::Checkpoint),
            7 => Some(EventKind::Amended),
            8 => Some(EventKind::Forgotten),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum HubError {
    Store(StoreError),
    /// The image is not a hub store, or carries no log root.
    NotAHub,
    /// An order id that is not in the log.
    UnknownOrder,
    /// An order id longer than the record header allows.
    OrderIdTooLong,
    /// A LOADED IMAGE DISAGREES WITH ITSELF: the chain does not deliver the
    /// records the root claims, or a record in it cannot be read back.
    ///
    /// This is the failure worth having an error for. An image cut short, or
    /// one byte of it changed in flight, used to LOAD -- and then answer
    /// `len() == 40` while handing back two events. A venue whose log silently
    /// drops its thirty-eight most recent orders has no way to notice, and
    /// every reply built on it is confidently wrong. Refusing is the only
    /// answer a caller can act on: it can re-fetch, restore, or alert.
    ///
    /// `chained` is `None` when the chain never ended -- a corrupted `next` ref
    /// can point backwards, and a reader that followed it would walk for ever.
    Corrupt { claimed: usize, chained: Option<usize> },
}


/// How much of an image is spent, and on what.
///
/// THE ARENA IS THE LIMIT NOBODY SEES UNTIL IT BITES. A bebop store is
/// append-only: every commit allocates a new generation and the old one is
/// never reclaimed, so an image is spent by the NUMBER OF WRITES as much as by
/// the data. `settings.rs` measured it — 313 empty commits before a fresh
/// roster refused — and the hub answers `arena_full` when it happens. By then
/// the venue is mid-service and an order is being refused.
///
/// MEASURED AGAINST THE CEILING, NOT THE CAPACITY, and the difference is the
/// whole reason this type is not two fields. The images fall into two kinds:
///
///   - The append logs (`Hub`, `StockLog`) persist with `store.to_bytes()`, so
///     the capacity they were created with is the capacity they keep. It fills,
///     and when it is full the write is refused. Ceiling == capacity.
///   - The KV images (`Catalog`, `Settings`, `Posts`) persist with
///     `compacted_bytes_fit`, which commits the live entries into a FRESH image
///     sized by doubling from 16 KiB. Their capacity is re-chosen on every save,
///     so `used/capacity` is a sawtooth: it climbs toward full, the next save
///     doubles the capacity, and it drops by half. Measured that way a healthy
///     image reads 942 per mille and the reading falls to 517 the moment it
///     grows — a gauge that cries full at something with nothing to reclaim.
///     What actually refuses them is `compacted_bytes_fit` running out of
///     doublings at `DEFAULT_*_BYTES`. That is the ceiling.
///
/// So `used_per_mille` answers one question for both kinds — how close is this
/// image to the write it will refuse — and `capacity_cells` is kept beside it
/// as the raw fact, not as the denominator.
///
/// THERE IS NO `dead` FIGURE HERE ON PURPOSE. The superblock carries a
/// `superseded_cells` column and this write path never writes it: `Tx::sup_delta`
/// is initialised to zero in `Store::begin` and nothing increments it, so
/// `live_cells` is really "every cell ever allocated" and superseded is flatly 0
/// in every image this crate produces. A `dead_per_mille` built on it would
/// return 0 forever while reading like a measurement. It is left out rather
/// than shipped as a column that cannot move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    pub used_cells: i64,
    /// What the image was built with. For a KV image this is re-chosen on every
    /// save; see the type docs before using it as a denominator.
    pub capacity_cells: i64,
    /// The largest this image may ever grow to — the point at which a write is
    /// refused. This is the denominator.
    pub ceiling_cells: i64,
    /// Does this image DOUBLE itself rather than refuse a write?
    ///
    /// THE TWO KINDS REFUSE DIFFERENTLY AND ONLY ONE OF THEM REFUSES AT ALL.
    /// The append logs (`Hub`, `StockLog`) copy their chain into an image twice
    /// the size when one fills, so "how full" is a sawtooth that predicts a
    /// doubling, not a failure — measured: a stock log went from 7168 cells to
    /// 523264 over four thousand events without once refusing. The compacted KV
    /// images do NOT grow past `DEFAULT_*_BYTES`; when they fill, the write is
    /// refused for real.
    ///
    /// A verdict built from all five treats an imminent doubling as an
    /// emergency and says "compact" about an image that cannot be compacted.
    pub grows: bool,
    pub generation: i64,
}

impl Usage {
    /// Tenths of a percent of the CEILING, so a caller needs no float to render it.
    pub fn used_per_mille(&self) -> i64 {
        if self.ceiling_cells <= 0 {
            return 0;
        }
        (self.used_cells * 1000) / self.ceiling_cells
    }
}

/// The usable cells in an image of `bytes` bytes: the arena is what lies past
/// the two superblocks, exactly as `Store::create_bytes` computes it.
pub(crate) fn ceiling_cells(bytes: usize) -> i64 {
    (bytes / 8) as i64 - bebop_store::ARENA as i64
}

pub(crate) fn usage_of(store: &Store, ceiling_cells: i64) -> Usage {
    usage_of_kind(store, ceiling_cells, false)
}

pub(crate) fn usage_of_kind(store: &Store, ceiling_cells: i64, grows: bool) -> Usage {
    match store.pick() {
        Some(sb) => Usage {
            // `arena_used` is an absolute cell index; the arena starts at 1024,
            // so the cells actually spent are what lies past that.
            used_cells: (sb.arena_used - bebop_store::ARENA as i64).max(0),
            capacity_cells: store.capacity_cells(),
            ceiling_cells,
            grows,
            generation: sb.generation,
        },
        None => Usage {
            used_cells: 0,
            capacity_cells: 0,
            ceiling_cells,
            grows,
            generation: 0,
        },
    }
}

impl HubError {
    /// Is this "the image has no room left", and how much was wanted?
    ///
    /// Exposed as a method rather than left to the caller to pattern-match,
    /// because `bebop_store` is this crate's dependency and not everybody
    /// else's -- and a caller reduced to matching on a Debug string would be
    /// one rename away from silently losing the case.
    pub fn arena_full(&self) -> Option<(i64, i64)> {
        match self {
            HubError::Store(StoreError::ArenaFull { need, capacity }) => Some((*need, *capacity)),
            _ => None,
        }
    }
}

impl From<StoreError> for HubError {
    fn from(e: StoreError) -> Self {
        HubError::Store(e)
    }
}

/// One hub's store image, in memory.
pub struct Hub {
    store: Store,
}

/// One event, as read back out of the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub kind: EventKind,
    pub order_id: String,
    /// The kernel's order JSON at the moment of the event.
    pub order_json: String,
    pub seq: u64,
}

impl Hub {
    /// A brand-new hub: a fresh image with the log schema created.
    pub fn create() -> Result<Self, HubError> {
        Self::create_sized(DEFAULT_IMAGE_BYTES)
    }

    pub fn create_sized(bytes: usize) -> Result<Self, HubError> {
        let mut store = Store::create_bytes(bytes);
        EvLog::init_bytes(&mut store)?;
        Ok(Hub { store })
    }

    /// Load an existing image. Refuses one with no valid superblock rather than
    /// carrying on against a store that will answer nonsense.
    pub fn load(bytes: &[u8]) -> Result<Self, HubError> {
        let store = Store::from_bytes(bytes);
        if store.pick().is_none() {
            return Err(HubError::NotAHub);
        }
        // AND REFUSES ONE THAT LOST RECORDS ON THE WAY HERE. A superblock
        // survives a truncation -- it is fifteen cells at the front of the
        // image -- so "the superblock is valid" was never the same statement
        // as "the log is all here". See `chain_is_whole`.
        chain_is_whole(&store)?;
        Ok(Hub { store })
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
        let mut records = EvLog::walk(&self.store);
        records.reverse();
        // DOUBLE, DO NOT JUMP TO THE DEFAULT. `.max(DEFAULT_IMAGE_BYTES)` was
        // here and it meant a hub born at 64 KiB went straight to 4 MiB on its
        // first overflow -- which on a Worker is five D1 rows read and written
        // on every request, and the resource limit a few orders later. The
        // floor exists so a corrupt zero-length image cannot produce a
        // zero-length one; it is not a target.
        let bigger = self.store.to_bytes().len().saturating_mul(2).max(64 * 1024);
        let mut fresh = Store::create_bytes(bigger);
        EvLog::init_bytes(&mut fresh)?;
        let mut last: Option<[u8; 32]> = None;
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

    /// Every record this build cannot read, newest first.
    ///
    /// THE COUNT IS THE POINT. `events()` has always skipped a record it could
    /// not parse, and a silent skip is a venue losing an order with nothing to
    /// say so — the same mistake as the storage error that was read as "no
    /// image". The law that replaces the silence is arithmetic and is asserted
    /// in the tests: `len() == events().len() + quarantined().len()`. Anything
    /// else means a record went somewhere neither list admits to.
    ///
    /// A non-zero count is a FAILING GATE, not a warning: `/api/owner/health`
    /// carries it, and a quarantine nobody notices is a data-loss feature.
    pub fn quarantined(&self) -> Vec<Quarantined> {
        EvLog::walk(&self.store)
            .iter()
            .enumerate()
            .filter_map(|(at, r)| {
                decode_or_reason(r).err().map(|reason| Quarantined { id: hex32(&r.id), at, reason })
            })
            .collect()
    }

    /// Every event, newest first. A record that cannot be read is left out and
    /// appears in `quarantined()` instead — never dropped silently.
    pub fn events(&self) -> Vec<Event> {
        EvLog::walk(&self.store)
            .into_iter()
            .filter_map(|r| decode(&r))
            .collect()
    }

    /// One order's events, OLDEST FIRST — the input to a fold.
    ///
    /// `events()` is newest-first, which is right for "what just happened" and
    /// backwards for replaying a history: applied in that order a delta lands
    /// before the state it changes. This is the order a fold needs, and it is
    /// the only order in which the answer is the same for a log of snapshots
    /// and a log of deltas.
    /// NON-ORDER EVENTS ARE LEFT OUT, the same rule `orders()` applies: a
    /// `Revealed` record names who read a customer's details, and its payload
    /// is an audit fact rather than an order. Folded into an order it would add
    /// fields no consumer expects to a record that is served to customers.
    pub fn history(&self, order_id: &str) -> Vec<Event> {
        let mut out: Vec<Event> = self
            .events()
            .into_iter()
            .filter(|e| e.order_id == order_id && e.kind.is_order())
            .collect();
        out.reverse();
        out
    }

    /// Every event, OLDEST FIRST. One pass for a caller that folds them all.
    pub fn events_oldest_first(&self) -> Vec<Event> {
        let mut out = self.events();
        out.reverse();
        out
    }

    /// The payload of an order's most recent EVENT.
    ///
    /// NOT NECESSARILY THE ORDER'S STATE any more, and the distinction is the
    /// whole of phase 3. An event may now be a DELTA -- what changed, not what
    /// is -- so the state is the fold of `history()`, which the Worker does
    /// with a real JSON parser (`fold.rs`; see `minijson`'s header for why not
    /// here). For a log written before deltas the two are the same thing,
    /// which is what makes every existing image read correctly.
    ///
    /// Kept because `events()`/`orders()` return events and a caller that
    /// genuinely wants the last one written should be able to say so.
    pub fn order(&self, order_id: &str) -> Result<String, HubError> {
        self.events()
            .into_iter()
            .find(|e| e.order_id == order_id)
            .map(|e| e.order_json)
            .ok_or(HubError::UnknownOrder)
    }

    /// The newest state of every order, newest order first. One pass, keeping
    /// the first sighting of each id because `events()` is already newest-first.
    ///
    /// NON-ORDER EVENTS ARE SKIPPED, and that is load-bearing rather than
    /// tidy. `Revealed` records an audit fact under a subject that is not an
    /// order id; without this filter it would take a slot in this list and
    /// every fold built on it -- the analytics, the promo use-count, the
    /// dashboard -- would count an audit entry as a sale.
    pub fn orders(&self) -> Vec<Event> {
        // A set, not a scanned list: with a list this was O(events × orders),
        // and it runs on every poll of every console and every tracking sheet.
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut out = Vec::new();
        for e in self.events() {
            if !e.kind.is_order() {
                continue;
            }
            if !seen.insert(e.order_id.clone()) {
                continue;
            }
            out.push(e);
        }
        out
    }

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
        let mut records = EvLog::walk(&self.store);
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
        let mut fresh = Store::create_bytes(self.store.to_bytes().len().max(64 * 1024));
        EvLog::init_bytes(&mut fresh)?;
        let check_id = content_id_chained(&tip, &payload);
        EvLog::append_tip_bytes(
            &mut fresh,
            &Record { id: check_id, prev: tip, actor_pubkey: [0u8; 32], actor_seq: 0, payload },
        )?;

        let mut last = check_id;
        for r in &records {
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
                EvLog::append_bytes(&mut fresh, r)?;
                last = r.id;
                continue;
            }
            if !keep(&ev.order_id) {
                continue;
            }
            EvLog::append_bytes(&mut fresh, r)?;
            last = r.id;
        }
        EvLog::set_tip_bytes(&mut fresh, &last)?;
        self.store = fresh;
        Ok(archived)
    }

    /// The checkpoints this image carries, newest first — what a reader follows
    /// to find the archives.
    pub fn checkpoints(&self) -> Vec<String> {
        self.events()
            .into_iter()
            .filter(|e| e.kind == EventKind::Checkpoint)
            .map(|e| e.order_json)
            .collect()
    }

    /// Walk the chain and check every id against the payload it names.
    ///
    /// THE POINT IS THAT IT CAN FAIL. An append-only log whose ids are never
    /// recomputed is an append-only log by assertion; this is the assertion
    /// being checked, and it is the half of I4 that was claimed in a comment
    /// and not delivered by any code.
    pub fn chain_check(&self) -> ChainCheck {
        let mut out = ChainCheck::default();
        let walked = EvLog::walk(&self.store);
        let tip = EvLog::tip(&self.store);
        for (at, r) in walked.iter().enumerate() {
            out.records += 1;
            if r.id == content_id_chained(&r.prev, &r.payload) {
                out.chained += 1;
            } else if r.id == content_id(&r.payload) {
                out.legacy += 1;
            } else if forget::tombstone_holds(&walked, at, tip) {
                out.redacted += 1;
            } else {
                out.broken += 1;
            }
        }
        out
    }

    /// The newest record's chain id, in hex. `None` for a log with no records.
    ///
    /// THE ONE VALUE A WITNESS NEEDS. Every id commits to the id before it, so
    /// the tip commits to the whole history: two logs with the same tip are the
    /// same log, and a log that no longer holds last night's tip has had its
    /// end rewritten. That is the failure `chain_check` cannot see — a
    /// truncation leaves a shorter chain that is perfectly valid — and it is
    /// why this is published rather than kept inside the check.
    pub fn tip(&self) -> Option<String> {
        EvLog::tip(&self.store).map(|t| hex32(&t))
    }

    /// Is this chain id anywhere in this log?
    ///
    /// The question a witness asks the next night: the tip I wrote down is
    /// still in there, so nothing between it and the start was rewritten. A
    /// `false` after a rotation is not yet an accusation — the record may have
    /// moved to an archive, verbatim and with the same id — so the caller asks
    /// the archives before it says anything out loud.
    pub fn holds(&self, id_hex: &str) -> bool {
        EvLog::walk(&self.store).iter().any(|r| hex32(&r.id) == id_hex)
    }

    /// Every audit event, newest first.
    pub fn reveals(&self) -> Vec<Event> {
        self.events().into_iter().filter(|e| e.kind == EventKind::Revealed).collect()
    }
}

/// Is this "the image has no room left"? Matched through the public shape
/// rather than a Debug string, for the reason `HubError::arena_full` gives.
pub(crate) fn e_is_full(e: &StoreError) -> bool {
    matches!(e, StoreError::ArenaFull { .. })
}

/// Does this image hold every record it says it holds?
///
/// WHY AT LOAD AND NOT AT USE. `load` is on the path of every request, and the
/// cost was the argument against checking anything here -- but the check is a
/// pointer walk over a chain of a few hundred records, while `load` has
/// already decoded the whole image into cells. It is a fraction of a cost
/// already paid, and D0's `reliability-over-latency` is not a slogan: the
/// alternative is every reader above deciding for itself whether a short
/// answer was the truth, which is the decision that was already got wrong.
///
/// WHAT IT DOES NOT DO is verify the ids -- that is `chain_check`, it hashes
/// every record, and it belongs in the nightly job. This asserts what a caller
/// is promised: the chain is as long as the root claims, it ends, and every
/// record on it reads back.
///
/// AND IT IS ABOUT THE IMAGE, NOT ABOUT ONE RECORD. An image that arrived
/// incomplete is refused, because the caller can re-fetch it. A single record
/// this build cannot parse is a different failure and gets the opposite
/// answer: it is QUARANTINED, counted and served around, because one bad
/// record must not close the restaurant. See `Hub::quarantined`.
fn chain_is_whole(store: &Store) -> Result<(), HubError> {
    let claimed = EvLog::len(store);
    let Some(chained) = EvLog::chain_len(store) else {
        return Err(HubError::Corrupt { claimed, chained: None });
    };
    if chained != claimed {
        return Err(HubError::Corrupt { claimed, chained: Some(chained) });
    }
    Ok(())
}

/// A record the log holds and this build cannot read.
///
/// IT IS EVIDENCE, NOT AN ERROR MESSAGE, which is why it carries the id: the
/// record stays in the image verbatim, and a human can find it there. Nothing
/// here repairs anything — an automatic repair of a record nobody has looked at
/// is how a corrupted order becomes a plausible one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quarantined {
    /// The record's chain id in hex, as `chain_check` and the archives name it.
    pub id: String,
    /// Its position in the chain, newest first — the same order `events()` uses.
    pub at: usize,
    /// Which of the payload's promises it broke.
    pub reason: &'static str,
}

pub(crate) fn hex32(b: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for x in b {
        s.push(char::from_digit((x >> 4) as u32, 16).unwrap_or('0'));
        s.push(char::from_digit((x & 15) as u32, 16).unwrap_or('0'));
    }
    s
}

/// WHY THE FAILURE HAS A NAME. `decode` returned `None` for four different
/// things and every caller turned that into "not an event", so a record with an
/// unknown kind was indistinguishable from one whose payload was shredded. The
/// quarantine list is evidence for a human, and "it did not decode" is not
/// evidence. One decoder, four named refusals, and `decode` is this with the
/// name thrown away.
fn decode_or_reason(r: &Record) -> Result<Event, &'static str> {
    if r.payload.len() < 2 {
        return Err("short");
    }
    // The high bit marks a record redacted in place (`forget.rs`); the kind
    // is the low seven.
    let kind = EventKind::from_byte(r.payload[0] & !forget::REDACTED_BIT).ok_or("kind")?;
    let id_len = r.payload[1] as usize;
    if r.payload.len() < 2 + id_len {
        return Err("framing");
    }
    let order_id = String::from_utf8(r.payload[2..2 + id_len].to_vec()).map_err(|_| "id-utf8")?;
    let order_json =
        String::from_utf8(r.payload[2 + id_len..].to_vec()).map_err(|_| "json-utf8")?;
    Ok(Event { kind, order_id, order_json, seq: r.actor_seq })
}

fn decode(r: &Record) -> Option<Event> {
    decode_or_reason(r).ok()
}

/// Content id over the PREVIOUS ID AND THE PAYLOAD — the cascade that makes
/// the chain tamper-evident.
///
/// WHAT THIS FIXES. `content_id` hashes the payload alone, so editing an event
/// in an image changed that event's id and NOTHING ELSE: the records after it
/// still verified, and `stock.rs` said in as many words that "editing any
/// event changes every content id after it", which was not true of the code
/// under the comment. Folding the previous id in makes it true: rewriting
/// event N breaks N's own id, and repairing that id breaks N+1's `prev` and
/// therefore N+1's id, all the way to the tip.
///
/// Still FNV rather than sha256, for the reason below: this crate has no
/// dependencies and the cryptographic commitment lives in the kernel, where
/// the keys are. What this gives is detection of an EDIT, not resistance to a
/// determined forger.
pub(crate) fn content_id_chained(prev: &[u8; 32], payload: &[u8]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(32 + payload.len());
    buf.extend_from_slice(prev);
    buf.extend_from_slice(payload);
    content_id(&buf)
}

/// What a walk of the chain found. `legacy` records were written before the
/// cascade and can only be checked against the old scheme, which is a fact
/// about them rather than a fault: pretending otherwise would make every image
/// in production look broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChainCheck {
    pub records: usize,
    /// Verified against `content_id_chained(prev, payload)`.
    pub chained: usize,
    /// Verified only against the old `content_id(payload)`.
    pub legacy: usize,
    /// Matched neither. An edited event, or a damaged one.
    pub broken: usize,
    /// Redacted in place by `Hub::forget`: verified by LINK, not content.
    /// Must equal what the `Forgotten` declarations name (conservation law 9).
    pub redacted: usize,
}

impl ChainCheck {
    /// Nothing in this log fails BOTH schemes.
    pub fn intact(&self) -> bool {
        self.broken == 0
    }
}

/// Content id over the payload. Not a cryptographic commitment — it is the
/// chain's own addressing, and it is derived from the bytes so two identical
/// events are indistinguishable by construction.
fn content_id(payload: &[u8]) -> [u8; 32] {
    // FNV-1a over 4 lanes, spread to 32 bytes. Deliberately NOT sha256: this
    // crate has zero dependencies, and the cryptographic chain commitment lives
    // in the kernel where the keys are.
    let mut out = [0u8; 32];
    for lane in 0..4u64 {
        let mut h: u64 = 0xcbf29ce484222325 ^ lane.wrapping_mul(0x9E3779B97F4A7C15);
        for b in payload {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        out[lane as usize * 8..lane as usize * 8 + 8].copy_from_slice(&h.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACTOR: [u8; 32] = [0xA1; 32];
    fn order(id: &str, status: &str) -> String {
        format!(r#"{{"id":"{id}","status":"{status}","subtotal":1800}}"#)
    }

    /// `Amended` IS AN ORDER EVENT: it folds into the order it names, travels
    /// on the socket, and survives a reload under its own byte. An unassigned byte is still
    /// nobody's, so the reader that predates a kind keeps quarantining it —
    /// which is the deploy-order rule: readers before writers.
    #[test]
    fn an_amendment_is_an_order_event_under_its_own_byte() {
        assert_eq!(EventKind::from_u8(7), Some(EventKind::Amended));
        assert!(EventKind::Amended.is_order());
        assert_eq!(EventKind::from_u8(0x7f), None);
        let mut h = Hub::create_sized(1 << 20).unwrap();
        h.append(EventKind::Placed, "ord_a", &order("ord_a", "PENDING"), 1, ACTOR).unwrap();
        h.append(EventKind::Amended, "ord_a", r#"{"_d":true,"total":900}"#, 2, ACTOR).unwrap();
        let back = Hub::load(&h.to_bytes_trimmed()).unwrap();
        assert!(back.quarantined().is_empty(), "a kind-7 record is readable by this build");
        let hist = back.history("ord_a");
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[1].kind, EventKind::Amended, "oldest first: the amendment is second");
        assert_eq!(back.orders().len(), 1, "one order, not an order and an amendment");
    }

    #[test]
    fn a_fresh_hub_is_empty_and_reloads() {
        let h = Hub::create_sized(1 << 20).unwrap();
        assert!(h.is_empty());
        let back = Hub::load(&h.to_bytes()).unwrap();
        assert!(back.is_empty(), "an empty hub survives the byte round-trip");
    }

    #[test]
    fn load_refuses_something_that_is_not_a_hub() {
        assert!(matches!(Hub::load(&[0u8; 4096]), Err(HubError::NotAHub)),
                "a zeroed image has no superblock and must be refused, not served");
    }

    /// AND REFUSES A HUB THAT LOST ORDERS ON THE WAY HERE.
    ///
    /// `load` checked the superblock and stopped there -- but a superblock is
    /// fifteen cells at the FRONT of the image, so it survives a short read
    /// that takes half the orders with it. The hub then answered `len() == 12`
    /// while `orders()` listed four, and every reply built from it -- the
    /// console, the tracking sheet, the day's takings -- was confidently
    /// missing the rest. An image that cannot deliver what it claims is
    /// refused, so the caller can re-fetch or alert instead of serving it.
    #[test]
    fn load_refuses_a_hub_that_cannot_deliver_the_orders_it_claims() {
        let mut h = Hub::create_sized(1 << 20).unwrap();
        for i in 0..12 {
            let id = format!("ord_{i:02}");
            h.append(EventKind::Placed, &id, &order(&id, "PENDING"), i as u64, ACTOR).unwrap();
        }
        let bytes = h.to_bytes_trimmed();
        assert_eq!(Hub::load(&bytes).unwrap().orders().len(), 12, "the whole image is whole");

        for tenth in 1..10usize {
            let keep = bytes.len() * tenth / 10;
            if let Ok(short) = Hub::load(&bytes[..keep]) {
                assert_eq!(
                    short.len(),
                    short.events().len() + short.quarantined().len(),
                    "a hub cut to {keep} bytes loaded and then lost a record to neither list"
                );
            }
        }

        // And the sharpest case: every byte is there, one `next` ref is not.
        let mut st = Store::from_bytes(&bytes);
        let root = st.root().unwrap();
        let newest = st.follow(root, 1).unwrap();
        st.cells[newest + 2 + 2] = 1 << 40;
        assert!(
            matches!(Hub::load(&st.to_bytes()), Err(HubError::Corrupt { claimed: 12, .. })),
            "a chain that stops early must be refused, not served short"
        );
    }

    /// L5 FROM THE RESILIENCE BLUEPRINT: one bad record must not close the
    /// restaurant, and must not vanish either.
    ///
    /// These are the two failures of the same byte. Refusing the whole image
    /// because one record is unreadable takes a venue off the air over a single
    /// order; skipping it silently takes the order off the books and says
    /// nothing. So: the hub LOADS, the record is left out of `events()`, and it
    /// is named in `quarantined()` with the promise it broke and an id a human
    /// can find in the image. The arithmetic is the whole guarantee —
    /// `len() == events().len() + quarantined().len()` — because it is the one
    /// statement that cannot be true while a record is quietly missing.
    #[test]
    fn a_record_this_build_cannot_read_is_quarantined_and_the_venue_still_serves() {
        let mut h = Hub::create_sized(1 << 20).unwrap();
        for i in 0..6 {
            let id = format!("ord_{i:02}");
            h.append(EventKind::Placed, &id, &order(&id, "PENDING"), i as u64, ACTOR).unwrap();
        }
        let bytes = h.to_bytes_trimmed();

        let mut st = Store::from_bytes(&bytes);
        let root = st.root().unwrap();
        let newest = st.follow(root, 1).unwrap();
        // v2 packs eight payload bytes per cell after a twelve-cell header, plus
        // four more when an actor key is present (bit 0 of cell 11 says so).
        let at = if st.get(newest, 11) & 1 != 0 { 16 } else { 12 };
        let cell = st.get(newest, at);
        // Byte 0 of the payload is the event kind, and 9 is not one of the six.
        st.cells[newest + 2 + at] = (cell & !0xFF) | 9;

        let broken = Hub::load(&st.to_bytes()).expect("one bad record must not refuse the image");
        assert_eq!(broken.len(), 6, "the log still holds six records");
        assert_eq!(broken.events().len(), 5, "the unreadable one is not served");
        let q = broken.quarantined();
        assert_eq!(q.len(), 1, "and it is named: {q:?}");
        assert_eq!(q[0].reason, "kind");
        assert_eq!(q[0].at, 0, "it was the newest record");
        assert_eq!(q[0].id.len(), 64, "the id is there for a human to find");
        assert_eq!(
            broken.len(),
            broken.events().len() + broken.quarantined().len(),
            "a record must be in exactly one of the two lists"
        );
        // The orders that are readable are still served, which is the point.
        assert!(broken.order("ord_00").is_ok(), "the venue keeps serving");
    }

    /// The point of the log: an order's state is the FOLD, so the newest event
    /// wins and the earlier one is still there to explain how it got there.
    #[test]
    fn an_order_reads_back_as_its_newest_event() {
        let mut h = Hub::create_sized(1 << 20).unwrap();
        h.append(EventKind::Placed, "ord_a", &order("ord_a", "PENDING"), 1, ACTOR).unwrap();
        assert!(h.order("ord_a").unwrap().contains("PENDING"));

        h.append(EventKind::Advanced, "ord_a", &order("ord_a", "CONFIRMED"), 2, ACTOR).unwrap();
        assert!(h.order("ord_a").unwrap().contains("CONFIRMED"), "the fold shows the newest state");

        assert_eq!(h.len(), 2, "and the earlier event is NOT overwritten");
        let evs = h.events();
        assert_eq!(evs[0].kind, EventKind::Advanced);
        assert_eq!(evs[1].kind, EventKind::Placed);
    }

    #[test]
    fn orders_lists_each_order_once_at_its_newest_state() {
        let mut h = Hub::create_sized(1 << 20).unwrap();
        h.append(EventKind::Placed, "ord_a", &order("ord_a", "PENDING"), 1, ACTOR).unwrap();
        h.append(EventKind::Placed, "ord_b", &order("ord_b", "PENDING"), 2, ACTOR).unwrap();
        h.append(EventKind::Advanced, "ord_a", &order("ord_a", "READY"), 3, ACTOR).unwrap();

        let list = h.orders();
        assert_eq!(list.len(), 2, "two orders, not three events");
        assert_eq!(list[0].order_id, "ord_a", "newest first");
        assert!(list[0].order_json.contains("READY"));
        assert!(list[1].order_json.contains("PENDING"));
    }

    #[test]
    fn unknown_order_is_an_error_not_an_empty_string() {
        let h = Hub::create_sized(1 << 20).unwrap();
        assert!(matches!(h.order("nope"), Err(HubError::UnknownOrder)));
    }

    /// ROTATION KEEPS WHAT IS LIVE AND MOVES WHAT IS NOT. The orders that stay
    /// must fold to exactly what they folded to before: a venue must not see
    /// its kitchen change because the log was tidied.
    #[test]
    fn a_rotation_keeps_the_live_orders_untouched() {
        let mut h = Hub::create_sized(256 * 1024).unwrap();
        for i in 0..6u64 {
            let id = format!("ord_{i}");
            h.append(EventKind::Placed, &id, &order(&id, "PENDING"), i + 1, ACTOR).unwrap();
            h.append(EventKind::Advanced, &id, &order(&id, "CONFIRMED"), i + 10, ACTOR).unwrap();
        }
        let before: Vec<String> =
            ["ord_4", "ord_5"].iter().map(|id| h.order(id).unwrap()).collect();

        let archived = h.rotate(|id| id == "ord_4" || id == "ord_5").unwrap();

        let after: Vec<String> =
            ["ord_4", "ord_5"].iter().map(|id| h.order(id).unwrap()).collect();
        assert_eq!(before, after, "a kept order must be untouched by the rotation");
        assert_eq!(h.orders().len(), 2, "and nothing else stayed");
        assert!(h.order("ord_0").is_err(), "a moved order is not in the hot log");

        // THE HISTORY IS NOT GONE, it is in the bytes the caller now holds.
        let cold = Hub::load(&archived).unwrap();
        assert_eq!(cold.orders().len(), 6, "every order is in the archive");
        assert!(cold.order("ord_0").unwrap().contains("ord_0"));

        // And the hot image holds much less than what it replaced. MEASURED
        // IN ARENA CELLS, not in bytes of image: every image carries a fixed
        // 1024-cell superblock region -- eight kilobytes that neither side
        // pays for twice -- and at this size that header is most of the file.
        let hot_cells = h.usage().used_cells;
        let cold_cells = Hub::load(&archived).unwrap().usage().used_cells;
        println!("rotation: hot {hot_cells} cells, archive {cold_cells}");
        assert!(hot_cells * 2 < cold_cells, "hot {hot_cells} cells against archive {cold_cells}");
    }

    /// THE CHECKPOINT IS VISIBLE. An unknown event kind used to be dropped by
    /// `decode` without a sound, so a mark in the log would have been a mark
    /// nobody could see -- which is the failure this project keeps meeting.
    #[test]
    fn the_checkpoint_is_an_event_the_log_returns() {
        let mut h = Hub::create_sized(128 * 1024).unwrap();
        h.append(EventKind::Placed, "ord_1", &order("ord_1", "PENDING"), 1, ACTOR).unwrap();
        let tip_before = h.checkpoints().len();
        assert_eq!(tip_before, 0);

        let archived = h.rotate(|_| false).unwrap();

        let events = h.events();
        assert_eq!(events.len(), 1, "the checkpoint is the only thing left: {events:?}");
        assert_eq!(events[0].kind, EventKind::Checkpoint);
        assert!(!events[0].kind.is_order(), "a checkpoint is not an order");
        assert!(h.orders().is_empty(), "and it does not appear as one");

        let marks = h.checkpoints();
        assert_eq!(marks.len(), 1);
        assert!(marks[0].starts_with("tip="), "{}", marks[0]);
        assert!(marks[0].contains("events=1"), "{}", marks[0]);
        assert!(
            marks[0].contains(&format!("bytes={}", archived.len())),
            "the mark names the archive it describes: {}",
            marks[0]
        );
    }

    /// THE AUDIT TRAIL SURVIVES A ROTATION. It is not an order, so `keep` was
    /// never asked about it and the first rotation would have deleted the
    /// venue's entire record of who read whose phone number -- out of the hot
    /// log, and out of every archive reader, which folds orders.
    #[test]
    fn a_rotation_keeps_the_audit_trail() {
        let mut h = Hub::create_sized(128 * 1024).unwrap();
        h.append(EventKind::Placed, "ord_1", &order("ord_1", "DELIVERED"), 1, ACTOR).unwrap();
        h.append(EventKind::Revealed, "cust:abc", r#"{"by":"owner_1","at":2}"#, 2, ACTOR).unwrap();

        // Nothing is kept: the strongest version of the test.
        h.rotate(|_| false).unwrap();

        let reveals = h.reveals();
        assert_eq!(reveals.len(), 1, "the reveal must still be in the hot log");
        assert_eq!(reveals[0].order_id, "cust:abc");
        assert!(reveals[0].order_json.contains("owner_1"));
        assert!(h.orders().is_empty(), "and it is still not an order");
    }

    /// A SECOND ROTATION CHAINS. Each archive names the tip of the one before
    /// it, so the archives form a list a reader can walk backwards; the hot
    /// image carries exactly one mark, never a pile of them.
    #[test]
    fn a_second_rotation_chains_to_the_first() {
        let mut h = Hub::create_sized(256 * 1024).unwrap();
        h.append(EventKind::Placed, "ord_1", &order("ord_1", "PENDING"), 1, ACTOR).unwrap();
        let first = h.rotate(|_| false).unwrap();
        let mark_one = h.checkpoints()[0].clone();

        h.append(EventKind::Placed, "ord_2", &order("ord_2", "PENDING"), 2, ACTOR).unwrap();
        let second = h.rotate(|_| false).unwrap();
        let mark_two = h.checkpoints()[0].clone();

        assert_eq!(h.checkpoints().len(), 1, "one mark, not a pile");
        assert_ne!(mark_one, mark_two);
        // The second archive holds the first mark, so the chain is walkable.
        let cold_two = Hub::load(&second).unwrap();
        assert_eq!(cold_two.checkpoints(), vec![mark_one], "the archive carries the older mark");
        let cold_one = Hub::load(&first).unwrap();
        assert!(cold_one.checkpoints().is_empty(), "the first archive predates any mark");
    }

    /// WHAT A WITNESS IS FOR, and it is the failure `chain_check` is blind to.
    ///
    /// Removing records from the END leaves a shorter chain that verifies
    /// perfectly: every id still commits to the one before it, because the
    /// cascade only ever looks backwards. What does change is the TIP. So a
    /// tip written down somewhere the editor cannot reach — off-site, or in
    /// another object — turns a silent truncation into a contradiction.
    #[test]
    fn a_truncated_log_still_verifies_and_no_longer_holds_its_tip() {
        let mut h = Hub::create_sized(256 * 1024).unwrap();
        for i in 0..6u64 {
            let id = format!("ord_{i}");
            h.append(EventKind::Placed, &id, &order(&id, "PENDING"), i + 1, ACTOR).unwrap();
        }
        let witnessed = h.tip().expect("a log with records has a tip");
        assert!(h.holds(&witnessed), "the tip is in its own log");

        // The truncation: rebuild the log from the first five records only,
        // which is what an editor with write access can do.
        let mut cut = Hub::create_sized(256 * 1024).unwrap();
        for i in 0..5u64 {
            let id = format!("ord_{i}");
            cut.append(EventKind::Placed, &id, &order(&id, "PENDING"), i + 1, ACTOR).unwrap();
        }
        assert!(cut.chain_check().intact(), "a truncated log passes the chain check");
        assert_eq!(cut.len(), 5);
        assert!(!cut.holds(&witnessed), "and the witnessed tip is gone — which is the tell");
        assert_ne!(cut.tip(), Some(witnessed), "the tip moved backwards");
    }

    /// AND A ROTATION IS NOT A TRUNCATION, which is the distinction that makes
    /// the witness usable rather than an alarm every night. The records move
    /// verbatim, ids and all, so last night's tip is still held — by the
    /// archive. A witness that could not tell these apart would be switched
    /// off within a week.
    #[test]
    fn a_rotation_moves_the_tip_into_the_archive_rather_than_losing_it() {
        let mut h = Hub::create_sized(256 * 1024).unwrap();
        for i in 0..4u64 {
            let id = format!("ord_{i}");
            h.append(EventKind::Placed, &id, &order(&id, "PENDING"), i + 1, ACTOR).unwrap();
        }
        let witnessed = h.tip().expect("tip");
        let archived = h.rotate(|id| id == "ord_3").unwrap();
        let cold = Hub::load(&archived).unwrap();
        assert!(cold.holds(&witnessed), "the archive holds the record verbatim");
        // The hot log holds it too here, because `ord_3` was kept; what
        // matters is that the pair of images between them never loses it.
        assert!(
            h.holds(&witnessed) || cold.holds(&witnessed),
            "a rotation must not lose a record between the two images"
        );
        // An empty log has no tip, and that is not a failure either.
        assert_eq!(Hub::create_sized(1 << 16).unwrap().tip(), None);
    }

    /// The cascade survives a rotation. Records move verbatim, so each still
    /// commits to the id before it -- even where the record before it is now
    /// in a different image.
    #[test]
    fn a_rotated_log_still_verifies() {
        let mut h = Hub::create_sized(256 * 1024).unwrap();
        for i in 0..4u64 {
            let id = format!("ord_{i}");
            h.append(EventKind::Placed, &id, &order(&id, "PENDING"), i + 1, ACTOR).unwrap();
        }
        let archived = h.rotate(|id| id == "ord_3").unwrap();
        let hot = h.chain_check();
        assert!(hot.intact(), "{hot:?}");
        assert_eq!(hot.broken, 0);
        assert_eq!(hot.records, 2, "the checkpoint and the one kept order");
        let cold = Hub::load(&archived).unwrap().chain_check();
        assert!(cold.intact(), "{cold:?}");
        assert_eq!(cold.records, 4);
    }

    /// An order half of whose events survived would fold to a lie, so `keep` is
    /// asked about the ORDER and every event of a kept order travels with it.
    #[test]
    fn a_kept_order_keeps_all_of_its_events() {
        let mut h = Hub::create_sized(256 * 1024).unwrap();
        for (status, at) in [("PENDING", 1u64), ("CONFIRMED", 2), ("COOKING", 3), ("DELIVERED", 4)]
        {
            h.append(
                if at == 1 { EventKind::Placed } else { EventKind::Advanced },
                "ord_1",
                &order("ord_1", status),
                at,
                ACTOR,
            )
            .unwrap();
        }
        h.rotate(|id| id == "ord_1").unwrap();
        let kept: Vec<Event> = h.history("ord_1");
        assert_eq!(kept.len(), 4, "every event of a kept order travels with it");
        assert!(kept[0].order_json.contains("PENDING"), "oldest first, and the first is the placement");
        assert!(kept[3].order_json.contains("DELIVERED"));
    }

    /// THE CASCADE IS REAL NOW, and this is the test that would have caught the
    /// comment being wrong. Every id commits to the record before it, so a
    /// walk can say whether the log has been edited.
    #[test]
    fn every_id_commits_to_the_record_before_it() {
        let mut h = Hub::create_sized(256 * 1024).unwrap();
        for i in 0..6 {
            h.append(EventKind::Placed, &format!("ord_{i}"), &order(&format!("ord_{i}"), "PENDING"),
                     i as u64 + 1, ACTOR).unwrap();
        }
        let check = h.chain_check();
        assert_eq!(check.records, 6);
        assert_eq!(check.chained, 6, "every record verifies against prev + payload");
        assert_eq!(check.legacy, 0);
        assert!(check.intact());

        // The same payload after a DIFFERENT predecessor is a different id.
        // Under the old scheme these two were identical, which is what made
        // "editing any event changes every id after it" false.
        let one = content_id_chained(&[0u8; 32], b"same bytes");
        let two = content_id_chained(&[9u8; 32], b"same bytes");
        assert_ne!(one, two, "the id has to depend on where in the chain it sits");
    }

    /// An event edited in the image is found. This is the property the log is
    /// FOR: the record keeps its old id, the payload no longer hashes to it,
    /// and the walk says so instead of folding the forgery into an order.
    #[test]
    fn an_edited_event_is_reported_as_broken() {
        let mut h = Hub::create_sized(256 * 1024).unwrap();
        for i in 0..4 {
            h.append(EventKind::Placed, &format!("ord_{i}"), &order(&format!("ord_{i}"), "PENDING"),
                     i as u64 + 1, ACTOR).unwrap();
        }
        assert!(h.chain_check().intact());

        // Edit one byte of one payload in the raw image: "PENDING" -> "PENDINH".
        let mut bytes = h.to_bytes();
        let at = bytes
            .windows(7)
            .position(|w| w == b"PENDING")
            .expect("the status is in the image as text");
        bytes[at + 6] = b'H';
        let tampered = Hub::load(&bytes).unwrap();

        let check = tampered.chain_check();
        assert_eq!(check.records, 4);
        assert_eq!(check.broken, 1, "the edited record must not verify: {check:?}");
        assert!(!check.intact());
    }

    /// A log written before the cascade reads as LEGACY, not as broken. Every
    /// image in production is one of these, and a check that called them
    /// tampered with would be an alarm that is always on.
    #[test]
    fn a_pre_cascade_log_is_legacy_rather_than_broken() {
        // One record written the OLD way, by hand: id over the payload alone.
        let payload = {
            let mut p = vec![EventKind::Placed as u8];
            let id = b"ord_old";
            p.push(id.len() as u8);
            p.extend_from_slice(id);
            p.extend_from_slice(br#"{"id":"ord_old","status":"PENDING"}"#);
            p
        };
        let rec = Record {
            // The OLD scheme: the payload alone.
            id: content_id(&payload),
            prev: [0u8; 32],
            actor_pubkey: [0u8; 32],
            actor_seq: 1,
            payload,
        };
        // Written straight into a store, because no public path writes an old
        // id any more -- which is the point.
        let mut st = Store::create_bytes(64 * 1024);
        EvLog::init_bytes(&mut st).unwrap();
        EvLog::append_tip_bytes(&mut st, &rec).unwrap();
        let h = Hub::load(&st.to_bytes()).unwrap();

        let check = h.chain_check();
        assert_eq!(check.records, 1);
        assert_eq!(check.legacy, 1, "an old id is old, not wrong: {check:?}");
        assert_eq!(check.broken, 0);
        assert!(check.intact());
    }

    /// The whole hub survives being written out and read back — which is the
    /// operation a Worker performs on every single request.
    #[test]
    fn the_log_survives_the_byte_round_trip() {
        let mut h = Hub::create_sized(1 << 20).unwrap();
        for i in 0..5 {
            h.append(EventKind::Placed, &format!("ord_{i}"), &order(&format!("ord_{i}"), "PENDING"),
                     i as u64 + 1, ACTOR).unwrap();
        }
        let bytes = h.to_bytes();
        let back = Hub::load(&bytes).unwrap();
        assert_eq!(back.len(), 5);
        assert_eq!(back.orders().len(), 5);
        assert!(back.order("ord_3").unwrap().contains("ord_3"));
        assert_eq!(back.to_bytes(), bytes, "reloading changes nothing");
    }

    /// THE TRIMMED IMAGE IS THE SAME HUB. This is the image the object stores
    /// and the backup carries, so "the same" has to mean byte-for-byte after a
    /// reload, not merely "the orders come back": the padding must land the
    /// arena, the capacity and the superblocks exactly where the full image
    /// had them, or the next append writes into a different store.
    #[test]
    fn a_trimmed_log_reloads_into_the_same_hub() {
        let mut h = Hub::create_sized(1 << 20).unwrap();
        for i in 0..5 {
            h.append(EventKind::Placed, &format!("ord_{i}"), &order(&format!("ord_{i}"), "PENDING"),
                     i as u64 + 1, ACTOR).unwrap();
        }
        let full = h.to_bytes();
        let trimmed = h.to_bytes_trimmed();
        assert!(trimmed.len() < full.len() / 4, "a 1 MiB image of five orders is mostly zeros");

        let back = Hub::load(&trimmed).unwrap();
        assert_eq!(back.len(), 5);
        assert_eq!(back.orders().len(), 5);
        assert!(back.order("ord_3").unwrap().contains("ord_3"));
        assert_eq!(back.to_bytes(), full, "the padded image is the full image");
        assert_eq!(back.usage().capacity_cells, h.usage().capacity_cells, "capacity survives");
    }

    /// A hub reloaded from a trimmed image keeps appending where it left off,
    /// and what it writes next is still the same bytes as the one that never
    /// left memory. A capacity lost in the round trip would show here as an
    /// early refusal or a `grow` at the wrong size.
    #[test]
    fn a_trimmed_log_keeps_appending() {
        let mut h = Hub::create_sized(64 * 1024).unwrap();
        h.append(EventKind::Placed, "ord_1", &order("ord_1", "PENDING"), 1, ACTOR).unwrap();

        let mut back = Hub::load(&h.to_bytes_trimmed()).unwrap();
        h.append(EventKind::Advanced, "ord_1", &order("ord_1", "COOKING"), 2, ACTOR).unwrap();
        back.append(EventKind::Advanced, "ord_1", &order("ord_1", "COOKING"), 2, ACTOR).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back.to_bytes(), h.to_bytes(), "the reloaded hub writes the same image");
        assert_eq!(back.to_bytes_trimmed(), h.to_bytes_trimmed());
    }

    /// The number this change exists for: a fresh hub is a few hundred bytes on
    /// the wire, not its whole arena. It crossed the Worker-to-object hop on
    /// every write.
    #[test]
    fn a_fresh_log_is_tiny_on_the_wire() {
        let h = Hub::create_sized(4 << 20).unwrap();
        let trimmed = h.to_bytes_trimmed();
        assert_eq!(h.to_bytes().len(), 4 << 20);
        assert!(trimmed.len() < 16 * 1024, "a fresh 4 MiB hub trims to {} bytes", trimmed.len());
        assert_eq!(Hub::load(&trimmed).unwrap().len(), 0);
    }

    #[test]
    fn an_over_long_order_id_is_refused_rather_than_truncated() {
        let mut h = Hub::create_sized(1 << 20).unwrap();
        let long = "x".repeat(256);
        assert!(matches!(h.append(EventKind::Placed, &long, "{}", 1, ACTOR),
                         Err(HubError::OrderIdTooLong)),
                "truncating an id would silently merge two different orders");
    }
}

#[cfg(test)]
mod audit_tests {
    use super::*;

    /// An audit entry must not be able to masquerade as an order. Every fold in
    /// the system -- the takings, the promo use-count, the dashboard -- is built
    /// on `orders()`, and one bogus row in it is a number that is quietly wrong
    /// everywhere at once.
    #[test]
    fn a_reveal_is_not_an_order() {
        let mut h = Hub::create().unwrap();
        h.append(EventKind::Placed, "ord_1", r#"{"total":900}"#, 1, [0u8; 32]).unwrap();
        h.append(
            EventKind::Revealed,
            "cust:+355690000000",
            r#"{"by":"ana@dubin.al","at":1789000000000}"#,
            2,
            [0u8; 32],
        )
        .unwrap();
        h.append(EventKind::Placed, "ord_2", r#"{"total":850}"#, 3, [0u8; 32]).unwrap();

        let orders = h.orders();
        assert_eq!(orders.len(), 2, "the audit entry took an order's place");
        assert!(orders.iter().all(|e| e.kind.is_order()));
        assert!(orders.iter().all(|e| e.order_id.starts_with("ord_")));

        assert_eq!(h.reveals().len(), 1);
        assert_eq!(h.reveals()[0].order_id, "cust:+355690000000");
        // It survives a round trip through the image, like everything else.
        let back = Hub::load(&h.to_bytes()).unwrap();
        assert_eq!(back.reveals().len(), 1);
        assert_eq!(back.orders().len(), 2);
    }

    /// The audit payload must not contain what it audits: a log of who read a
    /// phone number that also holds the phone number has doubled the exposure.
    #[test]
    fn the_audit_entry_carries_no_contact_details() {
        let mut h = Hub::create().unwrap();
        h.append(
            EventKind::Revealed,
            "cust:8f2a9c",
            r#"{"by":"ana@dubin.al","at":1789000000000}"#,
            1,
            [0u8; 32],
        )
        .unwrap();
        let e = &h.reveals()[0];
        for leak in ["+355", "@gmail", "Rruga"] {
            assert!(!e.order_json.contains(leak), "the audit entry carries {leak}");
        }
    }
}

#[cfg(test)]
mod usage_tests {
    use super::*;

    /// The number that matters is how close the arena is to refusing a write,
    /// and a reading that does not move as the log grows is a gauge that is not
    /// connected to anything.
    #[test]
    fn usage_climbs_as_the_log_is_written() {
        let mut hub = Hub::create_sized(256 * 1024).expect("hub");
        let empty = hub.usage();
        assert!(empty.capacity_cells > 0, "capacity must be readable: {empty:?}");
        assert_eq!(empty.used_per_mille(), 0, "a fresh image has spent nothing");

        for i in 0..20 {
            hub.append(
                EventKind::Placed,
                &format!("ord-{i}"),
                r#"{"status":"new","items":[]}"#,
                i as u64,
                [0u8; 32],
            )
            .expect("append");
        }
        let after = hub.usage();
        assert!(after.used_cells > empty.used_cells, "{empty:?} -> {after:?}");
        assert!(after.generation > empty.generation);
        assert!(after.used_per_mille() > 0, "the gauge must move: {after:?}");
        assert!(
            after.used_per_mille() < 1000,
            "and must not read full when it is not: {after:?}"
        );
    }

    /// THE KV IMAGES ARE THE ONES THIS GETS WRONG IF IT USES CAPACITY. A
    /// compacted image is re-sized on every save, so measured against its own
    /// capacity the reading sawtooths — it climbed to 942 per mille and fell
    /// back to 517 on the next save, which would send an owner compacting
    /// something with nothing to reclaim. Against the ceiling it only climbs.
    #[test]
    fn a_compacted_image_gauge_never_falls_as_it_grows() {
        let mut worst_drop = 0;
        let mut prev = 0;
        let mut last = 0;
        for n in [1usize, 20, 60, 100, 140, 180, 220, 400] {
            let mut s = settings::Settings::create().expect("settings");
            for i in 0..n {
                s.set(&format!("ai.k{i}"), &"x".repeat(60));
            }
            let bytes = s.to_bytes().expect("to_bytes");
            let u = settings::Settings::load(&bytes).expect("reload").usage();
            let now = u.used_per_mille();
            worst_drop = worst_drop.max(prev - now);
            prev = now;
            last = now;
        }
        assert_eq!(
            worst_drop, 0,
            "the reading fell by {worst_drop} per mille as the image GREW; \
             it is being measured against a capacity that is re-chosen on save"
        );
        assert!(last > 0, "and it has to move at all: {last}");
    }

    /// The ceiling is the point a write is actually refused, so a reading near
    /// full must mean the next save is near failing — not that a doubling is due.
    #[test]
    fn the_compacted_gauge_predicts_the_real_refusal() {
        let mut last_ok = 0;
        for n in 1.. {
            let mut s = settings::Settings::create().expect("settings");
            for i in 0..n * 100 {
                s.set(&format!("ai.k{i}"), &"x".repeat(60));
            }
            match s.to_bytes() {
                Ok(b) => {
                    last_ok = settings::Settings::load(&b)
                        .expect("reload")
                        .usage()
                        .used_per_mille()
                }
                Err(_) => break,
            }
            assert!(n < 100, "settings must refuse eventually");
        }
        assert!(
            last_ok > 700,
            "the last reading before settings refused a save was only {last_ok} per mille"
        );
    }

    /// AN APPEND LOG DOES NOT REFUSE, SO THE GAUGE CANNOT WARN OF A REFUSAL.
    ///
    /// This test used to assert that a 16 KiB hub filled and that the reading
    /// just before the refusal was above 800 per mille. It passed for the wrong
    /// reason: the hub DID refuse, but only because `append` grew the image for
    /// a full record and not for the tip update that follows it -- fifty-six
    /// cells free and an order turned away. With that fixed the log grows
    /// instead, so what this must assert is the opposite: the reading moves,
    /// and filling is not a failure.
    #[test]
    fn an_append_log_doubles_instead_of_refusing() {
        let mut hub = Hub::create_sized(16 * 1024).expect("hub");
        let first = hub.usage();
        assert!(first.grows, "the order log grows; the gauge must say so");
        let mut doubled = false;
        for i in 0..2000 {
            hub.append(EventKind::Placed, &format!("o{i}"), "{}", i as u64, [0u8; 32])
                .unwrap_or_else(|e| panic!("the log refused order {i} rather than growing: {e:?}"));
            if hub.usage().capacity_cells > first.capacity_cells {
                doubled = true;
            }
        }
        assert!(doubled, "2000 orders did not outgrow a 16 KiB image: {:?}", hub.usage());
        assert_eq!(hub.len(), 2000, "growing lost records");
    }

    /// A COMPACTED IMAGE DOES refuse, and there the warning is the whole point.
    #[test]
    fn a_compacted_image_still_warns_before_it_refuses() {
        let mut last = 0;
        for n in 1.. {
            let mut s = settings::Settings::create().expect("settings");
            for i in 0..n * 100 {
                s.set(&format!("k{i}"), &"x".repeat(60));
            }
            match s.to_bytes() {
                Ok(b) => {
                    let u = settings::Settings::load(&b).expect("reload").usage();
                    assert!(!u.grows, "a compacted image must not claim to grow");
                    last = u.used_per_mille();
                }
                Err(_) => break,
            }
            assert!(n < 100, "settings must refuse eventually");
        }
        assert!(last > 700, "the last reading before the refusal was only {last}");
    }
}
