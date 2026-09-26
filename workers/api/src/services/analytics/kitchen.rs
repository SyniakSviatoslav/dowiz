//! `GET /api/owner/analytics/kitchen?from=yyyy-mm-dd&to=yyyy-mm-dd` -- THE
//! KITCHEN'S NUMBERS (card I7): consumption per ingredient and per dish, by
//! day; cost of goods and food cost; margin per dish; waste by reason and
//! value; raw -> cooked losses; measured yields against the defaults; supplier
//! prices; days of cover and a reorder hint.
//!
//! ORCHESTRATION ONLY: it reads three images (orders, catalogue, stock) and
//! writes none; every number is decided in `sales`, `shelf` and `report`,
//! where it has a test.
//!
//! CPU, bounded by construction rather than by a timer: one pass over the
//! stock log (`StockLog::journal`), one over the venue's orders, and per sold
//! line one recipe lookup in a map built once. Nothing runs on the placement
//! path. The window is at most `MAX_DAYS`.

use serde_json::Value;
use std::collections::HashMap;
use worker::*;

use crate::owner::owner_and_venue;
use dowiz_hub::stock::meta::{day_number, day_of_local_ms, parse_day};
use dowiz_hub::tz::Zone;

pub mod report;
pub mod sales;
pub mod shelf;

use sales::{Dish, DishLine, Supply, Window};

/// The longest window one request folds.
pub const MAX_DAYS: i64 = 62;
/// The default window: the last seven days, today included.
pub const DEFAULT_DAYS: i64 = 7;
const DAY_MS: i64 = 86_400_000;

/// The window asked for, in the venue's days. `from`/`to` are `yyyy-mm-dd`;
/// `days` (without `from`) counts back from `to`; nothing: the last seven
/// days. Refused: a date that is not one, `from` after `to`, more than
/// `MAX_DAYS`.
pub fn window(zone: Zone, now: i64, from: Option<&str>, to: Option<&str>, days: Option<&str>) -> std::result::Result<Window, String> {
    let today = day_of_local_ms(dowiz_hub::tz::local_ms(zone, now));
    let day = |s: Option<&str>, d: i64| match s.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => parse_day(s).ok_or(format!("{s:?} is not a date (yyyy-mm-dd)")),
        None => Ok(d),
    };
    let to = day(to, today)?;
    let back = match days.map(str::trim).filter(|s| !s.is_empty()) {
        Some(d) => d.parse::<i64>().ok().filter(|d| (1..=MAX_DAYS).contains(d)).ok_or(format!("days is 1 to {MAX_DAYS}"))?,
        None => DEFAULT_DAYS,
    };
    let from = day(from, dowiz_hub::stock::meta::day_of_number(day_number(to) - (back - 1)))?;
    let span = day_number(to) - day_number(from) + 1;
    if span < 1 {
        return Err("from is after to".into());
    }
    if span > MAX_DAYS {
        return Err(format!("at most {MAX_DAYS} days at a time"));
    }
    // Each local midnight asked of the zone separately, from that day's UTC
    // noon: a 25-hour day stays one day.
    let start = |n: i64| dowiz_hub::tz::start_of_local_day_ms(zone, n * DAY_MS + DAY_MS / 2);
    let first = day_number(from);
    let starts: Vec<i64> = (0..span).map(|k| start(first + k)).collect();
    let days = (0..span).map(|k| dowiz_hub::stock::meta::day_of_number(first + k)).collect();
    Ok(Window { starts, end: start(first + span), days })
}

/// Every supply of the catalogue, as the report reads it.
pub fn supplies_of(cat: &dowiz_hub::catalog::Catalog) -> HashMap<String, Supply> {
    use crate::recipe::weights::{pm_of, CLEAN_MAX, COOK_MAX};
    cat.supplies()
        .into_iter()
        .filter_map(|(id, j)| {
            let v: Value = serde_json::from_str(&j).ok()?;
            let unit = v.get("unit").and_then(Value::as_str).unwrap_or("g").to_string();
            Some((id.clone(), Supply {
                name: v.get("name").and_then(Value::as_str).unwrap_or(&id).to_string(),
                basis: crate::recipe::basis_of(&unit),
                list_cost: v.get("costPerBasis").and_then(Value::as_i64),
                clean_pm: pm_of(&v, "cleanPm", CLEAN_MAX),
                cook_pm: pm_of(&v, "cookPm", COOK_MAX),
                unit,
                id,
            }))
        })
        .collect()
}

/// Every dish with its recipe's lines, weighed as the recipe editor weighs them.
pub fn dishes_of(cat: &dowiz_hub::catalog::Catalog) -> HashMap<String, Dish> {
    cat.products()
        .into_iter()
        .filter_map(|(id, j)| {
            let p: Value = serde_json::from_str(&j).ok()?;
            let lines = crate::recipe::lines_of_stored(p.get("bom").unwrap_or(&Value::Null), |s| cat.supply(s))
                .into_iter()
                .map(|l| DishLine { supply: l.supply, qty: l.qty, gross_g: l.w.gross, net_g: l.w.net, out_g: l.w.out })
                .collect();
            let name = p.get("name").and_then(Value::as_str).unwrap_or(&id).to_string();
            Some((id.clone(), Dish { id, name, lines }))
        })
        .collect()
}

pub async fn kitchen(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let url = req.url()?;
    let q = |k: &str| url.query_pairs().find(|(key, _)| key == k).map(|(_, v)| v.to_string());
    let (listed, cat) =
        futures_util::future::try_join(crate::hubstore::orders(&place), crate::hubstore::load_catalog(&place)).await?;
    let cat = cat.catalog;
    let stock = crate::hubstore::load_stock(&place).await?.stock;
    let zone = crate::hubstore::zone_of(cat.location().and_then(|j| serde_json::from_str::<Value>(&j).ok()).as_ref());
    let w = match window(zone, ctx.data.now_ms, q("from").as_deref(), q("to").as_deref(), q("days").as_deref()) {
        Ok(w) => w,
        Err(why) => return Response::error(why, 400),
    };
    let journal = match stock.journal() {
        Ok(j) => j,
        Err(e) => return Response::error(e.to_string(), 500),
    };
    let orders = crate::services::orders::mine::of_venue(listed, &loc);
    let (dishes, supplies) = (dishes_of(&cat), supplies_of(&cat));
    let sold = sales::fold(&orders, &dishes, &w);
    let moved = shelf::fold(&journal.entries, &supplies, &sold.placed_at, &w);
    let mut out = report::report(&sold, &moved, &dishes, &supplies, &journal, &w);
    out["currency"] = serde_json::json!(crate::services::venue::currency_of(&cat));
    Response::from_json(&out)
}

#[cfg(test)]
#[path = "kitchen/tests.rs"]
mod tests;
