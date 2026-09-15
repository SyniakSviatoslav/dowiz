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
pub mod subs;

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
}

impl EventKind {
    fn from_byte(b: u8) -> Option<Self> {
        match b {
            1 => Some(EventKind::Placed),
            2 => Some(EventKind::Advanced),
            3 => Some(EventKind::Paid),
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
        let gen = EvLog::append_bytes(&mut self.store, &rec)?;
        EvLog::set_tip_bytes(&mut self.store, &id)?;
        Ok(gen)
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
    pub fn orders(&self) -> Vec<Event> {
        let mut seen: Vec<String> = Vec::new();
        let mut out = Vec::new();
        for e in self.events() {
            if seen.iter().any(|s| s == &e.order_id) {
                continue;
            }
            seen.push(e.order_id.clone());
            out.push(e);
        }
        out
    }
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
