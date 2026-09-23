//! What may be sent to a person, pinned against the ways a consent record is
//! usually wrong: a pre-ticked box, a withdrawal that arrives and changes
//! nothing, one channel's permission spent on another, and a proof of consent
//! that cannot say what the person was told.

use super::*;
use crate::logimage::Entry;

const KEY: &str = "a1b2c3d4";
const OTHER: &str = "ffffffff";

fn act(state: State, at_ms: i64) -> Act {
    Act {
        key: KEY.to_string(),
        purpose: PURPOSE_MARKETING.to_string(),
        channel: CHANNEL_WHATSAPP.to_string(),
        state,
        at_ms,
        method: Method::CheckoutBox,
        evidence: String::new(),
        wording_id: wording_id("en"),
        via: "ord_1".to_string(),
    }
}

/// Newest first, which is the order `LogImage::entries` hands them over.
fn log(acts: &[Act]) -> Vec<Entry> {
    let mut out: Vec<Entry> = acts
        .iter()
        .enumerate()
        .map(|(i, a)| Entry {
            kind: KIND_ACT.to_string(),
            subject: subject_of(&a.key),
            json: a.to_json(),
            seq: i as u64,
        })
        .collect();
    out.reverse();
    out
}

// ── the fold ────────────────────────────────────────────────────────────────

/// THE DEFAULT IS NO. A venue with no consent record has no consent, and the
/// one shape that must never produce a `Consented` is the empty one — Recital
/// 32: silence is not consent, and neither is an absent record.
#[test]
fn nothing_written_is_not_consent() {
    assert!(state(&[], KEY, PURPOSE_MARKETING, CHANNEL_WHATSAPP).is_none());
}

#[test]
fn a_given_act_is_consent_and_carries_what_the_person_was_told() {
    let l = log(&[act(State::Given, 1_000)]);
    let c = state(&l, KEY, PURPOSE_MARKETING, CHANNEL_WHATSAPP).expect("given");
    assert_eq!(c.key(), KEY);
    assert_eq!(c.channel(), CHANNEL_WHATSAPP);
    assert_eq!(c.purpose(), PURPOSE_MARKETING);
    assert_eq!(c.at_ms(), 1_000);
    assert_eq!(c.wording_id(), wording_id("en"), "which sentence they read");
}

/// ART. 7(3): the withdrawal is the record that must WORK. A fold that kept
/// the first answer would leave a venue lawfully sending to someone who has
/// said stop — and the log would hold the proof of the defect.
#[test]
fn the_newest_act_wins_and_a_withdrawal_ends_it() {
    let l = log(&[act(State::Given, 1_000), act(State::Withdrawn, 2_000)]);
    assert!(state(&l, KEY, PURPOSE_MARKETING, CHANNEL_WHATSAPP).is_none());
}

#[test]
fn consent_given_again_after_a_withdrawal_is_consent() {
    let l = log(&[act(State::Given, 1_000), act(State::Withdrawn, 2_000), act(State::Given, 3_000)]);
    let c = state(&l, KEY, PURPOSE_MARKETING, CHANNEL_WHATSAPP).expect("given again");
    assert_eq!(c.at_ms(), 3_000);
}

/// A LOG IS NOT SORTED BY ITS CONTENT'S CLOCK. `entries()` is newest-first by
/// POSITION; the act carries its own `at_ms`, and a fold that trusted position
/// would be decided by which retry landed first.
#[test]
fn the_answer_does_not_depend_on_the_order_the_records_are_handed_in() {
    let mut l = log(&[act(State::Given, 1_000), act(State::Withdrawn, 2_000)]);
    assert!(state(&l, KEY, PURPOSE_MARKETING, CHANNEL_WHATSAPP).is_none());
    l.reverse();
    assert!(state(&l, KEY, PURPOSE_MARKETING, CHANNEL_WHATSAPP).is_none(), "still withdrawn");
}

