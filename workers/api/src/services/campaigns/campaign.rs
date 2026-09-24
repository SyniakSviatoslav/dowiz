//! THE `campaign` LOG: what was defined, who was sent it, what never arrived.
//!
//! §2.5/§3.6 of BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22. One append-only
//! image per venue, three kinds:
//!
//!   `def`  subject = campaign id   — the campaign as the owner wrote it; the
//!                                     newest `def` of an id is the campaign.
//!   `sent` subject = customer key  — one per person per campaign, written
//!                                     when the outbox entry is queued, with the
//!                                     consent it was queued under (when they
//!                                     said yes, which sentence). `about("sent",
//!                                     key)` answers "what has this person been
//!                                     sent" in one read.
//!   `gone` subject = campaign id   — an entry the drain removed without
//!                                     delivering: `withdrawn` (the fold said
//!                                     no at send time, G4) or `abandoned`
//!                                     (six failures, `outbox::MAX_TRIES`).
//!
//! NO RECIPIENT LIST IS STORED. `sent` rows exist only for people who were
//! actually queued, which is the send history the law asks the venue to keep,
//! not a segment frozen in time.
//!
//! PURE: every function here is a fold over `logimage::Entry` or a check of a
//! request body.

use std::collections::BTreeSet;

use dowiz_hub::logimage::Entry;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::segment::Segment;
use super::template::{self, Template};

pub const IMAGE_CAMPAIGN: &str = "campaign";
pub const KIND_DEF: &str = "def";
pub const KIND_SENT: &str = "sent";
pub const KIND_GONE: &str = "gone";

/// The only channel with a consent act today (`consent_log::at_placement`
/// files WhatsApp). A Telegram or Instagram campaign needs a recipient address
/// the venue does not hold for a customer, so it is not offered.
pub const CHANNEL: &str = dowiz_hub::consent::CHANNEL_WHATSAPP;

pub const NAME_MAX: usize = 60;
/// WhatsApp's body limit is 4096; a campaign is a short message and the STOP
/// line is appended after it.
pub const TEXT_MAX: usize = 1000;

/// META BILLS A MARKETING MESSAGE PER DELIVERY, in USD, by the recipient's
/// country. The rate card entry for Albania ("Rest of Central & Eastern
/// Europe", marketing) read 0.0860 USD when this was written (2026-09);
/// MICRO-dollars so the arithmetic stays integer. An ESTIMATE shown before a
/// send, never a charge -- the bill is Meta's.
pub const MICRO_USD_PER_MESSAGE: i64 = 86_000;
pub const COST_CURRENCY: &str = "USD";

/// What the owner writes. A CLOSED shape.
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct DefIn {
    /// Present to edit a campaign that has not been sent.
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    pub text: String,
    pub segment: Segment,
    /// A promo code whose redemptions are the campaign's only measure (§2.5).
    #[serde(default)]
    pub promo: Option<String>,
    /// The approved WhatsApp template (`template.rs`). Optional while the
    /// campaign is a draft; preview and send refuse one without it.
    #[serde(default)]
    pub template: Option<Template>,
}

/// One campaign, as it is filed.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Def {
    pub id: String,
    pub name: String,
    pub text: String,
    pub segment: Segment,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promo: Option<String>,
    /// Absent on a draft, and on every `def` filed before templates existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<Template>,
    pub channel: String,
    #[serde(rename = "atMs")]
    pub at_ms: i64,
    pub by: String,
}

/// A new campaign's id: the instant, in hex. Short, orderable, and free of
/// `:` -- the outbox entry id is `camp:<id>:<key>`.
pub fn new_id(now_ms: i64) -> String {
    format!("c{now_ms:x}")
}

fn id_ok(id: &str) -> bool {
    (2..=24).contains(&id.len()) && id.starts_with('c') && id[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

/// The owner's body → the record to file, or the refusal by name.
/// `existing` is the log's entries (to refuse editing a sent campaign).
pub fn define(body: DefIn, existing: &[Entry], by: &str, now_ms: i64) -> Result<Def, String> {
    let name = body.name.trim().to_string();
    let text = body.text.trim().to_string();
    if name.is_empty() || name.chars().count() > NAME_MAX {
        return Err(format!("a campaign name is 1 to {NAME_MAX} characters"));
    }
    if text.is_empty() || text.chars().count() > TEXT_MAX {
        return Err(format!("the message is 1 to {TEXT_MAX} characters"));
    }
    body.segment.check()?;
    if let Some(t) = &body.template {
        template::check(t)?;
    }
    let promo = match body.promo.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        None => None,
        Some(p) => {
            let code = dowiz_hub::promo::normalise(p);
            if !dowiz_hub::promo::valid_code(&code) {
                return Err(format!("{p:?} is not a promo code"));
            }
            Some(code)
        }
    };
    let id = match body.id {
        // TWO CAMPAIGNS IN ONE MILLISECOND would share an id and the second
        // would silently become an edit of the first.
        None if def_of(existing, &new_id(now_ms)).is_some() => return Err("try again".into()),
        None => new_id(now_ms),
        Some(id) => {
            if !id_ok(&id) || def_of(existing, &id).is_none() {
                return Err("no such campaign".into());
            }
            // A SENT CAMPAIGN IS HISTORY. Editing its text afterwards would
            // make the log say people were sent words they never saw.
            if !sent_to(existing, &id).is_empty() {
                return Err("this campaign has been sent; make a new one".into());
            }
            id
        }
    };
    Ok(Def { id, name, text, segment: body.segment, promo, template: body.template, channel: CHANNEL.into(), at_ms: now_ms, by: by.into() })
}

/// Every campaign, newest definition of each, newest campaign first.
pub fn defs(entries: &[Entry]) -> Vec<Def> {
    let mut seen = BTreeSet::new();
    entries
        .iter()
        .filter(|e| e.kind == KIND_DEF && seen.insert(e.subject.clone()))
        .filter_map(|e| serde_json::from_str::<Def>(&e.json).ok())
        .collect()
}

pub fn def_of(entries: &[Entry], id: &str) -> Option<Def> {
    entries
        .iter()
        .find(|e| e.kind == KIND_DEF && e.subject == id)
        .and_then(|e| serde_json::from_str::<Def>(&e.json).ok())
}

/// One `sent` row. `via` is the key the consent was filed under (a linked
/// spelling may be the one that said yes); `consentAt`/`wording` are the
/// proof the entry was queued under, copied so the history survives alone.
pub fn sent_row(campaign: &str, key: &str, via: &str, entry: &str, consent_at: i64, wording: &str, now_ms: i64) -> String {
    serde_json::json!({
        "campaign": campaign, "key": key, "via": via, "entry": entry,
        "consentAt": consent_at, "wording": wording, "atMs": now_ms,
    })
    .to_string()
}

fn field<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}

