//! Activation facts that used to disagree with the code that acts on them.
use super::*;
use dowiz_hub::settings::Settings;

fn circle() -> Value {
    json!({ "kind": "circle", "lat": 41_323_000, "lon": 19_441_000, "radius_m": 3000 })
}

/// `[]` is what `POST /api/owner/zones` stores to CLEAR the area, and
/// `/api/public/reach` answers "unrestricted" for it.
#[test]
fn has_delivery_zones_is_false_for_an_empty_list() {
    assert!(!has_delivery_zones(&json!({ "delivery_zones": [] })));
    assert!(!has_delivery_zones(&json!({})));
    // A list that parses to no zone is no zone, exactly as `reach` reads it.
    assert!(!has_delivery_zones(&json!({ "delivery_zones": [{ "kind": "blob" }] })));
}

#[test]
fn has_delivery_zones_is_true_for_one_circle() {
    assert!(has_delivery_zones(&json!({ "delivery_zones": [circle()] })));
}

#[test]
fn an_empty_zone_list_is_not_delivery_configured() {
    assert!(!delivery_configured(&json!({ "delivery_zones": [] })));
}

#[test]
fn a_circle_or_a_fee_is_delivery_configured() {
    assert!(delivery_configured(&json!({ "delivery_zones": [circle()] })));
    assert!(delivery_configured(&json!({ "delivery_fee": 300 })));
}

fn settings(pairs: &[(&str, &str)]) -> Settings {
    let mut s = Settings::create().expect("settings");
    for (k, v) in pairs {
        s.set(k, v);
    }
    s
}

#[test]
fn activation_counts_a_venue_owned_telegram_bot() {
    let s = settings(&[("notify.telegram.token", "123:abc"), ("notify.telegram.chat", "-100")]);
    assert_eq!(telegram_chats(false, &s), 1);
}

#[test]
fn a_venue_token_without_a_chat_counts_nothing() {
    let s = settings(&[("notify.telegram.token", "123:abc")]);
    assert_eq!(telegram_chats(false, &s), 0);
    // Nor does the platform's bot with nowhere to send.
    assert_eq!(telegram_chats(true, &s), 0);
}

#[test]
fn a_chat_is_heard_through_the_platform_bot_but_not_through_no_bot() {
    let s = settings(&[("notify.telegram.chat", "-100")]);
    assert_eq!(telegram_chats(true, &s), 1);
    assert_eq!(telegram_chats(false, &s), 0);
    // A blank token is no token.
    let blank = settings(&[("notify.telegram.token", "  "), ("notify.telegram.chat", "-100")]);
    assert_eq!(telegram_chats(false, &blank), 0);
}

/// What the console's editor starts from: the stored circles, and `[]` --
/// never null, never a list `reach` would not honour -- when there are none.
#[test]
fn delivery_zones_is_the_stored_list_or_empty() {
    let one = json!({ "delivery_zones": [circle()] });
    assert_eq!(delivery_zones(&one), json!([circle()]));
    assert_eq!(delivery_zones(&json!({})), json!([]));
    assert_eq!(delivery_zones(&json!({ "delivery_zones": [{ "kind": "blob" }] })), json!([]));
}
