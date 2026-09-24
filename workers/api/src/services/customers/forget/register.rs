//! THE ERASURE REGISTER (P3 of `BLUEPRINT-GDPR-AND-MCP-2026-09-24` §4.3).
//!
//! THE DEFECT. A nightly bundle holds the order log with every contact in
//! the clear, and `POST /api/owner/restore` imports it verbatim: a person
//! forgotten on Tuesday came back with Monday's backup, and nothing knew they
//! had asked. Cloudflare's point-in-time recovery of the object does the same.
//!
//! THE REGISTER is outside the venue's images -- in the platform object's own
//! `erasures` image -- so no restore of a venue can reach it, and after every
//! restore the Worker replays each of the venue's entries through the same
//! forget command (`run::replay`). Idempotent by construction: the command
//! declares only `tombstones - declared` (`forget::erase`).
//!
//! PSEUDONYMOUS, BY RULE AND BY TEST: an entry holds the venue, the person's
//! customer keys (HMACs under a secret the platform object does not hold),
//! their order ids (random), and the DAY of the erasure -- never a name, a
//! phone, or a millisecond instant long enough to look like one. The legacy
//! card ids are NOT kept: they are an unkeyed `sha256` of the raw number, and
//! a phone number's space is small enough to walk.
//!
//! WRITTEN BEFORE the object is asked to forget, so a forget that fails half
//! way is still replayed after a restore -- the register records what the
//! owner asked for, and the command converges on it.

use std::collections::BTreeSet;

use dowiz_hub::table::Table;
use serde::{Deserialize, Serialize};

/// The record kind in the platform's `erasures` image.
pub const KIND: &str = "forgot";

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// One forgotten person at one venue.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub venue: String,
    /// The key the owner forgot (the row's).
    pub key: String,
    /// Every key of their alias circle at the time, `key` included.
    pub keys: BTreeSet<String>,
    /// The orders found as theirs, hot and archived.
    pub orders: BTreeSet<String>,
    /// Days since the epoch of the FIRST erasure (a day, not an instant).
    pub day: i64,
}

impl Entry {
    pub fn new(venue: &str, key: &str, keys: &BTreeSet<String>, orders: &BTreeSet<String>, now_ms: i64) -> Self {
        let mut keys = keys.clone();
        keys.insert(key.to_string());
        Entry { venue: venue.into(), key: key.into(), keys, orders: orders.clone(), day: now_ms.div_euclid(DAY_MS) }
    }
}

pub fn id_of(venue: &str, key: &str) -> String {
    format!("{venue}/{key}")
}

/// PURE. File `e`, merged with what an earlier erasure of the same person
/// filed: keys and orders are UNIONED (a retry finds no phones, so what the
/// first run found must not be overwritten by the retry's nothing), and the
/// first day stays. Answers the entry as stored.
pub fn put(t: &mut Table, e: Entry) -> Result<Entry, String> {
    let id = id_of(&e.venue, &e.key);
    let merged = match t.get(KIND, &id).and_then(|j| serde_json::from_str::<Entry>(&j).ok()) {
        Some(mut old) => {
            old.keys.extend(e.keys);
            old.orders.extend(e.orders);
            old.day = old.day.min(e.day);
            old
        }
        None => e,
    };
    let json = serde_json::to_string(&merged).map_err(|x| x.to_string())?;
    t.put(KIND, &id, &json, &[], &[]).map_err(|x| format!("erasure register: {x:?}"))?;
    Ok(merged)
}

/// PURE. The venue's entries, in key order, and how many of its records did
/// not parse -- a replay REPORTS those, never treats them as "nobody".
pub fn of_venue(t: &Table, venue: &str) -> (Vec<Entry>, usize) {
    let prefix = format!("{venue}/");
    let (mut out, mut bad) = (Vec::new(), 0);
    for (_, j) in t.all(KIND).into_iter().filter(|(id, _)| id.starts_with(&prefix)) {
        match serde_json::from_str::<Entry>(&j) {
            Ok(e) => out.push(e),
            Err(_) => bad += 1,
        }
    }
    (out, bad)
}

/// File one entry in the platform object.
pub async fn file(env: &worker::Env, e: Entry) -> worker::Result<Entry> {
    use crate::platform_store::{with, ERASURES};
    with(env, ERASURES, move |t| put(t, e.clone()).map_err(worker::Error::RustError)).await
}

/// The venue's entries, and how many records could not be read.
pub async fn of(env: &worker::Env, venue: &str) -> worker::Result<(Vec<Entry>, usize)> {
    let loaded = crate::platform_store::load(env, crate::platform_store::ERASURES).await?;
    Ok(of_venue(&loaded.table, venue))
}

#[cfg(test)]
mod tests;