/// The customer keys a campaign was queued to. A SET: two presses racing can
/// each file a row, and a person is still one recipient.
pub fn sent_to(entries: &[Entry], campaign: &str) -> BTreeSet<String> {
    entries
        .iter()
        .filter(|e| e.kind == KIND_SENT)
        .filter_map(|e| serde_json::from_str::<Value>(&e.json).ok())
        .filter(|v| field(v, "campaign") == Some(campaign))
        .filter_map(|v| field(&v, "key").map(str::to_string))
        .collect()
}

/// The outbox entry ids the `sent` rows of a campaign name.
pub fn sent_entries(entries: &[Entry], campaign: &str) -> BTreeSet<String> {
    entries
        .iter()
        .filter(|e| e.kind == KIND_SENT)
        .filter_map(|e| serde_json::from_str::<Value>(&e.json).ok())
        .filter(|v| field(v, "campaign") == Some(campaign))
        .filter_map(|v| field(&v, "entry").map(str::to_string))
        .collect()
}

/// When the campaign was first queued, if ever: attribution counts from here.
pub fn first_sent_at(entries: &[Entry], campaign: &str) -> Option<i64> {
    entries
        .iter()
        .filter(|e| e.kind == KIND_SENT)
        .filter_map(|e| serde_json::from_str::<Value>(&e.json).ok())
        .filter(|v| field(v, "campaign") == Some(campaign))
        .filter_map(|v| v.get("atMs").and_then(Value::as_i64))
        .min()
}

pub fn gone_row(campaign: &str, entry: &str, why: &str, after: u32, now_ms: i64) -> String {
    serde_json::json!({ "campaign": campaign, "entry": entry, "why": why, "after": after, "atMs": now_ms })
        .to_string()
}

/// The report the owner reads: how many queued, still waiting, withdrawn
/// before delivery, abandoned after six failures -- and the rest delivered.
#[derive(Serialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    pub sent: usize,
    pub waiting: usize,
    pub withdrawn: usize,
    pub abandoned: usize,
    pub delivered: usize,
    pub redeemed: i64,
}

/// `waiting` is how many of this campaign's entries the outbox still holds.
pub fn report(entries: &[Entry], campaign: &str, waiting: usize, redeemed: i64) -> Report {
    let gone: BTreeSet<(String, String)> = entries
        .iter()
        .filter(|e| e.kind == KIND_GONE && e.subject == campaign)
        .filter_map(|e| serde_json::from_str::<Value>(&e.json).ok())
        .filter_map(|v| Some((field(&v, "entry")?.to_string(), field(&v, "why")?.to_string())))
        .collect();
    let count = |w: &str| gone.iter().filter(|(_, why)| why == w).count();
    let sent = sent_to(entries, campaign).len();
    let (withdrawn, abandoned) = (count("withdrawn"), count("abandoned"));
    let delivered = sent.saturating_sub(waiting + withdrawn + abandoned);
    Report { sent, waiting, withdrawn, abandoned, delivered, redeemed }
}

/// ATTRIBUTION (§2.5): orders placed with the campaign's code at or after the
/// first send, that the venue took money for. A redemption is the only
/// measure; nothing tracks who opened what.
pub fn redeemed(orders: &[Value], code: &str, since_ms: i64) -> i64 {
    orders
        .iter()
        .filter(|o| o.get("promo").and_then(|p| p.get("code")).and_then(Value::as_str) == Some(code))
        .filter(|o| o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0) >= since_ms)
        .filter(|o| crate::services::orders::status::took_money(o.get("status").and_then(Value::as_str).unwrap_or("")))
        .count() as i64
}

/// What the owner sees before pressing send.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    pub count: usize,
    /// How many of `count` were already sent this campaign (they are skipped).
    pub already: usize,
    pub cost_minor: i64,
    pub currency: &'static str,
    pub channel: &'static str,
}

/// The estimate, in the currency's minor units (cents), rounded UP.
pub fn cost_minor(messages: usize) -> i64 {
    (messages as i64 * MICRO_USD_PER_MESSAGE + 9_999) / 10_000
}

pub fn preview(count: usize, already: usize) -> Preview {
    Preview {
        count,
        already,
        cost_minor: cost_minor(count.saturating_sub(already)),
        currency: COST_CURRENCY,
        channel: CHANNEL,
    }
}

#[cfg(test)]
mod tests;
