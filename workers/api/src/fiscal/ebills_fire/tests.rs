//! CARD L70 CHECK, over a MOCK TILL -- nothing here, or anywhere in this
//! crate's tests, sends a byte to ebills.al. The mock models the platform's
//! one property that matters: a POST that registers a sale is an invoice,
//! whether or not its answer comes back.

use super::*;
use crate::fiscal::ebills_arm::{gated_plan, Arming, LinkState};
use crate::fiscal::ebills_sender::{plan, settle, Intent, IntentWrite, Stage};
use crate::fiscal::queue::entry;
use crate::fiscal::wire::Config;
use crate::outbox::Entry;
use futures_util::FutureExt;
use serde_json::json;

const T: i64 = 1_790_000_000_000;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Success,
    Error,
    Refuse,
    /// The sale is registered and the answer is lost (a timeout).
    LoseAnswer,
    /// 500 with no sale created.
    ServerError,
}

struct MockTill {
    sales: Vec<Value>,
    mode: Mode,
    list_notes: bool,
    list_fails: bool,
    posts: usize,
    reads: usize,
}

impl MockTill {
    fn new(mode: Mode) -> Self {
        let seed = json!({ "id": 7783, "notes": null, "totalValue": 200.0,
            "client": { "id": 1, "defaultClient": true, "nipt": "" },
            "pointOfSale": { "id": 1, "posName": "pos1" },
            "currency": { "id": 1, "currencyCode": "ALL" },
            "fiscalSatus": "FINISHED", "fic": "FIC-7783", "invOrdNum": 1,
            "logCis": [{ "status": "SUCCESS", "iic": "IIC-7783", "fic": "FIC-7783" }] });
        MockTill { sales: vec![seed], mode, list_notes: true, list_fails: false, posts: 0, reads: 0 }
    }
    /// Invoices the tax authority holds that dowiz created.
    fn ours(&self) -> usize {
        self.sales.iter().filter(|s| s["notes"].as_str().is_some_and(|n| n.starts_with("dowiz:"))).count()
    }
    fn register(&mut self, body: &Value, ok: bool) -> Value {
        let id = 9000 + self.sales.len() as i64;
        let log = if ok { json!([{ "status": "SUCCESS", "iic": format!("IIC{id}"), "fic": format!("FIC{id}") }]) } else { json!([{ "status": "ERROR", "faultStringMsg": "cert expired" }]) };
        let sale = json!({ "id": id, "notes": body["sale"]["notes"], "totalValue": body["sale"]["totalValue"],
            "fiscalSatus": if ok { "FINISHED" } else { "WEBSERVICEERROR" }, "invOrdNum": id - 8000, "logCis": log });
        self.sales.push(sale.clone());
        sale
    }
}

fn ok(ct: &str, body: String, status: u16) -> Answer {
    Answer { status, content_type: ct.into(), body, ..Answer::default() }
}

