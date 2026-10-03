//! WHO HAS ASKED TO BE TOLD, per venue, PURE.
//!
//! One image per venue, [`IMAGE_PUSH`], one record per (person, device):
//!   kind [`K_CUSTOMER`] -- keyed by the ORDER the customer's token is for;
//!   kind [`K_COURIER`]  -- keyed by the courier's id;
//!   kind [`K_STAFF`]    -- keyed by the owner's or staff member's id.
//! The record id is `{key}/{device}`, `device` = the first 16 hex digits of
//! SHA-256(endpoint), so the same phone subscribing twice is one record and a
//! person's phone and laptop are two.
//!
//! WHAT IS KEPT is what the browser handed over (RFC 8030 endpoint, RFC 8291
//! `p256dh` + `auth`), the language to write in, and when. These identify a
//! DEVICE to its push service, so they are personal data (registry row
//! "push"): a customer's go when the order ends and in any case after
//! [`CUSTOMER_KEEP_MS`]; anyone's go on a 404/410 from the push service, and
//! on "turn off".
//!
//! THE ENDPOINT IS A URL THE WORKER WILL POST TO, chosen by whoever holds a
//! token -- so it is checked against the push services browsers actually use
//! ([`push_host`]); anything else would make the drain a request forwarder.

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use serde::{Deserialize, Serialize};

pub const IMAGE_PUSH: &str = "push";
/// Small: a record is ~300 bytes; 2 MiB is thousands of devices.
pub const PUSH_BYTES: usize = 2 * 1024 * 1024;
pub const K_CUSTOMER: &str = "pc";
pub const K_COURIER: &str = "pk";
pub const K_STAFF: &str = "ps";
/// A customer's subscription outlives its order by at most this (two days):
/// enough for "delivered" and a scheduled order's day, not a marketing list.
pub const CUSTOMER_KEEP_MS: i64 = 2 * 86_400_000;
/// Devices one person may have told us about; the oldest goes first.
pub const PER_PERSON: usize = 5;
pub const ENDPOINT_MAX: usize = 1024;

/// One device's subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sub {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub lang: String,
    pub at_ms: i64,
}

/// What the browser posts: `PushSubscription.toJSON()` plus a language.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubIn {
    pub endpoint: String,
    pub keys: KeysIn,
    #[serde(default)]
    pub lang: Option<String>,
    /// `PushSubscription.toJSON()` carries it; it is read and ignored.
    #[serde(default, rename = "expirationTime")]
    pub _expiration_time: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeysIn {
    pub p256dh: String,
    pub auth: String,
}

/// The push services' hosts: Google FCM (Chrome, Edge on Android, Samsung,
/// Opera), Mozilla autopush (Firefox), Apple (Safari, iOS 16.4+ home-screen
/// apps), Microsoft WNS (Edge on Windows).
pub fn push_host(host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    h == "fcm.googleapis.com"
        || h == "updates.push.services.mozilla.com"
        || h.ends_with(".push.apple.com")
        || h.ends_with(".notify.windows.com")
}

impl SubIn {
    /// Check everything the drain will later trust.
    pub fn check(self, now_ms: i64) -> Result<Sub, &'static str> {
        let endpoint = self.endpoint.trim().to_string();
        if endpoint.len() > ENDPOINT_MAX {
            return Err("that endpoint is too long");
        }
        let host = endpoint
            .strip_prefix("https://")
            .and_then(|r| r.split(['/', '?', '#']).next())
            .filter(|a| !a.contains('@'))
            .map(|a| a.split(':').next().unwrap_or(a))
            .ok_or("the endpoint must be an https URL")?;
        if !push_host(host) {
            return Err("that endpoint is not a browser push service");
        }
        let point = B64.decode(self.keys.p256dh.trim()).map_err(|_| "p256dh is not base64url")?;
        if point.len() != super::ece::POINT_LEN || p256::PublicKey::from_sec1_bytes(&point).is_err() {
            return Err("p256dh is not a P-256 public key");
        }
        let auth = B64.decode(self.keys.auth.trim()).map_err(|_| "auth is not base64url")?;
        if auth.len() != super::ece::AUTH_LEN {
            return Err("auth is not 16 bytes");
        }
        let lang = self.lang.as_deref().map(str::trim).filter(|l| dowiz_hub::lang::is_lang(l)).unwrap_or("en").to_string();
        Ok(Sub { endpoint, p256dh: B64.encode(point), auth: B64.encode(auth), lang, at_ms: now_ms })
    }
}

/// The device part of a record id.
pub fn device(endpoint: &str) -> String {
    crate::auth::sha256_hex(endpoint).chars().take(16).collect()
}

/// The record id of `key`'s device.
pub fn record_id(key: &str, endpoint: &str) -> String {
    format!("{key}/{}", device(endpoint))
}

/// The ids of `key`'s records among `all` (`(id, json)` of one kind).
pub fn of_key<'a>(all: &'a [(String, String)], key: &str) -> Vec<(&'a str, Sub)> {
    let prefix = format!("{key}/");
    all.iter()
        .filter(|(id, _)| id.starts_with(&prefix))
        .filter_map(|(id, j)| serde_json::from_str::<Sub>(j).ok().map(|s| (id.as_str(), s)))
        .collect()
}

/// Which of `key`'s OTHER devices to drop so that adding one keeps at most
/// [`PER_PERSON`]: the oldest first.
pub fn over_cap(mine: &[(&str, Sub)], adding: &str) -> Vec<String> {
    let mut others: Vec<&(&str, Sub)> = mine.iter().filter(|(id, _)| *id != adding).collect();
    if others.len() < PER_PERSON {
        return Vec::new();
    }
    others.sort_by_key(|(id, s)| (s.at_ms, id.to_string()));
    let drop = others.len() + 1 - PER_PERSON;
    others.into_iter().take(drop).map(|(id, _)| id.to_string()).collect()
}

/// Customer records past [`CUSTOMER_KEEP_MS`], unreadable ones included.
pub fn stale_customers(all: &[(String, String)], now_ms: i64) -> Vec<String> {
    all.iter()
        .filter(|(_, j)| serde_json::from_str::<Sub>(j).map_or(true, |s| now_ms - s.at_ms > CUSTOMER_KEEP_MS))
        .map(|(id, _)| id.clone())
        .collect()
}

#[cfg(test)]
mod tests;
