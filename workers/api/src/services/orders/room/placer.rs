//! WHO PLACED A ROUND, when it was placed by somebody in the room.
//!
//! `storefront::place` is the one placement path, and it stays public: a guest
//! at a table can still order from the storefront. What this adds is the
//! waiter's round (BLUEPRINT-POS-THE-ROOM §2.3): the same command, the same
//! pricer, the same stock reservation — plus a SIGNER and a SITTING, which are
//! the two things every later room event names.
//!
//! A TOKEN THAT IS PRESENT IS NEVER IGNORED WHEN IT IS A ROOM TOKEN. A staff or
//! owner token that cannot place here — another venue, no `take_orders`, a
//! delivery basket — refuses the placement rather than quietly placing it as an
//! anonymous guest's, because an unsigned round is exactly the record the room
//! exists to stop writing. Any OTHER token (a customer's, from an earlier
//! order) is none of this module's business and is left alone, as before.

use crate::auth::{self, Cap, Claims};
use worker::*;

/// A round placed by a person in the room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Staffed {
    /// The signer: a member of staff or the venue's owner.
    pub by: String,
    /// The check this round belongs to. Minted when the table is opened.
    pub sitting_id: String,
}

/// A sitting id the client sent: the Worker's own ids are UUIDs, so anything
/// outside `[A-Za-z0-9_-]{8,64}` was not minted here and is refused rather than
/// stored in every round's envelope.
pub fn sitting_id_ok(s: &str) -> bool {
    (8..=64).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// The pure half of the decision, once the signer is known: a waiter places
/// rounds IN THE ROOM, and a round names a sitting — the one it was sent with,
/// or a new one when the table is being opened.
pub fn staffed(
    by: String,
    kind: &str,
    sent: Option<&str>,
    minted: impl FnOnce() -> Option<String>,
) -> std::result::Result<Staffed, (u16, &'static str)> {
    if kind != "dine_in" {
        return Err((403, "a room token places rounds at a table, not deliveries"));
    }
    let sitting_id = match sent.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) if sitting_id_ok(s) => s.to_string(),
        Some(_) => return Err((400, "that sitting id was not minted here")),
        None => minted().ok_or((500, "no platform CSPRNG"))?,
    };
    Ok(Staffed { by, sitting_id })
}

/// The door, over a live request. `Ok(None)` is an anonymous placement, as the
/// storefront has always had.
pub async fn placer(
    req: &Request,
    ctx: &RouteContext<crate::Req>,
    venue: &str,
    kind: &str,
    sent_sitting: Option<&str>,
) -> std::result::Result<Option<Staffed>, Response> {
    let Ok(raw) = auth::bearer(req) else { return Ok(None) };
    let room = matches!(
        auth::verify(&ctx.env, &raw, ctx.data.now_ms),
        Ok(Claims::Staff { .. } | Claims::Owner { .. })
    );
    if !room {
        return Ok(None);
    }
    let (by, _) = crate::courier::staff_at(req, ctx, venue, Cap::TakeOrders).await?;
    staffed(by, kind, sent_sitting, crate::edge_id)
        .map(Some)
        .map_err(|(s, m)| Response::error(m, s).unwrap())
}

// ── THE GUEST AT A TABLE (A9) ───────────────────────────────────────────────

/// Who a guest's round names as its signer. Not a person: the round is born
/// PENDING and a member of staff confirms it (`guest_confirm`).
pub const GUEST: &str = "guest";

/// A round a guest placed from a table's code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guest {
    /// The live sitting at that table, or a new one.
    pub sitting_id: String,
    /// `zone:n`, from the SIGNED code — never from the basket.
    pub table: String,
}

/// The pure half. `table` is the verified `(zone, n)` or `None` when the basket
/// carried no code; `live` is the open sitting at that table, if any.
///
/// A GUEST NEVER NAMES A SITTING. Only the signed table decides which check a
/// round joins; a `sitting_id` in a guest's basket would let anyone who saw one
/// add rounds to a stranger's bill, so it is refused, code or no code.
pub fn guest_seat(
    kind: &str,
    sent_sitting: Option<&str>,
    table: Option<(String, i64)>,
    live: Option<String>,
    minted: impl FnOnce() -> Option<String>,
) -> std::result::Result<Option<Guest>, (u16, &'static str)> {
    if sent_sitting.is_some_and(|s| !s.trim().is_empty()) {
        return Err((400, "a guest cannot choose a check; the code on the table decides it"));
    }
    let Some((zone, n)) = table else { return Ok(None) };
    if kind != "dine_in" {
        return Err((400, "a table code orders to the table; the fulfilment must be dine_in"));
    }
    let sitting_id = match live {
        Some(s) => s,
        None => minted().ok_or((500, "no platform CSPRNG"))?,
    };
    Ok(Some(Guest { sitting_id, table: super::table_link::table_text(&zone, n) }))
}

/// The door for a guest, over a live request. The PLAN is the venue record's
/// (`loc_json`, already loaded by the caller); the sittings are the venue's own
/// order log. Nothing about the table or the check is taken from the client
/// except the signed code.
pub async fn guest(
    ctx: &RouteContext<crate::Req>,
    place: &crate::hubstore::Place,
    loc_id: &str,
    loc_json: &str,
    kind: &str,
    code: Option<&str>,
    sent_sitting: Option<&str>,
) -> std::result::Result<Option<Guest>, Response> {
    let refuse = |m: &str, s: u16| Response::error(m, s).unwrap();
    let code = code.map(str::trim).filter(|c| !c.is_empty());
    let table = match code {
        None => None,
        Some(c) => {
            let key = auth::signing_key(&ctx.env);
            if key.is_empty() {
                return Err(refuse("table codes are not configured", 503));
            }
            let plan = plan_of(loc_json);
            match super::table_link::verify_table(&key, loc_id, &plan, c) {
                Ok(t) => Some((t, plan)),
                Err(why) => return Err(refuse(why.message(), 400)),
            }
        }
    };
    // Only read the log when there IS a table: a delivery pays nothing for this.
    let live = match &table {
        Some(((zone, n), plan)) => {
            let listed: Vec<_> = crate::hubstore::orders(place)
                .await
                .map_err(|e| refuse(&e.to_string(), 500))?
                .into_iter()
                .filter(|o| {
                    serde_json::from_str::<serde_json::Value>(&o.order_json)
                        .is_ok_and(|v| v.get("location_id").and_then(|l| l.as_str()) == Some(loc_id))
                })
                .collect();
            super::table_link::live_sitting(plan, &listed, zone, *n)
        }
        None => None,
    };
    guest_seat(kind, sent_sitting, table.map(|(t, _)| t), live, crate::edge_id).map_err(|(s, m)| refuse(m, s))
}

/// The owner's plan out of the venue record (`booking::set_plan` stores it as
/// `floor_plan`). One that will not parse is EMPTY, so every code is refused
/// as naming no table rather than guessed at.
pub fn plan_of(loc_json: &str) -> dowiz_hub::tables::Plan {
    let raw = serde_json::from_str::<serde_json::Value>(loc_json)
        .ok()
        .and_then(|v| v.get("floor_plan").map(|p| p.to_string()))
        .unwrap_or_default();
    dowiz_hub::tables::from_json(&raw).unwrap_or_default()
}
