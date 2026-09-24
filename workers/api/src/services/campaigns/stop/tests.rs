//! STOP in a thread → a withdrawal, and the drain's re-check drops the entry.
//! The key derivation and the alias circle are passed in, as the hook does.

use super::*;
use std::collections::BTreeSet;

use crate::services::campaigns::audience::recipients;
use crate::services::campaigns::campaign::{define, DefIn};
use crate::services::campaigns::segment::{Now, Segment, DAY_MS};
use crate::services::campaigns::send::{gate, plan};
use dowiz_hub::consent::{wording_id, KIND_ACT};
use dowiz_hub::logimage::Entry as LogEntry;
use serde_json::json;

fn key_of(p: &str) -> String {
    format!("k{}", p.chars().filter(char::is_ascii_digit).collect::<String>())
}
fn alone(k: &str) -> Vec<String> {
    vec![k.to_string()]
}

#[test]
fn the_stop_words_match_whole_messages_only() {
    for yes in ["STOP", "stop", "Stop", " stop. ", "STOP!", "NDALO", "ndalo", "СТОП", "стоп", "Стоп!", "СТОР", "Stop promotions", "unsubscribe"] {
        assert!(is_stop(yes), "{yes:?} is a stop");
    }
    for no in ["", "  ", "don't stop the soup", "stop by at 8?", "STOPP", "two philadelphia", "ndalo porosinë"] {
        assert!(!is_stop(no), "{no:?} is not a stop");
    }
}

#[test]
fn a_whatsapp_stop_withdraws_every_spelling_of_the_number() {
    let acts = acts_for("whatsapp", "355691234567", "Stop", "whatsapp:wamid.1", 5_000, key_of, alone);
    let keys: BTreeSet<&str> = acts.iter().map(|a| a.key.as_str()).collect();
    assert_eq!(keys, BTreeSet::from(["k355691234567", "k0691234567", "k691234567"]));
    for a in &acts {
        assert_eq!((a.state, a.channel.as_str(), a.purpose.as_str()), (State::Withdrawn, "whatsapp", "marketing"));
        assert_eq!((a.method, a.at_ms, a.via.as_str()), (Method::WhatsappKeyword, 5_000, "whatsapp:wamid.1"));
    }
    // The twin: the same sender saying something else files nothing.
    assert!(acts_for("whatsapp", "355691234567", "hi, table for 2?", "w2", 5_000, key_of, alone).is_empty());
}

#[test]
fn linked_keys_are_withdrawn_and_other_channels_file_their_own() {
    let circle = |k: &str| if k == "k355691234567" { vec![k.to_string(), "kLINKED".into()] } else { vec![k.to_string()] };
    let acts = acts_for("whatsapp", "355691234567", "STOP", "w", 1, key_of, circle);
    assert!(acts.iter().any(|a| a.key == "kLINKED"), "a number linked to this one stops too");
    let ig = acts_for("instagram", "17841400000000001", "stop", "instagram:m1", 1, key_of, alone);
    assert_eq!(ig.len(), 1);
    assert_eq!((ig[0].channel.as_str(), ig[0].key.as_str()), ("instagram", "k17841400000000001"));
    let tg = acts_for("telegram", "12345", "СТОП", "t", 1, key_of, alone);
    assert_eq!(tg[0].channel, "telegram");
    assert!(acts_for("sms", "355691234567", "STOP", "s", 1, key_of, alone).is_empty(), "an unknown channel files nothing");
    assert!(acts_for("whatsapp", "355691234567", "STOP", "w", 0, key_of, alone).is_empty(), "no instant, no act");
}

/// G4 THROUGH THE INBOX: two consented, a campaign queued, one writes STOP
/// on WhatsApp → the drain's gate drops exactly that one entry. The consent
/// was filed under the NATIONAL spelling the checkout was typed with; the
/// STOP arrives from the E.164 `wa_id`.
#[test]
fn a_stop_in_the_thread_drops_the_queued_campaign_entry() {
    let given = |k: &str| {
        let a = Act { key: k.into(), purpose: "marketing".into(), channel: "whatsapp".into(), state: State::Given,
            at_ms: 10, method: Method::CheckoutBox, evidence: String::new(), wording_id: wording_id("sq"), via: "o".into() };
        LogEntry { kind: KIND_ACT.into(), subject: format!("cust:{k}"), json: a.to_json(), seq: 0 }
    };
    let orders = vec![
        json!({ "contact": { "phone": "069 123 4567" }, "created_at_ms": 1, "status": "DELIVERED" }),
        json!({ "contact": { "phone": "069 765 4321" }, "created_at_ms": 1, "status": "DELIVERED" }),
    ];
    let mut acts = vec![given("k0691234567"), given("k0697654321")];
    let now = Now { utc_ms: 100 * DAY_MS, local_ms: 100 * DAY_MS };
    let body = json!({ "name": "A", "text": "t", "segment": { "kind": "everyone_consented" }, "template": { "name": "a", "lang": "sq" } });
    let d = define(serde_json::from_value::<DefIn>(body).unwrap(), &[], "o", 1).unwrap();
    let r = recipients(&orders, key_of, |k| k.to_string(), |_| Vec::new(), |_| None, &acts, &Segment::EveryoneConsented, now);
    let queue: Vec<_> = plan(&d, &r, &BTreeSet::new(), &[], 20).unwrap().queue.into_iter().map(|(_, e, _)| e).collect();
    assert_eq!(queue.iter().filter(|e| gate(e, &acts).is_some()).count(), 2, "the twin: before the STOP both go");

    for a in acts_for("whatsapp", "355691234567", "stop", "whatsapp:wamid.9", 50, key_of, alone) {
        acts.push(LogEntry { kind: KIND_ACT.into(), subject: format!("cust:{}", a.key), json: a.to_json(), seq: 0 });
    }
    let going: Vec<&str> = queue.iter().filter(|e| gate(e, &acts).is_some()).map(|e| e.to.as_str()).collect();
    assert_eq!(going, vec!["355697654321"], "the one who wrote STOP is dropped at the drain");
}
