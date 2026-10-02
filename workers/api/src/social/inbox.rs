//! THE VENUE'S SIDE OF THE ORDER CHATS (W-WIRE row 1): every thread this
//! venue has, newest first, with what the venue has not read yet.
//!
//! The customer writes from the kit (`kit/screens/chat.js`); until this list
//! existed no venue screen could find those threads -- a thread is keyed by
//! its order id, and nothing enumerated them. One read of the threads image,
//! folded per thread by the KERNEL (`to_thread`, `thread::unread_for`), so the
//! unread number is the same one the thread route answers.
//!
//! `GET /api/owner/threads` -- the owner, or staff who work orders (read-only,
//! `party::thread_party`); a customer or a courier is refused.

use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use super::{party, to_thread, MessageRow, IMAGE_THREADS, K_MSG};
use dowiz_kernel::thread::{self, Body, Party};

/// One row per thread, newest activity first. PURE: `(thread id, row)` in.
/// A thread the kernel will not replay is listed with its error rather than
/// dropped -- a conversation that silently vanishes is the defect.
fn summaries(rows: Vec<(String, MessageRow)>) -> Vec<Value> {
    let mut by: Vec<(String, Vec<MessageRow>)> = Vec::new();
    for (id, r) in rows {
        match by.iter_mut().find(|(k, _)| *k == id) {
            Some((_, v)) => v.push(r),
            None => by.push((id, vec![r])),
        }
    }
    let mut out: Vec<(i64, Value)> = by
        .into_iter()
        .map(|(id, mut v)| {
            v.sort_by(|a, b| a.seq.cmp(&b.seq).then(a.from_party.cmp(&b.from_party)));
            let at = v.iter().map(|r| r.sent_at_ms).max().unwrap_or(0);
            let row = match to_thread(&v) {
                Ok(t) => {
                    let last = t.messages.iter().rev().find_map(|m| match &m.body {
                        Body::Text(s) => Some(json!({ "from": m.from.as_str(), "body": s, "sentAtMs": m.sent_at_ms })),
                        _ => None,
                    });
                    json!({
                        "id": id,
                        "last": last,
                        "atMs": at,
                        "unread": thread::unread_for(&t, Party::Venue),
                        "count": t.messages.iter().filter(|m| matches!(m.body, Body::Text(_))).count(),
                    })
                }
                Err(why) => json!({ "id": id, "atMs": at, "error": why, "unread": 0, "count": 0 }),
            };
            (at, row)
        })
        .collect();
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out.into_iter().map(|(_, v)| v).collect()
}

/// `GET /api/owner/threads`
pub async fn list(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let venue = match crate::owner::location_of(&req) {
        Some(v) => v,
        None => crate::hubstore::Place::of_any(&req, &ctx).await?.venue,
    };
    let p = match crate::auth::principal_at(&req, &ctx.env, &venue, ctx.data.now_ms).await {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    // Reading as the VENUE: the owner, or staff who work orders.
    if let Err((code, why)) = party::thread_party(&p, "", false).and_then(|side| match side {
        Party::Venue => Ok(side),
        _ => Err((403, "only the venue reads its conversations")),
    }) {
        return Response::error(why, code);
    }
    let place = crate::hubstore::Place::of_authorised(&ctx, &venue)?;
    let rows: Vec<(String, MessageRow)> = crate::hubstore::load_log(&place, IMAGE_THREADS)
        .await?
        .log
        .about(K_MSG, None, usize::MAX)
        .into_iter()
        .filter_map(|e| serde_json::from_str::<MessageRow>(&e.json).ok().map(|r| (e.subject, r)))
        .collect();
    let threads = summaries(rows);
    let unread: u64 = threads.iter().filter_map(|t| t["unread"].as_u64()).sum();
    Response::from_json(&json!({ "threads": threads, "unread": unread }))
}

#[cfg(test)]
#[path = "inbox/tests.rs"]
mod tests;
