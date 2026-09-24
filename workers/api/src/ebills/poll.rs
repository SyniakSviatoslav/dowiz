//! THE POLLER, on the minute cron that already exists (`lib.rs`'s
//! `scheduled`; wrangler's `"* * * * *"`). Per venue: ask the venue's object
//! what is due (`tick`), read ebills.al through the allow-listed client, map,
//! and hand the result back as ONE command (`import` / `floor`). A failure is
//! LOUD -- the venue's error log, the health lines, a backoff -- and is never
//! read as "no sales" (§6.7).
//!
//! BOUNDED PER FIRING: one login at most (two requests), the floor, one or
//! two lists, the menu once a day, and `walk::BUDGET` details.

use super::client::Path;
use super::cmd::{FloorIn, ImportIn, ImportOut, ReportIn, TickIn};
use super::fetch::Client;
use super::import::Mapped;
use super::judge::Fail;
use super::map::{floor, to_order, to_paid};
use super::state::{Noted, Plan, LEAD_IDS, RECHECK_BUDGET};
use super::walk::{Step, Walker, BUDGET};
use super::{classify, whole, parse_detail, parse_items, parse_list, parse_tables, Kind};
use crate::hubstore::Place;
use serde_json::Value;
use worker::*;

/// The largest page the list honours (measured: `size=500`, §1.3).
const PAGE: u32 = 500;

async fn hub<I: serde::Serialize, O: serde::de::DeserializeOwned>(place: &Place, what: &str, input: &I) -> std::result::Result<O, Fail> {
    crate::command::send(place, what, input).await.map_err(|(s, m)| Fail::Hub(s, m))
}

/// Every venue, every minute. A venue with no link configured answers
/// `enabled: false` from one storage read and costs nothing else.
pub(crate) async fn sweep(env: &Env, now_ms: i64) {
    let registry = match crate::identity_store::registry(env).await {
        Ok(t) => t,
        Err(e) => return console_error!("ebills sweep: registry unreadable: {e}"),
    };
    for (venue, _) in registry.all(crate::identity_store::K_LOC) {
        let Ok(ns) = env.durable_object("HUB") else { return console_error!("ebills sweep: no HUB binding") };
        let place = Place { ns, venue: venue.clone() };
        let plan: Plan = match hub(&place, "ebills/tick", &TickIn { now_ms }).await {
            Ok(p) => p,
            Err(f) => {
                crate::loud!(&place.ns, Some(&venue), "ebills.tick", "{}", f.line());
                continue;
            }
        };
        if !plan.enabled || !(plan.floor || plan.sales || plan.reread || plan.recheck) {
            continue;
        }
        if let Err(f) = run(&place, &plan, now_ms).await {
            let report = ReportIn {
                now_ms,
                what: f.line(),
                halt: matches!(f, Fail::Mfa | Fail::TenantNeeded),
                drop_session: f.is_auth(),
            };
            if let Err(g) = hub::<_, Value>(&place, "ebills/report", &report).await {
                console_error!("ebills {venue}: the failure could not be recorded: {}", g.line());
            }
            crate::loud!(&place.ns, Some(&venue), "ebills.poll", "{}", f.line());
        }
    }
}

/// The live floor into its own image (`hubdo/ebills.rs`), with the session
/// when this read rotated it.
async fn read_floor(place: &Place, c: &mut Client, pos: i64, now_ms: i64) -> std::result::Result<(), Fail> {
    let tables = parse_tables(&c.read(&Path::Tables { pos }).await?).map_err(Fail::Shape)?;
    let rows = floor(&tables).map_err(|e| Fail::Shape(format!("floor: {e:?}")))?;
    let session = std::mem::take(&mut c.changed).then(|| c.session.clone());
    hub::<_, Value>(place, "ebills/floor", &FloorIn { now_ms, tables: rows, session }).await.map(|_| ())
}

/// SALES REFUSED FOR A REASON TIME CAN UNDO (`state::transient`), read again
/// by id: one that maps now is imported as if first seen -- the import keeps
/// it once -- and one that still does not stays refused as it was.
async fn retry(c: &mut Client, pos: i64, venue: &str, ids: &[i64], input: &mut ImportIn) -> std::result::Result<(), Fail> {
    for &id in ids {
        let sale = match c.read(&Path::Sale { id, pos }).await {
            Err(Fail::NotFound) => continue,
            other => parse_detail(&other?).map_err(Fail::Shape)?,
        };
        let taken = match classify(&sale) {
            Ok(Kind::Bill) => to_paid(&sale).map(|paid| Mapped::Bill { sale_id: id, paid }).ok(),
            _ => to_order(&sale, venue).map(|envelope| Mapped::Order { sale_id: id, envelope }).ok(),
        };
        input.sales.extend(taken);
    }
    Ok(())
}

