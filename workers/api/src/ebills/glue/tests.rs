//! The object's image decisions, natively: every refusal leaves the table's
//! bytes exactly as they were, every acceptance has its twin.

use super::*;
use crate::ebills::client::Session;
use crate::ebills::import::{decide, Lookups, Mapped, Waiting};
use crate::ebills::state::{Mapping, CEILING, PENDING_FOR_MS};
use crate::hubdo::OrderView;

fn table() -> Table {
    Table::create(CEILING).unwrap()
}
fn bytes(t: &mut Table) -> Vec<u8> {
    t.to_bytes().unwrap()
}
fn cfg_in(user: &str, password: Option<&str>) -> ConfigIn {
    ConfigIn { enabled: true, pos_id: 1, user: user.into(), password: password.map(String::from) }
}
fn st(t: &Table) -> State {
    state::get(t, K_STATE, ONE).unwrap().unwrap_or_default()
}

#[test]
fn a_refused_config_writes_nothing_and_a_good_one_clears_a_halt() {
    let mut t = table();
    let before = bytes(&mut t);
    for bad in [ConfigIn { pos_id: 0, ..cfg_in("u", Some("p")) }, cfg_in("  ", Some("p")), cfg_in(&"u".repeat(201), None)] {
        assert!(matches!(apply_config(&mut t, bad), Err(Refused::Invalid(_))));
        assert_eq!(bytes(&mut t), before, "a refusal leaves the bytes");
    }
    apply_report(&mut t, ReportIn { now_ms: 1, what: "mfa".into(), halt: true, drop_session: false }).unwrap();
    assert!(st(&t).halted);
    assert_eq!(apply_config(&mut t, cfg_in("u", Some("p"))).unwrap()["usable"], json!(true));
    let s = st(&t);
    assert!(!s.halted && s.failures == 0 && s.last_error.is_none());
}

/// No password keeps the stored one; a new user forgets the session; the
/// same user and password keep it.
#[test]
fn the_password_is_kept_and_a_new_user_forgets_the_session() {
    let mut t = table();
    apply_config(&mut t, cfg_in("u", Some("secret1"))).unwrap();
    keep_session(&mut t, Session { jar: vec![("JSESSIONID".into(), "x".into())], tenant: None }).unwrap();
    apply_config(&mut t, cfg_in("u", None)).unwrap();
    let cfg: Config = state::get(&t, K_CONFIG, ONE).unwrap().unwrap();
    assert_eq!(cfg.secret, "secret1");
    assert!(st(&t).session.is_some(), "same user, same password: the session stands");
    apply_config(&mut t, cfg_in("other", None)).unwrap();
    assert!(st(&t).session.is_none());
}

#[test]
fn a_mapping_needs_a_real_product_and_a_refusal_writes_nothing() {
    let mut t = table();
    let has = |p: &str| p == "p-roll";
    let before = bytes(&mut t);
    let m = |code: &str, pid: Option<&str>| MapIn { code: code.into(), product_id: pid.map(String::from), now_ms: 5 };
    assert!(matches!(apply_map(&mut t, m("56", Some("p-ghost")), &has), Err(Refused::Invalid(_))));
    assert!(matches!(apply_map(&mut t, m(" ", Some("p-roll")), &has), Err(Refused::Invalid(_))));
    assert_eq!(bytes(&mut t), before);
    apply_map(&mut t, m("56", Some("p-roll")), &has).unwrap();
    assert_eq!(state::get::<Mapping>(&t, K_MAP, "56").unwrap().map(|x| x.product_id), Some("p-roll".into()));
    apply_map(&mut t, m("56", None), &has).unwrap();
    assert!(!t.has(K_MAP, "56"), "an empty product clears");
}

#[test]
fn a_report_counts_halts_and_drops_the_session() {
    let mut t = table();
    keep_session(&mut t, Session::default()).unwrap();
    apply_report(&mut t, ReportIn { now_ms: 7, what: "401".into(), halt: false, drop_session: true }).unwrap();
    apply_report(&mut t, ReportIn { now_ms: 8, what: "401".into(), halt: false, drop_session: false }).unwrap();
    let s = st(&t);
    assert_eq!((s.failures, s.halted, s.session.is_none()), (2, false, true));
    assert_eq!(s.last_error.map(|e| e.at_ms), Some(8));
}

