//! The pane's words and the sender's last-firing record, each with its twin.

use super::*;
use crate::fiscal::ebills_sender::Stage;
use crate::fiscal::sender::Codes;

#[test]
fn a_firing_that_could_not_send_is_an_error_and_one_that_reached_the_till_is_not() {
    let mut st = SenderState::default();
    st.after(&[("u".into(), Outcome::NotSent("ebills unreachable".into()))], 5);
    assert_eq!(st.last_error, Some((5, "ebills unreachable".into())));
    assert_eq!(st.last_ok_ms, 0);
    let codes = Codes { iic: "I".into(), fic: "F".into(), inv_ord_num: "1".into() };
    st.after(&[("u".into(), Outcome::Sent { sale_id: 1, codes })], 9);
    assert_eq!((st.last_ok_ms, st.last_error.clone(), st.sent), (9, None, 1));
    st.after(&[], 12);
    assert_eq!(st.last_ok_ms, 9, "an empty firing moves nothing");
}

#[test]
fn the_view_names_each_waiting_document_with_its_stage_and_deadline() {
    let e = crate::outbox::Entry::new("u1".into(), crate::fiscal::queue::KIND, "o1".into(), "{}".into(), 100);
    let i = Intent { uuid: "u1".into(), order_id: "o1".into(), stage: Stage::Blocked, at_ms: 150, unknowns: 0, sale_id: None, why: Some("\"Cola\" has no eBills item".into()), prev: None };
    let v = view(&["not armed".into()], &Arming::default(), Some(1), &[e], &[i], &[], &SenderState::default(), vec!["7".into()], vec![], 200);
    assert_eq!(v["armed"], json!(false));
    assert_eq!(v["waiting"][0]["stage"], json!("blocked"));
    assert!(v["waiting"][0]["why"].as_str().unwrap().contains("Cola"));
    assert_eq!(v["waiting"][0]["deadline"], json!(100 + crate::fiscal::queue::DEADLINE_MS));
    assert_eq!(v["confirm"], json!(CONFIRM));
    let armed = view(&[], &Arming::default(), Some(1), &[], &[], &[], &SenderState::default(), vec![], vec![], 200);
    assert_eq!((armed["armed"].clone(), armed["backlog"].clone()), (json!(true), json!(0)));
}
