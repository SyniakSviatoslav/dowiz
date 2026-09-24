//! The owner's campaign routes. ORCHESTRATION ONLY: the segment, the join,
//! the plan and the report are pure and tested beside their files.
//!
//! THE VENUE IS THE AUTHORISED ONE and every fact is loaded HERE: the orders,
//! the cards, the consent log, the campaign log and the outbox are read
//! server-side on each request. The client sends a campaign id and, to send,
//! a confirmation -- never a recipient, a count or a consent.
//!
//! NOTHING IS SENT FROM A REQUEST. `send` only QUEUES; the minute cron's
//! drain delivers, and asks the consent fold again first (G4).

use serde::Deserialize;
use serde_json::json;
use worker::*;

use super::audience::{recipients, Recipient};
use super::campaign::{self, DefIn, IMAGE_CAMPAIGN, KIND_DEF};
use super::segment::Now;
use super::send;
use crate::hubstore::Place;
use crate::owner::owner_and_venue;
use crate::services::customers::alias::Aliases;
use crate::services::customers::handlers::{customer_key, signing_secret};
use crate::services::customers::record::{KIND as CARD, TAGS};
use crate::services::orders::mine::of_venue as orders_of;

fn bad(why: impl Into<String>, code: u16) -> Result<Response> {
    Response::error(why.into(), code)
}

