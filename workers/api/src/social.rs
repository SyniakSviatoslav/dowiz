//! Message threads — the transport for `dowiz_kernel::thread`.
//!
//! **This module decides nothing.** Ordering, idempotency, what a read mark may
//! claim and whether a message is well formed are the kernel's answers; here
//! there is SQL and HTTP.
//!
//! # Why polling and not a socket
//!
//! A thread is an append-only log ordered by a sender-assigned sequence, so a
//! reader only ever needs "everything after N". That is a GET, it survives a
//! tunnel and a locked phone, and it costs one D1 read. A socket would buy
//! lower latency for a conversation whose messages arrive minutes apart, at the
//! cost of a Durable Object per thread and a reconnect path that has to answer
//! the same "everything after N" question anyway.
//!
//! There is no presence here and no typing indicator, because a thread does not
//! know either: both are facts about a transport, and storing them would make
//! them wrong the moment a phone loses signal.

use serde::Deserialize;
use serde_json::json;
use worker::*;

use dowiz_kernel::thread::{self, Body, Message, Party, Thread};

use crate::owner::now_ms;

#[derive(Deserialize)]
struct MessageRow {
    id: String,
    from_party: String,
    seq: i64,
    kind: String,
    body: String,
    read_through: Option<i64>,
    sent_at_ms: i64,
}

/// Rebuild the kernel's `Thread` from rows.
///
/// A row the kernel refuses stops the load. A message log that will not replay
/// must not be shown as if it had — the alternative is a conversation that is
/// silently missing whichever line was malformed.
fn to_thread(rows: &[MessageRow]) -> std::result::Result<Thread, String> {
    let mut t = Thread::default();
    for r in rows {
        let from = Party::from_str(&r.from_party)
            .ok_or_else(|| format!("unknown party {:?}", r.from_party))?;
        let body = match r.kind.as_str() {
            "TEXT" => Body::Text(r.body.clone()),
            "READ" => Body::Read {
                through_seq: r.read_through.unwrap_or(0).max(0) as u64,
            },
            other => return Err(format!("unknown message kind {other:?}")),
        };
        let msg = Message {
            id: r.id.parse::<u64>().unwrap_or_else(|_| id64(&r.id)),
            from,
            seq: r.seq.max(0) as u64,
            sent_at_ms: r.sent_at_ms,
            body,
        };
        t = thread::append(t, msg).map_err(|e| e.message())?;
    }
    Ok(t)
}

fn id64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Bind an integer to D1.
///
/// NOT `i64::into()`. That produces a JavaScript **BigInt**, which the D1 driver
/// rejects — and rejects by throwing, so the Worker returns a bare 500 with no
/// body and nothing in the response says why. Every integer in this file goes
/// through here, as `accounts.rs` already does. Timestamps in milliseconds and
/// slot minutes are far below 2^53, so f64 carries them exactly.
fn num(n: i64) -> worker::wasm_bindgen::JsValue {
    worker::wasm_bindgen::JsValue::from_f64(n as f64)
}


async fn load(db: &D1Database, thread_id: &str, location: &str) -> Result<Vec<MessageRow>> {
    Ok(db
        .prepare(
            "SELECT id, from_party, seq, kind, body, read_through, sent_at_ms \
             FROM thread_messages WHERE thread_id = ?1 AND location_id = ?2 \
             ORDER BY seq ASC, from_party ASC",
        )
        .bind(&[thread_id.into(), location.into()])?
        .all()
        .await?
        .results()?)
}

/// `GET /api/public/locations/:slug/threads/:id?after=<seq>`
pub async fn messages(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let (Some(slug), Some(id)) = (ctx.param("slug").cloned(), ctx.param("id").cloned()) else {
        return Response::error("missing slug or id", 400);
    };
    let after: u64 = req
        .url()?
        .query_pairs()
        .find(|(k, _)| k == "after")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);

    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    // AUTHENTICATED, AND TO THIS VENUE. A thread holds what a customer and a
    // venue said to each other; the id was the only thing standing in front
    // of it.
    if let Err(r) =
        crate::auth::principal_at(&req, &ctx.env, &db, &place.venue, crate::owner::now_ms()).await
    {
        return Ok(r);
    }
    let rows = load(&db, &id, &place.venue).await?;

    let t = match to_thread(&rows) {
        Ok(t) => t,
        Err(why) => return Response::error(format!("thread unreadable: {why}"), 500),
    };

    let out: Vec<_> = t
        .messages
        .iter()
        .filter(|m| m.seq > after)
        .map(|m| {
            json!({
                "id": m.id.to_string(),
                "from": m.from.as_str(),
                "seq": m.seq,
                "sentAtMs": m.sent_at_ms,
                "kind": match &m.body { Body::Text(_) => "TEXT", Body::Read { .. } => "READ" },
                "body": match &m.body { Body::Text(s) => s.clone(), _ => String::new() },
                "readThrough": match &m.body {
                    Body::Read { through_seq } => Some(*through_seq), _ => None
                },
            })
        })
        .collect();

    Response::from_json(&json!({
        "messages": out,
        // What a caller sends next, so it never has to guess a sequence.
        "nextSeq": {
            "CUSTOMER": thread::next_seq(&t, Party::Customer),
            "VENUE":    thread::next_seq(&t, Party::Venue),
            "COURIER":  thread::next_seq(&t, Party::Courier),
        },
        "unread": {
            "CUSTOMER": thread::unread_for(&t, Party::Customer),
            "VENUE":    thread::unread_for(&t, Party::Venue),
            "COURIER":  thread::unread_for(&t, Party::Courier),
        },
    }))
}

