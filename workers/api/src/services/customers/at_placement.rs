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
/// caller answers it 409. `key` is the customer's `customer_key`.
pub async fn allergy_check(
    place: &Place,
    key: &str,
    dishes: &[(String, String)],
) -> Result<std::result::Result<(), String>> {
    let people = crate::hubstore::load_table(place, IMAGE_PEOPLE, PEOPLE_BYTES).await?;
    let card = allergy::of_record(people.table.get(KIND, key).as_deref());
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
pub async fn remember(place: &Place, key: &str, legacy: &str, now_ms: i64) {
    let (key, legacy) = (key.to_string(), legacy.to_string());
    let wrote = crate::hubstore::with_table(place, IMAGE_PEOPLE, PEOPLE_BYTES, move |t| {
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
