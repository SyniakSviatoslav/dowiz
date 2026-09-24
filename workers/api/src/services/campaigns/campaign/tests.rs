//! The campaign log's folds and the owner's body, each refusal with its twin.

use super::*;
use dowiz_hub::logimage::LogImage;
use serde_json::json;

fn body(v: serde_json::Value) -> DefIn {
    serde_json::from_value(v).expect("a well-formed body")
}

fn good() -> DefIn {
    body(json!({ "name": "Autumn", "text": "Soup is back.", "segment": { "kind": "everyone_consented" }, "promo": "soup10" }))
}

fn log_with(defs: &[&Def], sent: &[(&str, &str, i64)]) -> Vec<dowiz_hub::logimage::Entry> {
    let mut log = LogImage::create().unwrap();
    for d in defs {
        log.append(KIND_DEF, &d.id, &serde_json::to_string(d).unwrap()).unwrap();
    }
    for (c, k, at) in sent {
        log.append(KIND_SENT, k, &sent_row(c, k, k, &format!("camp:{c}:{k}"), 1, "w", *at)).unwrap();
    }
    log.entries()
}

#[test]
fn a_campaign_is_defined_with_its_code_normalised_and_the_channel_fixed() {
    let d = define(good(), &[], "owner-1", 1_000).unwrap();
    assert_eq!(d.id, "c3e8");
    assert_eq!(d.promo.as_deref(), Some("SOUP10"));
    assert_eq!(d.channel, "whatsapp");
    assert_eq!(d.by, "owner-1");
}

#[test]
fn the_body_is_refused_by_name_and_its_twin_passes() {
    assert!(define(good(), &[], "o", 1).is_ok());
    let empty = body(json!({ "name": " ", "text": "x", "segment": { "kind": "everyone_consented" } }));
    assert!(define(empty, &[], "o", 1).unwrap_err().contains("name"));
    let long = body(json!({ "name": "n", "text": "x".repeat(TEXT_MAX + 1), "segment": { "kind": "everyone_consented" } }));
    assert!(define(long, &[], "o", 1).unwrap_err().contains("message"));
    let unlisted = body(json!({ "name": "n", "text": "x", "segment": { "kind": "tag", "tag": "gold-member" } }));
    assert!(define(unlisted, &[], "o", 1).unwrap_err().contains("closed list"));
    let bad_code = body(json!({ "name": "n", "text": "x", "segment": { "kind": "everyone_consented" }, "promo": "x!" }));
    assert!(define(bad_code, &[], "o", 1).is_err());
    assert!(serde_json::from_value::<DefIn>(json!({ "name": "n", "text": "x", "segment": { "kind": "everyone_consented" }, "to": ["+355"] })).is_err(),
        "a recipient list in the body is refused at the door");
}

#[test]
fn an_unsent_campaign_is_edited_and_a_sent_one_is_history() {
    let d = define(good(), &[], "o", 1_000).unwrap();
    let edit = || body(json!({ "id": d.id, "name": "Autumn 2", "text": "Soup!", "segment": { "kind": "everyone_consented" } }));
    let unsent = log_with(&[&d], &[]);
    let e = define(edit(), &unsent, "o", 2_000).unwrap();
    assert_eq!((e.id.as_str(), e.name.as_str()), (d.id.as_str(), "Autumn 2"));
    let sent = log_with(&[&d], &[(&d.id, "k1", 1_500)]);
    assert!(define(edit(), &sent, "o", 2_000).unwrap_err().contains("sent"));
    let ghost = body(json!({ "id": "c999", "name": "n", "text": "x", "segment": { "kind": "everyone_consented" } }));
    assert!(define(ghost, &unsent, "o", 2_000).is_err(), "no such campaign");
}

#[test]
fn two_campaigns_in_one_millisecond_are_refused_not_merged() {
    let d = define(good(), &[], "o", 1_000).unwrap();
    let log = log_with(&[&d], &[]);
    assert!(define(good(), &log, "o", 1_000).is_err());
    assert!(define(good(), &log, "o", 1_001).is_ok());
}

#[test]
fn the_newest_def_of_an_id_is_the_campaign() {
    let a = define(good(), &[], "o", 1_000).unwrap();
    let mut a2 = a.clone();
    a2.name = "renamed".into();
    let b = define(good(), &[], "o", 5_000).unwrap();
    let log = log_with(&[&a, &a2, &b], &[]);
    let all = defs(&log);
    assert_eq!(all.len(), 2);
    assert_eq!(def_of(&log, &a.id).unwrap().name, "renamed");
}

#[test]
fn a_person_reached_twice_is_one_recipient_and_the_report_adds_up() {
    let d = define(good(), &[], "o", 1_000).unwrap();
    let mut log = LogImage::create().unwrap();
    log.append(KIND_DEF, &d.id, &serde_json::to_string(&d).unwrap()).unwrap();
    for (k, at) in [("k1", 10), ("k2", 11), ("k1", 12), ("k3", 13), ("k4", 14)] {
        log.append(KIND_SENT, k, &sent_row(&d.id, k, k, &format!("camp:{}:{k}", d.id), 1, "w", at)).unwrap();
    }
    log.append(KIND_SENT, "k9", &sent_row("cother", "k9", "k9", "camp:cother:k9", 1, "w", 5)).unwrap();
    log.append(KIND_GONE, &d.id, &gone_row(&d.id, "camp:x:k2", "withdrawn", 0, 20)).unwrap();
    log.append(KIND_GONE, &d.id, &gone_row(&d.id, "camp:x:k3", "abandoned", 6, 21)).unwrap();
    let e = log.entries();
    assert_eq!(sent_to(&e, &d.id).len(), 4, "k1 twice is one person; another campaign's k9 is not ours");
    assert_eq!(first_sent_at(&e, &d.id), Some(10));
    let r = report(&e, &d.id, 1, 7);
    assert_eq!(r, Report { sent: 4, waiting: 1, withdrawn: 1, abandoned: 1, delivered: 1, redeemed: 7 });
    assert_eq!(first_sent_at(&e, "cnever"), None);
}

#[test]
fn a_redemption_counts_only_after_the_send_and_only_money_taken() {
    let o = |code: &str, at: i64, st: &str| json!({ "promo": { "code": code }, "created_at_ms": at, "status": st });
    let orders = [
        o("SOUP10", 100, "DELIVERED"),
        o("SOUP10", 200, "CONFIRMED"),
        o("SOUP10", 50, "DELIVERED"),  // before the send
        o("SOUP10", 300, "REJECTED"),  // the venue took nothing
        o("OTHER", 400, "DELIVERED"),
        json!({ "created_at_ms": 500, "status": "DELIVERED" }),
    ];
    assert_eq!(redeemed(&orders, "SOUP10", 100), 2);
    assert_eq!(redeemed(&orders, "SOUP10", 0), 3);
}

#[test]
fn the_cost_is_integer_and_rounds_up() {
    assert_eq!(cost_minor(0), 0);
    assert_eq!(cost_minor(1), 9, "8.6 cents is shown as 9, never as less than it may cost");
    assert_eq!(cost_minor(100), 860);
    let p = preview(10, 4);
    assert_eq!((p.count, p.already, p.cost_minor, p.channel), (10, 4, cost_minor(6), "whatsapp"));
}
