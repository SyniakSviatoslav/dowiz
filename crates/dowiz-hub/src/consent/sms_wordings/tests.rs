//! The SMS box's proof (W-SMS): its own sentence, its own id, its own pair,
//! and erasure reaches it.

use super::super::*;
use super::*;
use crate::logimage::LogImage;

const KEY: &str = "a1b2c3d4";

fn grant(purpose: &str, channel: &str, wording: String) -> Act {
    Act {
        key: KEY.into(),
        purpose: purpose.into(),
        channel: channel.into(),
        state: State::Given,
        at_ms: 10,
        method: Method::CheckoutBox,
        evidence: String::new(),
        wording_id: wording,
        via: "ord_1".into(),
    }
}

#[test]
fn every_language_has_an_sms_sentence_with_its_own_id() {
    for l in LANGS {
        let id = sms_wording_id(l);
        assert_eq!(id.len(), 16, "{l}");
        assert_ne!(id, wording_id(l), "{l}: the SMS id must never equal the offers id");
        assert_eq!(lang_of_sms_wording(&id), Some(l));
        assert!(sms_wording_of(l).unwrap().1.contains("{venue}"), "{l}: the venue is named");
        assert!(sms_wording_json(l).contains("order_status"));
    }
    assert_eq!(sms_wording_id("pt"), "", "a language with no sentence has no id");
}

#[test]
fn an_sms_grant_is_written_and_folds_to_consent() {
    let mut l = LogImage::create().unwrap();
    log::write(&mut l, &grant(PURPOSE_ORDER_STATUS, CHANNEL_SMS, sms_wording_id("sq"))).unwrap();
    let c = state(&l.entries(), KEY, PURPOSE_ORDER_STATUS, CHANNEL_SMS).expect("consented");
    assert_eq!(c.wording_id(), sms_wording_id("sq"));
    // The sentence itself is in the image, once.
    assert_eq!(l.about(KIND_WORDING, Some(&sms_wording_id("sq")), 9).len(), 1);
    // And it is NOT marketing consent on any channel.
    for ch in CHANNELS {
        assert!(state(&l.entries(), KEY, PURPOSE_MARKETING, ch).is_none(), "{ch}");
    }
}

#[test]
fn a_box_cannot_borrow_the_other_boxs_sentence() {
    let mut l = LogImage::create().unwrap();
    let e = log::write(&mut l, &grant(PURPOSE_ORDER_STATUS, CHANNEL_SMS, wording_id("en"))).unwrap_err();
    assert!(e.contains("unknown wording"), "{e}");
    let e = log::write(&mut l, &grant(PURPOSE_MARKETING, CHANNEL_WHATSAPP, sms_wording_id("en"))).unwrap_err();
    assert!(e.contains("unknown wording"), "{e}");
}

#[test]
fn a_pair_nobody_defined_is_refused() {
    for (p, ch) in [(PURPOSE_ORDER_STATUS, CHANNEL_WHATSAPP), (PURPOSE_MARKETING, CHANNEL_SMS)] {
        let e = check(&grant(p, ch, sms_wording_id("en"))).unwrap_err();
        assert!(e.contains("is not asked on"), "{p}/{ch}: {e}");
    }
}

#[test]
fn a_stop_after_the_tick_wins_and_erasure_withdraws_sms_too() {
    let mut l = LogImage::create().unwrap();
    log::write(&mut l, &grant(PURPOSE_ORDER_STATUS, CHANNEL_SMS, sms_wording_id("en"))).unwrap();
    let mut stop = grant(PURPOSE_ORDER_STATUS, CHANNEL_SMS, String::new());
    stop.state = State::Withdrawn;
    stop.method = Method::OwnerEntered;
    stop.at_ms = 11;
    log::write(&mut l, &stop).unwrap();
    assert!(state(&l.entries(), KEY, PURPOSE_ORDER_STATUS, CHANNEL_SMS).is_none(), "STOP must win");

    let mut l = LogImage::create().unwrap();
    log::write(&mut l, &grant(PURPOSE_ORDER_STATUS, CHANNEL_SMS, sms_wording_id("en"))).unwrap();
    forget::forget(&mut l, KEY, 20).unwrap();
    assert!(state(&l.entries(), KEY, PURPOSE_ORDER_STATUS, CHANNEL_SMS).is_none(), "erasure must reach SMS");
}
