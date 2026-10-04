use super::*;
use dowiz_hub::consent::sms_wordings::sms_wording_id;
use dowiz_hub::consent::wording_id;

/// The key, visibly a function of the E.164 number it was given.
fn k(e164: &str) -> String {
    format!("key({e164})")
}

fn tick(lang: &str) -> SmsIn {
    SmsIn { order_status: true, wording: sms_wording_id(lang) }
}

#[test]
fn no_box_or_an_unticked_one_is_nothing() {
    assert_eq!(at_placement(None, "+355691234567", "ALL", "V", 5, k), Ok(None));
    let un = SmsIn { order_status: false, wording: sms_wording_id("en") };
    assert_eq!(at_placement(Some(&un), "+355691234567", "ALL", "V", 5, k), Ok(None));
}

#[test]
fn a_tick_becomes_the_act_and_the_stamp() {
    let t = at_placement(Some(&tick("uk")), "069 123 4567", "ALL", "Sushi Durrës", 5, k).unwrap().unwrap();
    assert_eq!((t.act.purpose.as_str(), t.act.channel.as_str(), t.act.state), (PURPOSE_ORDER_STATUS, CHANNEL_SMS, State::Given));
    assert_eq!((t.act.method, t.act.at_ms, t.act.wording_id.clone()), (Method::CheckoutBox, 5, sms_wording_id("uk")));
    assert_eq!(t.stamp, json!({"key": "key(+355691234567)", "lang": "uk", "venue": "Sushi Durrës", "to": "+355691234567", "at_ms": 5}));
    assert!(!stale(&t.stamp.to_string(), 5 + STAMP_TTL_MS), "alive for its whole life");
    assert!(stale(&t.stamp.to_string(), 6 + STAMP_TTL_MS), "and gone after it");
    assert!(stale("not json", 0), "an unreadable stamp is removed, not kept");
}

#[test]
fn a_tick_the_hub_cannot_act_on_is_refused_by_name() {
    assert!(at_placement(Some(&tick("en")), "", "ALL", "V", 5, k).unwrap_err().contains("phone number"));
    assert!(at_placement(Some(&tick("en")), "069 123 4567", "EUR", "V", 5, k).unwrap_err().contains("full phone number"));
    let offers = SmsIn { order_status: true, wording: wording_id("en") };
    assert!(at_placement(Some(&offers), "+355691234567", "ALL", "V", 5, k).unwrap_err().contains("out of date"), "the offers sentence is not the SMS sentence");
}

#[test]
fn the_owners_stop_is_a_withdrawal_that_needs_nothing_else() {
    let a = stop_act("k1", "owner_7", 9);
    assert_eq!((a.state, a.method, a.via.as_str()), (State::Withdrawn, Method::OwnerEntered, "owner_7"));
    assert!(dowiz_hub::consent::check(&a).is_ok());
}