#[test]
fn the_floor_is_rewritten_only_when_a_table_changed() {
    let row = |occ: bool| FloorRow { table: "12".into(), status: if occ { "OCCUPIED" } else { "ACTIVE" }.into(), occupied: occ, unpaid: occ.then_some(500) };
    let first = floor_bytes(None, &[row(true)], 1).expect("nothing stored yet");
    assert_eq!(floor_bytes(Some(&first), &[row(true)], 2), None, "the same room, a minute later");
    assert!(floor_bytes(Some(&first), &[row(false)], 3).is_some(), "a table was billed");
}

fn import_in(watermark: i64, complete: bool, sales: Vec<Mapped>) -> ImportIn {
    ImportIn { now_ms: 1_000, sales, watermark, backlog: false, listed: true, reread: false, refused: vec![],
               items: vec![("9".into(), "uji".into(), 100)], session: None, complete, lead_below: 0, recheck: Some((50, 90)) }
}

/// THE ONE TURN, NATIVELY: decide on the log, then the state -- a partial
/// firing keeps the failure count for `report`; a complete one clears it;
/// the watermark never moves back; waiting bills are carried.
#[test]
fn an_import_turn_moves_the_state_with_what_it_did() {
    let mut t = table();
    apply_report(&mut t, ReportIn { now_ms: 1, what: "x".into(), halt: false, drop_session: false }).unwrap();
    let mut hub = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    let mut stock = dowiz_hub::stock::StockLog::create_sized(64 * 1024).unwrap();
    let none = |_: &str| None;
    let look = Lookups { map: &none, product: &none };
    let bill = Mapped::Bill { sale_id: 9, paid: json!({ "table": "3", "total": 100, "at_ms": 5, "external": { "uuid": "b" } }) };
    let out = decide(&mut hub, &mut stock, &[], &look, Waiting::default(), &[bill], 1_000).unwrap();
    let s = record_import(&mut t, &import_in(40, false, vec![]), &out, None).unwrap();
    assert_eq!((s.watermark, s.failures, s.pending.len(), s.recheck_from), (40, 1, 1, 50));
    assert!(t.has(K_SEEN, "9"), "the till's menu is kept for the crosswalk");
    let listed: Vec<OrderView> = crate::hubstore::orders_state(&hub).into_iter().map(OrderView::of_event).collect();
    let out = decide(&mut hub, &mut stock, &listed, &look, Waiting { bills: s.pending.clone(), leads: vec![] }, &[], 1_000 + PENDING_FOR_MS + 1).unwrap();
    let s = record_import(&mut t, &import_in(30, true, vec![]), &out, None).unwrap();
    assert_eq!((s.watermark, s.failures, s.pending.len(), s.refused.len()), (40, 0, 0, 1));
}

/// THE DAILY RE-READ NAMES A REFUSAL ONCE: the same sale refused for the same
/// reason is not added again (the owner's window stays a window). Its twin:
/// a sale that MAPS on a later firing (the retry of a not-yet-fiscalised
/// sale) leaves the refused list, and a different reason is still added.
#[test]
fn a_refusal_is_named_once_and_leaves_when_the_sale_is_taken() {
    let mut t = table();
    let empty = Outcome::default();
    let no = |id: i64, why: &str| Noted { at_ms: 1, sale_id: id, why: why.into() };
    let mut once = import_in(10, true, vec![]);
    once.refused = vec![no(7, "NotFiscalised"), no(8, "Payment(\"MULTIPLE\")")];
    record_import(&mut t, &once, &empty, None).unwrap();
    let s = record_import(&mut t, &once, &empty, None).unwrap();
    assert_eq!(s.refused.len(), 2, "the same two refusals, read twice, are two");
    let mut other = import_in(10, true, vec![]);
    other.refused = vec![no(8, "Currency(\"EUR @ 1\")")];
    assert_eq!(record_import(&mut t, &other, &empty, None).unwrap().refused.len(), 3, "a new reason is a new line");
    let env = json!({ "id": "ebills:u7", "total": 100 });
    let taken = import_in(10, true, vec![Mapped::Order { sale_id: 7, envelope: env }]);
    let s = record_import(&mut t, &taken, &empty, None).unwrap();
    assert_eq!(s.refused.iter().map(|n| n.sale_id).collect::<Vec<_>>(), vec![8, 8], "sale 7 is no longer refused");
}
