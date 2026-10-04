//! The bag link, the landing word and the owner's card, over synthetic logs.

use super::*;

const VENUE: &str = "loc_1";
const T: i64 = 1_790_000_000_000;

fn order(i: i64, phone: Option<&str>, status: &str, bag: Option<Option<&str>>, channel: &str, total: i64) -> OrderView {
    let mut o = json!({
        "id": format!("o{i}"), "location_id": VENUE, "status": status, "total": total,
        "created_at_ms": T + i * 60_000, "channel": channel,
    });
    if let Some(p) = phone {
        o["contact"] = json!({ "phone": p });
    }
    if let Some(c) = bag {
        stamp(&mut o, c);
    }
    OrderView { order_id: format!("o{i}"), kind: 1, seq: 0, order_json: o.to_string() }
}

/// Digits only, as the venue's resolver collapses `+355 69…` and `00355 69…`.
fn key(p: &str) -> String {
    p.chars().filter(char::is_ascii_digit).collect::<String>().trim_start_matches("00").to_string()
}

#[test]
fn the_link_is_the_venue_host_and_nothing_per_guest() {
    assert_eq!(landing_url("dubin-sushi.dowiz.org", None), "https://dubin-sushi.dowiz.org/?src=bag");
    assert_eq!(landing_url("dubin-sushi.dowiz.org", Some("spring")), "https://dubin-sushi.dowiz.org/?src=bag&c=spring");
}

#[test]
fn a_campaign_is_a_short_plain_word() {
    assert_eq!(campaign(None), Ok(None));
    assert_eq!(campaign(Some("  ")), Ok(None));
    assert_eq!(campaign(Some("wolt-oct2")), Ok(Some("wolt-oct2".into())));
    for bad in ["Spring", "a b", "a&src=x", "x/../y", &"a".repeat(CAMPAIGN_MAX + 1)] {
        assert!(campaign(Some(bad)).is_err(), "{bad}");
    }
}

#[test]
fn only_the_bag_word_is_a_landing() {
    assert_eq!(source(None), Ok(None));
    assert_eq!(source(Some(&SrcIn { src: "bag".into(), c: Some("spring".into()) })), Ok(Some(Some("spring".into()))));
    assert!(source(Some(&SrcIn { src: "wolt".into(), c: None })).is_err());
    assert!(source(Some(&SrcIn { src: "bag".into(), c: Some("x y".into()) })).is_err());
    assert!(serde_json::from_value::<SrcIn>(json!({"src": "bag", "fp": "abc"})).is_err(), "strict");
}

#[test]
fn the_card_counts_bag_orders_guests_and_their_repeat_orders() {
    let a = "+355691111111";
    let a2 = "00355691111111"; // the same guest, spelled the other way
    let b = "+355692222222";
    let log = vec![
        order(0, Some(a), "DELIVERED", None, "storefront", 999),             // before the bag: not counted
        order(1, Some(a), "DELIVERED", Some(Some("spring")), "storefront", 2000),
        order(2, Some(a2), "DELIVERED", None, "storefront", 1500),           // repeat
        order(3, Some(a), "REJECTED", None, "storefront", 7000),             // took no money
        order(4, Some(b), "PICKED_UP", Some(None), "storefront", 1000),
        order(5, None, "DELIVERED", Some(Some("spring")), "storefront", 800), // a bag order, no phone
        order(6, Some(b), "DELIVERED", None, "wolt", 5000),                  // a marketplace order: not direct
        order(7, Some(b), "DELIVERED", Some(None), "storefront", 1200),      // a second scan is a repeat
    ];
    let st = stats(&log, VENUE, key, None);
    assert_eq!((st.orders, st.guests, st.repeat), (4, 2, 2), "{st:?}");
    assert_eq!(st.direct_total, 2000 + 1500 + 1000 + 1200);
    assert_eq!(st.saved, None, "no percent typed, none invented");
    assert_eq!(st.by_campaign.get("spring"), Some(&2));
    assert_eq!(st.by_campaign.get(""), Some(&2));
    let st = stats(&log, VENUE, key, Some(30));
    assert_eq!(st.saved, Some(5700 * 30 / 100));
    assert_eq!(st.to_json()["scans"], Value::Null, "scans are not counted, and the card says so");
}

#[test]
fn another_venue_is_not_counted() {
    let log = vec![order(1, Some("+355691111111"), "DELIVERED", Some(None), "storefront", 2000)];
    assert_eq!(stats(&log, "loc_other", key, None), Stats::default());
}
