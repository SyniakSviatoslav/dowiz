//! THE SMS BOX AT CHECKOUT (W-SMS). PURE.
//!
//! UNTICKED, AND SEPARATE FROM THE OFFERS BOX: placing an order is not
//! consent (Recital 32), and order-status texts are not offers. The box sends
//! `{"sms": {"order_status": true, "wording": <id>}}` only when ticked
//! (`public/store/sms-box.js`); this file turns it into
//!   * the consent act (`order_status` on `sms`, method `checkout_box`,
//!     naming the sentence shown), filed in the venue's consent image once the
//!     order has an id -- the same place and shape as the offers box; and
//!   * the order's STAMP: the customer's consent key, the language, the
//!     venue's name and the number in E.164 -- what the order turn needs to
//!     queue a text and the drain needs to re-ask consent. Kept as a record
//!     (`STAMP_KIND`, keyed by the order) in the venue's OUTBOX image, because
//!     the order log keeps the canonical order and no field it does not know;
//!     removed when the order ends (`hubdo/sms_turn.rs`) or after `STAMP_TTL_MS`.
//!
//! A TICK THE HUB CANNOT ACT ON IS REFUSED BY NAME, before the order exists:
//! no number, a number that is not international and not from the venue's
//! country, or a sentence this build cannot show.

use serde::Deserialize;
use serde_json::{json, Value};

use dowiz_hub::consent::sms_wordings::lang_of_sms_wording;
use dowiz_hub::consent::{Act, Method, State, CHANNEL_SMS, PURPOSE_ORDER_STATUS};

/// The box, as the order body carries it. A CLOSED shape.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct SmsIn {
    pub order_status: bool,
    /// The id of the sentence shown (`sms_wording_id`).
    pub wording: String,
}

/// What a ticked box becomes: the act to file and the order's stamp.
#[derive(Debug, PartialEq, Eq)]
pub struct Ticked {
    pub act: Act,
    pub stamp: Value,
}

/// `Ok(None)` for no box or an unticked one. `currency` is the venue's (ALL
/// completes `069...` as +355); `key_of` is `customer_key` under the venue's
/// secret, applied to the E.164 number -- ONE spelling, so the owner's STOP
/// (`routes::stop`) finds the same key however either of them typed it.
pub fn at_placement(c: Option<&SmsIn>, phone: &str, currency: &str, venue: &str, now_ms: i64, key_of: impl Fn(&str) -> String) -> Result<Option<Ticked>, String> {
    let Some(c) = c.filter(|c| c.order_status) else { return Ok(None) };
    if phone.trim().is_empty() {
        return Err("the SMS box needs a phone number to text".into());
    }
    let Some(to) = super::phone::e164(phone, currency) else {
        return Err("the SMS box needs a full phone number, like +355 69 123 4567".into());
    };
    let key = key_of(&to);
    let key = key.as_str();
    let Some(lang) = lang_of_sms_wording(&c.wording) else {
        return Err("the SMS sentence on this page is out of date; reload and tick it again".into());
    };
    let act = Act {
        key: key.to_string(),
        purpose: PURPOSE_ORDER_STATUS.into(),
        channel: CHANNEL_SMS.into(),
        state: State::Given,
        at_ms: now_ms,
        method: Method::CheckoutBox,
        evidence: String::new(),
        wording_id: c.wording.clone(),
        via: String::new(),
    };
    dowiz_hub::consent::check(&act)?;
    let stamp = json!({ "key": key, "lang": lang, "venue": super::words::venue_short(venue), "to": to, "at_ms": now_ms });
    Ok(Some(Ticked { act, stamp }))
}

/// The outbox record kind of a stamp; its id is the order's.
pub const STAMP_KIND: &str = "sms_o";
/// A stamp outlives no order by more than this (an order nobody finished).
pub const STAMP_TTL_MS: i64 = 2 * 86_400_000;

/// After the order is written: file the act (naming the order) and keep the
/// stamp. Neither can fail the order; a failure is loud in `consent_log::file`
/// and here.
pub async fn keep(place: &crate::hubstore::Place, order_id: &str, mut t: Ticked) {
    t.act.via = order_id.to_string();
    let _ = crate::services::customers::consent_log::file(place, &t.act).await;
    let (id, rec) = (order_id.to_string(), t.stamp.to_string());
    let r = crate::hubstore::with_table(place, crate::outbox::IMAGE_OUTBOX, crate::outbox::OUTBOX_BYTES, move |tb| {
        tb.put(STAMP_KIND, &id, &rec, &[], &[]).map_err(|e| worker::Error::RustError(format!("outbox: {e:?}")))
    })
    .await;
    if let Err(e) = r {
        crate::loud!(&place.ns, Some(&place.venue), "sms.stamp", "order {order_id}: the SMS box was ticked and its stamp was NOT kept: {e}");
    }
}

/// Is a kept stamp past its life? PURE.
pub fn stale(stamp: &str, now_ms: i64) -> bool {
    serde_json::from_str::<Value>(stamp).ok().and_then(|v| v.get("at_ms").and_then(Value::as_i64)).is_none_or(|at| now_ms - at > STAMP_TTL_MS)
}

/// The STOP an owner files for a customer (`POST /api/owner/sms/stop`).
/// Art. 7(3): as easy as the tick -- a withdrawal needs nothing but the person.
pub fn stop_act(key: &str, by: &str, now_ms: i64) -> Act {
    Act {
        key: key.to_string(),
        purpose: PURPOSE_ORDER_STATUS.into(),
        channel: CHANNEL_SMS.into(),
        state: State::Withdrawn,
        at_ms: now_ms,
        method: Method::OwnerEntered,
        evidence: String::new(),
        wording_id: String::new(),
        via: by.to_string(),
    }
}

#[cfg(test)]
mod tests;
