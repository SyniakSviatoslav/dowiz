//! The hub's images, in a Durable Object — one object per venue.
//!
//! WHY THIS EXISTS. `hubstore`'s header has named this destination since the
//! first version: every request read the whole hub image out of D1, parsed it,
//! answered, and threw it away, so the cost of asking "what is in the queue"
//! grew with how long the venue had been open. Three things were wrong with
//! that and they are all the same thing — the image had no home that outlived a
//! request.
//!
//!   1. THE READ. A Durable Object is one instance that stays alive between
//!      requests, so the image is already in memory. D1 was a round trip and a
//!      megabyte of hex; this is a field access.
//!   2. THE WRITE. Two Workers reading the same image and writing back would
//!      lose one of the two appends. `hubstore` answers that with a generation
//!      guard and a bounded retry, which is correct but is a retry loop standing
//!      in for serialisation. A Durable Object IS the serialisation: one
//!      instance, one writer, no race to guard against. The guard is kept
//!      anyway — see `put_image` — because a guard that is never needed costs
//!      nothing and a guard that was removed the day it became needed costs an
//!      order.
//!   3. THE TENANT. `id_from_name(location_id)` gives each venue its own object
//!      with its own storage. That is P67's hub-per-tenant, arriving as a
//!      property of where the bytes live rather than as a column everything has
//!      to remember to filter on.
//!
//! NO SQL REACHES THIS MODULE. Storage here is a key/value map addressed by
//! exactly the key we ask for; nothing queries inside an image, which was always
//! true and is now also true of the thing storing it.
//!
//! CHUNKED AT 96 KiB because a Durable Object refuses a single value over 128,
//! the same shape of limit D1 had at a million. The meta record is written LAST,
//! for the same reason `hubstore` writes the head last: a crash between the two
//! leaves the previous generation readable rather than half of the new one.

use std::cell::RefCell;
use std::collections::HashMap;

use worker::wasm_bindgen::{JsCast, JsValue};
use worker::*;

/// Under the Durable Object's 128 KiB per-value ceiling with room for the key
/// and the framing, and derived here rather than written as a number so the
/// next person can check it: 96 * 1024.
const CHUNK: usize = 96 * 1024;

/// The image the projections are about. Named here as well as in `hubstore`
/// because this object folds THAT image and no other: a settings image has no
/// orders in it, and a projection of one would be an empty list rather than an
/// error.
const LOG_IMAGE: &str = "log";

/// The catalogue image, which holds the venue's own record as well as its
/// dishes. Named here because `/fold/venue` reads it and nothing else does.
const CATALOG_IMAGE: &str = "catalog";

/// How long a courier's fix is worth serving. `live_eta` reads the newest fix
/// within twenty minutes; anything older is not a position, it is a memory.
const POSITION_KEEP_MS: i64 = 20 * 60 * 1000;

/// What a stored image is, apart from its bytes.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
struct Meta {
    generation: i64,
    chunks: usize,
    len: usize,
}

/// The last few events, so a client that was away can be told what it missed
/// instead of being handed the whole venue again.
///
/// A RING, AND A SMALL ONE. This is not a second copy of the log -- the log is
/// the log -- it is the window in which "what changed since generation N" can
/// be answered cheaply. Past the window the honest answer is "ask for
/// everything", which is also the answer after a hibernation, and a client
/// that hears it re-reads the list. Snapshot-and-delta from twenty years of
/// game netcode: the delta is an optimisation, the snapshot is the truth.
const RECENT_KEEP: usize = 256;

/// One event as a catch-up carries it.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Change {
    pub generation: i64,
    pub kind: u8,
    pub order_id: String,
    pub payload: String,
}

/// The changes after `since`, or `None` when this object cannot say.
///
/// `None` is not an error and must not be treated as one: it means the window
/// does not reach back that far -- a cold object, a long absence, a busy hour
/// -- and the caller should read the list instead. Returning an empty slice
/// there would be a lie shaped exactly like "nothing has changed".
pub fn changes_since(recent: &[Change], since: i64) -> Option<Vec<Change>> {
    let oldest = recent.first().map(|c| c.generation)?;
    let newest = recent.last().map(|c| c.generation)?;
    // The client's generation must be one this window covers. `since` equal to
    // the oldest - 1 is the edge that still works: everything after it is here.
    if since + 1 < oldest {
        return None;
    }
    // AND A CLIENT AHEAD OF THIS WINDOW IS NOT UP TO DATE, it is somewhere
    // else. A venue restored from a backup starts its object at generation 1
    // while a console's copy still says 500; answering "nothing changed"
    // would pin that copy forever, because `apply` ignores a generation it is
    // already past. Say the window cannot help and let it read the list.
    if since > newest {
        return None;
    }
    Some(recent.iter().filter(|c| c.generation > since).cloned().collect())
}

/// What a socket is subscribed to. Tags are how a hibernated object finds its
/// sockets again -- it has forgotten everything else about them.
///
/// THREE AUDIENCES, THREE TAGS, and the separation is not tidiness: a customer
/// watching one order must not be sent another customer's address, and the
/// broadcast is the only thing standing between the two.
pub const TAG_CONSOLE: &str = "console";
pub const TAG_COURIER: &str = "courier";
/// `order:<id>` — one customer, one order.
pub fn tag_order(order_id: &str) -> String {
    format!("order:{order_id}")
}
/// `courier:<id>` — WHICH courier, said by the Worker at connect time.
///
/// A socket carries this BESIDE `courier`, so the queue broadcast still finds
/// every courier with one tag while a GPS frame is attributed from the tag
/// rather than from the frame. A message cannot name a principal; a tag the
/// Worker attached can.
pub fn tag_courier(courier_id: &str) -> String {
    format!("{TAG_COURIER}:{courier_id}")
}

/// The courier a socket belongs to, from its tags.
fn courier_of(tags: &[String]) -> Option<String> {
    tags.iter().find_map(|t| t.strip_prefix("courier:").map(str::to_string))
}

/// A courier's last known position, held in the object rather than in D1.
///
/// GPS was a D1 ROW PER FIX, kept for nobody: the map reads the newest fix per
/// courier within twenty minutes and nothing else ever reads the table. In
/// memory it is a field; when the object hibernates the fixes are lost, which
/// is exactly right for a value whose meaning expires in minutes.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
pub struct Fix {
    pub lat_e6: i64,
    pub lng_e6: i64,
    pub at_ms: i64,
}

/// One order as a projection carries it: the fold, and the two facts about the
/// event that produced it.
///
/// This is what crosses the Worker↔object hop instead of the image. A console
/// poll used to ship the whole log -- every order the venue has ever taken --
/// so that the Worker could throw away all but the last day of it.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct OrderView {
    pub order_id: String,
    /// `dowiz_hub::EventKind` as its byte, because the enum is not serialisable
    /// and the number is what the log itself stores.
    pub kind: u8,
    pub seq: u64,
    /// The FOLDED order, as JSON text. Text rather than a `Value` because every
    /// consumer parses it themselves and re-serialising it here would be a
    /// second encoding of the same bytes.
    pub order_json: String,
}

impl OrderView {
    pub(crate) fn of_event(e: dowiz_hub::Event) -> Self {
        Self::of(e)
    }

    fn of(e: dowiz_hub::Event) -> Self {
        OrderView { order_id: e.order_id, kind: e.kind as u8, seq: e.seq, order_json: e.order_json }
    }
}

/// What an append asks for. The kernel has already decided; this is the record.
#[derive(serde::Deserialize)]
struct AppendIn {
    kind: u8,
    order_id: String,
    payload: String,
    clock: u64,
}

