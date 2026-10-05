//! W-REFUND: the card's share, each refusal beside its positive twin, through
//! the real `decide_card` / `apply` / `close` over a real Hub and StockLog.

use super::*;
use dowiz_hub::stock::StockLog;

const NOW: i64 = 1_790_000_000_000;
const SEQ: u64 = 1_789_999_000_000;

/// A card order: 1500 lek, paid by card (the webhook's evidence), CONFIRMED.
fn card_order() -> Value {
    json!({
        "id": "o1", "status": "CONFIRMED", "location_id": "v1", "created_at_ms": 1,
        "items": [{"product_id": "maki", "quantity": 1, "unit_price": 1500, "name": "Maki"}],
        "subtotal": 1500, "discount": 0, "delivery_fee": 0, "tip": 0, "total": 1500,
        "payment": "card", "payment_status": "paid", "payment_intent": "pi_1", "amount_received": 1500,
        "at": {"CONFIRMED": 5}
    })
}

fn view(o: &Value, seq: u64) -> OrderView {
    OrderView { order_id: "o1".into(), kind: 1, seq, order_json: o.to_string() }
}

fn input(card_amount: Option<i64>) -> RefundIn {
    RefundIn {
        order_id: "o1".into(), location_id: "v1".into(), by: "p1".into(), reason: "customer_request".into(),
        complete: false, now_ms: NOW, at_door: false, note: None, card_amount, stripe_on: true,
    }
}

fn images(o: &Value) -> (dowiz_hub::Hub, StockLog) {
    let mut h = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    h.append(EventKind::Placed, "o1", &o.to_string(), SEQ, [0u8; 32]).unwrap();
    (h, StockLog::create_sized(16 * 1024).unwrap())
}

fn folded(h: &dowiz_hub::Hub) -> Value {
    serde_json::from_str(&crate::hubstore::orders_state(h)[0].order_json).unwrap()
}

/// Start a refund of `o` and return (order, job), checking the fold agrees.
fn start(o: &Value, amount: Option<i64>) -> (Value, Option<Job>, dowiz_hub::Hub, u64) {
    let (mut h, mut s) = images(o);
    let (m, w, job) = decide_card(&mut h, &mut s, Some(&view(o, SEQ)), &input(amount), "ALL", true).expect("lands");
    assert_eq!(folded(&h), m, "the fold agrees with what was decided");
    (m, job, h, w.last().unwrap().2)
}

fn refused(o: &Value, i: &RefundIn, stripe_on: bool) -> Refused {
    let (mut h, mut s) = images(o);
    let hb = h.to_bytes();
    let r = decide_card(&mut h, &mut s, Some(&view(o, SEQ)), i, "ALL", stripe_on).unwrap_err();
    assert_eq!(h.to_bytes(), hb, "the log moved on a refusal");
    r
}

/// THE POSITIVE TWIN: a card order's refund queues the WHOLE card part by
/// default, keyed by (venue, order, 1), and stays REFUNDING.
#[test]
fn a_card_refund_queues_everything_the_card_paid_and_waits_in_refunding() {
    let (m, job, _, _) = start(&card_order(), None);
    let job = job.expect("a job for Stripe");
    assert_eq!(m["status"], json!("REFUNDING"));
    assert_eq!((job.amount, job.pi.as_str(), job.n, job.currency.as_str()), (1500, "pi_1", 1, "ALL"));
    assert_eq!(job.key, idem_key("v1", "o1", 1));
    let a = &m["refund"]["card"]["attempts"][0];
    assert_eq!((a["status"].as_str(), a["amount"].as_i64(), a["by"].as_str()), (Some("queued"), Some(1500), Some("p1")));
    assert_eq!(m["refund"]["card"]["paid"], json!(1500));
}

/// RED PROOF 1 — THE KEY IS STABLE ACROSS RETRIES: the same venue, order and
/// attempt give the same key every time; any one of the three moved gives
/// another, so a second partial refund is never mistaken for a retry.
#[test]
fn the_idempotency_key_is_stable_and_names_venue_order_and_attempt() {
    let k = idem_key("v1", "o1", 1);
    assert_eq!(k, idem_key("v1", "o1", 1));
    assert_eq!(start(&card_order(), None).1.unwrap().key, start(&card_order(), None).1.unwrap().key, "two turns, one key");
    for other in [idem_key("v2", "o1", 1), idem_key("v1", "o2", 1), idem_key("v1", "o1", 2)] {
        assert_ne!(k, other);
    }
    assert!(k.starts_with("dowiz-rf-") && k.len() == 49, "{k}");
}

