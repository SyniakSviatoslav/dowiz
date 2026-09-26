//! The alert once the venue has groups (W-TG): one routed entry carrying the
//! alert in every language the messages are written in, each group reading
//! its own. The legacy chat keeps today's single text (exceptions/tests.rs).
use super::*;
use crate::exceptions::fold::VOID_AFTER_KITCHEN;
use crate::notify::route::{self, Group, Mode};

const T0: i64 = 1_790_000_000_000;
const MIN: i64 = 60_000;

fn voids(n: i64) -> Vec<Row> {
    (0..n)
        .map(|i| Row {
            at: T0 + i * MIN, kind: VOID_AFTER_KITCHEN, order_id: Some(format!("r{i}")), till_id: None,
            reason: Some("mistake".into()), amount: 100, currency: Some("ALL".into()), by: format!("p{i}"), tx_id: None,
        })
        .collect()
}

fn voice() -> Voice<'static> {
    Voice { venue: "v1", zone: dowiz_hub::tz::zone("Europe/Tirane").unwrap(), lang: "sq" }
}

#[test]
fn a_routed_alert_carries_every_language_and_each_group_reads_its_own() {
    let now = T0 + 2 * MIN;
    let got = due(&voids(3), T0, now, 3, &voice(), "@order.exception");
    assert_eq!(got.len(), 1);
    let e = &got[0];
    assert_eq!((e.kind.as_str(), e.to.as_str()), (route::ROUTE_KIND, "order.exception"));
    assert_eq!(e.id, format!("exceptions/{T0}/{VOID_AFTER_KITCHEN}/3"), "the same id: a replayed turn overwrites its own entry");
    let mut groups = Vec::new();
    for lang in route::render::langs() {
        let mut g = Group::fresh(lang.into(), format!("-{lang}"), None, String::new(), String::new(), lang);
        g.subs.insert("order.exception".into(), Mode::Now);
        groups.push(g);
    }
    let sent = route::fan_out(e, &groups, voice().zone, now).send;
    assert_eq!(sent.len(), route::render::langs().len());
    let text = |l: &str| sent.iter().find(|x| x.to == format!("-{l}")).unwrap().text.clone();
    assert!(text("en").contains("3 exceptions · void after kitchen"));
    assert!(text("sq").contains("anulim pas kuzhinës"));
    assert!(text("uk").contains("скасування після кухні"));
    assert!(text("ru").contains("3 исключений · отмена после кухни"), "{}", text("ru"));
}

#[test]
fn the_legacy_chat_gets_the_venues_language_as_before() {
    let got = due(&voids(3), T0, T0 + 2 * MIN, 3, &voice(), "chat-1");
    assert_eq!((got[0].kind.as_str(), got[0].to.as_str()), ("telegram", "chat-1"));
    assert!(got[0].text.contains("anulim pas kuzhinës"));
    assert!(due(&voids(3), T0, T0 + 2 * MIN, 3, &voice(), " ").is_empty(), "no chat, nobody to tell");
}