/// `GET /api/owner/campaigns` — every campaign, newest first, with how many
/// it has reached, and the closed lists the form offers.
pub async fn list(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = Place::of_any(&req, &ctx).await?;
    let (_, _, loaded) =
        match crate::owner::owner_beside(&req, &ctx, &place, crate::hubstore::load_log(&place, IMAGE_CAMPAIGN)).await {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
    let entries = loaded.log.entries();
    let out: Vec<_> = campaign::defs(&entries)
        .into_iter()
        .map(|d| {
            let sent = campaign::sent_to(&entries, &d.id).len();
            json!({ "campaign": d, "sent": sent })
        })
        .collect();
    Response::from_json(&json!({
        "campaigns": out,
        "segments": ["everyone_consented", "not_seen_since", "tag", "birthday_this_week"],
        "tags": TAGS,
    }))
}

/// `POST /api/owner/campaigns` — define a campaign, or edit one not yet sent.
pub async fn define(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: DefIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return bad(format!("bad request body: {e}"), 400),
    };
    let (who, loc) = match owner_and_venue(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = Place::of_authorised(&ctx, &loc)?;
    let now = ctx.data.now_ms;
    // THE CHECK READS THE LOG IT WRITES, under its guard: an edit racing the
    // first send cannot slip in after the `sent` row lands.
    let filed = crate::hubstore::with_log(&place, IMAGE_CAMPAIGN, move |log| {
        let def = match campaign::define(body.clone(), &log.entries(), &who, now) {
            Ok(d) => d,
            Err(why) => return Ok(Err(why)),
        };
        let rec = serde_json::to_string(&def).map_err(|e| Error::RustError(e.to_string()))?;
        log.append(KIND_DEF, &def.id, &rec).map_err(|e| Error::RustError(format!("{e:?}")))?;
        Ok(Ok(def))
    })
    .await?;
    match filed {
        Ok(def) => Response::from_json(&json!({ "campaign": def })),
        Err(why) => bad(why, 400),
    }
}

/// Everything a preview and a send are computed from, read together.
struct Loaded {
    def: campaign::Def,
    camp: Vec<dowiz_hub::logimage::Entry>,
    people: Vec<Recipient>,
}

async fn load(place: &Place, ctx: &RouteContext<crate::Req>, loc: &str, id: &str) -> Result<Option<Loaded>> {
    let (listed, people, consent, camp, venue) = futures_util::future::try_join5(
        crate::hubstore::orders(place),
        crate::hubstore::load_table(place, crate::hubstore::IMAGE_PEOPLE, crate::hubstore::PEOPLE_BYTES),
        crate::hubstore::load_log(place, crate::services::customers::consent_log::IMAGE_CONSENT),
        crate::hubstore::load_log(place, IMAGE_CAMPAIGN),
        crate::hubstore::venue_record(place),
    )
    .await?;
    let camp = camp.log.entries();
    let Some(def) = campaign::def_of(&camp, id) else { return Ok(None) };
    let secret = signing_secret(&ctx.env);
    let aliases = Aliases::of(&people.table);
    let zone = crate::hubstore::zone_of(venue.as_ref());
    let utc = ctx.data.now_ms;
    let now = Now { utc_ms: utc, local_ms: dowiz_hub::tz::local_ms(zone, utc) };
    let people = recipients(
        &orders_of(listed, loc),
        |phone| customer_key(&secret, phone),
        |k| aliases.resolve(k),
        |k| aliases.members(k),
        |k| people.table.get(CARD, k),
        &consent.log.entries(),
        &def.segment,
        now,
    );
    Ok(Some(Loaded { def, camp, people }))
}

fn param(ctx: &RouteContext<crate::Req>) -> Option<String> {
    ctx.param("id").cloned().filter(|id| id.len() <= 24 && id.starts_with('c'))
}

/// `POST /api/owner/campaigns/:id/preview` — the COUNT and the cost, before
/// anything is queued. The count is `recipients(...).len()`, the same list a
/// send would queue.
pub async fn preview(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let (_, loc) = match owner_and_venue(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = Place::of_authorised(&ctx, &loc)?;
    let Some(id) = param(&ctx) else { return bad("no such campaign", 404) };
    let Some(l) = load(&place, &ctx, &loc, &id).await? else { return bad("no such campaign", 404) };
    // NO TEMPLATE, NO PREVIEW: a count for a campaign WhatsApp would refuse
    // is a number that invites a press of "send". Refused with the fix.
    let tpl = match super::template::required(&l.def) {
        Ok(t) => t,
        Err(why) => return bad(why, 409),
    };
    let already = campaign::sent_to(&l.camp, &id);
    let again = l.people.iter().filter(|r| already.contains(&r.key)).count();
    Response::from_json(&json!({
        "preview": campaign::preview(l.people.len(), again),
        "template": tpl,
        "sample": l.def.text,
        // The approved template carries the way out; the consent sentence
        // promised it in these words.
        "stop": send::stop_line("sq"),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SendIn {
    confirm: bool,
}

/// `POST /api/owner/campaigns/:id/send` — the owner pressed "send". Queues one
/// outbox entry per consented recipient not already reached, within budget.
pub async fn send_now(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let ok = matches!(req.json::<SendIn>().await, Ok(SendIn { confirm: true }));
    if !ok {
        return bad("a send needs {\"confirm\": true}", 400);
    }
    let (_, loc) = match owner_and_venue(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = Place::of_authorised(&ctx, &loc)?;
    let Some(id) = param(&ctx) else { return bad("no such campaign", 404) };
    let Some(l) = load(&place, &ctx, &loc, &id).await? else { return bad("no such campaign", 404) };
    let waiting = crate::outbox::waiting(&place).await?;
    let already = campaign::sent_to(&l.camp, &id);
    let plan = match send::plan(&l.def, &l.people, &already, &waiting, ctx.data.now_ms) {
        Ok(p) => p,
        Err(why) => return bad(why, 409),
    };

    // 1. THE OUTBOX: entries and their marks in ONE write (`send::admit`).
    //    A message owed is written before the history says it was sent.
    let queue = std::sync::Arc::new(plan.queue);
    let q = queue.clone();
    let (fresh, marked) = crate::hubstore::with_table(&place, crate::outbox::IMAGE_OUTBOX, crate::outbox::OUTBOX_BYTES, move |t| {
        send::admit(t, &q).map_err(Error::RustError)
    })
    .await?;

    // 2. THE HISTORY. A failure here is retried by pressing send again: the
    //    marks make that press queue nobody twice (`send::admit`).
    let (q, cid) = (queue.clone(), id.clone());
    let wrote = crate::hubstore::with_log(&place, IMAGE_CAMPAIGN, move |log| {
        send::file_history(log, &cid, &q, &marked).map_err(Error::RustError)?;
        Ok(campaign::sent_entries(&log.entries(), &cid))
    })
    .await;
    let filed = match wrote {
        Ok(f) => f,
        Err(e) => {
            crate::loud!(&place.ns, Some(&place.venue), "campaign.send", "{id}: queued but history not written: {e}");
            return bad("messages are queued but their history was not written; pressing send again is safe and completes it", 500);
        }
    };
    // 3. THE MARKS GO once their rows are filed. Best effort: a failure leaves
    //    marks the next press drops.
    let cid = id.clone();
    if let Err(e) = crate::hubstore::with_table(&place, crate::outbox::IMAGE_OUTBOX, crate::outbox::OUTBOX_BYTES, move |t| {
        Ok(send::prune(t, &cid, &filed))
    })
    .await
    {
        console_error!("campaign.send {id}: marks not pruned yet: {e}");
    }
    Response::from_json(&json!({ "queued": fresh, "already": plan.already, "left": plan.left }))
}

/// `GET /api/owner/campaigns/:id` — the campaign and its report: queued,
/// waiting, withdrawn, abandoned, delivered, and orders placed with its code.
pub async fn report(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = Place::of_any(&req, &ctx).await?;
    let work = futures_util::future::try_join3(
        crate::hubstore::load_log(&place, IMAGE_CAMPAIGN),
        crate::outbox::waiting(&place),
        crate::hubstore::orders(&place),
    );
    let (_, loc, (camp, waiting, listed)) = match crate::owner::owner_beside(&req, &ctx, &place, work).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let Some(id) = param(&ctx) else { return bad("no such campaign", 404) };
    let entries = camp.log.entries();
    let Some(def) = campaign::def_of(&entries, &id) else { return bad("no such campaign", 404) };
    let queued_now = waiting.iter().filter(|e| send::parse_id(&e.id).map(|(c, _)| c) == Some(id.as_str())).count();
    let redeemed = match (&def.promo, campaign::first_sent_at(&entries, &id)) {
        (Some(code), Some(since)) => campaign::redeemed(&orders_of(listed, &loc), code, since),
        _ => 0,
    };
    Response::from_json(&json!({ "campaign": def, "report": campaign::report(&entries, &id, queued_now, redeemed) }))
}
