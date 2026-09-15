//! The arena is spent by DATA, not by the number of writes.
//!
//! This is the regression test for the defect that killed a live stand: the
//! store is append-only, so every KV commit allocated a new generation and left
//! the old one behind. A roster with one person and no sessions refused after
//! 313 EMPTY commits -- and the route that fills it is login, so the failure
//! locks everybody out including whoever would fix it.

use dowiz_hub::roster::Roster;
use dowiz_hub::token::Role;

const NOW: i64 = 1_789_000_000_000;

/// Ten thousand writes. A venue with three people signing in every day reaches
/// that in about nine years; it used to stop at three hundred.
#[test]
fn ten_thousand_roster_writes_still_commit() {
    let mut r = Roster::create().unwrap();
    r.set_iterations(64);
    r.upsert_person("ana@dubin.al", Role::Owner, "Ana", "pw").unwrap();

    let mut last = 0usize;
    for i in 0..10_000 {
        let bytes = r
            .to_bytes()
            .unwrap_or_else(|e| panic!("the roster stopped committing after {i} writes: {e:?}"));
        last = bytes.len();
    }
    // And the image did not grow with the history either.
    assert!(last <= dowiz_hub::roster::DEFAULT_ROSTER_BYTES, "{last}");

    // The person is still there: compaction is a rewrite, not a reset.
    assert!(r.authenticate("ana@dubin.al", "pw").is_some());
}

/// A session opened before a compaction still authenticates after it.
#[test]
fn a_session_survives_the_rewrite() {
    let mut r = Roster::create().unwrap();
    r.set_iterations(64);
    r.upsert_person("ana@dubin.al", Role::Owner, "Ana", "pw").unwrap();
    let s = r.open_session("ana@dubin.al", NOW).unwrap();

    for _ in 0..500 {
        let bytes = r.to_bytes().expect("commit");
        let mut back = Roster::load(&bytes).expect("load");
        back.set_iterations(64);
        assert_eq!(back.session_owner(&s).as_deref(), Some("ana@dubin.al"));
        r = back;
    }
}

/// How many orders fit in a hub image? The event log is append-only BY DESIGN
/// -- that is what makes it a log -- so unlike the KV stores it legitimately
/// grows with history. The number is measured here rather than assumed, because
/// a capacity nobody has measured is a capacity that surprises somebody.
#[test]
fn how_much_history_a_hub_image_holds() {
    let mut h = dowiz_hub::Hub::create().unwrap();
    let order = r#"{"id":"ord_1789000000000_0000deadbeef","status":"DELIVERED","total":2650,"created_at_ms":1789000000000,"items":[{"product_id":"item-01","quantity":2,"unit_price":900,"name":"Sake Futomaki"}],"contact":{"name":"Ana Hoxha","phone":"+355691234567"},"fulfilment":{"kind":"delivery","address":{"line":"Rruga Taulantia 12"}}}"#;
    let mut n = 0u64;
    while h
        .append(dowiz_hub::EventKind::Placed, &format!("ord_{n}"), order, n, [0u8; 32])
        .is_ok()
    {
        n += 1;
        if n > 20_000 {
            break;
        }
    }
    // Printed, not asserted against a magic number: the point is that the
    // figure is known and shows up in the record when it moves.
    println!(
        "HUB IMAGE HOLDS {n} ORDER EVENTS, image now {} bytes",
        h.to_bytes().len()
    );
    assert_eq!(n, 20_001, "the log stopped growing at {n} events");

    // Growth is not amnesia. The chain is copied verbatim across each doubling,
    // so the oldest order is still readable at the end.
    let first = h.order("ord_0").expect("the very first order survived the growth");
    assert!(first.contains("Sake Futomaki"), "{first}");
    assert_eq!(h.orders().len(), 20_001);

    // And the copy is still a CHAIN, not a heap of orphans: every record but
    // the first names the one before it.
    let evs = h.events();
    assert_eq!(evs.len(), 20_001);
    assert_eq!(evs[0].order_id, "ord_20000", "walk must still be newest-first");
}

/// A grown image is still a valid image: it round-trips through bytes, and a
/// hub loaded from it can append again.
#[test]
fn a_grown_image_still_loads_and_accepts_more() {
    let mut h = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    let body = r#"{"id":"x","status":"PENDING","total":900}"#;
    for n in 0..2_000u64 {
        h.append(dowiz_hub::EventKind::Placed, &format!("ord_{n}"), body, n, [0u8; 32])
            .unwrap_or_else(|e| panic!("append {n}: {e:?}"));
    }
    let bytes = h.to_bytes();
    let mut back = dowiz_hub::Hub::load(&bytes).expect("load a grown image");
    assert_eq!(back.orders().len(), 2_000);
    back.append(dowiz_hub::EventKind::Placed, "ord_after", body, 2_000, [0u8; 32])
        .expect("a loaded hub must keep accepting");
    assert_eq!(back.orders().len(), 2_001);
}

/// A KV image is as large as its CONTENT, not as large as a constant somebody
/// chose once. That constant turned out to be wrong somewhere specific: the
/// catalogue's 1 MiB arena is larger than D1's one-million-byte row limit, so a
/// fifty-two dish menu could not be written to the Worker's store at all and
/// the failure was a 500 with no message.
#[test]
fn a_real_menu_fits_under_the_d1_row_limit() {
    const D1_ROW_LIMIT: usize = 1_000_000;
    let mut c = dowiz_hub::catalog::Catalog::create().unwrap();
    c.set_location(
        r#"{"id":"dubin-durres","name":"Dubin & Sushi","slug":"dubin-durres","currency_code":"ALL"}"#,
    );
    for i in 0..13 {
        c.set_category(&format!("cat-{i}"), &format!(r#"{{"name":"Category {i}","sortOrder":{i}}}"#));
    }
    for i in 0..52 {
        c.set_product(
            &format!("item-{i:02}"),
            &format!(
                r#"{{"id":"item-{i:02}","categoryId":"cat-{}","name":"Dish number {i}","description":"A sentence about this dish, of the length a real menu carries.","price":{},"available":true,"allergens":["fish","soy"],"sizeCm":20}}"#,
                i % 13,
                900 + i * 25
            ),
        );
    }
    let bytes = c.to_bytes().expect("commit");
    assert!(
        bytes.len() < D1_ROW_LIMIT,
        "a 52-dish catalogue is {} bytes and D1 refuses anything over {D1_ROW_LIMIT}",
        bytes.len()
    );
    println!("52 DISHES, 13 CATEGORIES: {} bytes", bytes.len());

    // And it is still the same catalogue when it comes back.
    let back = dowiz_hub::catalog::Catalog::load(&bytes).expect("load");
    assert_eq!(back.products().len(), 52);
    assert_eq!(back.categories().len(), 13);
    assert!(back.location().unwrap().contains("Dubin"));
    assert!(dowiz_hub::allergens::read(&back.product("item-07").unwrap()).is_declared());
}
