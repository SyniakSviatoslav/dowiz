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

/// The venue's conversations with its customers. An append log: a message
/// arrives and is replayed in order, which is exactly what `ORDER BY seq ASC`
/// was asking a table to pretend to be.
pub const IMAGE_THREADS: &str = "threads";
const K_MSG: &str = "m";

/// One thread's messages, oldest first, replayed for the kernel.
///
/// `location_id` is gone from the question because the image IS the venue. The
/// old query carried it in a `WHERE` clause that somebody had to remember, and
/// the table it read has no venue of its own — which is the shape six of this
/// platform's defects had.
async fn load(place: &crate::hubstore::Place, thread_id: &str) -> Result<Vec<MessageRow>> {
    let mut rows: Vec<MessageRow> = crate::hubstore::load_log(place, IMAGE_THREADS)
        .await?
        .log
        .about(K_MSG, Some(thread_id), usize::MAX)
        .into_iter()
        .filter_map(|e| serde_json::from_str::<MessageRow>(&e.json).ok())
        .collect();
    // `about` is newest first; the kernel replays oldest first. The secondary
    // sort on the party is kept because the old `ORDER BY seq ASC, from_party
    // ASC` had it: the two sides number their own messages, so a seq alone is
    // not a total order and an unstable one would replay differently on two
    // reads of the same log.
    rows.sort_by(|a, b| a.seq.cmp(&b.seq).then(a.from_party.cmp(&b.from_party)));
    Ok(rows)
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
    let rows = load(&place, &id).await?;

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

    // ── THE THREAD MUST BE ABOUT SOMETHING OF THIS VENUE'S ──
    //
    // This read `SELECT id FROM threads WHERE id = ?1 AND location_id = ?2` and
    // NOTHING IN THIS CODEBASE EVER INSERTED INTO `threads`. The table is empty
    // in production — verified, zero rows — so every send has answered 404 for
    // as long as the route has existed. The check was not protecting the
    // feature; it was the feature's only gate and it was shut.
    //
    // What the check was FOR was that a caller cannot open a conversation
    // inside somebody else's hub. In the venue's own image that is structural:
    // the bytes are this venue's object. What is still worth refusing is a
    // thread id that names nothing, so a typo does not silently create a
    // conversation nobody will ever find — so a thread is valid when it already
    // has messages, or when its id is an ORDER in this venue's log.
    let rows = load(&place, &thread_id).await?;
    if rows.is_empty() {
        let known = crate::hubstore::orders(&place)
            .await?
            .into_iter()
            .any(|o| o.order_id == thread_id);
        if !known {
            return Response::error("not found", 404);
        }
    }
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
    if rows.iter().any(|r| r.id == msg_id) {
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

    let stored = json!({
        "id": msg_id,
        "from_party": from.as_str(),
        "seq": candidate.seq as i64,
        "kind": match candidate.body { Body::Read { .. } => "READ", _ => "TEXT" },
        "body": b.body,
        "read_through": match candidate.body {
            Body::Read { through_seq } => json!(through_seq as i64),
            _ => serde_json::Value::Null,
        },
        "sent_at_ms": now,
    })
    .to_string();
    let subject = thread_id.clone();
    crate::hubstore::with_log(&place, IMAGE_THREADS, move |log| {
        log.append(K_MSG, &subject, &stored)
            .map_err(|e| Error::RustError(format!("thread: {e:?}")))
    })
    .await?;

    Response::from_json(&json!({
        "id": msg_id,
        "seq": candidate.seq,
        "replayed": false,
        "unreadForOther": thread::unread_for(&appended, Party::Venue),
    }))
}

