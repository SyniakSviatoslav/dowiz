//! P13: the freezing rule's edges, and the trace that finds an order under
//! its lot.

use super::*;
use crate::stock::meta::Meta;
use crate::stock::{reservations_for, settle, StockLog};

const ROLL: &str = r#"{"id":"r","bom":[{"supply":"salmon","qty":100}]}"#;

fn frozen(hours: i64, temp_c: i64) -> Frozen {
    Frozen { item: "salmon".into(), lot: "L1".into(), at: 1_000, hours, temp_c, store: "freezer".into(), by: "p1".into() }
}

/// 853/2004 Annex III VIII: -20 C for 24 h, or -35 C for 15 h. One hour or
/// one degree short meets nothing.
#[test]
fn the_freezing_rule_at_its_edges() {
    assert_eq!(rule(-20, 23), None, "-20 C for 23 h is one hour short");
    assert_eq!(rule(-20, 24), Some(RULE_20));
    assert_eq!(rule(-19, 200), None, "-19 C is never cold enough");
    assert_eq!(rule(-35, 14), None);
    assert_eq!(rule(-35, 15), Some(RULE_35));
    assert_eq!(rule(-40, 30), Some(RULE_20), "both met: the first rule is named");
}

/// A record that meets no rule is STILL written (never a gate); a malformed
/// one is refused with nothing written.
#[test]
fn a_freezing_record_is_written_whatever_rule_it_meets() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.record_frozen(&frozen(23, -20)).unwrap();
    log.record_frozen(&frozen(24, -20)).unwrap();
    let n = log.len();
    assert!(matches!(log.record_frozen(&Frozen { by: "".into(), ..frozen(24, -20) }), Err(StockError::Unsigned)));
    assert!(log.record_frozen(&frozen(0, -20)).is_err());
    assert!(log.record_frozen(&frozen(24, 5)).is_err());
    assert!(log.record_frozen(&Frozen { lot: " ".into(), ..frozen(24, -20) }).is_err());
    assert_eq!(log.len(), n);
    let t = log.trace().unwrap();
    let rules: Vec<_> = t.freezing.iter().map(|r| (r.how, r.hours, r.rule)).collect();
    assert_eq!(rules, vec![("in_house", Some(23), None), ("in_house", Some(24), Some(RULE_20))]);
    // A freezing record moves nothing.
    assert!(log.journal().unwrap().ledger.items().is_empty());
}

/// THE TRACE: an order is found under the lot it ate (FEFO across two
/// lots), and a supplier-treated receipt is in the freezing log.
#[test]
fn the_trace_finds_an_order_under_its_lot() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(5_000);
    let soon = Meta { lot: Some("L-soon".into()), expiry: Some(20261006), treated: Some("CERT-77".into()), ..Meta::default() };
    log.receive_with("salmon", 150, &soon).unwrap();
    log.receive_with("salmon", 1000, &Meta { expiry: Some(20261020), ..Meta::default() }).unwrap();
    log.set_clock(9_000);
    log.append_all(&reservations_for("o-7", &[(ROLL.into(), 2)])).unwrap();
    let led = log.ledger().unwrap();
    log.append_all(&settle(&led, "o-7", true)).unwrap();
    let t = log.trace().unwrap();
    let mine: Vec<_> = t.draws.iter().map(|d| (d.order.as_str(), d.lot.as_str(), d.qty, d.at)).collect();
    assert_eq!(mine, vec![("o-7", "L-soon", 150, Some(9_000)), ("o-7", "#1", 50, Some(9_000))], "200 g: L-soon first, then the unlabelled lot");
    assert_eq!(t.freezing.len(), 1);
    assert_eq!((t.freezing[0].how, t.freezing[0].doc.as_deref(), t.freezing[0].lot.as_str()), ("supplier", Some("CERT-77"), "L-soon"));
}
