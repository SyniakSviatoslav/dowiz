//! The wire tail's door: every refusal named, each with its positive twin, and nothing written
//! by a refused merge.

use super::super::merge::tests::{bootstrapped, hub, T0};
use super::super::merge::merge;
use super::super::tablet;
use super::*;

fn one(line: &str) -> (StockLog, Tail) {
    let mut h = hub(5000, 10);
    let (img, base) = bootstrapped(&mut h);
    let img = tablet::record(&img, "kitchen", line, T0).unwrap().0;
    (h, tablet::tail(&img, "kitchen", &base).unwrap())
}

/// The tail with record 0 replaced and the tip recomputed: a tablet that WROTE this.
fn rewritten(t: &Tail, rec: &str) -> Tail {
    let tip = crate::content_id_chained(&t.base, rec.as_bytes());
    Tail { recs: vec![rec.to_string()], tip, ..t.clone() }
}

fn refused(h: &mut StockLog, t: &Tail) -> Why {
    let n = h.len();
    let e = merge(&h.to_bytes_trimmed(), h, t).unwrap_err();
    assert_eq!(h.len(), n, "a refused tail writes nothing");
    e.why
}

#[test]
fn a_guest_key_in_a_kitchen_tail_is_refused_by_name() {
    let (mut h, t) = one(r#"{"k":"wasted","item":"rice","qty":1,"reason":"spoiled","by":"chef"}"#);
    let bad = rewritten(&t, r#"{"k":"served","item":"rice","qty":1,"order":"o-9","phone":"+355690000000"}"#);
    assert_eq!(refused(&mut h, &bad), Why::Tracking { pos: 0, key: "phone".into() });
    let taste = rewritten(&t, r#"{"k":"served","item":"rice","qty":1,"order":"o-9","taste":"umami"}"#);
    assert_eq!(refused(&mut h, &taste), Why::Tracking { pos: 0, key: "taste".into() });
    // The twin: the same sale without the key is admitted.
    let good = rewritten(&t, r#"{"k":"served","item":"rice","qty":1,"order":"o-9"}"#);
    assert_eq!(merge(&h.to_bytes_trimmed(), &mut h, &good).unwrap().merged, 1);
}

#[test]
fn a_record_edited_after_it_was_chained_is_refused() {
    let (mut h, t) = one(r#"{"k":"wasted","item":"rice","qty":1,"reason":"spoiled","by":"chef"}"#);
    let mut forged = t.clone();
    forged.recs[0] = forged.recs[0].replace("\"qty\":1", "\"qty\":9");
    assert_eq!(refused(&mut h, &forged), Why::TipMismatch);
    assert_eq!(merge(&h.to_bytes_trimmed(), &mut h, &t).unwrap().merged, 1, "the record as chained glues");
}

#[test]
fn a_base_the_hub_never_held_is_refused() {
    let (mut h, t) = one(r#"{"k":"wasted","item":"rice","qty":1,"reason":"spoiled","by":"chef"}"#);
    let mut other = t.clone();
    other.base = [7u8; 32];
    other.tip = crate::content_id_chained(&other.base, other.recs[0].as_bytes());
    assert_eq!(refused(&mut h, &other), Why::NoCommonAncestor);
}

#[test]
fn a_note_or_a_grant_is_not_a_tablets_to_send() {
    let (mut h, t) = one(r#"{"k":"wasted","item":"rice","qty":1,"reason":"spoiled","by":"chef"}"#);
    let grant = rewritten(&t, r#"{"k":"note","note":"rights","ctx":"kitchen","item":"salmon","qty":99}"#);
    assert_eq!(refused(&mut h, &grant), Why::Tracking { pos: 0, key: "note".into() });
    let unknown = rewritten(&t, r#"{"k":"teleported","item":"rice","qty":1}"#);
    assert_eq!(refused(&mut h, &unknown), Why::NotAMovement { pos: 0 });
    let long = rewritten(&t, &format!(r#"{{"k":"wasted","item":"{}","qty":1,"reason":"spoiled","by":"chef"}}"#, "r".repeat(REC_MAX)));
    assert_eq!(refused(&mut h, &long), Why::TooLong { pos: 0 });
}

#[test]
fn keys_are_read_from_flat_records_only() {
    assert_eq!(keys_of(r#"{"k":"served","qty":1,"order":"a,\"b}"}"#).unwrap(), vec!["k", "qty", "order"]);
    assert_eq!(keys_of("{}").unwrap(), Vec::<String>::new());
    assert!(keys_of(r#"{"k":"x","card":{"phone":"1"}}"#).is_none(), "a nested object is not a stock record");
    assert!(keys_of(r#"{"k":"x","list":[1]}"#).is_none());
    assert!(keys_of(r#"{"k":"x""#).is_none());
}
