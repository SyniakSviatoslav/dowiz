//! A TABLE LINK: the QR code on a table (L66, BLUEPRINT-POS-THE-ROOM §7, A9).
//!
//! `https://<venue host>/store/?t=<zone>.<n>.<sig>`, where `sig` is the first
//! 16 hex of HMAC-SHA256(the key `auth::sign` uses, "table|<loc>|<zone>|<n>").
//! The signature is what lets a guest's basket name a table the server then
//! believes: a hand-typed `?t=salla.4.0000…` is refused, and a code printed
//! for one venue does nothing at another, because the venue is in the MAC.
//!
//! THE TABLE MUST STILL STAND. A signed link to a table the owner has since
//! removed from the plan is refused: the kitchen would carry a dish to a table
//! that is not there.
//!
//! PURE. The key, the plan and the orders are handed in; nothing here reads
//! an environment, a clock or an image.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::command::floor::table_of;
use crate::command::sitting;
use crate::hubdo::OrderView;
use dowiz_hub::tables::Plan;
use serde_json::Value;

/// How much of the MAC travels: 64 bits. A guess is a 1-in-2^64 shot per try
/// at ONE table, and it earns a PENDING round a waiter must still confirm.
pub const SIG_HEX: usize = 16;

/// The signature for one table at one venue.
pub fn table_sig(key: &[u8], loc: &str, zone: &str, n: i64) -> String {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC takes any key length");
    mac.update(format!("table|{loc}|{zone}|{n}").as_bytes());
    let full: String = mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect();
    full[..SIG_HEX].to_string()
}

/// The `t` value printed into a table's code.
pub fn table_param(key: &[u8], loc: &str, zone: &str, n: i64) -> String {
    format!("{zone}.{n}.{}", table_sig(key, loc, zone, n))
}

/// The whole link. `host` is the venue's own host (`<slug>.<platform>`).
pub fn table_url(host: &str, key: &[u8], loc: &str, zone: &str, n: i64) -> String {
    format!("https://{host}/store/?t={}", table_param(key, loc, zone, n))
}

/// The table text a round carries (`fulfilment.table`): `zone:n`, the form
/// `command::floor::table_of` places on the plan without guessing.
pub fn table_text(zone: &str, n: i64) -> String {
    format!("{zone}:{n}")
}

/// Why a `t` was refused. Each one is a 400 with its own words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkRefused {
    /// Not `<zone>.<n>.<16 hex>`.
    Malformed,
    /// Well formed, and not signed by this venue for this table.
    BadSignature,
    /// Signed, but the table is no longer on the plan.
    NoSuchTable,
}

impl LinkRefused {
    pub fn message(&self) -> &'static str {
        match self {
            LinkRefused::Malformed => "that table code cannot be read; scan the code on the table again",
            LinkRefused::BadSignature => "that table code was not issued by this venue",
            LinkRefused::NoSuchTable => "that table is no longer on this venue's plan; ask a waiter",
        }
    }
}

/// Split a `t` into its three parts. Zone ids are `[a-z0-9_-]` (`tables::from_json`
/// refuses anything else), so a `.` can only be a separator.
pub fn parse(t: &str) -> Option<(String, i64, String)> {
    let mut p = t.trim().split('.');
    let (zone, n, sig) = (p.next()?, p.next()?, p.next()?);
    if p.next().is_some() || zone.is_empty() || sig.len() != SIG_HEX {
        return None;
    }
    if !sig.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
        return None;
    }
    let n: i64 = n.parse().ok()?;
    Some((zone.to_string(), n, sig.to_string()))
}

/// THE CHECK. `Ok((zone, n))` is a table on this venue's plan whose code this
/// venue signed. The comparison is constant-time.
pub fn verify_table(key: &[u8], loc: &str, plan: &Plan, t: &str) -> Result<(String, i64), LinkRefused> {
    let (zone, n, sig) = parse(t).ok_or(LinkRefused::Malformed)?;
    let want = table_sig(key, loc, &zone, n);
    if !bool::from(want.as_bytes().ct_eq(sig.as_bytes())) {
        return Err(LinkRefused::BadSignature);
    }
    plan.find(&zone, n).ok_or(LinkRefused::NoSuchTable)?;
    Ok((zone, n))
}

/// THE LIVE SITTING AT A TABLE, if there is one: among the sittings whose
/// table text places on `(zone, n)` and which are still open
/// (`sitting::open` — a round cooking, or a bill not yet paid), the one with
/// the newest round. A paid-up sitting is history: the next guest opens a new one.
pub fn live_sitting(plan: &Plan, listed: &[OrderView], zone: &str, n: i64) -> Option<String> {
    let mut best: Option<(i64, String)> = None;
    let mut seen: Vec<String> = Vec::new();
    for v in listed {
        let Ok(o) = serde_json::from_str::<Value>(&v.order_json) else { continue };
        let Some(sid) = o.get("sitting_id").and_then(Value::as_str) else { continue };
        if seen.iter().any(|s| s == sid) {
            continue;
        }
        seen.push(sid.to_string());
        let rounds = sitting::rounds(listed, sid);
        let here = rounds
            .iter()
            .rev()
            .find_map(|r| r.order.pointer("/fulfilment/table").and_then(Value::as_str))
            .and_then(|t| table_of(plan, t))
            .is_some_and(|(z, k)| z == zone && k == n);
        if !here || !sitting::open(&rounds) {
            continue;
        }
        let newest = rounds.iter().map(|r| r.int("created_at_ms")).max().unwrap_or(0);
        if best.as_ref().is_none_or(|(at, _)| newest > *at) {
            best = Some((newest, sid.to_string()));
        }
    }
    best.map(|(_, s)| s)
}

#[cfg(test)]
mod tests;