/// META'S OPT-IN IS PER CHANNEL AND THE LAW'S IS PER PURPOSE. One permission
/// spent on another channel is the defect this separation exists to stop.
#[test]
fn consent_is_per_purpose_and_per_channel() {
    let l = log(&[act(State::Given, 1_000)]);
    assert!(state(&l, KEY, PURPOSE_MARKETING, CHANNEL_TELEGRAM).is_none(), "another channel");
    assert!(state(&l, KEY, "loyalty", CHANNEL_WHATSAPP).is_none(), "another purpose");
    assert!(state(&l, OTHER, PURPOSE_MARKETING, CHANNEL_WHATSAPP).is_none(), "another person");
}

/// A record this build cannot read is not a permission. The order log made the
/// same call: an undecodable record is quarantined, never assumed.
#[test]
fn a_record_that_does_not_decode_is_not_consent() {
    let bad = Entry {
        kind: KIND_ACT.to_string(),
        subject: subject_of(KEY),
        json: "{not json".to_string(),
        seq: 0,
    };
    assert!(state(&[bad], KEY, PURPOSE_MARKETING, CHANNEL_WHATSAPP).is_none());
}

/// The wording record shares the image with the acts. A fold that did not
/// check the kind would read a SENTENCE as a permission.
#[test]
fn a_wording_record_is_not_an_act() {
    let w = Entry {
        kind: KIND_WORDING.to_string(),
        subject: wording_id("en"),
        json: wording_json("en"),
        seq: 0,
    };
    assert!(state(&[w], KEY, PURPOSE_MARKETING, CHANNEL_WHATSAPP).is_none());
}

/// An act that would be REFUSED on the way in is refused on the way out too.
/// The log is append-only: a record written by an older build, or by a hand,
/// cannot be taken back, so the fold applies the same rules the writer does.
#[test]
fn an_act_the_writer_would_refuse_is_not_consent_when_it_is_read_back() {
    let mut a = act(State::Given, 1_000);
    a.wording_id = String::new();
    let l = log(&[a]);
    assert!(state(&l, KEY, PURPOSE_MARKETING, CHANNEL_WHATSAPP).is_none());
}

// ── what the writer refuses ─────────────────────────────────────────────────

#[test]
fn a_grant_that_cannot_say_what_the_person_read_is_refused() {
    let mut a = act(State::Given, 1_000);
    a.wording_id = String::new();
    assert!(check(&a).is_err(), "ICO: 'what you told people' is part of the record");
}

/// ART. 7(3): withdrawal must be AT LEAST AS EASY as giving. Requiring the
/// person to reproduce the sentence they once agreed to in order to stop would
/// be the opposite, so a withdrawal needs no wording.
#[test]
fn a_withdrawal_needs_no_wording() {
    let mut a = act(State::Withdrawn, 2_000);
    a.wording_id = String::new();
    assert!(check(&a).is_ok());
}

/// "HOW" IS THE FIELD A DISPUTE TURNS ON. An owner typing "they said yes" with
/// nothing behind it is a claim, not evidence, so the type refuses it.
#[test]
fn owner_entered_consent_without_evidence_is_refused() {
    let mut a = act(State::Given, 1_000);
    a.method = Method::OwnerEntered;
    assert!(check(&a).is_err());
    a.evidence = "signed paper card, kept at the till".to_string();
    assert!(check(&a).is_ok());
}

#[test]
fn an_unknown_purpose_or_channel_is_refused() {
    let mut a = act(State::Given, 1_000);
    a.purpose = "everything".to_string();
    assert!(check(&a).is_err());
    let mut a = act(State::Given, 1_000);
    a.channel = "carrier-pigeon".to_string();
    assert!(check(&a).is_err());
}

/// THE CLOCK IS THE REQUEST'S. A record with no instant cannot be ordered
/// against a withdrawal, which is the one comparison this log exists to make.
#[test]
fn an_act_with_no_instant_is_refused() {
    let a = act(State::Given, 0);
    assert!(check(&a).is_err());
    let a = act(State::Given, -1);
    assert!(check(&a).is_err());
}

#[test]
fn a_key_that_is_not_a_handle_is_refused() {
    let mut a = act(State::Given, 1_000);
    a.key = String::new();
    assert!(check(&a).is_err());
}

