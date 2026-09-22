//! How an order reaches its customer, over nothing but a string and a number.

use super::*;
use serde_json::json;

/// THE DEFECT THIS SET CLOSES. `kind` came off the wire as a free string and
/// only two words were ever compared against, so anything else was accepted,
/// charged the full delivery fee, and never asked for an address — because the
/// address check fires on the exact word `"delivery"` and nothing else.
#[test]
fn a_kind_nobody_has_heard_of_is_not_a_kind() {
    assert!(known("delivery") && known("pickup") && known("dine_in"));
    assert!(!known("banana"));
    assert!(!known("DELIVERY"), "the wire is lower case; a near miss is a miss");
    assert!(!known(""));
}

/// ONLY DELIVERY LEAVES THE BUILDING, and it is an allow-list on purpose: the
/// FOURTH kind, whenever it arrives, must be refused a courier by default
/// rather than inherit one from `!= "pickup"`.
#[test]
fn only_a_delivery_gets_a_courier_an_eta_and_a_fee() {
    assert!(leaves_the_building("delivery"));
    assert!(!leaves_the_building("pickup"));
    assert!(!leaves_the_building("dine_in"));
    assert!(!leaves_the_building("a_kind_invented_next_year"));
}

/// A TABLE PAYS NO DELIVERY FEE. The fee buys a courier's trip; there is no
/// trip. This is the line that would have charged 300 lek to somebody sitting
/// in the restaurant.
#[test]
fn nobody_who_is_already_in_the_room_pays_to_have_it_brought() {
    assert_eq!(fee("delivery", 1000, None, 300), 300);
    assert_eq!(fee("pickup", 1000, None, 300), 0);
    assert_eq!(fee("dine_in", 1000, None, 300), 0);
}

/// AND THE FREE-DELIVERY THRESHOLD IS CHECKED FIRST, FOR EVERY KIND. A venue
/// that set one has said "over this much, no fee"; a kind that never had a fee
/// is already at zero, so the order of these two rules cannot change an answer
/// — which is why it is asserted rather than assumed.
#[test]
fn the_threshold_is_asked_before_the_kind_and_it_changes_nothing() {
    assert_eq!(fee("delivery", 2000, Some(2000), 300), 0, "at the threshold, not over it");
    assert_eq!(fee("delivery", 1999, Some(2000), 300), 300);
    assert_eq!(fee("dine_in", 1999, Some(2000), 300), 0);
    assert_eq!(fee("dine_in", 2000, Some(2000), 300), 0);
}

/// EACH KIND CANNOT EXIST WITHOUT ITS OWN THING, and `dine_in` is NOT `pickup`
/// with a table: they agree on everything the courier, the ETA and the tally
/// ask, and differ here. Folding them together would allow an order placed for
/// "the restaurant" with no table on it — an order nobody can carry anywhere.
#[test]
fn a_table_order_cannot_exist_without_a_table() {
    assert_eq!(needs("delivery"), Needs::Address);
    assert_eq!(needs("dine_in"), Needs::Table);
    assert_eq!(needs("pickup"), Needs::Nothing);
}

/// AN ORDER WITH NO KIND IS A DELIVERY, and the rule has a date on it: orders
/// restored from an archive or an old backup predate this field, and a fold
/// that turned a missing value into an unknown kind would hit no branch at all.
#[test]
fn an_order_from_before_this_existed_is_a_delivery() {
    assert_eq!(of(&json!({})), "delivery");
    assert_eq!(of(&json!({ "fulfilment": {} })), "delivery");
    assert_eq!(of(&json!({ "fulfilment": { "kind": "dine_in" } })), "dine_in");
    // AND SO IS ONE CARRYING A KIND THIS BUILD DOES NOT KNOW. Answering with
    // the unknown word would put it through every `match` arm's `_` branch;
    // answering "delivery" puts it through the strictest one.
    assert_eq!(of(&json!({ "fulfilment": { "kind": "banana" } })), "delivery");
}

/// The table is read in one place and trimmed there, so a table named " " is
/// no table — which matters, because that is what an empty form field sends.
#[test]
fn a_blank_table_is_not_a_table() {
    assert_eq!(table_of(&json!({ "fulfilment": { "table": "12" } })), Some("12"));
    assert_eq!(table_of(&json!({ "fulfilment": { "table": "  7 " } })), Some("7"));
    assert_eq!(table_of(&json!({ "fulfilment": { "table": "   " } })), None);
    assert_eq!(table_of(&json!({ "fulfilment": { "kind": "pickup" } })), None);
}

/// THE STOCK IS THE SAME STOCK, and this test exists because that is true by
/// CONSTRUCTION and construction can be changed by accident.
///
/// `place_command::decide` reserves ingredients before it looks at anything
/// else and never reads the fulfilment, so a table order and a delivery order
/// compete for the last portion of salmon on equal terms. The operator asked
/// for exactly that. If somebody later passes the kind into the reservation to
/// "skip stock for dine-in", this is what stops it.
#[test]
fn a_table_order_and_a_delivery_draw_on_one_shelf() {
    let dish = json!({ "id": "d1", "bom": [{ "supply": "salmon", "qty": 40 }] }).to_string();
    let mut stock = dowiz_hub::stock::StockLog::create_sized(64 * 1024).expect("stock");
    stock
        .append_all(&[dowiz_hub::stock::StockEvent::Received { item: "salmon".into(), qty: 40 }])
        .expect("receive");

    // The delivery takes the last forty grams.
    let first = dowiz_hub::stock::reservations_for("delivery-1", &[(dish.clone(), 1)]);
    stock.append_all(&first).expect("the delivery reserves");

    // The table order, for the same dish, finds the shelf empty. If the two
    // ever stop sharing a ledger this line starts passing for the wrong reason.
    let second = dowiz_hub::stock::reservations_for("table-1", &[(dish, 1)]);
    let refused = stock.append_all(&second);
    assert!(refused.is_err(), "the shelf is shared: the second order must be refused");
    let msg = format!("{}", refused.unwrap_err()).to_lowercase();
    assert!(msg.contains("salmon"), "and the refusal must name the ingredient: {msg}");
}
