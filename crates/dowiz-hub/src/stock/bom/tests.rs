use super::*;

const ROLL: &str = r#"{"id":"p1","name":"Sake","bom":[{"supply":"salmon","qty":40},{"supply":"rice","qty":90}]}"#;
const MAKI: &str = r#"{"id":"p2","name":"Ebi","bom":[{"supply":"rice","qty":60},{"supply":"prawn","qty":30}]}"#;
const WATER: &str = r#"{"id":"p3","name":"Water","price":100}"#;

#[test]
fn a_recipe_reads_back() {
    assert_eq!(
        bom_of(ROLL),
        vec![
            BomLine::whole("salmon", 40),
            BomLine::whole("rice", 90),
        ]
    );
}

/// A dish with no recipe reserves nothing, and that is a normal venue --
/// a bought-in bottle of water has no bill of materials worth keeping.
#[test]
fn a_dish_with_no_recipe_is_not_an_error() {
    assert!(bom_of(WATER).is_empty());
    assert!(reservations_for("o1", &[(WATER.into(), 3)]).is_empty());
}

/// Quantities multiply. Getting this wrong is how a kitchen runs out
/// mid-service while the ledger says it is fine.
#[test]
fn quantities_multiply_by_the_portions_ordered() {
    let evs = reservations_for("o1", &[(ROLL.into(), 2)]);
    let salmon = evs.iter().find(|e| e.item() == "salmon").expect("salmon");
    match salmon {
        StockEvent::Reserved { qty, .. } => assert_eq!(*qty, 80),
        other => panic!("{other:?}"),
    }
}

/// Two different dishes sharing an ingredient are checked against the
/// TOTAL they need, not one line at a time.
#[test]
fn a_shared_ingredient_is_summed_across_the_basket() {
    let evs = reservations_for("o1", &[(ROLL.into(), 1), (MAKI.into(), 2)]);
    let rice = evs.iter().find(|e| e.item() == "rice").expect("rice");
    match rice {
        // 90 for one roll + 60x2 for two maki
        StockEvent::Reserved { qty, .. } => assert_eq!(*qty, 210),
        other => panic!("{other:?}"),
    }
    assert_eq!(evs.len(), 3, "salmon, rice, prawn — one event each");
    // Sorted, so the same basket always produces the same sequence.
    let names: Vec<&str> = evs.iter().map(|e| e.item()).collect();
    assert_eq!(names, vec!["prawn", "rice", "salmon"]);
}

/// The whole reason to route a basket through the ledger.
#[test]
fn a_basket_that_exceeds_the_shelf_reserves_nothing() {
    let mut log = StockLog::create().expect("create");
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 100 }).unwrap();
    log.append(&StockEvent::Received { item: "rice".into(), qty: 1000 }).unwrap();

    // Three portions need 120g of salmon and there are 100.
    let evs = reservations_for("o1", &[(ROLL.into(), 3)]);
    assert!(log.append_all(&evs).is_err());
    assert_eq!(log.ledger().unwrap().level("rice").reserved, 0, "rice was not held either");

    // Two portions fit.
    let evs = reservations_for("o2", &[(ROLL.into(), 2)]);
    assert!(log.append_all(&evs).is_ok());
    assert_eq!(log.ledger().unwrap().available("salmon"), 20);
}

/// Settlement comes from what the LEDGER holds, not from the basket: if the
/// recipe changed between placing and cooking, releasing a recomputed
/// quantity would strand the difference forever.
#[test]
fn settlement_releases_exactly_what_was_reserved() {
    let mut log = StockLog::create().expect("create");
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 200 }).unwrap();
    log.append(&StockEvent::Received { item: "rice".into(), qty: 500 }).unwrap();
    log.append_all(&reservations_for("o1", &[(ROLL.into(), 1)])).unwrap();

    let led = log.ledger().unwrap();
    let release = settle(&led, "o1", false);
    assert_eq!(release.len(), 2);
    log.append_all(&release).unwrap();

    let led = log.ledger().unwrap();
    assert!(led.stranded().is_empty(), "nothing left held");
    assert_eq!(led.level("salmon"), StockLevel { on_hand: 200, reserved: 0 });

    // And consuming instead takes it off the shelf.
    log.append_all(&reservations_for("o2", &[(ROLL.into(), 1)])).unwrap();
    let led = log.ledger().unwrap();
    log.append_all(&settle(&led, "o2", true)).unwrap();
    assert_eq!(log.ledger().unwrap().level("salmon"), StockLevel { on_hand: 160, reserved: 0 });
}

/// Settling one order must not touch another's reservations.
#[test]
fn settlement_is_scoped_to_its_own_order() {
    let mut log = StockLog::create().expect("create");
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 500 }).unwrap();
    log.append(&StockEvent::Received { item: "rice".into(), qty: 500 }).unwrap();
    log.append_all(&reservations_for("o1", &[(ROLL.into(), 1)])).unwrap();
    log.append_all(&reservations_for("o2", &[(ROLL.into(), 1)])).unwrap();

    let led = log.ledger().unwrap();
    log.append_all(&settle(&led, "o1", false)).unwrap();
    let led = log.ledger().unwrap();
    assert_eq!(led.stranded().len(), 2, "o2 still holds its two lines");
    assert!(led.stranded().iter().all(|(o, _, _)| o == "o2"));
}

#[test]
fn a_malformed_recipe_is_ignored_rather_than_fatal() {
    for junk in [
        r#"{"id":"p","bom":"not an array"}"#,
        r#"{"id":"p","bom":[]}"#,
        r#"{"id":"p","bom":[{"supply":"","qty":5}]}"#,
        r#"{"id":"p","bom":[{"supply":"x","qty":0}]}"#,
        r#"{"id":"p","bom":[{"supply":"x","qty":-3}]}"#,
        r#"{"id":"p"}"#,
    ] {
        assert!(bom_of(junk).is_empty(), "accepted {junk}");
    }
}