// ── the record on the wire ──────────────────────────────────────────────────

#[test]
fn an_act_round_trips_through_its_json() {
    let mut a = act(State::Given, 1_700_000_000_000);
    a.method = Method::OwnerEntered;
    a.evidence = "verbal, at the till, \"yes\" — with a quote in it".to_string();
    let back = Act::parse(&a.to_json()).expect("parses");
    assert_eq!(back, a);
}

/// THE RECORD HOLDS NO CONTACT DETAILS, the same rule `Revealed` follows: a
/// log of who may be messaged that also holds the number has doubled the
/// exposure, and this one is backed up and read by a console.
#[test]
fn the_record_carries_no_phone_and_no_name() {
    let a = act(State::Given, 1_000);
    let j = a.to_json();
    for forbidden in ["phone", "name", "address", "email"] {
        assert!(!j.contains(forbidden), "{forbidden} has no business in a consent record: {j}");
    }
}

// ── the wordings ────────────────────────────────────────────────────────────

/// THREE LANGUAGES ARE THREE WORDINGS. "What you told people" is the sentence
/// in the language they read it in, and a shared id would make the record
/// unable to say which of the three that was.
#[test]
fn each_language_is_its_own_wording() {
    let ids: Vec<String> = LANGS.iter().map(|l| wording_id(l)).collect();
    assert_eq!(ids.len(), 3);
    for (i, a) in ids.iter().enumerate() {
        assert!(!a.is_empty());
        for b in ids.iter().skip(i + 1) {
            assert_ne!(a, b, "two languages must not share one wording id");
        }
    }
    assert_eq!(wording_id("en"), wording_id("en"), "and the id is stable");
    assert_eq!(wording_id("pt"), String::new(), "a language with no wording has no id");
}

/// THE SENTENCE IS STORED UNDER ITS OWN HASH. A record that named a wording it
/// could not produce would be a proof of nothing.
#[test]
fn the_sentence_is_recoverable_from_the_id() {
    let j = wording_json("sq");
    assert!(j.contains("\"lang\":\"sq\""));
    assert!(j.contains("STOP"), "the withdrawal instruction is part of the sentence");
    assert!(wording_of("sq").is_some());
    assert!(wording_of("pt").is_none());
}

/// THE BOX IS UNTICKED AND PLACING AN ORDER IS NOT CONSENT (Recital 32). The
/// sentence has to say who is sending, on which channel, and how to stop.
#[test]
fn every_wording_names_the_channel_and_the_way_out() {
    for lang in LANGS {
        let (_, text) = wording_of(lang).expect("a wording per language");
        assert!(text.contains("WhatsApp"), "{lang}: names the channel");
        assert!(text.contains("STOP"), "{lang}: names the way out");
    }
}

// ── the witness ─────────────────────────────────────────────────────────────

/// G1. `Consented` is the only thing a send may be minted from, and this
/// module is the only place one can come from. The compiler half of that
/// promise is proved by `tools/gates/consent.prove.sh`, which tries to build a
/// `Consented` from outside and reads the refusal; what is checked here is the
/// half a test can see: the fold does not hand one out for a person who never
/// said yes.
#[test]
fn a_consented_exists_only_where_a_person_said_yes() {
    let given = log(&[act(State::Given, 1_000)]);
    let withdrawn = log(&[act(State::Given, 1_000), act(State::Withdrawn, 2_000)]);
    let people = [
        (state(&given, KEY, PURPOSE_MARKETING, CHANNEL_WHATSAPP), true),
        (state(&withdrawn, KEY, PURPOSE_MARKETING, CHANNEL_WHATSAPP), false),
        (state(&[], KEY, PURPOSE_MARKETING, CHANNEL_WHATSAPP), false),
    ];
    let sendable = people.iter().filter(|(c, _)| c.is_some()).count();
    assert_eq!(sendable, 1, "one of the three may be sent to");
    for (c, want) in &people {
        assert_eq!(c.is_some(), *want);
    }
}

// ── the `consent` image: what a placement and a withdrawal write ────────────

use crate::logimage::LogImage;