#[durable_object]
pub struct HubImages {
    state: State,
    /// The images this object has in hand. THE WHOLE POINT: a Durable Object
    /// outlives a request, so this survives between them and the storage below
    /// is touched only when the object is cold or something is written.
    mem: RefCell<HashMap<String, (Meta, Vec<u8>)>>,
    /// The catch-up window: recent changes, oldest first. See `changes_since`.
    recent: RefCell<Vec<Change>>,
    /// Where the couriers are, as they last said. Not persisted on purpose:
    /// see `Fix`.
    positions: RefCell<HashMap<String, Fix>>,
    /// The log image, FOLDED, at the generation it was folded from.
    ///
    /// A venue's consoles, couriers and customers all poll; between two polls
    /// nothing has usually changed, and re-folding an unchanged log is the
    /// same answer computed again. Keyed by generation so it cannot go stale:
    /// a write bumps the generation, and a generation that does not match is
    /// simply refolded.
    folded: RefCell<Option<(i64, Vec<OrderView>)>>,
}

/// A stored chunk comes back as whatever the platform decided to hand us —
/// `Uint8Array` or the `ArrayBuffer` behind one. Accept both rather than assume,
/// because assuming is a corrupt image reported a long way from here.
/// Which chunks of `new` differ from `old`, by index. Without an old image
/// every chunk is new. A chunk past the end of the old image is new. A chunk
/// is unchanged only when the STORED chunk has the same length as the new
/// slice and the same bytes: a stored chunk that is longer (the image shrank
/// and the new tail is a prefix of the old one) must be rewritten, or the
/// meta's `len` and the bytes on disk disagree on the next cold load.
fn changed_chunks(old: Option<&[u8]>, new: &[u8], chunk: usize) -> Vec<usize> {
    let chunks = new.len().div_ceil(chunk).max(1);
    (0..chunks)
        .filter(|&n| {
            let at = n * chunk;
            let end = (at + chunk).min(new.len());
            match old {
                Some(o) if (at + chunk).min(o.len()) == end => o[at..end] != new[at..end],
                _ => true,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{changed_chunks, OrderView};

    /// A CLIENT THAT WAS AWAY IS TOLD WHAT IT MISSED, or told to ask again --
    /// and the difference matters more than either answer. An empty list where
    /// the window does not reach is a lie shaped exactly like "nothing has
    /// changed", and a console would believe it for as long as it stayed open.
    #[test]
    fn a_catch_up_says_what_changed_or_says_it_cannot() {
        let mk = |g: i64| super::Change {
            generation: g,
            kind: dowiz_hub::EventKind::Advanced as u8,
            order_id: format!("ord_{g}"),
            payload: String::new(),
        };
        let window: Vec<super::Change> = (10..=14).map(mk).collect();

        // Inside the window: only what is newer.
        let got = super::changes_since(&window, 12).expect("the window covers 12");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].generation, 13);
        assert_eq!(got[1].generation, 14);

        // The exact edge: a client at 9 has seen everything before 10.
        assert!(super::changes_since(&window, 9).is_some());
        assert_eq!(super::changes_since(&window, 9).unwrap().len(), 5);

        // Before the edge: the window cannot say, and says so.
        assert!(super::changes_since(&window, 8).is_none());
        assert!(super::changes_since(&window, 0).is_none());

        // Up to date: nothing changed, which is a real answer.
        assert_eq!(super::changes_since(&window, 14).unwrap().len(), 0);

        // A cold object has no window at all.
        assert!(super::changes_since(&[], 14).is_none());
    }

    /// The projection crosses a boundary as JSON, so its shape is a contract.
    /// `kind` travels as the byte the log itself stores, because the enum
    /// cannot cross and a name could drift from the number.
    #[test]
    fn an_order_view_survives_the_json_it_crosses_on() {
        let view = OrderView {
            order_id: "ord_1".into(),
            kind: dowiz_hub::EventKind::Advanced as u8,
            seq: 1789000000000,
            order_json: r#"{"id":"ord_1","status":"COOKING"}"#.into(),
        };
        let wire = serde_json::to_string(&view).expect("serialise");
        let back: OrderView = serde_json::from_str(&wire).expect("parse");
        assert_eq!(back.order_id, view.order_id);
        assert_eq!(back.seq, view.seq);
        assert_eq!(back.order_json, view.order_json);
        assert_eq!(
            dowiz_hub::EventKind::from_u8(back.kind),
            Some(dowiz_hub::EventKind::Advanced),
            "the byte has to name the same kind on the other side"
        );
    }


    #[test]
    fn without_an_old_image_every_chunk_is_written() {
        assert_eq!(changed_chunks(None, &[1u8; 250], 100), vec![0, 1, 2]);
        assert_eq!(changed_chunks(None, &[], 100), vec![0]);
    }

    #[test]
    fn an_identical_image_writes_nothing() {
        let img = [7u8; 250];
        assert_eq!(changed_chunks(Some(&img), &img, 100), Vec::<usize>::new());
    }

    #[test]
    fn an_append_touches_the_front_and_the_tail_only() {
        let mut old = vec![0u8; 250];
        old[3] = 1;
        let mut new = old.clone();
        new[3] = 2; // the superblock moved
        new[249] = 9; // the tail moved
        assert_eq!(changed_chunks(Some(&old), &new, 100), vec![0, 2]);
    }

    #[test]
    fn growth_writes_the_new_chunks_and_the_last_old_one_it_extends() {
        let old = vec![0u8; 250];
        let mut new = vec![0u8; 420];
        new[300] = 1;
        // chunk 2 is now [200,300) where before it was [200,250): a longer
        // slice than storage holds, so it is written; chunks 3 and 4 are
        // wholly new; chunks 0 and 1 are untouched.
        assert_eq!(changed_chunks(Some(&old), &new, 100), vec![2, 3, 4]);
    }

    #[test]
    fn a_shorter_image_rewrites_the_chunk_that_got_shorter() {
        let old = vec![5u8; 420];
        let new = vec![5u8; 250];
        // chunks 0 and 1 are the same 100 bytes; chunk 2 is now 50 bytes
        // where storage holds 100, so it MUST be written even though those
        // 50 bytes match -- the reviewer's case: skip it and the next cold
        // load assembles 300 bytes under a meta that says 250.
        assert_eq!(changed_chunks(Some(&old), &new, 100), vec![2]);
    }
}

fn chunk_bytes(v: &JsValue) -> Option<Vec<u8>> {
    if let Some(a) = v.dyn_ref::<js_sys::Uint8Array>() {
        return Some(a.to_vec());
    }
    if let Some(b) = v.dyn_ref::<js_sys::ArrayBuffer>() {
        return Some(js_sys::Uint8Array::new(b).to_vec());
    }
    None
}

impl HubImages {
    fn meta_key(id: &str) -> String {
        format!("m:{id}")
    }
    fn chunk_key(id: &str, n: usize) -> String {
        format!("c:{id}:{n}")
    }

    /// The image, from memory if this object has it, from storage if not.
    async fn image(&self, id: &str) -> Result<Option<(Meta, Vec<u8>)>> {
        if let Some(hit) = self.mem.borrow().get(id) {
            return Ok(Some(hit.clone()));
        }
        let store = self.state.storage();
        // AN ERROR IS NOT AN ABSENCE, and `.ok().flatten()` said it was.
        //
        // `Storage::get` answers Ok(None) for a key that is not there and Err
        // for a store that could not answer -- a timeout, an internal error, a
        // `Meta` that stopped deserialising. Folding the second into the first
        // tells every caller above that this venue has NO IMAGE, and every
        // guard downstream is built to be helpful about that: `load` hands the
        // handler a fresh 64 KiB hub, `put_image` reads the generation as 0,
        // agrees with the caller's 0, and writes chunk zero plus a meta saying
        // `chunks: 1`. The venue's entire order log is then unreachable -- the
        // old chunks are still on disk with nothing naming them. The same
        // conflation defeats `import`'s never-overwrite check and
        // `seed_fresh_hub`'s emptiness test, and makes `/fold/orders` answer an
        // empty list at generation 0 instead of failing.
        //
        // The chunk read two lines down has always propagated its errors with
        // `?`. Only the meta read swallowed them.
        let Some(meta) = store.get::<Meta>(&Self::meta_key(id)).await? else {
            return Ok(None);
        };
        // ALL CHUNKS IN ONE CALL. Asking for them one at a time would reproduce
        // the round-trip-per-chunk shape that made the D1 version slow, on a
        // store where the calls are cheaper but not free.
        let keys: Vec<String> = (0..meta.chunks).map(|n| Self::chunk_key(id, n)).collect();
        let got = store.get_multiple(keys.clone()).await?;
        let mut bytes = Vec::with_capacity(meta.len);
        for key in &keys {
            let v = got.get(&JsValue::from_str(key));
            let Some(part) = chunk_bytes(&v) else {
                // A meta record whose chunks are not all there is not an image.
                // Refusing names it; assembling what IS there would hand the
                // kernel a truncated arena.
                return Err(Error::RustError(format!("image {id} is missing chunk {key}")));
            };
            bytes.extend_from_slice(&part);
        }
        if bytes.len() != meta.len {
            return Err(Error::RustError(format!(
                "image {id} is {} bytes, its meta says {}",
                bytes.len(),
                meta.len
            )));
        }
        let entry = (meta, bytes);
        self.mem.borrow_mut().insert(id.to_string(), entry.clone());
        Ok(Some(entry))
    }

    /// The log image's folded orders, newest first — from the memo when the
    /// generation has not moved, from the bytes when it has.
    ///
    /// THE FOLD HAPPENS HERE AND NOT IN THE WORKER, which is the whole of
    /// phase 2. The image lives in this object; a Worker that wants the queue
    /// used to be handed every order the venue had ever taken so it could keep
    /// the last day. Now it is handed the last day.
    async fn orders_view(&self) -> Result<(i64, Vec<OrderView>)> {
        let Some((meta, bytes)) = self.image(LOG_IMAGE).await? else {
            return Ok((0, Vec::new()));
        };
        if let Some((gen, view)) = self.folded.borrow().as_ref() {
            if *gen == meta.generation {
                return Ok((*gen, view.clone()));
            }
        }
        let hub = dowiz_hub::Hub::load(&bytes)
            .map_err(|_| Error::RustError("hub image is unreadable".into()))?;
        let view: Vec<OrderView> =
            crate::hubstore::orders_state(&hub).into_iter().map(OrderView::of).collect();
        *self.folded.borrow_mut() = Some((meta.generation, view.clone()));
        Ok((meta.generation, view))
    }

    /// Tell everyone who is watching that an event landed.
    ///
    /// INTEREST MANAGEMENT, the oldest trick in multiplayer networking: the
    /// console hears about every order, a courier hears about the queue, and a
    /// customer hears about THEIR order and nothing else. The alternative --
    /// one channel carrying everything -- would put one customer's address on
    /// another customer's socket, and no amount of client-side filtering makes
    /// that acceptable.
    ///
    /// A BROADCAST IS NOT A REQUEST. Outgoing messages on an accepted socket
    /// are not billed as requests, which is the whole economy of this phase: a
    /// venue's consoles, couriers and customers stop asking every twelve
    /// seconds whether anything happened, and the object tells them when
    /// something does.
    fn broadcast(&self, kind: u8, order_id: &str, payload: &str, generation: i64) {
        // ONLY ORDER EVENTS TRAVEL. A `Revealed` record is an audit fact under
        // a subject that is not an order id ("cust:<key>"), and its payload is
        // not a delta -- a client folding it would invent a row with a
        // reason and a reader and no items, and keep it. The Rust folds have
        // skipped non-order kinds since they were written; this is the same
        // rule on the wire, applied before the event leaves.
        //
        // The generation still moved, so the window gets a gap instead: the
        // next `?since=` is answered with "ask for the list".
        if dowiz_hub::EventKind::from_u8(kind).is_none_or(|k| !k.is_order()) {
            self.recent.borrow_mut().clear();
            let msg = serde_json::json!({ "t": "moved", "generation": generation }).to_string();
            for ws in self.state.get_websockets_with_tag(TAG_CONSOLE) {
                let _ = ws.send_with_str(&msg);
            }
            return;
        }
        {
            let mut recent = self.recent.borrow_mut();
            recent.push(Change {
                generation,
                kind,
                order_id: order_id.to_string(),
                payload: payload.to_string(),
            });
            if recent.len() > RECENT_KEEP {
                let cut = recent.len() - RECENT_KEEP;
                recent.drain(0..cut);
            }
        }
        let msg = serde_json::json!({
            "t": "event",
            "kind": kind,
            "orderId": order_id,
            "payload": payload,
            "generation": generation,
        })
        .to_string();
        let mut seen: Vec<String> = Vec::new();
        for tag in [TAG_CONSOLE.to_string(), TAG_COURIER.to_string(), tag_order(order_id)] {
            if seen.contains(&tag) {
                continue;
            }
            for ws in self.state.get_websockets_with_tag(&tag) {
                // A SEND THAT FAILS IS A SOCKET THAT WENT AWAY, which is the
                // ordinary end of every socket. It is not this append's
                // problem and must not become the caller's error.
                let _ = ws.send_with_str(&msg);
            }
            seen.push(tag);
        }
    }

    /// Accept one socket, tagged by what it is allowed to hear.
    ///
    /// HIBERNATION IS THE POINT. `accept_websocket_with_tags` hands the socket
    /// to the runtime, which keeps it open while the object sleeps; duration
    /// is billed only while the object is actually running. A socket held by
    /// the object itself would keep it awake and turn a free connection into a
    /// billed one.
    fn accept(&self, tag: &str, protocol: Option<&str>) -> Result<Response> {
        let pair = WebSocketPair::new()?;
        // A COURIER GETS TWO TAGS: `courier`, which the queue broadcast fans
        // out to, and `courier:<id>`, which says whose socket this is. The
        // second is the only thing the object knows about the client, so it is
        // what a position is attributed to.
        if let Some(id) = tag.strip_prefix("courier:") {
            let both = [TAG_COURIER, tag];
            let _ = id;
            self.state.accept_websocket_with_tags(&pair.server, &both);
        } else {
            self.state.accept_websocket_with_tags(&pair.server, &[tag]);
        }
        // ── THE SUBPROTOCOL IS ECHOED WHERE THE RESPONSE IS BUILT ──
        //
        // A client that offers subprotocols requires the server to choose one,
        // or the browser fails the handshake on its own side. The echo was
        // being set on the Worker's side, on the response that came BACK from
        // this object -- and a Response that already wraps a platform one has
        // immutable headers, so that threw and the handshake answered 500.
        // Here the response has not been built yet, so the header is part of
        // its construction.
        let res = Response::from_websocket(pair.client)?;
        match protocol {
            Some(p) => {
                let headers = Headers::new();
                headers.set("Sec-WebSocket-Protocol", p)?;
                Ok(res.with_headers(headers))
            }
            None => Ok(res),
        }
    }

    /// Append one event to the log and persist it, under the same generation
    /// guard a write of the whole image carries.
    ///
    /// THE DECISION IS STILL THE WORKER'S. The kernel decided this transition
    /// was legal and computed the payload; this is the record of it. What
    /// moves here is the WRITE, which no longer means shipping the whole image
    /// back -- an append sends a few hundred bytes instead of the log.
    async fn append(&self, expected: i64, ev: AppendIn) -> Result<Option<(i64, usize)>> {
        let (generation, bytes) = match self.image(LOG_IMAGE).await? {
            Some((meta, bytes)) => (meta.generation, Some(bytes)),
            None => (0, None),
        };
        if expected != generation {
            return Ok(None);
        }
        let mut hub = match bytes {
            Some(b) => dowiz_hub::Hub::load(&b)
                .map_err(|_| Error::RustError("hub image is unreadable".into()))?,
            // BORN SMALL, as `hubstore::load` does it: a hub created at 4 MiB
            // made the third order of the day exceed the isolate's limit.
            None => dowiz_hub::Hub::create_sized(64 * 1024)
                .map_err(|_| Error::RustError("cannot create hub image".into()))?,
        };
        let kind = dowiz_hub::EventKind::from_u8(ev.kind)
            .ok_or_else(|| Error::RustError(format!("unknown event kind {}", ev.kind)))?;
        hub.append(kind, &ev.order_id, &ev.payload, ev.clock, [0u8; 32])
            .map_err(|e| Error::RustError(format!("hub append failed: {e:?}")))?;
        let len = hub.len();
        match self.put_image(LOG_IMAGE, generation, &hub.to_bytes_trimmed()).await? {
            Some(next) => {
                // AFTER THE WRITE LANDED, never before: a subscriber told about
                // an event that was not persisted would be told the truth about
                // a log that does not say it.
                self.broadcast(ev.kind, &ev.order_id, &ev.payload, next);
                Ok(Some((next, len)))
            }
            // Cannot happen inside a serialised object -- the generation was
            // read two lines ago -- but the caller's contract already says
            // what to do about it, so say it rather than assume.
            None => Ok(None),
        }
    }

    /// PLACE AN ORDER: the reservation, the redemption and the append, in ONE
    /// object turn.
    ///
    /// WHAT THIS REPLACED, and it is worth stating where the code is rather
    /// than only in `command::place`'s header: the Worker used to run this as a
    /// saga over the network -- write the stock image, write the log image, and
    /// if the second failed, write the stock image again to release what the
    /// first had held. If THAT failed it printed a line and gave up, so a
    /// stranded reservation made a kitchen believe it was out of something it
    /// had, and nothing but a sampled trace knew.
    ///
    /// THE COMPENSATION IS GONE BECAUSE THERE IS NOTHING TO COMPENSATE.
    /// `command::place::decide` does every fallible thing against images held in
    /// memory; this function writes only after it has returned `Ok`. A refusal
    /// drops both copies and the venue's stored state was never touched.
    ///
    /// THE LOG IS WRITTEN FIRST, AND THE ORDER IS DELIBERATE. Both writes
    /// happen in one turn, so the runtime will normally commit them together --
    /// but "normally" is not a guarantee this code is entitled to make, so the
    /// order is chosen for what happens if the second one is lost:
    ///   * log then stock (this) leaves an order whose ingredients are not
    ///     held. The kitchen can see the order, the conservation audit can see
    ///     the disagreement, and a human can fix it.
    ///   * stock then log leaves ingredients held for an order that does not
    ///     exist. Nothing shows it to anyone. That is the failure this whole
    ///     change exists to remove, so it is not the one to re-introduce at a
    ///     smaller size.
    async fn place(
        &self,
        input: crate::command::place::PlaceIn,
    ) -> Result<std::result::Result<crate::command::place::PlaceOut, crate::command::Refused>>
    {
        // The projection is what the promo count is taken from -- the same one
        // every reader uses, memoised against this generation, so counting a
        // code's uses does not fold the log a second time.
        let (log_generation, listed) = self.orders_view().await?;
        let mut hub = match self.image(LOG_IMAGE).await? {
            Some((_, bytes)) => dowiz_hub::Hub::load(&bytes)
                .map_err(|_| Error::RustError("hub image is unreadable".into()))?,
            // BORN SMALL, as `append` does it: a hub created at 4 MiB made the
            // third order of the day exceed the isolate's limit.
            None => dowiz_hub::Hub::create_sized(64 * 1024)
                .map_err(|_| Error::RustError("cannot create hub image".into()))?,
        };
        let stock_image = crate::hubstore::IMAGE_STOCK;
        let (stock_generation, mut stock) = match self.image(stock_image).await? {
            Some((meta, bytes)) => (
                meta.generation,
                dowiz_hub::stock::StockLog::load(&bytes)
                    .map_err(|_| Error::RustError("stock image is unreadable".into()))?,
            ),
            None => (
                0,
                dowiz_hub::stock::StockLog::create_sized(64 * 1024)
                    .map_err(|_| Error::RustError("cannot create stock image".into()))?,
            ),
        };
        let reserved_before = stock.len();

        let stored = match crate::command::place::decide(&mut hub, &mut stock, &listed, &input) {
            Ok(v) => v,
            // NOTHING HAS BEEN WRITTEN. Both images go out of scope here.
            Err(r) => return Ok(Err(r)),
        };

        let events = hub.len();
        let Some(next) = self.put_image(LOG_IMAGE, log_generation, &hub.to_bytes_trimmed()).await?
        else {
            // Cannot happen inside a serialised object -- the generation was
            // read above and nothing else runs between -- but the caller's
            // contract already says what a moved generation means.
            return Ok(Err(crate::command::Refused::Append(
                "the log generation moved during a placement".into(),
            )));
        };

        // A venue that models no ingredients reserved none, and writing the
        // stock image to say so would cost a request per order for no change.
        if stock.len() != reserved_before {
            if self
                .put_image(stock_image, stock_generation, &stock.to_bytes_trimmed())
                .await?
                .is_none()
            {
                // LOUD, AND THE ORDER IS NAMED. The order exists and its
                // ingredients are not held; that is the recoverable direction
                // (see the header) but it is not a silent one.
                console_error!(
                    "stock: order {} was placed and its reservations were NOT written",
                    input.order_id
                );
            }
        }

        // ── THE BELL IS OWED THE MOMENT THE ORDER EXISTS ──
        //
        // Written HERE, in the turn that wrote the order, rather than awaited
        // by the Worker afterwards. `storefront::place` used to `await
        // notify::order_placed` once, with no memory of it: Telegram refuses,
        // or the isolate is cut off at the end of the response, and the kitchen
        // is never told about an order that exists and is paid for.
        //
        // THE TEXT CROSSES AND THE RECIPIENTS DO NOT. Rendering needs the
        // basket lines, which the Worker is holding; WHO it goes to is in the
        // settings image, which this object is holding. So neither side reads
        // an image it did not already have.
        //
        // A FAILURE TO ENQUEUE DOES NOT FAIL THE ORDER. The order is in the log
        // and the customer has been charged; refusing now would lose a placed
        // order to protect a message about it. It is loud instead.
        if let Some(text) = input.notify_text.as_deref() {
            if let Err(e) = self.enqueue_bell(&input.order_id, text, input.now_ms).await {
                console_error!("outbox: order {} was placed and the bell was NOT queued: {e}", input.order_id);
            }
        }

        // AFTER THE WRITE LANDED, never before.
        self.broadcast(dowiz_hub::EventKind::Placed as u8, &input.order_id, &stored, next);
        Ok(Ok(crate::command::place::PlaceOut { stored, generation: next, events }))
    }

    /// ADVANCE AN ORDER AND SETTLE THE SHELF, in one turn.
    ///
    /// The Worker used to append through `/fold/append` and then settle the
    /// stock ledger in a SECOND round trip whose failure it could only log --
    /// deliberately, because failing the transition afterwards would have left
    /// the order and the ledger disagreeing the other way round. Both are now
    /// decided against copies in memory and written once both have succeeded,
    /// so neither direction is reachable from a failure in the other.
    async fn advance(
        &self,
        input: crate::command::advance::AdvanceIn,
    ) -> Result<std::result::Result<crate::command::advance::AdvanceOut, crate::command::Refused>>
    {
        let (log_generation, listed) = self.orders_view().await?;
        let current = listed.iter().find(|o| o.order_id == input.order_id).map(|o| o.order_json.clone());
        let Some((_, log_bytes)) = self.image(LOG_IMAGE).await? else {
            return Ok(Err(crate::command::Refused::NotFound));
        };
        let mut hub = dowiz_hub::Hub::load(&log_bytes)
            .map_err(|_| Error::RustError("hub image is unreadable".into()))?;
        let stock_image = crate::hubstore::IMAGE_STOCK;
        let (stock_generation, mut stock) = match self.image(stock_image).await? {
            Some((meta, bytes)) => (
                meta.generation,
                dowiz_hub::stock::StockLog::load(&bytes)
                    .map_err(|_| Error::RustError("stock image is unreadable".into()))?,
            ),
            None => (
                0,
                dowiz_hub::stock::StockLog::create_sized(64 * 1024)
                    .map_err(|_| Error::RustError("cannot create stock image".into()))?,
            ),
        };
        let settled_before = stock.len();

        let merged = match crate::command::advance::decide(
            &mut hub,
            &mut stock,
            current.as_deref(),
            &input,
        ) {
            Ok(v) => v,
            // NOTHING HAS BEEN WRITTEN.
            Err(r) => return Ok(Err(r)),
        };

        let Some(next) = self.put_image(LOG_IMAGE, log_generation, &hub.to_bytes_trimmed()).await?
        else {
            return Ok(Err(crate::command::Refused::Append(
                "the log generation moved during a transition".into(),
            )));
        };
        if stock.len() != settled_before
            && self
                .put_image(stock_image, stock_generation, &stock.to_bytes_trimmed())
                .await?
                .is_none()
        {
            console_error!(
                "stock: order {} advanced to {} and the settlement was NOT written",
                input.order_id,
                input.next
            );
        }
        let body = crate::fold::delta(
            &serde_json::from_str(current.as_deref().unwrap_or("{}")).unwrap_or_default(),
            &merged,
        )
        .to_string();
        self.broadcast(dowiz_hub::EventKind::Advanced as u8, &input.order_id, &body, next);
        Ok(Ok(crate::command::advance::AdvanceOut {
            merged: merged.to_string(),
            generation: next,
        }))
    }

    /// HAND AN ORDER TO A COURIER: the assignment record and the event that
    /// makes it visible, in one turn.
    ///
    /// THE LOG IS WRITTEN FIRST, and for this command the reasoning is the
    /// mirror of the old bug. The assignment record is also the LOCK -- the
    /// refusal of a second courier is `ops.get("asg", id).is_some()` -- so it
    /// used to land BEFORE the event that shows it, and a lost append left an
    /// invisible lock that the console could neither see nor clear. Writing the
    /// log first means a lost ops write leaves an order that names its courier
    /// and no record behind it: visible, and re-assignable.
    async fn assign(
        &self,
        input: crate::command::assign::AssignIn,
    ) -> Result<std::result::Result<crate::command::assign::AssignOut, crate::command::Refused>>
    {
        let (log_generation, listed) = self.orders_view().await?;
        let current = listed.iter().find(|o| o.order_id == input.order_id).map(|o| o.order_json.clone());
        let Some((_, log_bytes)) = self.image(LOG_IMAGE).await? else {
            return Ok(Err(crate::command::Refused::NotFound));
        };
        let mut hub = dowiz_hub::Hub::load(&log_bytes)
            .map_err(|_| Error::RustError("hub image is unreadable".into()))?;
        let ops_image = crate::hubstore::IMAGE_OPS;
        let ceiling = crate::hubstore::OPS_BYTES;
        let (ops_generation, mut ops) = match self.image(ops_image).await? {
            Some((meta, bytes)) => (
                meta.generation,
                // A CORRUPT IMAGE IS NOT AN EMPTY ONE: reading it as empty here
                // would hand out an order that already has a courier.
                dowiz_hub::table::Table::load(&bytes, ceiling)
                    .map_err(|_| Error::RustError("ops image is unreadable".into()))?,
            ),
            None => (
                0,
                dowiz_hub::table::Table::create(ceiling)
                    .map_err(|_| Error::RustError("cannot create ops image".into()))?,
            ),
        };

        if let Err(r) =
            crate::command::assign::decide(&mut hub, &mut ops, current.as_deref(), &input)
        {
            return Ok(Err(r));
        }

        let Some(next) = self.put_image(LOG_IMAGE, log_generation, &hub.to_bytes_trimmed()).await?
        else {
            return Ok(Err(crate::command::Refused::Append(
                "the log generation moved during an assignment".into(),
            )));
        };
        let ops_bytes = ops
            .to_bytes()
            .map_err(|e| Error::RustError(format!("ops image will not serialise: {e:?}")))?;
        if self.put_image(ops_image, ops_generation, &ops_bytes).await?.is_none() {
            console_error!(
                "ops: order {} names courier {} and the assignment was NOT written",
                input.order_id,
                input.courier_id
            );
        }
        let body = serde_json::json!({
            "_d": { "courier_id": input.courier_id, "assigned_at_ms": input.now_ms }
        })
        .to_string();
        self.broadcast(dowiz_hub::EventKind::Noted as u8, &input.order_id, &body, next);
        Ok(Ok(crate::command::assign::AssignOut { generation: next }))
    }

    /// REFOLD FROM THE BYTES AND DIFF AGAINST WHAT IS BEING SERVED.
    ///
    /// IT DELIBERATELY IGNORES THE MEMO IT IS CHECKING. `orders_view` answers
    /// from `self.folded` when the generation has not moved, and that memo is
    /// what every reader in the platform gets — so nothing else in the system
    /// is in a position to disagree with it. This folds the image again, from
    /// the stored bytes, and compares.
    ///
    /// AND IT CROSSES THE TWO IMAGES, which is the check nothing did. The log
    /// can say an order is over while the stock ledger is still holding its
    /// ingredients; `StockLedger::stranded()` has been able to report that
    /// since it was written and nothing in production ever called it.
    ///
    /// READ-ONLY. It writes nothing, takes no generation and cannot repair
    /// anything — a gate that fixes what it finds is a gate whose findings
    /// nobody ever sees.
    async fn rebuild(&self) -> Result<crate::rebuild::Report> {
        let memo: Vec<(String, String)> = self
            .folded
            .borrow()
            .as_ref()
            .map(|(_, v)| v.iter().map(|o| (o.order_id.clone(), o.order_json.clone())).collect())
            .unwrap_or_default();
        let Some((_, bytes)) = self.image(LOG_IMAGE).await? else {
            return Ok(crate::rebuild::Report::default());
        };
        // FRESH, from the bytes. `Hub::load` parses the arena and
        // `orders_state` replays the chain; neither consults the memo.
        let hub = dowiz_hub::Hub::load(&bytes)
            .map_err(|_| Error::RustError("hub image is unreadable".into()))?;
        let fresh: Vec<(String, String)> = crate::hubstore::orders_state(&hub)
            .into_iter()
            .map(OrderView::of)
            .map(|o| (o.order_id, o.order_json))
            .collect();

        // WHEN THE MEMO IS COLD there is nothing to compare it against, and
        // saying "everything agrees" would be a measurement of nothing. The
        // fresh fold is used for both sides, so `stale` is empty BECAUSE
        // nothing was being served, not because it was right.
        let memo = if memo.is_empty() { fresh.clone() } else { memo };

        let (held, modelled) = match self.image(crate::hubstore::IMAGE_STOCK).await? {
            Some((_, b)) => {
                let log = dowiz_hub::stock::StockLog::load(&b)
                    .map_err(|_| Error::RustError("stock image is unreadable".into()))?;
                let modelled = log.len() > 0;
                let led = log
                    .ledger()
                    .map_err(|e| Error::RustError(format!("stock ledger: {e}")))?;
                (led.stranded(), modelled)
            }
            None => (Vec::new(), false),
        };
        Ok(crate::rebuild::compare(&fresh, &memo, &held, modelled))
    }

    /// Queue the kitchen's message for every channel this venue has configured.
    ///
    /// THE RECIPIENTS COME FROM THE SETTINGS IMAGE THIS OBJECT ALREADY HOLDS,
    /// which is the only reason this costs nothing: the Worker would have had
    /// to fetch it.
    ///
    /// ONE ENTRY PER CHANNEL, keyed by order AND channel, so a command replayed
    /// by the idempotency layer writes over its own entry rather than adding a
    /// second message to a kitchen. Idempotency at the queue as well as at the
    /// route.
    ///
    /// A VENUE THAT HAS CONFIGURED NOTHING QUEUES NOTHING, and that is not a
    /// failure: it is most venues, and writing an entry with nowhere to send it
    /// would fill an image with messages that can only ever be abandoned.
    async fn enqueue_bell(&self, order_id: &str, text: &str, now_ms: i64) -> Result<()> {
        let settings = match self.image(crate::hubstore::IMAGE_SETTINGS).await? {
            Some((_, bytes)) => dowiz_hub::settings::Settings::load(&bytes)
                .map_err(|_| Error::RustError("settings image is unreadable".into()))?,
            None => return Ok(()),
        };
        let mut queued: Vec<crate::outbox::Entry> = Vec::new();
        let chat = settings.known("notify.telegram.chat").trim().to_string();
        if !chat.is_empty() {
            queued.push(crate::outbox::Entry::new(
                format!("{order_id}/telegram"),
                "telegram",
                chat,
                text.to_string(),
                now_ms,
            ));
        }
        // ASKED FOR, NOT ASSUMED, exactly as the inline version had it: Meta
        // bills every one of these and Telegram carries the same text for
        // nothing, so a venue that has not set `notify.whatsapp.status = on` is
        // not billed for announcements it never asked for.
        if settings.flag("notify.whatsapp.status") {
            if let Some(wa) = crate::channels::whatsapp_cfg(&settings) {
                if !wa.to.is_empty() {
                    queued.push(crate::outbox::Entry::new(
                        format!("{order_id}/whatsapp"),
                        "whatsapp",
                        wa.to.clone(),
                        text.to_string(),
                        now_ms,
                    ));
                }
            }
        }
        if queued.is_empty() {
            return Ok(());
        }

        let image = crate::outbox::IMAGE_OUTBOX;
        let (generation, mut table) = match self.image(image).await? {
            Some((meta, bytes)) => (
                meta.generation,
                dowiz_hub::table::Table::load(&bytes, crate::outbox::OUTBOX_BYTES)
                    .map_err(|_| Error::RustError("outbox image is unreadable".into()))?,
            ),
            None => (
                0,
                dowiz_hub::table::Table::create(crate::outbox::OUTBOX_BYTES)
                    .map_err(|_| Error::RustError("cannot create outbox image".into()))?,
            ),
        };
        for e in &queued {
            let rec = serde_json::to_string(e).unwrap_or_default();
            table
                .put(crate::outbox::KIND, &e.id, &rec, &[], &[])
                .map_err(|e| Error::RustError(format!("outbox: {e:?}")))?;
        }
        let bytes = table
            .to_bytes()
            .map_err(|e| Error::RustError(format!("outbox will not serialise: {e:?}")))?;
        if self.put_image(image, generation, &bytes).await?.is_none() {
            return Err(Error::RustError("the outbox generation moved".into()));
        }
        Ok(())
    }

    /// Write, with the SAME generation guard the D1 version used.
    ///
    /// A Durable Object serialises its own requests, so two writers cannot
    /// interleave here and the guard should never fire. It is kept because it
    /// costs one comparison, because it is what lets a caller tell "someone
    /// else moved it" from "it failed", and because the day this moves again is
    /// the day the guard is load-bearing.
    async fn put_image(&self, id: &str, expected: i64, bytes: &[u8]) -> Result<Option<i64>> {
        let current = self.image(id).await?.map(|(m, _)| m.generation).unwrap_or(0);
        if expected != current {
            return Ok(None);
        }
        let next = current + 1;
        let store = self.state.storage();

        let chunks = bytes.len().div_ceil(CHUNK).max(1);
        // ONLY THE CHUNKS THAT MOVED. The log is append-only: an append touches
        // the superblock at the front and the tail at the back, and every
        // chunk between them is byte-for-byte what storage already holds.
        // Writing all of them made one order cost `ceil(image / 96 KiB)` row
        // writes -- 45 at a 4 MB log, and growing with every order the venue
        // ever took. The comparison is against the copy in memory, which
        // `image()` above has just made current.
        let changed = {
            let mem = self.mem.borrow();
            changed_chunks(mem.get(id).map(|(_, b)| b.as_slice()), bytes, CHUNK)
        };
        let meta = Meta { generation: next, chunks, len: bytes.len() };
        let written = async {
            for n in changed {
                let at = n * CHUNK;
                let end = (at + CHUNK).min(bytes.len());
                let part = js_sys::Uint8Array::from(&bytes[at..end]);
                store.put_raw(&Self::chunk_key(id, n), part).await?;
            }
            // META LAST. Between the chunks and this line the old meta still
            // describes the old image, so an interruption leaves the previous
            // generation readable rather than a head pointing at a tail that
            // has not landed.
            store.put(&Self::meta_key(id), meta).await
        }
        .await;
        if let Err(e) = written {
            // STORAGE IS THE ONLY TRUTH AFTER A FAILED WRITE. Some chunks may
            // have landed and some not; the copy in memory no longer says
            // what is on disk, and the next write's diff would trust it and
            // skip a chunk that is wrong. Forget the copy: the next read
            // rebuilds it from storage and the next diff is against the truth.
            self.mem.borrow_mut().remove(id);
            return Err(e);
        }

        // Chunks past the end of the new image are unreachable now that the
        // meta describes a shorter one, and only now.
        //
        // THE BORROW IS RELEASED BEFORE THE AWAITS. Holding a `Ref` across a
        // storage call leaves a window in which any re-entry that borrows
        // mutably -- `image()` on a cold key -- panics the whole object. The
        // number is copied out first; it is a `usize`.
        let old_chunks = self.mem.borrow().get(id).map(|(m, _)| m.chunks).unwrap_or(0);
        for n in chunks..old_chunks {
            let _ = store.delete(&Self::chunk_key(id, n)).await;
        }
        self.mem.borrow_mut().insert(id.to_string(), (meta, bytes.to_vec()));
        // THE PROJECTION IS DERIVED FROM THIS IMAGE, so it dies with the write
        // that replaced it. Keyed by generation it could only ever be stale
        // for a moment; dropped here it cannot be stale at all.
        if id == LOG_IMAGE {
            *self.folded.borrow_mut() = None;
            // ── A WHOLE-IMAGE WRITE IS A GAP, AND IT HAS TO BE ONE ──
            //
            // Not every write to this log comes through `append`. A placement
            // still does its own read-modify-write (a promotion's last use has
            // to be counted and spent in one breath), and so does a rotation.
            // Those move the generation WITHOUT putting a `Change` in the
            // window -- and a window that stayed quiet would answer the next
            // `?since=` with "nothing changed" about a log that gained an
            // order. A console would believe it, advance its copy's
            // generation, and never see that order again.
            //
            // So the window is CLEARED, which makes `changes_since` answer
            // `None` -- "ask for the list" -- and the sockets are told the log
            // moved so they ask now rather than in ninety seconds.
            self.recent.borrow_mut().clear();
            let msg = serde_json::json!({ "t": "moved", "generation": next }).to_string();
            for tag in [TAG_CONSOLE, TAG_COURIER] {
                for ws in self.state.get_websockets_with_tag(tag) {
                    let _ = ws.send_with_str(&msg);
                }
            }
        }
        Ok(Some(next))
    }
}

impl DurableObject for HubImages {
    fn new(state: State, _env: Env) -> Self {
        Self {
            state,
            mem: RefCell::new(HashMap::new()),
            recent: RefCell::new(Vec::new()),
            positions: RefCell::new(HashMap::new()),
            folded: RefCell::new(None),
        }
    }

    /// The object's whole surface. NOT REACHABLE FROM THE INTERNET: a Durable
    /// Object is addressable only through a stub held by a Worker that has the
    /// binding, so these paths need no authentication of their own — the
    /// handlers that call them have already done it.
    ///
    /// TWO KINDS OF ROUTE, and the difference is phase 2. `/img/...` hands over
    /// BYTES: the catalogue, the settings, a backup, anything whose reader is
    /// not this object. `/fold/...` hands over ANSWERS: the queue, one order,
    /// an append. The bytes route stays because an image still has to be
    /// exportable and importable; the fold route exists so that asking what is
    /// in the queue stops costing the whole history of the venue.
    async fn fetch(&self, req: Request) -> Result<Response> {
        let path = req.path();
        let mut seg = path.split('/').filter(|s| !s.is_empty());
        let head = seg.next().unwrap_or("");
        if head == "fold" {
            let what = seg.next().unwrap_or("");
            return match (req.method(), what) {
                // ── THE SOCKET ──
                //
                // The Worker has already decided WHO this is and which topic
                // they may hear; the tag it sends is that decision. This object
                // is not reachable from the internet, so the tag arriving here
                // is the Worker's word, not a client's.
                (Method::Get, "socket") => {
                    let url = req.url()?;
                    let tag = url
                        .query_pairs()
                        .find(|(k, _)| k == "tag")
                        .map(|(_, v)| v.to_string())
                        .unwrap_or_default();
                    if tag.is_empty() {
                        return Response::error("a socket needs a tag", 400);
                    }
                    let protocol = url
                        .query_pairs()
                        .find(|(k, _)| k == "proto")
                        .map(|(_, v)| v.to_string());
                    self.accept(&tag, protocol.as_deref())
                }
                // Where the couriers are, as they last said over their sockets.
                // Empty after a hibernation, which is honest: a position whose
                // meaning expires in minutes should not survive a sleep.
                (Method::Get, "positions") => {
                    let now = Date::now().as_millis() as i64;
                    let fresh: HashMap<String, Fix> = self
                        .positions
                        .borrow()
                        .iter()
                        .filter(|(_, f)| now - f.at_ms < POSITION_KEEP_MS)
                        .map(|(k, v)| (k.clone(), *v))
                        .collect();
                    Response::from_json(&fresh)
                }
                // THE VENUE'S OWN RECORD, and nothing else in the catalogue.
                //
                // The owner's dashboard needs one field from it -- the time
                // zone, so "today" starts at the venue's midnight rather than
                // UTC's. Reaching that through `load_catalog` would pull the
                // whole catalogue image, which on a venue with a real menu is
                // half a megabyte, on every poll. The object already holds
                // those bytes; parsing them HERE and answering with the ~1 KB
                // that was asked for is the same move phase 2 made for the log.
                //
                // A venue with no catalogue yet answers `null`, which is not an
                // error: the caller falls back to the default zone and says so.
                (Method::Get, "venue") => {
                    let rec = match self.image(CATALOG_IMAGE).await? {
                        Some((_, bytes)) => dowiz_hub::catalog::Catalog::load(&bytes)
                            .ok()
                            .and_then(|c| c.location()),
                        None => None,
                    };
                    match rec {
                        Some(json) => Response::ok(json).map(|mut r| {
                            let _ = r.headers_mut().set("content-type", "application/json");
                            r
                        }),
                        None => Response::ok("null").map(|mut r| {
                            let _ = r.headers_mut().set("content-type", "application/json");
                            r
                        }),
                    }
                }
                // THE GENERATION ALONE. A writer that needs nothing but the
                // guard used to ask for the orders and throw them away, which
                // on a venue with a thousand of them is a list built for a
                // number.
                (Method::Get, "generation") => {
                    let generation =
                        self.image(LOG_IMAGE).await?.map(|(m, _)| m.generation).unwrap_or(0);
                    let mut res = Response::from_json(
                        &serde_json::json!({ "generation": generation }),
                    )?;
                    res.headers_mut().set("x-generation", &generation.to_string())?;
                    Ok(res)
                }
                // WHAT CHANGED SINCE, for a client that already has a copy.
                // `full: true` means the window does not reach that far and
                // the list is the answer -- which is also what a cold object
                // says, and is not an error.
                (Method::Get, "changes") => {
                    let url = req.url()?;
                    let since: i64 = url
                        .query_pairs()
                        .find(|(k, _)| k == "since")
                        .and_then(|(_, v)| v.parse().ok())
                        .unwrap_or(-1);
                    let generation =
                        self.image(LOG_IMAGE).await?.map(|(m, _)| m.generation).unwrap_or(0);
                    let body = match changes_since(&self.recent.borrow(), since) {
                        Some(changes) => serde_json::json!({
                            "generation": generation, "full": false, "changes": changes
                        }),
                        None => serde_json::json!({
                            "generation": generation, "full": true, "changes": []
                        }),
                    };
                    let mut res = Response::from_json(&body)?;
                    res.headers_mut().set("x-generation", &generation.to_string())?;
                    Ok(res)
                }
                (Method::Get, "orders") => {
                    let (generation, view) = self.orders_view().await?;
                    let mut res = Response::from_json(&view)?;
                    res.headers_mut().set("x-generation", &generation.to_string())?;
                    Ok(res)
                }
                (Method::Get, "order") => {
                    // THE ID TRAVELS AS A QUERY PARAMETER, not as a path
                    // segment: an order id is not this object's to constrain,
                    // and `query_pairs` decodes it exactly once, where a hand
                    // written path split would decode it never.
                    let url = req.url()?;
                    let Some(id) = url
                        .query_pairs()
                        .find(|(k, _)| k == "id")
                        .map(|(_, v)| v.to_string())
                    else {
                        return Response::error("no order named", 400);
                    };
                    let (generation, view) = self.orders_view().await?;
                    let found = view.into_iter().find(|o| o.order_id == id);
                    let mut res = match found {
                        Some(o) => Response::from_json(&o)?,
                        None => Response::empty()?.with_status(404),
                    };
                    res.headers_mut().set("x-generation", &generation.to_string())?;
                    Ok(res)
                }
                // ── THE COMMAND SURFACE ──
                //
                // `append` above is the Worker's decision being recorded;
                // `place` is the object making one. That is the difference
                // `BLUEPRINT-ARCHITECTURE-EVOLUTION` P1 is about, and it is why
                // this route takes no `x-generation`: there is no read-modify-
                // write over a hop to guard, because the read and the write are
                // two statements inside the same turn.
                (Method::Post, "place") => {
                    let mut req = req;
                    let input: crate::command::place::PlaceIn = req.json().await?;
                    match self.place(input).await? {
                        Ok(out) => {
                            let mut res = Response::from_json(&out)?;
                            res.headers_mut().set("x-generation", &out.generation.to_string())?;
                            Ok(res)
                        }
                        Err(r) => Response::error(r.message().to_string(), r.status()),
                    }
                }
                // THE REBUILD, read-only, for the conservation audit's law 8.
                (Method::Get, "rebuild") => Response::from_json(&self.rebuild().await?),
                (Method::Post, "advance") => {
                    let mut req = req;
                    let input: crate::command::advance::AdvanceIn = req.json().await?;
                    match self.advance(input).await? {
                        Ok(out) => Response::from_json(&out),
                        Err(r) => Response::error(r.message().to_string(), r.status()),
                    }
                }
                (Method::Post, "assign") => {
                    let mut req = req;
                    let input: crate::command::assign::AssignIn = req.json().await?;
                    match self.assign(input).await? {
                        Ok(out) => Response::from_json(&out),
                        Err(r) => Response::error(r.message().to_string(), r.status()),
                    }
                }
                (Method::Post, "append") => {
                    let expected: i64 = req
                        .headers()
                        .get("x-generation")
                        .ok()
                        .flatten()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(-1);
                    if expected < 0 {
                        return Response::error("x-generation is required on a write", 400);
                    }
                    let mut req = req;
                    let ev: AppendIn = req.json().await?;
                    match self.append(expected, ev).await? {
                        Some((generation, len)) => {
                            let mut res = Response::from_json(
                                &serde_json::json!({ "generation": generation, "events": len }),
                            )?;
                            res.headers_mut().set("x-generation", &generation.to_string())?;
                            Ok(res)
                        }
                        None => Response::error("generation moved", 409),
                    }
                }
                _ => Response::error("no such projection", 404),
            };
        }

        let id = path.rsplit('/').next().unwrap_or("").to_string();
        if id.is_empty() {
            return Response::error("no image named", 400);
        }
        match req.method() {
            Method::Get => match self.image(&id).await? {
                Some((meta, bytes)) => {
                    let mut res = Response::from_bytes(bytes)?;
                    res.headers_mut().set("x-generation", &meta.generation.to_string())?;
                    Ok(res)
                }
                // Absent is not an error: the caller creates a fresh image, which
                // is how a venue's very first order is placed.
                None => {
                    let mut res = Response::empty()?.with_status(204);
                    res.headers_mut().set("x-generation", "0")?;
                    Ok(res)
                }
            },
            Method::Put => {
                let expected: i64 = req
                    .headers()
                    .get("x-generation")
                    .ok()
                    .flatten()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(-1);
                if expected < 0 {
                    return Response::error("x-generation is required on a write", 400);
                }
                let mut req = req;
                let bytes = req.bytes().await?;
                match self.put_image(&id, expected, &bytes).await? {
                    Some(next) => {
                        let mut res = Response::empty()?;
                        res.headers_mut().set("x-generation", &next.to_string())?;
                        Ok(res)
                    }
                    // The caller re-reads and replays. Same contract the D1
                    // generation guard had, and `hubstore`'s retry already
                    // speaks it.
                    None => Response::error("generation moved", 409),
                }
            }
            _ => Response::error("method not allowed", 405),
        }
    }

    /// What a client may say over its socket.
    ///
    /// DELIBERATELY SMALL. A socket is not a second API: everything that
    /// changes an order still goes through the Worker, where the kernel
    /// decides and the caller is authenticated per request. Two messages are
    /// accepted here and both are about the connection itself.
    ///
    ///   `{"t":"ping"}`  — keep-alive; answered with a pong.
    ///   `{"t":"gps", "courier":"…", "lat_e6":…, "lng_e6":…}` — a courier's
    ///   position, which lands in memory and in no table.
    ///
    /// A MESSAGE FROM A SOCKET IS NOT A PRINCIPAL. The tag the Worker attached
    /// at accept time is what this object knows about the client, so a GPS fix
    /// is taken only from a socket tagged `courier`; a console or a customer
    /// saying the same thing is ignored rather than believed.
    async fn websocket_message(
        &self,
        ws: WebSocket,
        message: WebSocketIncomingMessage,
    ) -> Result<()> {
        let WebSocketIncomingMessage::String(text) = message else {
            // Binary frames mean nothing here. Dropping them is the whole
            // handling: answering would teach a client to send more.
            return Ok(());
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { return Ok(()) };
        match v.get("t").and_then(serde_json::Value::as_str) {
            Some("ping") => {
                let _ = ws.send_with_str(r#"{"t":"pong"}"#);
            }
            Some("gps") => {
                // WHOSE POSITION THIS IS COMES FROM THE TAG. It used to come
                // from the frame, so one courier's socket could move another
                // courier's pin on the venue's map -- and through it every
                // customer's estimate. The tag was attached by the Worker from
                // a verified claim; the frame is whatever was typed into it.
                let Some(courier) = courier_of(&self.state.get_tags(&ws)) else {
                    return Ok(());
                };
                let (Some(lat), Some(lng)) = (
                    v.get("lat_e6").and_then(serde_json::Value::as_i64),
                    v.get("lng_e6").and_then(serde_json::Value::as_i64),
                ) else {
                    return Ok(());
                };
                // MICRO-DEGREES, as integers, exactly as an order carries them:
                // a float crossing into this system is what MANIFESTO C2
                // forbids, and a position is not an exception.
                if !(-90_000_000..=90_000_000).contains(&lat)
                    || !(-180_000_000..=180_000_000).contains(&lng)
                {
                    return Ok(());
                }
                self.positions.borrow_mut().insert(
                    courier,
                    Fix { lat_e6: lat, lng_e6: lng, at_ms: Date::now().as_millis() as i64 },
                );
            }
            _ => {}
        }
        Ok(())
    }

    /// A socket closing is the ordinary end of a socket. The runtime forgets
    /// it; there is nothing here to clean up, and the default handler would
    /// have panicked.
    async fn websocket_close(
        &self,
        _ws: WebSocket,
        _code: usize,
        _reason: String,
        _was_clean: bool,
    ) -> Result<()> {
        Ok(())
    }

    /// Same, for a socket that ended badly. Logged, because a storm of these
    /// is worth seeing, and not turned into an error that would take the
    /// object down with it.
    async fn websocket_error(&self, _ws: WebSocket, error: Error) -> Result<()> {
        console_log!("hub socket error: {error}");
        Ok(())
    }
}
