//! THE DRAIN'S I/O FOR SUMMARIES AND FOR CHATS THAT CHANGED (W-TG T4/T1).
//! The rules are pure in `outbox::digest`, `notify::route` and `notify::tg`.

use std::collections::BTreeMap;

use serde_json::Value;
use worker::*;

use super::{backoff_ms, digest, tgrail::Ops, Entry, MAX_TRIES};
use crate::notify::route::{self, groups, Group, State, DIGEST_KIND};
use crate::notify::tg::{self, Fail};

/// The orders and names, read once per drain and only when a summary is due.
#[derive(Default)]
pub struct Folded {
    loaded: bool,
    orders: Vec<Value>,
    names: BTreeMap<String, String>,
    currency: String,
}

async fn fold(place: &crate::hubstore::Place, f: &mut Folded) -> Result<()> {
    if f.loaded {
        return Ok(());
    }
    let (listed, cat) = futures_util::future::try_join(crate::hubstore::orders(place), crate::hubstore::load_catalog(place)).await?;
    let cat = cat.catalog;
    f.orders = crate::services::orders::mine::of_venue(listed, &place.venue);
    for o in &f.orders {
        for it in o.get("items").and_then(Value::as_array).into_iter().flatten() {
            if let Some(id) = it.get("product_id").and_then(Value::as_str) {
                let name = cat
                    .product(id)
                    .and_then(|j| serde_json::from_str::<Value>(&j).ok())
                    .and_then(|p| p.get("name").and_then(Value::as_str).map(str::to_string))
                    .unwrap_or_else(|| id.to_string());
                f.names.insert(id.to_string(), name);
            }
        }
    }
    f.currency = crate::services::venue::currency_of(&cat);
    f.loaded = true;
    Ok(())
}

/// Send one due summary and move its entry on. Returns a line for the
/// venue's error log when the summary could not be sent today.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    place: &crate::hubstore::Place,
    image: &dowiz_hub::table::Table,
    e: &Entry,
    groups: &[Group],
    zone: dowiz_hub::tz::Zone,
    record: Option<&Value>,
    token: &str,
    now_ms: i64,
    folded: &mut Folded,
    ops: &mut Ops,
) -> Option<String> {
    let Some(g) = groups.iter().find(|g| g.id == e.to && g.state == State::Active) else {
        ops.remove(&e.id);
        return None;
    };
    let weekly = digest::weekly(e);
    if let Err(err) = fold(place, folded).await {
        ops.put(Entry { next_at_ms: now_ms + backoff_ms(3), ..e.clone() });
        return Some(format!("{}: the orders did not fold: {err}", e.id));
    }
    let venue = record.and_then(|r| r.get("name")).and_then(Value::as_str).unwrap_or(&place.venue).to_string();
    let names = &folded.names;
    let sum = digest::summary(&venue, &folded.orders, zone, now_ms, weekly, &folded.currency, &|id| names.get(id).cloned().unwrap_or_else(|| id.to_string()));
    // The weekly summary is numbers only; the lines are the daily one's.
    let (lines, keys) = if weekly { (Vec::new(), Vec::new()) } else { digest::lines_for(&image.all(DIGEST_KIND), &g.id) };
    let text = route::render::digest(&g.lang, weekly, &sum, &lines);
    let tomorrow = route::next_at(g.digest_at, weekly, zone, now_ms);
    match tg::send(token, &g.target(), &text).await {
        Ok(()) => {
            ops.put(Entry { next_at_ms: tomorrow, tries: 0, ..e.clone() });
            for k in keys {
                ops.records.push((DIGEST_KIND, k, None));
            }
            None
        }
        Err(Fail::RetryAfter(s)) => {
            ops.put(Entry { next_at_ms: now_ms + s * 1000, ..e.clone() });
            None
        }
        Err(Fail::Gone(why)) => {
            ops.gone.push((tg::target_of(&g.target()).chat, why.clone()));
            ops.put(Entry { next_at_ms: tomorrow, tries: 0, ..e.clone() });
            Some(format!("{}: the bot is gone from {}: {why}", e.id, g.chat))
        }
        Err(f) => {
            let tries = e.tries + 1;
            if tries >= MAX_TRIES {
                // A SUMMARY IS NEVER ABANDONED FOR GOOD: it skips to tomorrow, loudly.
                ops.put(Entry { next_at_ms: tomorrow, tries: 0, ..e.clone() });
                return Some(format!("{}: today's summary was not sent: {}", e.id, f.words()));
            }
            ops.put(Entry { next_at_ms: now_ms + backoff_ms(tries), tries, ..e.clone() });
            None
        }
    }
}

/// Chats that moved or that the bot is gone from: the groups follow (and the
/// old single-chat keys, which are the same chats under another name).
pub async fn chats_changed(place: &crate::hubstore::Place, gone: &[(String, String)], moved: &[(String, String)], lang: &str) -> Result<()> {
    let (gone, moved, lang) = (gone.to_vec(), moved.to_vec(), lang.to_string());
    crate::hubstore::with_settings(place, move |s| {
        let mut list = crate::notify::hook::editable(s, &lang).map_err(Error::RustError)?;
        let mut changed = false;
        for (from, to) in &moved {
            changed |= groups::migrate(&mut list, from, to);
            for k in [groups::LEGACY_CHAT, groups::LEGACY_BAR] {
                if s.known(k).trim() == from {
                    s.set(k, to);
                }
            }
        }
        for (chat, _) in &gone {
            changed |= !groups::left(&mut list, chat).is_empty();
        }
        if changed {
            crate::notify::hook::store(s, &list);
        }
        Ok(())
    })
    .await
}
