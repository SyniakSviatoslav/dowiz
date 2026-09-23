//! G3, native half (§5): "`Hub::forget` on a 40-record image with 6 of the
//! subject's -> 6 redacted, `chain_check.chained + redacted == records`, `tip()`
//! unchanged, `holds(old_tip)` true."

use super::*;

const ACTOR: [u8; 32] = [0xA1; 32];
const SUBJECT: &str = "+355691111111";

fn order(id: &str, phone: &str, status: &str) -> String {
    format!(
        r#"{{"id":"{id}","status":"{status}","total":1800,"contact":{{"name":"N {id}","phone":"{phone}"}},"fulfilment":{{"kind":"delivery","address":{{"line":"Rruga {id}"}}}}}}"#
    )
}

/// Ten orders of four events each; orders 3 and 7 are the subject's, but only
/// three of each of their events carry the contact (the fourth is a status
/// delta), so the subject has exactly 6 records.
fn forty() -> Hub {
    let mut h = Hub::create_sized(1 << 20).unwrap();
    let mut seq = 0;
    for o in 0..10 {
        let id = format!("ord_{o}");
        let phone = if o == 3 || o == 7 { SUBJECT.to_string() } else { format!("+3556900000{o:02}") };
        for s in ["PENDING", "CONFIRMED", "PREPARING"] {
            seq += 1;
            let kind = if s == "PENDING" { EventKind::Placed } else { EventKind::Advanced };
            h.append(kind, &id, &order(&id, &phone, s), seq, ACTOR).unwrap();
        }
        seq += 1;
        h.append(EventKind::Advanced, &id, r#"{"_d":true,"status":"DELIVERED"}"#, seq, ACTOR).unwrap();
    }
    h
}

/// The Worker's redactor, in miniature: the person's fields emptied, the rest
/// byte-for-byte.
fn redactor(e: &Event) -> Option<String> {
    if !e.order_json.contains(SUBJECT) {
        return None;
    }
    let id = &e.order_id;
    Some(e.order_json.replace(SUBJECT, "").replace(&format!("N {id}"), "").replace(&format!("Rruga {id}"), ""))
}

#[test]
fn six_records_redacted_in_place_keep_the_chain_the_tip_and_the_money() {
    let mut h = forty();
    assert_eq!(h.len(), 40);
    let tip = h.tip().unwrap();
    let before: Vec<(EventKind, String)> = h.events().into_iter().map(|e| (e.kind, e.order_id)).collect();

    assert_eq!(h.redact(redactor).unwrap(), 6);

    let c = h.chain_check();
    assert_eq!(c.records, 40);
    assert_eq!(c.redacted, 6, "{c:?}");
    assert_eq!(c.chained + c.redacted, c.records, "{c:?}");
    assert_eq!(c.broken, 0);
    assert_eq!(h.tip().unwrap(), tip, "the tip does not move");
    assert!(h.holds(&tip), "last night's witness still holds");

    // EVERY READER STILL READS EVERY RECORD, under the kind it had.
    assert!(h.quarantined().is_empty());
    let after: Vec<(EventKind, String)> = h.events().into_iter().map(|e| (e.kind, e.order_id)).collect();
    assert_eq!(after, before, "same kinds, same orders, same order");
    assert!(h.events().iter().all(|e| !e.order_json.contains(SUBJECT)), "the number is gone");
    let o3 = h.history("ord_3");
    assert!(o3[0].order_json.contains(r#""total":1800"#), "the money stays");
    assert!(o3[0].order_json.contains(r#""phone":"""#), "the person does not");

    // AND IT SURVIVES THE BYTES: a reloaded image says the same.
    let back = Hub::load(&h.to_bytes_trimmed()).unwrap();
    assert_eq!(back.chain_check(), c);

    // A SECOND PASS FINDS NOTHING: a tombstone is not redacted twice.
    assert_eq!(h.redact(redactor).unwrap(), 0);
}

/// THE DECLARATION IS THE OTHER SIDE OF LAW 9, and it is not an order.
#[test]
fn the_declaration_counts_what_was_redacted_and_folds_into_nothing() {
    let mut h = forty();
    let n = h.redact(redactor).unwrap();
    let orders = h.orders().len();
    h.append(EventKind::Forgotten, "cust:0123456789abcdef", &format!(r#"{{"records":{n},"hot":{n}}}"#), 99, [0u8; 32])
        .unwrap();
    assert_eq!(h.declared(), 6);
    assert_eq!(h.chain_check().redacted, h.declared(), "tombstones = declarations");
    assert!(!EventKind::Forgotten.is_order());
    assert_eq!(h.orders().len(), orders, "a declaration is not a sale");
    let back = Hub::load(&h.to_bytes_trimmed()).unwrap();
    assert!(back.quarantined().is_empty(), "this build reads kind 8");
}

/// A TOMBSTONE WHOSE LINK IS CUT IS BROKEN, not redacted: the high bit is not
/// a licence to edit, only a declaration that the edit kept its place.
#[test]
fn a_tombstone_without_its_link_is_broken() {
    let mut h = forty();
    h.redact(redactor).unwrap();
    let mut walked = EvLog::walk(&h.store);
    let tip = EvLog::tip(&h.store);
    let at = walked.iter().position(|r| r.payload[0] & REDACTED_BIT != 0).unwrap();
    assert!(tombstone_holds(&walked, at, tip));
    // Cut the link: the record after it no longer names it.
    if at > 0 {
        walked[at - 1].prev = [0x55; 32];
        assert!(!tombstone_holds(&walked, at, tip));
    }
    // And an edited record with NO high bit is broken as it always was.
    assert!(!tombstone_holds(&walked, walked.len() - 1, tip) || walked[walked.len() - 1].payload[0] & REDACTED_BIT != 0);
}