/// RED PROOF 2 — NEVER MORE THAN THE CARD PAID MINUS WHAT WENT BACK: half,
/// then the rest is allowed; one lek over is refused, and so is a second
/// refund while the first is still in flight beyond what is left.
#[test]
fn the_amount_never_exceeds_what_the_card_paid_minus_what_went_back() {
    let r = refused(&card_order(), &input(Some(1501)), true);
    assert!(r.message().contains("at most 1500"), "{}", r.message());
    let (half, job, _, seq) = start(&card_order(), Some(750));
    assert_eq!(job.unwrap().amount, 750);
    // While 750 is in flight, 751 more is refused and 750 is the twin.
    let more = |amt| decide_card(&mut images(&half).0, &mut images(&half).1, Some(&view(&half, seq)), &input(Some(amt)), "ALL", true);
    assert!(more(751).unwrap_err().message().contains("at most 750"));
    let (rest, _, job2) = more(750).expect("the rest");
    let job2 = job2.unwrap();
    assert_eq!((job2.n, job2.amount), (2, 750));
    assert_eq!(rest["status"], json!("REFUNDING"), "a second card refund does not step the order");
    // Everything is now queued: nothing is left.
    let (mut h, mut s) = images(&rest);
    let e = decide_card(&mut h, &mut s, Some(&view(&rest, seq + 10)), &input(Some(1)), "ALL", true).unwrap_err();
    assert!(e.message().contains("nothing is left"), "{}", e.message());
    assert!(refused(&card_order(), &input(Some(0)), true).message().contains("positive"));
}

/// No Stripe key: the record says `manual`, nothing is queued, and asking
/// for a card amount is refused with the sentence the console shows.
#[test]
fn without_a_key_the_card_part_is_by_hand_and_a_card_amount_is_refused() {
    let (mut h, mut s) = images(&card_order());
    let (m, _, job) = decide_card(&mut h, &mut s, Some(&view(&card_order(), SEQ)), &input(None), "ALL", false).expect("lands");
    assert!(job.is_none());
    assert_eq!(m["refund"]["card"]["manual"], json!(true));
    assert_eq!(m["status"], json!("REFUNDING"));
    assert_eq!(refused(&card_order(), &input(Some(100)), false).message(), NO_KEY);
}

/// A cash order is the refund as before: no card record, no job; and a card
/// amount on it is refused.
#[test]
fn a_cash_order_has_no_card_part() {
    let mut o = card_order();
    for k in ["payment_intent", "amount_received", "payment_status"] {
        o.as_object_mut().unwrap().remove(k);
    }
    let (mut h, mut s) = images(&o);
    let (m, _, job) = decide_card(&mut h, &mut s, Some(&view(&o, SEQ)), &input(None), "ALL", true).unwrap();
    assert!(job.is_none() && m["refund"].get("card").is_none());
    assert_eq!(m["status"], json!("COMPENSATED_REFUND"), "nothing taken: ended in the same turn");
    assert!(refused(&o, &input(Some(10)), true).message().contains("no card money"));
}

fn told(key: Option<&str>, status: &str, fp: &str) -> Update {
    Update {
        venue: Some("v1".into()), pi: Some("pi_1".into()), refund_id: Some("re_1".into()), key: key.map(str::to_string),
        status: status.into(), amount: Some(1500), failure: None, fingerprint: Some(fp.into()), at: NOW + 1,
    }
}

