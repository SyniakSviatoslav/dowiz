//! What a PLACEMENT reads and writes of the customer's card (§3.1).
//!
//! I/O ONLY. The rules are `allergy::refuse` and `record::touch`, both pure
//! and tested natively; this file fetches the card and files the result.

use worker::*;

use super::allergy;
use super::record::{touch, KIND};
use crate::hubstore::{Place, IMAGE_PEOPLE, PEOPLE_BYTES};

/// Refuse a basket that holds a dish the customer's card says will hurt them.
///
/// `Ok(Err(reason))` is the refusal, already RECORDED in the venue's error log
/// with its reason -- a refusal is a record, never a silent default -- and the
/// caller answers it 409. The card read is every card of the person (§3.4,
/// `alias::allergens_of`), including the link this placement will write.
pub async fn allergy_check(
    place: &Place,
    secret: &[u8],
    phone: &str,
    dishes: &[(String, String)],
) -> Result<std::result::Result<(), String>> {
    // No phone, no card: the empty key is every phone-less guest at once.
    if !super::roll::names_a_person(phone) {
        return Ok(Ok(()));
    }
    let key = super::handlers::customer_key(secret, phone);
    let pending = super::identity::alias_at_placement(secret, phone);
    let people = crate::hubstore::load_table(place, IMAGE_PEOPLE, PEOPLE_BYTES).await?;
    let card = super::alias::allergens_of(&people.table, &key, pending.as_deref());
    match allergy::refuse(&card, dishes) {
        Ok(()) => Ok(Ok(())),
        Err(why) => {
            crate::loud!(&place.ns, Some(&place.venue), "storefront.allergy", "cust:{key} refused: {why}");
            Ok(Err(why))
        }
    }
}

/// File the card's birth for this customer, and drop the row the old
/// placement wrote under `legacy` (the unkeyed `sha256_hex(phone)`).
///
/// AFTER THE ORDER, AND IT CANNOT FAIL IT: the order is already in the log.
/// A failure is recorded, loudly, rather than swallowed as `let _` was.
///
/// And THE RULE-SITE of §3.4 in the same turn: `alias_to` is the E.164
/// spelling's key when this order's phone was typed another way
/// (`identity::alias_at_placement`). The alias is written BEFORE the card, because the
/// card's absence is what says this spelling is appearing for the first time
/// (`alias::rule_link`).
pub async fn remember_spelling(place: &Place, key: &str, legacy: &str, alias_to: Option<String>, now_ms: i64) {
    let (key, legacy) = (key.to_string(), legacy.to_string());
    let wrote = crate::hubstore::with_table(place, IMAGE_PEOPLE, PEOPLE_BYTES, move |t| {
        if let Some(to) = &alias_to {
            super::alias::rule_link(t, &key, to, now_ms);
        }
        if let Some(rec) = touch(t.get(KIND, &key).as_deref(), now_ms) {
            t.put(KIND, &key, &rec, &[], &[])
                .map_err(|e| Error::RustError(format!("customer card: {e:?}")))?;
        }
        // AND THE ENUMERABLE ROW GOES. `remove` answering false is the ordinary
        // case -- most customers have no legacy row.
        if legacy != key {
            t.remove(KIND, &legacy);
        }
        Ok(())
    })
    .await;
    if let Err(e) = wrote {
        crate::loud!(&place.ns, Some(&place.venue), "storefront.card", "not filed: {e}");
    }
}
