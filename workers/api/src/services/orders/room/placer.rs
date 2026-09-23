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
