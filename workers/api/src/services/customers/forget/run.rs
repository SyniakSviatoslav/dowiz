//! THE WORKER'S HALF OF A FORGET, shared by the owner's route and by the
//! replay after a restore (P3). ORCHESTRATION: every rule it applies is a pure
//! function of `forget`, `forget::queued`, `forget::register`,
//! `booking::forget` or `alias`, tested beside its file.
//!
//! WHO, IN FOUR READS: the `people` image (the alias circle, G8), the hot
//! fold, each archive's fold, and the bookings image. The secret the keys are
//! HMACs under is here and not in the object, which is why these reads are
//! the Worker's; WHAT IS DONE happens in the object's one turn
//! (`hubdo/forget.rs`).

use std::collections::BTreeSet;

use worker::*;

use super::{find_any, legacy_ids, register, Found};
use crate::hubdo::forget::{ForgetIn, ForgetOut};
use crate::hubstore::Place;
use crate::services::customers::alias::Aliases;

/// Everything the object is told, found here.
pub struct Scope {
    /// The alias circle: every key the person is filed under.
    pub keys: BTreeSet<String>,
    pub found: Found,
    /// The archives holding any of their orders.
    pub archives: Vec<String>,
    /// Their reservations (`booking::forget::ids_of`).
    pub bookings: Vec<String>,
}

/// PURE. The circle `key` belongs to, plus keys an earlier erasure recorded
/// (after a restore the `alias` rows may be gone, the register's are not).
pub fn circle(aliases: &Aliases, key: &str, also: &BTreeSet<String>) -> BTreeSet<String> {
    let mut keys: BTreeSet<String> = aliases.circle(key, None).into_iter().collect();
    keys.extend(also.iter().cloned());
    keys
}

/// Find the person: their circle, their orders hot and archived, their bookings.
pub async fn scope(place: &Place, loc: &str, key: &str, also: &BTreeSet<String>, secret: &[u8]) -> Result<Scope> {
    use crate::hubstore::{IMAGE_PEOPLE, PEOPLE_BYTES};
    let people = crate::hubstore::load_table(place, IMAGE_PEOPLE, PEOPLE_BYTES).await?;
    let keys = circle(&Aliases::of(&people.table), key, also);
    let mut found = Found::default();
    find_any(&crate::hubstore::orders(place).await?, loc, &keys, secret, &mut found);
    let mut archives = Vec::new();
    for id in crate::hubstore::archives_of(&crate::hubstore::load_settings(place).await?.settings) {
        let before = found.orders.len();
        let views = crate::hubstore::archive_orders(place, &id).await?.unwrap_or_default();
        find_any(&views, loc, &keys, secret, &mut found);
        if found.orders.len() > before {
            archives.push(id);
        }
    }
    let bookings = crate::hubstore::load_table(place, crate::booking::IMAGE_BOOKINGS, crate::booking::BOOKINGS_BYTES).await?;
    let bookings = crate::booking::forget::ids_of(&bookings.table, &keys, |p| crate::booking::forget::keys_of_phone(secret, p));
    Ok(Scope { keys, found, archives, bookings })
}

/// PURE. The object's input. `also_orders` are orders an earlier erasure
/// found (the register's), joined so a replay reaches them even where no fold
/// could match a phone any more.
pub fn input(s: Scope, key: &str, reason: &str, by: &str, now_ms: i64, also_orders: &BTreeSet<String>) -> ForgetIn {
    ForgetIn {
        key: key.to_string(),
        reason: reason.to_string(),
        by: by.to_string(),
        now_ms,
        orders: s.found.orders.iter().chain(also_orders.iter()).cloned().collect::<BTreeSet<_>>().into_iter().collect(),
        archives: s.archives,
        legacy: legacy_ids(&s.found.phones),
        keys: s.keys.into_iter().collect(),
        bookings: s.bookings,
    }
}

