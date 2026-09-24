//! WHO A CAMPAIGN REACHES, computed at send time and held for one request.
//!
//! The join of §2.5: the customer fold (`roll`, grouped by alias so a person
//! linked under two spellings is ONE row and gets ONE message), the consent
//! fold (`consent::state`, which alone can mint the `Consented` witness), and
//! the cards (tags, birthday, language). A `Recipient` CARRIES the witness:
//! there is no way to build one for somebody the fold did not say yes for.
//!
//! PURE. The key derivation, the alias resolution and the card lookup are
//! passed in, the way `roll` takes them, so this is tested without a secret
//! or a Durable Object.

use std::collections::BTreeMap;

use dowiz_hub::consent::{self, Consented, PURPOSE_MARKETING};
use dowiz_hub::logimage::Entry;
use serde_json::Value;

use super::campaign::CHANNEL;
use super::segment::{matches, ConsentState, Now, Record, Segment};
use crate::services::customers::identity::{canonical_digits, VENUE_DIAL};
use crate::services::customers::roll::{roll, Row, Sort};

/// One person a campaign may be sent to.
pub struct Recipient {
    /// The row's (canonical) key: what `sent` is filed under.
    pub key: String,
    /// The WhatsApp address: E.164 digits, no `+`.
    pub to: String,
    /// The language the message's STOP line is written in.
    pub lang: String,
    /// The proof. Private fields, no constructor: only `consent::state`.
    pub witness: Consented,
}

/// Every key's newest spelling of the number, from the orders.
pub fn phones_by_key(orders: &[Value], key_of: impl Fn(&str) -> String) -> BTreeMap<String, String> {
    let mut out: BTreeMap<String, (i64, String)> = BTreeMap::new();
    for o in orders {
        let Some(phone) = o.get("contact").and_then(|c| c.get("phone")).and_then(Value::as_str) else {
            continue;
        };
        let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        let slot = out.entry(key_of(phone)).or_insert((i64::MIN, String::new()));
        if at >= slot.0 {
            *slot = (at, phone.to_string());
        }
    }
    out.into_iter().map(|(k, (_, p))| (k, p)).collect()
}

/// A typed phone → WhatsApp's `to`: the venue's E.164 digits when the number
/// normalises, else its digits without an international `00`.
pub fn address(phone: &str) -> String {
    canonical_digits(phone, VENUE_DIAL).unwrap_or_else(|| {
        let d: String = phone.chars().filter(char::is_ascii_digit).collect();
        d.strip_prefix("00").unwrap_or(&d).to_string()
    })
}

/// The recipients of `seg`, one per PERSON.
///
/// THE WITNESS IS LOOKED FOR ACROSS THE PERSON'S KEYS: the row's own, then
/// every spelling linked into it (`members`). Consent filed under either
/// spelling is the person's consent -- and the message goes to the spelling
/// that gave it, the number they said yes on.
pub fn recipients(
    orders: &[Value],
    key_of: impl Fn(&str) -> String,
    resolve: impl Fn(&str) -> String,
    members: impl Fn(&str) -> Vec<String>,
    card: impl Fn(&str) -> Option<String>,
    acts: &[Entry],
    seg: &Segment,
    now: Now,
) -> Vec<Recipient> {
    let phones = phones_by_key(orders, &key_of);
    let rows: Vec<Row> = roll(orders, &key_of, resolve, |n| n.to_string(), |p| p.to_string(), Sort::Recent);
    let mut out = Vec::new();
    for row in &rows {
        let mut keys = vec![row.key.clone()];
        keys.extend(members(&row.key));
        let witness = keys.iter().find_map(|k| consent::state(acts, k, PURPOSE_MARKETING, CHANNEL));
        let cards: Vec<String> = keys.iter().filter_map(|k| card(k)).collect();
        let rec = Record::of_cards(cards.iter().map(String::as_str));
        let given = ConsentState { given: witness.is_some() };
        if !matches(seg, row, &rec, &given, now) {
            continue;
        }
        let Some(witness) = witness else { continue };
        let phone = phones.get(witness.key()).unwrap_or(&row.phone);
        let lang = rec
            .lang
            .clone()
            .or_else(|| consent::log::lang_of_wording(witness.wording_id()).map(str::to_string))
            .unwrap_or_else(|| "sq".into());
        out.push(Recipient { key: row.key.clone(), to: address(phone), lang, witness });
    }
    out
}

#[cfg(test)]
mod tests;
