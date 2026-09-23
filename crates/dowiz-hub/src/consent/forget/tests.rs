//! §3.3 step 2, pinned: the person's words and thread are gone from the BYTES,
//! the proof that the venue stopped is there, and the chain says so honestly.

use super::*;
use crate::consent::{state, wording_id, CHANNEL_WHATSAPP, PURPOSE_MARKETING};

const KEY: &str = "a1b2c3d4e5f60718";
const OTHER: &str = "ffffffffffffffff";
const PHONE: &str = "+355691111111";

fn grant(key: &str, method: Method, evidence: &str, via: &str, at_ms: i64) -> Act {
    Act {
        key: key.into(),
        purpose: PURPOSE_MARKETING.into(),
        channel: CHANNEL_WHATSAPP.into(),
        state: State::Given,
        at_ms,
        method,
        evidence: evidence.into(),
        wording_id: wording_id("en"),
        via: via.into(),
    }
}

/// The subject has an owner-entered grant whose evidence names the phone and a
/// keyword grant on their own thread; a stranger has one of each too.
fn image() -> LogImage {
    let mut l = LogImage::create().unwrap();
    let evidence = format!("paper form, Arben, {PHONE}");
    for a in [
        grant(KEY, Method::OwnerEntered, &evidence, "owner_7", 10),
        grant(KEY, Method::WhatsappKeyword, "", "whatsapp/355691111111", 20),
        grant(OTHER, Method::OwnerEntered, "paper form, Besa", "owner_7", 30),
        grant(OTHER, Method::WhatsappKeyword, "", "whatsapp/355692222222", 40),
    ] {
        crate::consent::log::write(&mut l, &a).unwrap();
    }
    l
}

fn has(bytes: &[u8], s: &str) -> bool {
    bytes.windows(s.len()).any(|w| w == s.as_bytes())
}

#[test]
fn the_persons_words_and_thread_leave_the_bytes_and_the_proof_stays() {
    let mut l = image();
    assert!(has(&l.to_bytes(), PHONE) && has(&l.to_bytes(), "355691111111"));
    let tip_before = chain_check(&l);
    assert!(tip_before.holds());

    let got = forget(&mut l, KEY, 99).unwrap();
    assert_eq!(got, Forgot { redacted: 2, withdrawn: 3 }, "two acts scrubbed, one withdrawal per channel");

    let bytes = l.to_bytes();
    assert!(!has(&bytes, PHONE), "the phone the owner typed is still in the image");
    assert!(!has(&bytes, "355691111111"), "the thread (the number) is still in the image");
    assert!(!has(&bytes, "Arben"), "the name the owner typed is still in the image");
    // The stranger is untouched.
    assert!(has(&bytes, "Besa") && has(&bytes, "355692222222"));

    // THE PROOF: the pseudonym, when, how, what they read, which owner -- and
    // the withdrawal on every channel, so nothing can be sent.
    let acts: Vec<Act> =
        l.about(KIND_ACT, Some(&subject_of(KEY)), 99).iter().filter_map(|e| Act::parse(&e.json)).collect();
    let owner = acts.iter().find(|a| a.method == Method::OwnerEntered).unwrap();
    assert_eq!((owner.evidence.as_str(), owner.via.as_str(), owner.at_ms), (ERASED, "owner_7", 10));
    assert_eq!(owner.wording_id, wording_id("en"));
    for ch in CHANNELS {
        assert!(state(&l.entries(), KEY, PURPOSE_MARKETING, ch).is_none(), "{ch} still consented");
        assert!(acts.iter().any(|a| a.channel == ch && a.method == Method::Erasure && a.state == State::Withdrawn));
    }
    assert!(state(&l.entries(), OTHER, PURPOSE_MARKETING, CHANNEL_WHATSAPP).is_some(), "the stranger lost consent");

    let c = chain_check(&l);
    assert_eq!((c.redacted, c.declared, c.broken), (2, 2, 0), "{c:?}");
    assert!(c.holds());
    assert_eq!(c.chained + c.redacted, c.records);
}

#[test]
fn a_second_erasure_writes_nothing() {
    let mut l = image();
    forget(&mut l, KEY, 99).unwrap();
    let once = l.to_bytes();
    assert_eq!(forget(&mut l, KEY, 100).unwrap(), Forgot::default());
    assert_eq!(l.to_bytes(), once);
}

#[test]
fn a_person_with_no_acts_still_gets_the_withdrawals_and_no_declaration() {
    let mut l = LogImage::create().unwrap();
    assert_eq!(forget(&mut l, KEY, 5).unwrap(), Forgot { redacted: 0, withdrawn: 3 });
    assert!(l.about(KIND_FORGOTTEN, None, 9).is_empty());
    assert!(chain_check(&l).holds());
}

/// The negative twin of `holds`: a redaction nobody declared is a breach.
#[test]
fn an_undeclared_redaction_fails_the_law() {
    let mut l = image();
    assert_eq!(redact_acts(&mut l, KEY).unwrap(), 2);
    let c = chain_check(&l);
    assert_eq!((c.redacted, c.declared), (2, 0));
    assert!(!c.holds());
}