/// A map refusal as the owner reads it.
fn refused(sale_id: i64, why: impl std::fmt::Debug, now_ms: i64) -> Noted {
    Noted { at_ms: now_ms, sale_id, why: format!("{why:?}") }
}

async fn run(place: &Place, plan: &Plan, now_ms: i64) -> std::result::Result<(), Fail> {
    let mut c = Client::new(&plan.user, &plan.secret, plan.session.clone());
    let pos = plan.pos_id;
    // THE FLOOR DOES NOT HOLD THE SALES HOSTAGE: one table whose running
    // total is not whole lek refuses the floor (by name, reported below), and
    // the sales -- the orders and the stock -- are still read.
    let floor_failed = match plan.floor {
        true => read_floor(place, &mut c, pos, now_ms).await.err(),
        false => None,
    };
    if !(plan.sales || plan.reread || plan.recheck) {
        return floor_failed.map_or(Ok(()), Err);
    }
    let mut input = ImportIn {
        now_ms,
        sales: Vec::new(),
        watermark: plan.watermark,
        backlog: false,
        listed: false,
        reread: false,
        refused: Vec::new(),
        items: Vec::new(),
        session: None,
        complete: false,
        lead_below: 0,
        recheck: None,
    };
    // A FAILURE MID-WALK STILL IMPORTS WHAT WAS READ, with the watermark at
    // the last id handled; the failure is then returned and reported.
    let mut stopped: Option<Fail> = None;
    if plan.sales {
        stopped = walk(&mut c, plan, &place.venue, &mut input, now_ms).await.err();
        input.listed = stopped.is_none();
    }
    if !plan.retry.is_empty() && stopped.is_none() {
        stopped = retry(&mut c, pos, &place.venue, &plan.retry, &mut input).await.err();
    }
    if plan.reread && stopped.is_none() {
        stopped = reread(&mut c, plan, &mut input, now_ms).await.err();
        input.reread = stopped.is_none();
    }
    let pass = input.recheck.or((plan.recheck).then_some((plan.recheck_from, plan.recheck_until)));
    if let (Some((from, until)), None) = (pass, &stopped) {
        stopped = recheck(&mut c, plan.pos_id, (from, until), &mut input, now_ms).await.err();
    }
    input.session = c.changed.then(|| c.session.clone());
    input.complete = stopped.is_none();
    let stopped = stopped.or(floor_failed);
    if !input.complete && input.sales.is_empty() && input.refused.is_empty() && input.session.is_none() {
        return stopped.map_or(Ok(()), Err);
    }
    let out: ImportOut = hub(place, "ebills/import", &input).await?;
    if out.placed + out.noted + out.paid > 0 || !out.refused.is_empty() {
        console_log!(
            "ebills {}: {} placed, {} noted, {} paid, {} unchanged, {} bills waiting, {} refused",
            place.venue, out.placed, out.noted, out.paid, out.unchanged, out.pending, out.refused.len()
        );
    }
    stopped.map_or(Ok(()), Err)
}

/// The five-minute read: the list for yesterday and today, then the ids it
/// hides (`walk.rs`), each fetched and mapped. A refusal to MAP one sale is
/// named and passed over; a refusal to PARSE is the platform changing its
/// answer, and stops the walk where it is.
async fn walk(c: &mut Client, plan: &Plan, venue: &str, input: &mut ImportIn, now_ms: i64) -> std::result::Result<(), Fail> {
    let pos = plan.pos_id;
    let path = Path::Sales { begin: plan.yesterday.clone(), end: plan.today.clone(), pos, size: PAGE };
    let list = parse_list(&c.read(&path).await?).map_err(Fail::Shape)?;
    let ids: Vec<i64> = list.sales.iter().map(|s| s.id).collect();
    let bills: Vec<i64> = list.sales.iter().filter(|s| classify(s) == Ok(Kind::Bill)).map(|s| s.id).collect();
    let mut w = Walker::new(plan.watermark, &ids, &bills, BUDGET, LEAD_IDS, plan.lead_below);
    let result = loop {
        match w.step() {
            Step::Done => break Ok(()),
            Step::Listed(id) => {
                if let Some(row) = list.sales.iter().find(|s| s.id == id) {
                    match to_paid(row) {
                        Ok(paid) => input.sales.push(Mapped::Bill { sale_id: id, paid }),
                        Err(e) => input.refused.push(refused(id, e, now_ms)),
                    }
                }
                w.done();
            }
            Step::Fetch(id) => match c.read(&Path::Sale { id, pos }).await {
                Err(Fail::NotFound) => w.missing(),
                Err(f) => break Err(f),
                Ok(body) => {
                    let sale = match parse_detail(&body) {
                        Ok(s) => s,
                        Err(e) => break Err(Fail::Shape(e)),
                    };
                    // BEFORE THE FIRST WINDOW: a course is a lead, a bill is
                    // passed over -- its courses are further back still.
                    let mapped = match (classify(&sale), w.is_lead(id)) {
                        (Ok(Kind::Bill), true) => None,
                        (Ok(Kind::Bill), false) => Some(to_paid(&sale).map(|paid| Mapped::Bill { sale_id: id, paid })),
                        (_, true) => Some(to_order(&sale, venue).map(|envelope| Mapped::Lead { sale_id: id, envelope })),
                        (_, false) => Some(to_order(&sale, venue).map(|envelope| Mapped::Order { sale_id: id, envelope })),
                    };
                    match mapped {
                        Some(Ok(m)) => input.sales.push(m),
                        Some(Err(e)) if !w.is_lead(id) => input.refused.push(refused(id, e, now_ms)),
                        _ => {}
                    }
                    w.done();
                }
            },
        }
    };
    input.watermark = w.handled.max(plan.watermark);
    input.backlog = w.backlog;
    input.lead_below = w.lead_below;
    result
}