#[derive(Deserialize)]
struct SendBody {
    from: String,
    /// TEXT | READ
    #[serde(default = "default_kind")]
    kind: String,
    #[serde(default)]
    body: String,
    #[serde(default, rename = "readThrough")]
    read_through: Option<u64>,
    /// The caller's idempotency key. Resending the same one is a no-op.
    #[serde(rename = "clientId")]
    client_id: String,
}

fn default_kind() -> String {
    "TEXT".into()
}

/// `POST /api/public/locations/:slug/threads/:id/messages`
pub async fn send(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let (Some(slug), Some(thread_id)) =
        (ctx.param("slug").cloned(), ctx.param("id").cloned())
    else {
        return Response::error("missing slug or id", 400);
    };
    let b: SendBody = match req.json().await {
        Ok(v) => v,
        Err(e) => return Response::error(format!("bad request: {e}"), 400),
    };
    if b.client_id.trim().is_empty() {
        return Response::error("clientId is required", 400);
    }

    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    // WHO IS SPEAKING COMES FROM THE TOKEN, not from the body. `from` was a
    // string the caller chose, so anyone with a thread id could post as the
    // venue -- or as the customer -- and the message would look exactly like
    // the real one. The body's `from` is now ignored; a role decides.
    let from = match crate::auth::principal_at(&req, &ctx.env, &db, &place.venue, crate::owner::now_ms()).await {
        Ok(crate::auth::Principal::Owner { .. }) => Party::Venue,
        Ok(crate::auth::Principal::Customer { .. }) => Party::Customer,
        Ok(crate::auth::Principal::Courier { .. }) => {
            return Response::error("a courier does not speak in this thread", 403)
        }
        Err(r) => return Ok(r),
    };
    let _ = &b.from;

    // The thread must exist and belong to this venue. Creating one implicitly
    // would let any caller open a conversation inside somebody else's hub.
    let exists: Option<serde_json::Value> = db
        .prepare("SELECT id FROM threads WHERE id = ?1 AND location_id = ?2")
        .bind(&[thread_id.clone().into(), place.venue.clone().into()])?
        .first(None)
        .await?;
    if exists.is_none() {
        return Response::error("not found", 404);
    }

    let rows = load(&db, &thread_id, &place.venue).await?;
    let t = match to_thread(&rows) {
        Ok(t) => t,
        Err(why) => return Response::error(format!("thread unreadable: {why}"), 500),
    };

    let msg_id = format!("msg_{:016x}", id64(&format!("{thread_id}:{}", b.client_id)));

    // ── IDEMPOTENCY IS CHECKED FIRST ──
    // A resend carries a NEW `sent_at_ms`, so it is not byte-for-byte the
    // message already stored and the kernel rightly refuses it as a different
    // message under a used id. Asking the database before asking the kernel is
    // what makes a retry free. Found by a probe against production, where the
    // second send answered "already used by a different message".
    let already: Option<serde_json::Value> = db
        .prepare("SELECT id FROM thread_messages WHERE id = ?1")
        .bind(&[msg_id.clone().into()])?
        .first(None)
        .await?;
    if already.is_some() {
        return Response::from_json(&json!({ "id": msg_id, "replayed": true }));
    }

    let now = now_ms();
    let candidate = Message {
        id: id64(&msg_id),
        from,
        seq: thread::next_seq(&t, from),
        sent_at_ms: now,
        body: match b.kind.as_str() {
            "READ" => Body::Read {
                through_seq: b.read_through.unwrap_or(0),
            },
            _ => Body::Text(b.body.clone()),
        },
    };

    // ── THE DECISION IS THE KERNEL'S ──
    // Append against the loaded thread first. If it refuses, nothing is written.
    let appended = match thread::append(t.clone(), candidate.clone()) {
        Ok(next) => next,
        Err(e) => return Response::error(e.message(), 422),
    };

    db.prepare(
        "INSERT INTO thread_messages (id, thread_id, location_id, from_party, seq, kind, body, \
         read_through, sent_at_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
    )
    .bind(&[
        msg_id.clone().into(),
        thread_id.into(),
        place.venue.clone().into(),
        from.as_str().into(),
        num(candidate.seq as i64),
        match candidate.body {
            Body::Read { .. } => "READ",
            _ => "TEXT",
        }
        .into(),
        b.body.into(),
        match candidate.body {
            Body::Read { through_seq } => num(through_seq as i64),
            _ => JsValue::NULL,
        },
        num(now),
    ])?
    .run()
    .await?;

    Response::from_json(&json!({
        "id": msg_id,
        "seq": candidate.seq,
        "replayed": false,
        "unreadForOther": thread::unread_for(&appended, Party::Venue),
    }))
}

use worker::wasm_bindgen::JsValue;
