//! THE BOOKKEEPING: the plan claims with an intent row before any send, the
//! answers settle it, the queue's own drain rules on them, and a registered
//! order gets `Noted{fiscal}` -- each rule beside its twin.

use super::*;
use crate::fiscal::queue::{drain, entry};
use crate::outbox::Verdict;
use crate::services::ordering::tax_block::stamp;
use crate::services::ordering::tax_cfg::VenueTax;
use dowiz_core::tax::RatePpm;

const T: i64 = 1_790_000_000_000;
const V20: VenueTax = VenueTax { default: RatePpm(200_000), inclusive: true, fee: RatePpm(200_000) };

pub(crate) fn order(id: &str) -> Value {
    let mut o = json!({
        "id": id, "location_id": "v1", "status": "DELIVERED", "created_at_ms": T, "currency": "ALL",
        "items": [{ "product_id": "p-cola", "name": "Cola", "quantity": 1, "unit_price": 200 }], "total": 200,
        "payments": [{ "method": "cash", "amount": 200 }],
    });
    stamp(&mut o, &V20, 0, 0, 0).unwrap();
    o
}

fn doc(id: &str, at: i64) -> Document {
    crate::fiscal::document::document(&order(id), "ALL", at).unwrap()
}

fn codes() -> Codes {
    Codes { iic: "IIC1".into(), fic: "FIC1".into(), inv_ord_num: "57".into() }
}

fn intent(e: &Entry, stage: Stage, at: i64) -> Intent {
    Intent { uuid: e.id.clone(), order_id: e.to.clone(), stage, at_ms: at, unknowns: 0, sale_id: Some(9001), why: None, prev: None }
}

/// THE INTENT IS WRITTEN BEFORE THE SEND: a fresh document is claimed as
/// `Sending` in the plan itself, and at most `BATCH` are taken, oldest first.
#[test]
fn the_plan_claims_each_document_with_an_intent_before_any_send() {
    let es: Vec<Entry> = (0..7).map(|i| entry(&doc(&format!("o{i}"), T + i))).collect();
    let (items, writes) = plan(&es, &[], T + 100);
    assert_eq!(items.len(), BATCH);
    assert_eq!(items[0].order_id, "o0", "oldest first");
    assert!(items.iter().all(|i| i.action == Action::Send));
    assert_eq!(writes.len(), BATCH);
    assert!(writes.iter().all(|w| w.stage == Stage::Sending && w.prev.is_none() && w.at_ms == T + 100));
}

/// An unanswered send is reconciled first; a held one is passed over and
/// does not block the one behind it (non-head-blocking).
#[test]
fn the_plan_reconciles_the_unanswered_and_passes_over_the_held() {
    let (a, b, c, d) = (entry(&doc("a", T)), entry(&doc("b", T + 1)), entry(&doc("c", T + 2)), entry(&doc("d", T + 3)));
    let now = T + 100;
    let ints = [intent(&a, Stage::Sending, now - 60_000), intent(&b, Stage::Refused, now), intent(&c, Stage::Unfiscalised, now - 1)];
    let (items, writes) = plan(&[a.clone(), b, c, d.clone()], &ints, now);
    let got: Vec<(&str, Action)> = items.iter().map(|i| (i.order_id.as_str(), i.action.clone())).collect();
    assert_eq!(got, vec![("a", Action::ReconcileThenSend), ("d", Action::Send)]);
    assert_eq!(writes[0].prev, Some(Stage::Sending), "the claim remembers what it was");
    // The unfiscalised sale is RE-READ once it has waited -- a read, never a Send.
    let (items, _) = plan(&[entry(&doc("c", T + 2))], &[intent(&entry(&doc("c", T + 2)), Stage::Unfiscalised, now - LOOK_AGAIN_MS)], now);
    assert_eq!(items[0].action, Action::Recheck(9001));
    let _ = d;
}

/// ERROR DOES NOT RESEND: an unfiscalised sale settles as `Unfiscalised`
/// with its sale id, and the next plan never offers it as a Send.
#[test]
fn an_unfiscalised_answer_is_never_offered_for_sending_again() {
    let e = entry(&doc("o1", T));
    let claimed = intent(&e, Stage::Sending, T);
    let w = settle(&[claimed], &[(e.id.clone(), Outcome::Unfiscalised { sale_id: 42, fault: "cert".into() })], T + 1);
    let IntentWrite::Put(i) = &w[0] else { panic!("{w:?}") };
    assert_eq!((i.stage, i.sale_id), (Stage::Unfiscalised, Some(42)));
    for later in [T + 2, T + LOOK_AGAIN_MS * 10] {
        let (items, writes) = plan(&[e.clone()], std::slice::from_ref(i), later);
        assert!(items.iter().all(|x| !matches!(x.action, Action::Send | Action::ReconcileThenSend)), "{items:?}");
        assert!(writes.is_empty());
    }
}

