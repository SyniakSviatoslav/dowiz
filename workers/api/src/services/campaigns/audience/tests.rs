//! G1's native half: the recipients are exactly the consented, one per
//! person, and the preview's count is the send's count.

use super::*;
use crate::services::campaigns::campaign::preview;
use crate::services::campaigns::segment::DAY_MS;
use dowiz_hub::consent::{wording_id, Act, Method, State, KIND_ACT};
use dowiz_hub::logimage::Entry;
use serde_json::json;

const NOW: Now = Now { utc_ms: 100 * DAY_MS, local_ms: 100 * DAY_MS };

fn order(phone: &str, at: i64) -> Value {
    json!({ "contact": { "phone": phone, "name": "N" }, "created_at_ms": at, "total": 1000, "status": "DELIVERED" })
}

/// The key a phone is filed under, in these tests: its digits as typed (a
/// handle never holds whitespace -- `consent::check` refuses one).
fn key(p: &str) -> String {
    format!("k{}", p.chars().filter(char::is_ascii_digit).collect::<String>())
}

pub(super) fn act(k: &str, state: State, at: i64) -> Entry {
    let a = Act {
        key: k.into(),
        purpose: "marketing".into(),
        channel: "whatsapp".into(),
        state,
        at_ms: at,
        method: Method::CheckoutBox,
        evidence: String::new(),
        wording_id: wording_id("en"),
        via: "order-1".into(),
    };
    Entry { kind: KIND_ACT.into(), subject: format!("cust:{k}"), json: a.to_json(), seq: 0 }
}

fn everyone(orders: &[Value], acts: &[Entry]) -> Vec<Recipient> {
    recipients(orders, key, |k| k.to_string(), |_| Vec::new(), |_| None, acts, &Segment::EveryoneConsented, NOW)
}

/// THE BLUEPRINT'S PROVE (§5 G1): three keys, one withdraws, the campaign
/// reaches exactly two and the withdrawn key is absent.
#[test]
fn three_consent_one_withdraws_two_are_reached() {
    let orders = [order("0691111111", 1), order("0692222222", 2), order("0693333333", 3)];
    let mut acts = vec![
        act(&key("0691111111"), State::Given, 10),
        act(&key("0692222222"), State::Given, 10),
        act(&key("0693333333"), State::Given, 10),
    ];
    assert_eq!(everyone(&orders, &acts).len(), 3, "the positive twin: all three said yes");
    acts.push(act(&key("0692222222"), State::Withdrawn, 20));
    let r = everyone(&orders, &acts);
    let keys: Vec<&str> = r.iter().map(|x| x.key.as_str()).collect();
    assert_eq!(r.len(), 2);
    assert!(!keys.contains(&key("0692222222").as_str()), "the withdrawn key is absent");
    for x in &r {
        assert_eq!(x.witness.key(), x.key, "each carries its own witness");
    }
}

#[test]
fn a_customer_without_consent_is_never_a_recipient_whatever_the_segment() {
    let orders = [order("0691111111", 1), order("0692222222", 2)];
    let acts = [act(&key("0691111111"), State::Given, 10)];
    let tagged = |_: &str| Some(r#"{"tags":["regular"],"birthday_md":"04-11"}"#.to_string());
    for seg in [
        Segment::EveryoneConsented,
        Segment::NotSeenSince { days: 1 },
        Segment::Tag { tag: "regular".into() },
        Segment::BirthdayThisWeek,
    ] {
        let r = recipients(&orders, key, |k| k.to_string(), |_| Vec::new(), tagged, &acts, &seg, NOW);
        assert!(r.iter().all(|x| x.key != key("0692222222")), "{seg:?}: no consent, no message");
        assert_eq!(r.len(), 1, "{seg:?}: the consented twin is reached");
    }
    // Day 100 is 1970-04-11: the birthday above is today, so all four match.
}

/// C4: A PERSON LINKED UNDER TWO SPELLINGS IS ONE ROW AND GETS ONE MESSAGE --
/// even when the consent was given under the spelling that is not the row's.
#[test]
fn a_linked_pair_is_one_recipient_addressed_on_the_number_that_said_yes() {
    let national = "069 123 4567";
    let e164 = "+355691234567";
    let orders = [order(e164, 1), order(national, 2)];
    let acts = [act(&key(national), State::Given, 10)];
    let resolve = |k: &str| if k == key(national) { key(e164) } else { k.to_string() };
    let members = |k: &str| if k == key(e164) { vec![key(national)] } else { Vec::new() };
    let r = recipients(&orders, key, resolve, members, |_| None, &acts, &Segment::EveryoneConsented, NOW);
    assert_eq!(r.len(), 1, "two spellings, one person, one message");
    assert_eq!(r[0].key, key(e164), "filed under the row's key");
    assert_eq!(r[0].witness.key(), key(national), "the witness is the spelling that consented");
    assert_eq!(r[0].to, "355691234567", "WhatsApp's E.164 digits");
    assert_eq!(r[0].lang, "en", "the language of the sentence they read");

    // The twin: unlinked, the same orders are two rows and still one recipient.
    let apart = everyone(&orders, &acts);
    assert_eq!(apart.len(), 1);
}

#[test]
fn the_preview_count_is_the_recipients_count() {
    let orders = [order("0691111111", 1), order("0692222222", 2), order("0693333333", 3)];
    let acts = [act(&key("0691111111"), State::Given, 10), act(&key("0693333333"), State::Given, 10)];
    let r = everyone(&orders, &acts);
    assert_eq!(preview(r.len(), 0).count, r.len());
    assert_eq!(r.len(), 2);
}

#[test]
fn the_card_language_wins_and_the_address_is_digits() {
    let orders = [order("00355 69 111 1111", 1)];
    let acts = [act(&key("00355 69 111 1111"), State::Given, 10)];
    let uk = |_: &str| Some(r#"{"lang":"uk"}"#.to_string());
    let r = recipients(&orders, key, |k| k.to_string(), |_| Vec::new(), uk, &acts, &Segment::EveryoneConsented, NOW);
    assert_eq!((r[0].lang.as_str(), r[0].to.as_str()), ("uk", "355691111111"));
    assert_eq!(address("+383 44 123 456"), "38344123456", "another country's number is kept as typed digits");
}