/// ONE ACT, ONE `c` RECORD, AND THE SENTENCE ONCE. The CHECK of §6 item 2 is
/// `about("c", key) = 1` with the right `wording_id` for the language shown;
/// the `w` record is what lets that id be shown back as words.
#[test]
fn a_grant_writes_one_act_and_its_wording_once() {
    let mut log = LogImage::create().unwrap();
    let mut a = act(State::Given, 1_000);
    a.wording_id = wording_id("sq");
    log::write(&mut log, &a).expect("a grant with a known wording");
    assert_eq!(log.about(KIND_ACT, Some(&subject_of(KEY)), 10).len(), 1);
    let w = log.about(KIND_WORDING, Some(&wording_id("sq")), 10);
    assert_eq!(w.len(), 1, "the sentence is filed under its own id");
    assert!(w[0].json.contains("STOP"));

    // A SECOND PERSON READING THE SAME SENTENCE does not file it twice.
    let mut b = a.clone();
    b.key = OTHER.to_string();
    log::write(&mut log, &b).unwrap();
    assert_eq!(log.about(KIND_WORDING, None, 10).len(), 1, "one wording, two acts");
    assert_eq!(log.about(KIND_ACT, None, 10).len(), 2);
    // AND THE FOLD READS WHAT WAS WRITTEN.
    let c = state(&log.entries(), KEY, PURPOSE_MARKETING, CHANNEL_WHATSAPP).expect("given");
    assert_eq!(c.wording_id(), wording_id("sq"));
}

/// THE WRITER REFUSES WHAT THE FOLD WOULD IGNORE, and says why, before a byte
/// lands: an append-only log cannot take a bad record back.
#[test]
fn the_writer_refuses_a_grant_it_cannot_prove_and_writes_nothing() {
    let mut log = LogImage::create().unwrap();
    let mut a = act(State::Given, 1_000);
    a.wording_id = "0000000000000000".into();
    let why = log::write(&mut log, &a).expect_err("a sentence nobody can show");
    assert!(why.contains("wording"), "{why}");
    a.wording_id = String::new();
    assert!(log::write(&mut log, &a).is_err());
    assert_eq!(log.len(), 0, "nothing was written");
}

/// A WITHDRAWAL IS A NEW RECORD, AND IT ENDS THE CONSENT. The grant stays in
/// the log -- it is the proof of what the venue was allowed to do before.
#[test]
fn a_withdrawal_is_appended_and_the_fold_stops() {
    let mut log = LogImage::create().unwrap();
    log::write(&mut log, &act(State::Given, 1_000)).unwrap();
    let mut w = act(State::Withdrawn, 2_000);
    w.wording_id = String::new();
    log::write(&mut log, &w).expect("a withdrawal needs no wording");
    assert_eq!(log.about(KIND_ACT, Some(&subject_of(KEY)), 10).len(), 2, "both acts are kept");
    assert!(state(&log.entries(), KEY, PURPOSE_MARKETING, CHANNEL_WHATSAPP).is_none());
}

#[test]
fn every_wording_id_names_its_language_back() {
    for l in LANGS {
        assert_eq!(log::lang_of_wording(&wording_id(l)), Some(l));
    }
    assert_eq!(log::lang_of_wording("nope"), None);
}

/// §3.3 STEP 2: FORGETTING A PERSON WITHDRAWS, and the withdrawal outlives
/// them. An "erasure" that granted would be a consent nobody gave.
#[test]
fn an_erasure_withdraws_and_can_never_grant() {
    let mut w = act(State::Withdrawn, 5_000);
    w.method = Method::Erasure;
    w.wording_id = String::new();
    assert!(check(&w).is_ok());
    assert_eq!(Act::parse(&w.to_json()).unwrap().method, Method::Erasure);
    let mut g = act(State::Given, 5_000);
    g.method = Method::Erasure;
    assert!(check(&g).is_err());
    let l = log(&[act(State::Given, 1_000), w]);
    assert!(state(&l, KEY, PURPOSE_MARKETING, CHANNEL_WHATSAPP).is_none());
}
