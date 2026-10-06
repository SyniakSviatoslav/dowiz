//! W-STORE2 through the stock door, pure: a kitchen station bound to a
//! storage (`bound {station, store}`), and a freezing record's START, typed in
//! the venue's local time and turned into ms by the object, which holds the
//! venue's zone (`stamp_started`, called from `turn::from_catalogue`).
//!
//! A binding to a storage this venue does not have, or one archived, is a
//! 400 that names it (`dowiz_hub::stock::storages::bind`); so is a station
//! that is not kitchen, sushi or bar.

use dowiz_hub::stock::StockLog;
use dowiz_hub::tz::{offset_minutes, Zone};
use serde_json::{json, Value};

use super::super::turn::{StockTurnIn, Told};

pub use dowiz_hub::stock::storages::bind::{BOUND, STATIONS};

/// The keys the object writes the start and the end into; a client cannot
/// send them (the strict body has no such fields).
pub const STARTED_MS: &str = "startedMs";
pub const ENDED_MS: &str = "endedMs";

/// `bound`: bind a station, or unbind it (`store` ""). A refusal writes nothing.
pub fn run(log: &mut StockLog, input: &StockTurnIn) -> Result<(Value, Told), (u16, String)> {
    log.set_clock(input.now_ms);
    let b = &input.body;
    let s = |k: &str| b.get(k).and_then(Value::as_str).unwrap_or("").trim().to_string();
    let (station, store) = (s("station"), s("store"));
    if !STATIONS.contains(&station.as_str()) {
        return Err((400, format!("{station:?}: a station is kitchen, sushi or bar")));
    }
    log.bind_station(&station, &store, &input.by).map_err(|e| (400, e.to_string()))?;
    Ok((json!({ "ok": true, "kind": BOUND, "station": station, "store": store }), Vec::new()))
}

/// `"2026-10-05T14:30"` as LOCAL wall-clock ms (`tz::local_ms`'s frame), or
/// `None`. Minutes precision; seconds, if a browser sends them, are refused.
pub fn parse_local(s: &str) -> Option<i64> {
    let s = s.trim();
    let (day, time) = s.split_once('T').or_else(|| s.split_once(' '))?;
    let d = dowiz_hub::stock::meta::parse_day(day)?;
    let (h, m) = time.split_once(':')?;
    if h.len() != 2 || m.len() != 2 {
        return None;
    }
    let (h, m): (i64, i64) = (h.parse().ok()?, m.parse().ok()?);
    if !(0..24).contains(&h) || !(0..60).contains(&m) {
        return None;
    }
    Some(dowiz_hub::stock::meta::day_number(d) * 86_400_000 + h * 3_600_000 + m * 60_000)
}

/// The UTC instant a LOCAL wall-clock time names in `z`: the offset is the
/// one in force at that instant (asked twice, so the hour around a summer-time
/// change resolves to one of its two readings, never an error).
pub fn local_to_utc(z: Zone, local: i64) -> i64 {
    let guess = local - offset_minutes(z, local) * 60_000;
    local - offset_minutes(z, guess) * 60_000
}

/// The object's half: a `frozen` body's `started` and `ended` (local text)
/// become [`STARTED_MS`] and [`ENDED_MS`]. Whatever the body carried under
/// those keys is dropped first; text that is not a local time is left for
/// `started_of` / `ended_of` to refuse.
pub fn stamp_started(body: &mut Value, z: Zone) {
    let Some(obj) = body.as_object_mut() else { return };
    for (text, key) in [("started", STARTED_MS), ("ended", ENDED_MS)] {
        obj.remove(key);
        let ms = obj.get(text).and_then(Value::as_str).and_then(parse_local).map(|l| local_to_utc(z, l));
        if let Some(ms) = ms {
            obj.insert(key.into(), json!(ms));
        }
    }
}

fn stamped(b: &Value, text: &str, key: &str, what: &str) -> Result<Option<i64>, (u16, String)> {
    match b.get(text).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => b
            .get(key)
            .and_then(Value::as_i64)
            .map(Some)
            .ok_or((400, format!("{s:?}: the {what} is the venue's local time, yyyy-mm-ddThh:mm"))),
    }
}

/// The start as the turn reads it: `None` when not given, the ms the object
/// stamped, or a 400 for text that is not a local time.
pub fn started_of(b: &Value) -> Result<Option<i64>, (u16, String)> {
    stamped(b, "started", STARTED_MS, "start")
}

/// The end, as [`started_of`] reads the start.
pub fn ended_of(b: &Value) -> Result<Option<i64>, (u16, String)> {
    stamped(b, "ended", ENDED_MS, "end")
}

#[cfg(test)]
#[path = "bind_tests.rs"]
mod tests;