/// Ask the object. A refusal is loud and answered with its status.
pub async fn send(place: &Place, input: &ForgetIn) -> std::result::Result<ForgetOut, (u16, String)> {
    match crate::command::send(place, "forget", input).await {
        Ok(out) => Ok(out),
        Err((status, msg)) => {
            crate::loud!(&place.ns, Some(&place.venue), "customers.forget", "cust:{} not forgotten ({status}): {msg}", input.key);
            Err((status, msg))
        }
    }
}

/// What a replay did: entries replayed, records redacted, and the entries
/// that could not be read or failed (each named in the venue's error log).
#[derive(Debug, Default, serde::Serialize)]
pub struct Replayed {
    pub entries: usize,
    pub redacted: usize,
    pub declared: usize,
    pub failed: usize,
}

/// P3: RE-FORGET EVERYBODY THE REGISTER NAMES FOR THIS VENUE. Called after a
/// restore; no live-order refusal (a restored order "on its way" is a copy of
/// the past, and the person asked). One command per person, each idempotent.
pub async fn replay(env: &Env, place: &Place, loc: &str, secret: &[u8], now_ms: i64) -> Result<Replayed> {
    let (entries, unreadable) = register::of(env, loc).await?;
    let mut out = Replayed { failed: unreadable, ..Replayed::default() };
    if unreadable > 0 {
        crate::loud!(&place.ns, Some(&place.venue), "customers.reforget", "{unreadable} erasure register records unreadable");
    }
    for e in entries {
        let s = scope(place, loc, &e.key, &e.keys, secret).await?;
        let input = input(s, &e.key, "re-applied after a restore", "restore", now_ms, &e.orders);
        match send(place, &input).await {
            Ok(o) => {
                out.entries += 1;
                out.redacted += o.redacted;
                out.declared += o.declared;
            }
            Err(_) => out.failed += 1,
        }
    }
    Ok(out)
}

/// P3: A RESTORE MUST NOT BRING A FORGOTTEN PERSON BACK. The bundle may
/// predate an erasure, so every erasure the register holds for this venue is
/// applied again before the owner is told it worked (`operations::restore`).
/// A replay that fails is a 500 that says so, never a quiet "restored".
pub async fn after_restore(ctx: &RouteContext<crate::Req>, place: &Place, loc: &str, written: Vec<String>) -> Result<Response> {
    let secret = crate::services::customers::handlers::signing_secret(&ctx.env);
    match replay(&ctx.env, place, loc, &secret, ctx.data.now_ms).await {
        Ok(r) if r.failed == 0 => Response::from_json(&serde_json::json!({ "restored": written, "reforgotten": r })),
        Ok(r) => Response::error(
            format!("restored, but {} of the venue's erasures could not be applied again; the error log names them", r.failed),
            500,
        ),
        Err(e) => {
            crate::loud!(&place.ns, Some(&place.venue), "customers.reforget", "after restore: {e}");
            Response::error(format!("restored, but the erasures could not be applied again: {e}"), 500)
        }
    }
}

/// `POST /api/owner/customers/reforget` -- the SAME replay, by hand (P3).
///
/// A restore through `/api/owner/restore` replays by itself. Cloudflare's
/// point-in-time recovery of the venue's object does not pass through this
/// Worker, so the PITR runbook's last step is this call: owner-only, the
/// venue from the caller's authority, and idempotent (a second call finds the
/// log already redacted and declares nothing new).
pub async fn reforget(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = Place::of_authorised(&ctx, &loc)?;
    let secret = crate::services::customers::handlers::signing_secret(&ctx.env);
    match replay(&ctx.env, &place, &loc, &secret, ctx.data.now_ms).await {
        Ok(r) if r.failed == 0 => Response::from_json(&serde_json::json!({ "reforgotten": r })),
        Ok(r) => Response::error(format!("{} of the venue's erasures could not be applied again; the error log names them", r.failed), 500),
        Err(e) => {
            crate::loud!(&place.ns, Some(&place.venue), "customers.reforget", "by hand: {e}");
            Response::error(format!("the erasures could not be applied again: {e}"), 500)
        }
    }
}