/// RED PROOF 3 — A FOREIGN VENUE'S REFUND EVENT IS REFUSED; another payment
/// too; the twin from this venue lands.
#[test]
fn a_refund_event_of_another_venue_or_payment_is_refused() {
    let (m, job, _, _) = start(&card_order(), None);
    let key = job.unwrap().key;
    let foreign = Update { venue: Some("v2".into()), ..told(Some(&key), "succeeded", "evt_a") };
    assert!(apply(&m, "v1", &foreign).unwrap_err().message().contains("another venue"));
    assert!(matches!(apply(&m, "v2", &told(Some(&key), "succeeded", "evt_a")), Err(Refused::NotFound)), "the order is not v2's");
    let other_pi = Update { pi: Some("pi_9".into()), ..told(Some(&key), "succeeded", "evt_a") };
    assert!(apply(&m, "v1", &other_pi).unwrap_err().message().contains("another payment"));
    assert!(apply(&m, "v1", &told(Some(&key), "succeeded", "evt_a")).unwrap().is_some(), "the twin");
}

/// RED PROOF 4 — COMPENSATED_REFUND ONLY AFTER `succeeded` COVERING THE CARD:
/// pending closes nothing, half succeeded closes nothing, the whole does; a
/// repeated event is applied once; an older word never overwrites a newer.
#[test]
fn the_order_ends_only_when_stripe_says_succeeded_for_the_whole_card_part() {
    let (m, job, _, _) = start(&card_order(), None);
    let key = job.unwrap().key;
    let pending = apply(&m, "v1", &told(Some(&key), "pending", "evt_1")).unwrap().unwrap();
    assert_eq!(close(&pending, NOW).unwrap(), None, "pending is not money back");
    assert_eq!(apply(&pending, "v1", &told(Some(&key), "pending", "evt_1")).unwrap(), None, "a repeated delivery");
    let done = apply(&pending, "v1", &told(Some(&key), "succeeded", "evt_2")).unwrap().unwrap();
    assert_eq!(done["refund"]["card"]["attempts"][0]["id"], json!("re_1"));
    assert_eq!(apply(&done, "v1", &told(Some(&key), "pending", "evt_3")).unwrap(), None, "late pending after succeeded");
    let delta = close(&done, NOW).unwrap().expect("covered: it closes");
    assert!(delta.contains("COMPENSATED_REFUND") && delta.contains("\"by\":\"stripe\""), "{delta}");

    // Half the card back: still REFUNDING.
    let (half, j1, _, _) = start(&card_order(), Some(750));
    let half = apply(&half, "v1", &Update { amount: Some(750), ..told(Some(&j1.unwrap().key), "succeeded", "evt_h") }).unwrap().unwrap();
    assert_eq!(close(&half, NOW).unwrap(), None, "half the card is not the card part");
    // A cash part too: Stripe's word never closes it; a person does.
    let mut mixed = done.clone();
    mixed["cash_collected"] = json!(200);
    assert_eq!(close(&mixed, NOW).unwrap(), None, "cash still needs complete: true");
}

/// A failure is written on the attempt with Stripe's reason; the next card
/// refund is then allowed again (failed is not in flight).
#[test]
fn a_failed_card_refund_says_why_and_frees_the_amount_for_a_try_again() {
    let (m, job, _, seq) = start(&card_order(), None);
    let f = Update { failure: Some("expired_or_canceled_card".into()), ..told(job.as_ref().map(|j| j.key.as_str()), "failed", "evt_f") };
    let failed = apply(&m, "v1", &f).unwrap().unwrap();
    assert_eq!(failed["refund"]["card"]["attempts"][0]["failure"], json!("expired_or_canceled_card"));
    assert_eq!(sums(&failed), (0, 0));
    let (mut h, mut s) = images(&failed);
    let (_, _, again) = decide_card(&mut h, &mut s, Some(&view(&failed, seq)), &input(Some(1500)), "ALL", true).expect("try again");
    assert_eq!(again.unwrap().n, 2, "a new attempt, a new key");
}

/// A refund made by hand in the Stripe dashboard arrives with no key: it is
/// recorded as Stripe's, so the sums stay true.
#[test]
fn a_refund_made_in_the_dashboard_is_recorded_too() {
    let (m, _, _, _) = start(&card_order(), Some(500));
    let by_hand = Update { key: None, refund_id: Some("re_hand".into()), venue: None, amount: Some(1000), ..told(None, "succeeded", "evt_d") };
    let after = apply(&m, "v1", &by_hand).unwrap().unwrap();
    assert_eq!(after["refund"]["card"]["attempts"][1]["by"], json!("stripe"));
    assert_eq!(sums(&after), (1000, 500));
}