#[test]
fn settle_moves_each_outcome_to_its_stage() {
    let e = entry(&doc("o1", T));
    let fresh = Intent { prev: None, sale_id: None, ..intent(&e, Stage::Sending, T) };
    let one = |o: Outcome, was: &Intent| settle(std::slice::from_ref(was), &[(e.id.clone(), o)], T + 5).remove(0);
    assert_eq!(one(Outcome::Sent { sale_id: 1, codes: codes() }, &fresh), IntentWrite::Remove(e.id.clone()));
    let IntentWrite::Put(u) = one(Outcome::Unknown("timeout".into()), &fresh) else { panic!() };
    assert_eq!((u.stage, u.unknowns), (Stage::Sending, 1), "one unknown: reconcile next firing");
    let IntentWrite::Put(u2) = one(Outcome::Unknown("timeout".into()), &u) else { panic!() };
    assert_eq!(u2.stage, Stage::Inconclusive, "two unknowns stop it for the owner");
    // NOT SENT restores what the firing claimed: nothing, or the earlier stage.
    assert_eq!(one(Outcome::NotSent("link".into()), &fresh), IntentWrite::Remove(e.id.clone()));
    let blocked_then_claimed = Intent { prev: Some(Stage::Blocked), ..fresh.clone() };
    let IntentWrite::Put(b) = one(Outcome::NotSent("link".into()), &blocked_then_claimed) else { panic!() };
    assert_eq!(b.stage, Stage::Blocked);
    let unknown_then_claimed = Intent { unknowns: 1, prev: Some(Stage::Sending), ..fresh };
    let IntentWrite::Put(s) = one(Outcome::NotSent("link".into()), &unknown_then_claimed) else { panic!() };
    assert_eq!((s.stage, s.unknowns), (Stage::Sending, 1), "an unanswered send stays owed a reconcile");
}

/// THE QUEUE'S OWN DRAIN RULES ON THE ANSWERS: sent -> removed with its
/// codes, held -> untouched with an exception row, unknown -> retried later,
/// not taken this firing -> untouched and silent.
#[test]
fn the_answers_replayed_through_the_drain() {
    let ds = [doc("s", T), doc("h", T + 1), doc("u", T + 2), doc("n", T + 3)];
    let es: Vec<Entry> = ds.iter().map(entry).collect();
    let sender = EbillsSender(vec![
        (es[0].id.clone(), Outcome::Sent { sale_id: 9001, codes: codes() }),
        (es[1].id.clone(), Outcome::Blocked("\"Cola\" has no eBills item".into())),
        (es[2].id.clone(), Outcome::Unknown("timeout".into())),
    ]);
    let d = drain(&es, &sender, T + 10);
    assert_eq!(d.sent, vec![("s".to_string(), codes())]);
    assert_eq!(d.verdicts[0], (es[0].id.clone(), Verdict::Sent));
    assert!(matches!(d.verdicts[1], (ref id, Verdict::Retry { .. }) if *id == es[2].id));
    assert_eq!(d.verdicts.len(), 2, "held and not-taken have no verdict");
    assert_eq!(d.held, 2);
    assert_eq!(d.exceptions.len(), 1);
    assert!(d.exceptions[0].reason.as_deref().unwrap().contains("Cola"));
    assert!(d.corrective.is_empty());
}

fn view(o: &Value, seq: u64) -> OrderView {
    OrderView { order_id: "o1".into(), kind: 1, seq, order_json: o.to_string() }
}

/// SUCCESS WRITES `Noted{fiscal}` -- the field law N reads -- once.
#[test]
fn success_writes_one_noted_fiscal_and_a_second_writes_nothing() {
    let mut hub = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    let o = order("o1");
    let (body, seq) = note(&mut hub, Some(&view(&o, 5)), 9001, &codes(), T + 7).unwrap().expect("a first note writes");
    assert!(seq > 5);
    let d: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(d["fiscal"]["fic"], json!("FIC1"));
    assert_eq!(d["fiscal"]["sale_id"], json!(9001));
    assert_eq!(hub.events().len(), 1);
    assert_eq!(hub.events()[0].kind, dowiz_hub::EventKind::Noted);
    let mut noted = o;
    noted["fiscal"] = json!({ "fic": "FIC1" });
    assert_eq!(note(&mut hub, Some(&view(&noted, 9)), 9001, &codes(), T + 8).unwrap(), None);
    assert!(matches!(note(&mut hub, None, 1, &codes(), T), Err(Refused::NotFound)));
}
