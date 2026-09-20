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

pub mod catalog;
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
pub mod roster;
pub mod settings;
pub mod stock;
pub mod subs;
pub mod voice;
pub mod zone;
pub mod token;

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
}

impl EventKind {
    /// Does this event describe an ORDER? Everything that folds the log into
    /// orders asks this first.
    pub fn is_order(self) -> bool {
        matches!(
            self,
            EventKind::Placed | EventKind::Advanced | EventKind::Paid | EventKind::Noted
        )
    }

    fn from_byte(b: u8) -> Option<Self> {
        match b {
            1 => Some(EventKind::Placed),
            2 => Some(EventKind::Advanced),
            3 => Some(EventKind::Paid),
            4 => Some(EventKind::Revealed),
            5 => Some(EventKind::Noted),
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
        Ok(Hub { store })
    }

    /// The image to persist. The caller writes this wherever the hub lives.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.store.to_bytes()
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

        let id = content_id(&payload);
        let prev = EvLog::tip(&self.store).unwrap_or([0u8; 32]);
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
        // ── THE RECORD AND THE TIP FILL THE ARENA SEPARATELY ──
        //
        // MEASURED: the order log refused order 2450 with FIFTY-SIX CELLS still
        // free. The record fitted; the tip update that follows it did not, and
        // only the record's failure was handled -- so a hub with room to grow
        // answered `arena_full` and a venue stopped taking orders mid-service.
        // `stock.rs` had already found this exact shape and says so in its own
        // header ("on a nearly full arena the RECORD still fits while the tip
        // update does not"); the fix was made there and never brought here.
        //
        // GROWING AFTER THE RECORD IS IN IS SAFE AND RE-APPENDING IS NOT.
        // `grow` copies the object chain verbatim and re-points the tip at its
        // last record -- which IS this one -- so a tip that failed needs only
        // the copy. Re-appending the record instead would count it twice, which
        // is how the stock ledger once recorded 3002 deliveries for 3000 made.
        match EvLog::append_bytes(&mut self.store, &rec) {
            Ok(gen) => {
                if EvLog::set_tip_bytes(&mut self.store, &id).is_err() {
                    self.grow()?;
                    EvLog::set_tip_bytes(&mut self.store, &id)?;
                }
                Ok(gen)
            }
            Err(e) if e_is_full(&e) => {
                self.grow()?;
                let gen = EvLog::append_bytes(&mut self.store, &rec)?;
                if EvLog::set_tip_bytes(&mut self.store, &id).is_err() {
                    self.grow()?;
                    EvLog::set_tip_bytes(&mut self.store, &id)?;
                }
                Ok(gen)
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

    /// Every event, newest first.
    pub fn events(&self) -> Vec<Event> {
        EvLog::walk(&self.store)
            .into_iter()
            .filter_map(|r| decode(&r))
            .collect()
    }

    /// The current state of one order: the payload of its most recent event.
    /// A fold, not a lookup — which is why a status can never disagree with the
    /// log that produced it.
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

    /// Every audit event, newest first.
    pub fn reveals(&self) -> Vec<Event> {
        self.events().into_iter().filter(|e| e.kind == EventKind::Revealed).collect()
    }
}

/// Is this "the image has no room left"? Matched through the public shape
/// rather than a Debug string, for the reason `HubError::arena_full` gives.
fn e_is_full(e: &StoreError) -> bool {
    matches!(e, StoreError::ArenaFull { .. })
}

fn decode(r: &Record) -> Option<Event> {
    if r.payload.len() < 2 {
        return None;
    }
    let kind = EventKind::from_byte(r.payload[0])?;
    let id_len = r.payload[1] as usize;
    if r.payload.len() < 2 + id_len {
        return None;
    }
    let order_id = String::from_utf8(r.payload[2..2 + id_len].to_vec()).ok()?;
    let order_json = String::from_utf8(r.payload[2 + id_len..].to_vec()).ok()?;
    Some(Event { kind, order_id, order_json, seq: r.actor_seq })
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
