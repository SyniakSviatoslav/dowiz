//! The venue's own record: what it is called, what it charges in, when it is
//! open.

pub mod activation;
pub mod brand;
pub mod brand_extract;
pub mod place;
pub mod settings;
pub mod where_when;
pub mod zones;

use serde_json::Value;

/// The currency the venue's prices are in.
///
/// `currency_code` or `currency`, and ALL if the record says neither -- a
/// venue with no currency is a venue whose prices are still lek, not one whose
/// prices have no unit. A blank unit on a price is the kind of thing that gets
/// a number read as euros.
pub fn currency_of(cat: &dowiz_hub::catalog::Catalog) -> String {
    currency_of_record(cat.location().and_then(|j| serde_json::from_str::<Value>(&j).ok()).as_ref())
}

/// The same rule over the venue's record alone -- what `/fold/venue` answers
/// (`hubstore::venue_record`) -- for a reader that does not hold the
/// catalogue (BN1: the tips and the wallet legs asked for 538 KB to read
/// this one field).
pub fn currency_of_record(record: Option<&Value>) -> String {
    record
        .and_then(|l| {
            l.get("currency_code")
                .or_else(|| l.get("currency"))
                .and_then(Value::as_str)
                .map(String::from)
        })
        .unwrap_or_else(|| "ALL".into())
}