impl Transport for MockTill {
    async fn read(&mut self, p: &Path) -> Result<String, Fail> {
        self.reads += 1;
        match p {
            Path::Sales { .. } if self.list_fails => Err(Fail::Network("list timed out".into())),
            Path::Sales { .. } => {
                let mut rows: Vec<Value> = self.sales.iter().rev().cloned().collect();
                if !self.list_notes {
                    rows.iter_mut().for_each(|r| drop(r.as_object_mut().unwrap().remove("notes")));
                }
                Ok(json!({ "sales": rows, "total": 0.0 }).to_string())
            }
            Path::Sale { id, .. } => self.sales.iter().find(|s| s["id"] == json!(id)).map(|s| json!({ "sale": s }).to_string()).ok_or(Fail::NotFound),
            Path::Items { .. } => Ok(r#"[{"id":23,"itemCode":"16","item":"Cola","price":200.0,"vat":"VAT_20"}]"#.into()),
            Path::Tables { .. } => Ok(r#"[{"id":7,"identifier":"DELIVERY","type":"TABLE","status":"ACTIVE","posX":2.0,"posY":2.0,"server":"Ana"}]"#.into()),
            other => Err(Fail::NotAllowed(format!("{other:?}"))),
        }
    }
    async fn create(&mut self, body: &str) -> Result<Answer, Fail> {
        self.posts += 1;
        let b: Value = serde_json::from_str(body).expect("the sender posts JSON");
        assert!(b["sale"]["notes"].as_str().unwrap().starts_with("dowiz:"), "every create carries its marker");
        assert!(b["saleUnit"].get("server").is_none(), "the waiter's name is never echoed");
        match self.mode {
            Mode::Success => Ok(ok("application/json", self.register(&b, true).to_string(), 200)),
            Mode::Error => Ok(ok("application/json", self.register(&b, false).to_string(), 200)),
            Mode::Refuse => Ok(ok("application/problem+json", r#"{"errorKey":"negInventory"}"#.into(), 400)),
            Mode::LoseAnswer => {
                self.register(&b, true);
                Err(Fail::Network("the answer never came".into()))
            }
            Mode::ServerError => Ok(ok("application/json", r#"{"title":"Internal Server Error"}"#.into(), 500)),
        }
    }
}

fn order_doc(id: &str, product: &str) -> String {
    let mut o = crate::fiscal::ebills_sender::tests::order(id);
    o["items"][0]["product_id"] = json!(product);
    serde_json::to_string(&crate::fiscal::document::document(&o, "ALL", T).unwrap()).unwrap()
}

fn entry_for(id: &str, product: &str) -> Entry {
    let doc: crate::fiscal::document::Document = serde_json::from_str(&order_doc(id, product)).unwrap();
    entry(&doc)
}

fn batch(items: Vec<Item>) -> Batch {
    Batch {
        pos_id: 1, today: "2026-09-24".into(), yesterday: "2026-09-23".into(), sale_unit: "DELIVERY".into(),
        fee_item: None, crosswalk: vec![("16".into(), "p-cola".into())], items,
    }
}

/// One firing, the way the object runs it: plan (intents claimed), fire, settle.
fn firing(till: &mut MockTill, es: &[Entry], intents: &mut Vec<Intent>, now: i64) -> Vec<(String, Outcome)> {
    let (items, claims) = plan(es, intents, now);
    for c in claims {
        intents.retain(|i| i.uuid != c.uuid);
        intents.push(c);
    }
    let out = fire(till, &batch(items)).now_or_never().expect("the mock never waits");
    for w in settle(intents, &out, now) {
        match w {
            IntentWrite::Put(i) => {
                intents.retain(|x| x.uuid != i.uuid);
                intents.push(i);
            }
            IntentWrite::Remove(u) => intents.retain(|x| x.uuid != u),
        }
    }
    out
}

#[test]
fn success_is_sent_with_the_codes_after_one_post() {
    let mut till = MockTill::new(Mode::Success);
    let out = firing(&mut till, &[entry_for("o1", "p-cola")], &mut Vec::new(), T);
    let Outcome::Sent { sale_id, codes } = &out[0].1 else { panic!("{out:?}") };
    assert_eq!((*sale_id, codes.fic.as_str()), (9001, "FIC9001"));
    assert_eq!((till.posts, till.ours()), (1, 1));
}

/// ERROR DOES NOT RESEND: the created, unfiscalised sale is re-read by id
/// in later firings -- never POSTed again.
#[test]
fn an_error_answer_is_never_sent_again() {
    let mut till = MockTill::new(Mode::Error);
    let es = [entry_for("o1", "p-cola")];
    let mut ints = Vec::new();
    assert!(matches!(firing(&mut till, &es, &mut ints, T)[0].1, Outcome::Unfiscalised { sale_id: 9001, .. }));
    till.mode = Mode::Success;
    for n in 1..4 {
        firing(&mut till, &es, &mut ints, T + n * crate::fiscal::ebills_sender::LOOK_AGAIN_MS);
    }
    assert_eq!((till.posts, till.ours()), (1, 1), "one POST, one invoice, however many firings");
}

/// A TIMEOUT RECONCILES FIRST: the lost answer's sale is found in the list
/// by its marker -- Sent, and NO second POST.
#[test]
fn a_timeout_is_reconciled_by_the_list_and_not_sent_twice() {
    let mut till = MockTill::new(Mode::LoseAnswer);
    let es = [entry_for("o1", "p-cola")];
    let mut ints = Vec::new();
    assert!(matches!(firing(&mut till, &es, &mut ints, T)[0].1, Outcome::Unknown(_)));
    assert_eq!(ints[0].stage, Stage::Sending, "the intent stays: answer unknown");
    till.mode = Mode::Success;
    let out = firing(&mut till, &es, &mut ints, T + 60_000);
    assert!(matches!(out[0].1, Outcome::Sent { sale_id: 9001, .. }), "{out:?}");
    assert_eq!((till.posts, till.ours()), (1, 1), "found, not re-sent");
}

/// Its twin: the reconcile does NOT find it (a 500 created nothing) -- then
/// exactly one resend, and one invoice.
#[test]
fn a_timeout_the_list_does_not_show_is_sent_once_more() {
    let mut till = MockTill::new(Mode::ServerError);
    let es = [entry_for("o1", "p-cola")];
    let mut ints = Vec::new();
    assert!(matches!(firing(&mut till, &es, &mut ints, T)[0].1, Outcome::Unknown(_)));
    till.mode = Mode::Success;
    assert!(matches!(firing(&mut till, &es, &mut ints, T + 60_000)[0].1, Outcome::Sent { .. }));
    assert_eq!((till.posts, till.ours()), (2, 1));
}

/// A list with no `notes` cannot settle an unanswered send: stopped for the
/// owner, never a blind resend.
#[test]
fn a_list_without_notes_stops_the_resend() {
    let mut till = MockTill::new(Mode::LoseAnswer);
    let es = [entry_for("o1", "p-cola")];
    let mut ints = Vec::new();
    firing(&mut till, &es, &mut ints, T);
    till.list_notes = false;
    till.mode = Mode::Success;
    assert!(matches!(firing(&mut till, &es, &mut ints, T + 60_000)[0].1, Outcome::Inconclusive(_)));
    firing(&mut till, &es, &mut ints, T + 120_000);
    assert_eq!(till.posts, 1);
}

/// AN UNMAPPED DISH IS NOT SENT (refused by name); the twin is.
#[test]
fn an_unmapped_dish_is_blocked_by_name_without_a_post() {
    let mut till = MockTill::new(Mode::Success);
    let out = firing(&mut till, &[entry_for("o1", "p-unknown")], &mut Vec::new(), T);
    assert!(matches!(&out[0].1, Outcome::Blocked(w) if w.contains("\"Cola\"")), "{out:?}");
    assert_eq!(till.posts, 0);
}

#[test]
fn a_refusal_is_held_and_an_unknown_stops_the_rest_of_the_batch() {
    let mut till = MockTill::new(Mode::Refuse);
    let mut ints = Vec::new();
    let es = [entry_for("o1", "p-cola")];
    assert!(matches!(&firing(&mut till, &es, &mut ints, T)[0].1, Outcome::Refused(w) if w.contains("negInventory")));
    firing(&mut till, &es, &mut ints, T + 10 * crate::fiscal::ebills_sender::LOOK_AGAIN_MS);
    assert_eq!(till.posts, 1, "a refused create is not resent by itself");
    let mut lost = MockTill::new(Mode::LoseAnswer);
    let two = [entry_for("a", "p-cola"), entry_for("b", "p-cola")];
    let out = firing(&mut lost, &two, &mut Vec::new(), T);
    assert!(matches!(out[1].1, Outcome::NotSent(_)), "{out:?}");
    assert_eq!(lost.posts, 1);
}

/// NOTHING IS SENT when the list cannot be read, or when the venue is not
/// armed -- the gate claims nothing, and an empty batch makes no request.
#[test]
fn an_unreadable_list_or_an_unarmed_venue_sends_nothing() {
    let mut till = MockTill::new(Mode::Success);
    till.list_fails = true;
    let out = firing(&mut till, &[entry_for("o1", "p-cola")], &mut Vec::new(), T);
    assert!(matches!(out[0].1, Outcome::NotSent(_)));
    assert_eq!(till.posts, 0);
    let es = [entry_for("o1", "p-cola")];
    let link = LinkState { usable: true, halted: false, failures: 0, last_ok_ms: 1 };
    let unarmed = Arming { armed: false, sale_unit: "DELIVERY".into(), ..Arming::default() };
    assert!(gated_plan(&Config::From(0), &unarmed, &link, &es, &[], T).is_err());
    let mut quiet = MockTill::new(Mode::Success);
    assert!(fire(&mut quiet, &batch(Vec::new())).now_or_never().unwrap().is_empty());
    assert_eq!((quiet.reads, quiet.posts), (0, 0));
    let armed = Arming { armed: true, ..unarmed };
    assert_eq!(gated_plan(&Config::From(0), &armed, &link, &es, &[], T).unwrap().0.len(), 1, "the armed twin plans it");
}

#[test]
fn the_sale_unit_is_cut_to_six_keys_and_a_missing_one_is_named() {
    let t = r#"[{"id":7,"identifier":"DELIVERY","type":"TABLE","status":"ACTIVE","posX":2.0,"posY":2.0,"server":"Ana","activeUser":"x"}]"#;
    assert_eq!(sale_unit_of(t, "DELIVERY").unwrap(), json!({"id":7,"identifier":"DELIVERY","type":"TABLE","status":"ACTIVE","posX":2.0,"posY":2.0}));
    assert!(sale_unit_of(t, "9").unwrap_err().contains("\"9\""));
}
