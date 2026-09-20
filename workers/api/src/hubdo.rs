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

/// What a stored image is, apart from its bytes.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
struct Meta {
    generation: i64,
    chunks: usize,
    len: usize,
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
        let Some(meta) = store.get::<Meta>(&Self::meta_key(id)).await.ok().flatten() else {
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
            Some(next) => Ok(Some((next, len))),
            // Cannot happen inside a serialised object -- the generation was
            // read two lines ago -- but the caller's contract already says
            // what to do about it, so say it rather than assume.
            None => Ok(None),
        }
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
        if let Some((old, _)) = self.mem.borrow().get(id) {
            for n in chunks..old.chunks {
                let _ = store.delete(&Self::chunk_key(id, n)).await;
            }
        }
        self.mem.borrow_mut().insert(id.to_string(), (meta, bytes.to_vec()));
        // THE PROJECTION IS DERIVED FROM THIS IMAGE, so it dies with the write
        // that replaced it. Keyed by generation it could only ever be stale
        // for a moment; dropped here it cannot be stale at all.
        if id == LOG_IMAGE {
            *self.folded.borrow_mut() = None;
        }
        Ok(Some(next))
    }
}

impl DurableObject for HubImages {
    fn new(state: State, _env: Env) -> Self {
        Self { state, mem: RefCell::new(HashMap::new()), folded: RefCell::new(None) }
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
}
