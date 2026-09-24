//! THE WORKER'S HALF OF THE D7 GATE (`crates/bebop-wasm/tests/decide_agrees.rs`
//! is the other). The object's own path over the committed images -- the log
//! folded by `hubstore::orders_state` (what `orders_view` serves), the order
//! found in it, `command::amend::decide` / `command::pay::decide` -- must write
//! the committed delta BYTE FOR BYTE. The wasm export and the wasm32 module
//! under node are held to the same files, so three builds answer one set of
//! bytes and none of them wrote it.

use super::*;
use crate::command::pay;
use dowiz_hub::stock::StockLog;

macro_rules! fixture {
    ($name:literal) => {
        include_bytes!(concat!("../../../../../crates/bebop-wasm/fixtures/decide/", $name)).as_slice()
    };
}

/// The order the object would hand its decider: from the WHOLE projection,
/// exactly as `hubdo/room.rs` finds it -- not from `room::view::current`, so
/// the two derivations are held to one answer too.
fn listed(log: &dowiz_hub::Hub, order_id: &str) -> Option<OrderView> {
    crate::hubstore::orders_state(log).into_iter().map(OrderView::of_event).find(|o| o.order_id == order_id)
}

fn amend_through_the_worker(input: &[u8]) -> (u16, Vec<u8>) {
    let input: AmendIn = serde_json::from_slice(input).expect("an AmendIn");
    let mut hub = dowiz_hub::Hub::load(fixture!("amend.log")).expect("the log image");
    let mut stock = StockLog::load(fixture!("amend.stock")).expect("the stock image");
    let current = listed(&hub, &input.order_id);
    match decide(&mut hub, &mut stock, current.as_ref(), &input) {
        Ok((_, body, _)) => (0, body.into_bytes()),
        Err(r) => (r.status(), r.message().as_bytes().to_vec()),
    }
}

#[test]
fn the_worker_path_writes_the_committed_amend_delta_byte_for_byte() {
    let (status, body) = amend_through_the_worker(fixture!("amend.in.json"));
    assert_eq!(status, 0, "{}", String::from_utf8_lossy(&body));
    assert_eq!(String::from_utf8_lossy(&body), String::from_utf8_lossy(fixture!("amend.delta")));
}

/// The refusal agrees too: the same kind (409, `decide_abi::CONFLICT` = 34 on
/// the wasm side) and the same words.
#[test]
fn the_worker_path_refuses_the_stale_edit_in_the_committed_words() {
    let (status, words) = amend_through_the_worker(fixture!("amend_stale.in.json"));
    assert_eq!(status, 409);
    let committed = String::from_utf8_lossy(fixture!("amend_stale.refusal")).into_owned();
    assert_eq!(committed.strip_prefix("34 "), Some(String::from_utf8_lossy(&words).as_ref()));
}

#[test]
fn the_worker_path_writes_the_committed_pay_delta_byte_for_byte() {
    let input: pay::PayIn = serde_json::from_slice(fixture!("pay.in.json")).expect("a PayIn");
    let room: Value = serde_json::from_slice(fixture!("pay.room.json")).expect("the room");
    let room = pay::Room { open_till: room["open_till"].as_str(), venue_currency: room["venue_currency"].as_str().unwrap() };
    let mut hub = dowiz_hub::Hub::load(fixture!("pay.log")).expect("the log image");
    let current = listed(&hub, &input.order_id);
    let (_, body, _) = pay::decide(&mut hub, current.as_ref(), &input, &room).expect("the payment lands");
    assert_eq!(body, String::from_utf8_lossy(fixture!("pay.delta")));
}
