//! D26 / G8: consent is read, written and witnessed on the alias circle.

use super::*;
use dowiz_hub::consent::{log::write, wording_id, State};
use dowiz_hub::logimage::LogImage;

const K: &str = "aaaaaaaaaaaaaaaa"; // the row (+355...)
const K2: &str = "bbbbbbbbbbbbbbbb"; // the linked spelling (069...)

fn circle() -> Vec<String> {
    vec![K.to_string(), K2.to_string()]
}

fn grant(key: &str, at: i64) -> Act {
    owner_act(key, &OwnerActIn { state: "given".into(), evidence: "paper form".into(), lang: "en".into() }, "owner_1", at).unwrap()
}

fn withdraw() -> OwnerActIn {
    OwnerActIn { state: "withdrawn".into(), evidence: String::new(), lang: String::new() }
}

fn log_of(acts: &[Act]) -> LogImage {
    let mut log = LogImage::create().unwrap();
    for a in acts {
        write(&mut log, a).unwrap();
    }
    log
}

/// READ: consent given under the linked spelling K2 is the person's -- the
/// row K shows it. The twin: K alone (the old read) says no.
#[test]
fn consent_filed_under_the_linked_spelling_is_read_on_the_row() {
    let log = log_of(&[grant(K2, 10)]);
    let e = log.entries();
    let w = circle_state(&e, &circle(), PURPOSE_MARKETING, CHANNEL_WHATSAPP).expect("the circle consented");
    assert_eq!(w.key(), K2, "the witness names the number that said yes");
    assert!(circle_state(&e, &[K.to_string()], PURPOSE_MARKETING, CHANNEL_WHATSAPP).is_none());
    assert_eq!(w.wording_id(), wording_id("en"));
}

/// WITNESSED: a withdrawal on K, newer than K2's grant, stops the person --
/// the audit's case, where the campaign still sent on K2's witness. The twin:
/// an OLDER withdrawal on K does not cancel a newer grant on K2.
#[test]
fn the_newest_act_on_any_spelling_decides() {
    let w = owner_act(K, &withdraw(), "owner_1", 20).unwrap();
    let log = log_of(&[grant(K2, 10), w]);
    assert!(circle_state(&log.entries(), &circle(), PURPOSE_MARKETING, CHANNEL_WHATSAPP).is_none());

    let w = owner_act(K, &withdraw(), "owner_1", 5).unwrap();
    let log = log_of(&[w, grant(K2, 10)]);
    assert!(circle_state(&log.entries(), &circle(), PURPOSE_MARKETING, CHANNEL_WHATSAPP).is_some());
}

/// WRITTEN: an owner's withdrawal on the row is filed under every key of the
/// circle, so `send::gate`, which asks by the queued entry's own key, finds
/// the stop too. A grant is filed once, under the row's key.
#[test]
fn a_withdrawal_is_filed_on_every_spelling_and_a_grant_on_one() {
    let acts = owner_acts(K, &circle(), &withdraw(), "owner_1", 30).unwrap();
    let keys: Vec<&str> = acts.iter().map(|a| a.key.as_str()).collect();
    assert_eq!(keys, vec![K, K2]);
    assert!(acts.iter().all(|a| a.state == State::Withdrawn));
    let log = log_of(&[vec![grant(K2, 10)], acts].concat());
    assert!(dowiz_hub::consent::state(&log.entries(), K2, PURPOSE_MARKETING, CHANNEL_WHATSAPP).is_none(), "K2's own fold must stop");

    let body = OwnerActIn { state: "given".into(), evidence: "form".into(), lang: "sq".into() };
    let one = owner_acts(K, &circle(), &body, "owner_1", 40).unwrap();
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].key, K);
}