/// The daily re-read (§6.6): a week of the list, each row's fingerprint
/// handed to the import (an amended sale becomes `Noted`), each bill checked
/// against the one applied (`Recheck`: never queued, so a week of bills from
/// before the link existed does not flood the waiting list), and the till's
/// menu for the crosswalk screen.
async fn reread(c: &mut Client, plan: &Plan, input: &mut ImportIn, now_ms: i64) -> std::result::Result<(), Fail> {
    let path = Path::Sales { begin: plan.week_ago.clone(), end: plan.today.clone(), pos: plan.pos_id, size: PAGE };
    let list = parse_list(&c.read(&path).await?).map_err(Fail::Shape)?;
    // THE COURSES THE LIST HIDES are re-checked too, by id, a few a firing
    // (`recheck`): the pass runs from the week's oldest listed id up to the
    // watermark.
    if let Some(oldest) = list.sales.iter().map(|s| s.id).min().filter(|o| *o <= input.watermark) {
        input.recheck = Some((oldest, input.watermark));
    }
    for s in list.sales.iter().filter(|s| s.id <= input.watermark) {
        match classify(s) {
            Ok(Kind::Bill) => match to_paid(s) {
                Ok(paid) => input.sales.push(Mapped::Recheck { sale_id: s.id, paid }),
                Err(e) => input.refused.push(refused(s.id, e, now_ms)),
            },
            _ => match whole(s.total_value, "totalValue") {
                Ok(total) => input.sales.push(Mapped::Seen {
                    sale_id: s.id,
                    order_id: format!("ebills:{}", s.uuid),
                    total,
                    log_cis_len: s.log_cis.as_ref().map_or(0, Vec::len) as i64,
                }),
                Err(e) => input.refused.push(refused(s.id, e, now_ms)),
            },
        }
    }
    input.items = parse_items(&c.read(&Path::Items { size: PAGE }).await?).map_err(Fail::Shape)?;
    Ok(())
}

/// THE RE-CHECK PASS (§6.6): closed sales by id, `RECHECK_BUDGET` a firing,
/// each handed to the import as a fingerprint (`Seen`) or a bill to check
/// (`Recheck`) -- an amended course becomes `Noted`, an unknown one is
/// ignored, nothing is ever placed from here. A `404` is passed over.
async fn recheck(c: &mut Client, pos: i64, (from, until): (i64, i64), input: &mut ImportIn, now_ms: i64) -> std::result::Result<(), Fail> {
    let mut id = from;
    let stop = from.saturating_add(i64::from(RECHECK_BUDGET));
    let result = loop {
        if id > until || id >= stop {
            break Ok(());
        }
        match c.read(&Path::Sale { id, pos }).await {
            Err(Fail::NotFound) => {}
            Err(f) => break Err(f),
            Ok(body) => match parse_detail(&body) {
                Err(e) => break Err(Fail::Shape(e)),
                Ok(s) if classify(&s) == Ok(Kind::Bill) => match to_paid(&s) {
                    Ok(paid) => input.sales.push(Mapped::Recheck { sale_id: id, paid }),
                    Err(e) => input.refused.push(refused(id, e, now_ms)),
                },
                Ok(s) => match whole(s.total_value, "totalValue") {
                    Ok(total) => input.sales.push(Mapped::Seen {
                        sale_id: id,
                        order_id: format!("ebills:{}", s.uuid),
                        total,
                        log_cis_len: s.log_cis.as_ref().map_or(0, Vec::len) as i64,
                    }),
                    Err(e) => input.refused.push(refused(id, e, now_ms)),
                },
            },
        }
        id += 1;
    };
    // The pass is over when it passes `until`: (0, 0) says so.
    input.recheck = Some(if id > until { (0, 0) } else { (id, until) });
    result
}
