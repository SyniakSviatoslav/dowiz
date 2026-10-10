use crate::*;

/// An audit entry must not be able to masquerade as an order. Every fold in
/// the system -- the takings, the promo use-count, the dashboard -- is built
/// on `orders()`, and one bogus row in it is a number that is quietly wrong
/// everywhere at once.
#[test]
fn a_reveal_is_not_an_order() {
    let mut h = Hub::create().unwrap();
    h.append(EventKind::Placed, "ord_1", r#"{"total":900}"#, 1, [0u8; 32]).unwrap();
    h.append(
        EventKind::Revealed,
        "cust:+355690000000",
        r#"{"by":"ana@dubin.al","at":1789000000000}"#,
        2,
        [0u8; 32],
    )
    .unwrap();
    h.append(EventKind::Placed, "ord_2", r#"{"total":850}"#, 3, [0u8; 32]).unwrap();

    let orders = h.orders();
    assert_eq!(orders.len(), 2, "the audit entry took an order's place");
    assert!(orders.iter().all(|e| e.kind.is_order()));
    assert!(orders.iter().all(|e| e.order_id.starts_with("ord_")));

    assert_eq!(h.reveals().len(), 1);
    assert_eq!(h.reveals()[0].order_id, "cust:+355690000000");
    // It survives a round trip through the image, like everything else.
    let back = Hub::load(&h.to_bytes()).unwrap();
    assert_eq!(back.reveals().len(), 1);
    assert_eq!(back.orders().len(), 2);
}

/// The audit payload must not contain what it audits: a log of who read a
/// phone number that also holds the phone number has doubled the exposure.
#[test]
fn the_audit_entry_carries_no_contact_details() {
    let mut h = Hub::create().unwrap();
    h.append(
        EventKind::Revealed,
        "cust:8f2a9c",
        r#"{"by":"ana@dubin.al","at":1789000000000}"#,
        1,
        [0u8; 32],
    )
    .unwrap();
    let e = &h.reveals()[0];
    for leak in ["+355", "@gmail", "Rruga"] {
        assert!(!e.order_json.contains(leak), "the audit entry carries {leak}");
    }
}

/// The `at`-th record of a newest-first walk (its object index).
fn nth(st: &bebop_store::Store, at: usize) -> usize {
    let mut o = st.follow(st.root().unwrap(), 1).unwrap();
    for _ in 0..at {
        o = st.follow(o, 2).unwrap();
    }
    o
}

fn hub_of(n: usize) -> Hub {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    for i in 0..n {
        let kind = if i % 3 == 0 { EventKind::Placed } else { EventKind::Advanced };
        h.append(kind, &format!("ord-{}", i / 3), &format!(r#"{{"status":"s{i}","note":"{}"}}"#, "x".repeat(i % 23)), i as u64, [0u8; 32]).unwrap();
    }
    h
}

/// `events()` READS THE LOAD'S CRC VERDICTS INSTEAD OF HASHING AGAIN (W-LOOPB, R-LOOPS row 6),
/// and answers exactly what the re-hashing walk answers: on a clean log, and with ONE NAMED
/// CORRUPTED CELL -- payload cell 12 of the newest, a middle and the oldest record, flipped and
/// not re-sealed -- after a reload, after appends, after the grow that carries the failed crc,
/// and after a redaction that re-lays the chain (where it falls back to the long walk).
#[test]
fn events_from_the_carried_scan_equal_the_rehashing_walk() {
    let clean = hub_of(40).to_bytes_trimmed();
    let mut cases = vec![("clean", clean.clone(), None)];
    for at in [0usize, 17, 39] {
        let mut st = bebop_store::Store::from_bytes(&clean);
        let o = nth(&st, at);
        st.cells[o + 2 + 12] ^= 0x100;
        cases.push(("flipped", st.to_bytes(), Some(at)));
    }
    for (what, bytes, bad) in cases {
        let mut h = Hub::load(&bytes).unwrap();
        let q = h.quarantined();
        assert_eq!(q.iter().map(|q| (q.at, q.reason)).collect::<Vec<_>>(), bad.map(|a| (a, "crc")).into_iter().collect::<Vec<_>>(), "{what}");
        assert_eq!(h.events(), h.events_marked(), "{what} {bad:?}: loaded");
        assert_eq!(h.len(), h.events().len() + q.len(), "{what}: the law");
        // Appends past a 64 KiB birth size: at least one grow, which carries the bad crc.
        for i in 0..200 {
            h.append(EventKind::Placed, &format!("new-{i}"), &format!(r#"{{"status":"new","pad":"{}"}}"#, "y".repeat(200)), 100 + i, [0u8; 32]).unwrap();
        }
        assert!(h.to_bytes().len() > 64 * 1024, "the appends must have grown the image");
        assert_eq!(h.events(), h.events_marked(), "{what} {bad:?}: appended and grown");
        assert_eq!(h.quarantined().len(), bad.iter().count(), "{what}: still quarantined");
        let appended = h.appended().expect("only appended since the load");
        assert_eq!(appended, h.events()[..200].to_vec(), "{what}: appended() is the newest 200 events");
        // A redaction re-lays the chain: the carried scan is dropped, not trusted.
        h.redact(|e| (e.order_id == "ord-1").then(|| r#"{"status":"erased"}"#.to_string())).unwrap();
        assert!(h.appended().is_none(), "{what}: a re-laid chain is not 'appended since the load'");
        assert_eq!(h.events(), h.events_marked(), "{what}: redacted");
    }
}

/// THE CARRIED SCAN MUST NOT OUTLIVE ITS CHAIN: a rotation re-lays it too, and the events read
/// after one are the long walk's. `appended()` of a fresh hub is everything, of a loaded one
/// nothing until it appends.
#[test]
fn a_rotation_drops_the_carried_scan_and_appended_counts_from_the_load() {
    let mut h = hub_of(12);
    assert_eq!(h.appended().unwrap(), h.events(), "a created hub appended all of it");
    let mut back = Hub::load(&h.to_bytes_trimmed()).unwrap();
    assert_eq!(back.appended().unwrap(), Vec::<Event>::new());
    back.append(EventKind::Placed, "ord-x", r#"{"status":"new"}"#, 99, [0u8; 32]).unwrap();
    assert_eq!(back.appended().unwrap().iter().map(|e| e.order_id.as_str()).collect::<Vec<_>>(), vec!["ord-x"]);
    h.rotate(|id| id != "ord-0").unwrap();
    assert!(h.appended().is_none());
    assert_eq!(h.events(), h.events_marked());
}
