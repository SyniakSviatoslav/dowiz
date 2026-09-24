//! THE D7 GATE: the deciders exported here decide the bytes the Worker's
//! object decides.
//!
//! Three builds are held to ONE committed answer (`fixtures/decide/*.delta`):
//! this crate's export called natively (below), the Worker's own path
//! (`workers/api/src/command/amend/agrees.rs`: `hubstore::orders_state` +
//! `command::amend::decide`, the object's handler minus its I/O), and the
//! wasm32 module under node (`decide-gate.sh`). None of them writes the
//! answer; `BEBOP_WASM_BLESS=1` does, from the hub's typed call, and a changed
//! answer is a diff a reviewer reads.
#![cfg(feature = "decide")]

#[path = "decide/case.rs"]
mod case;

use bebop_wasm::decide::{amend_decide, amend_images, pay_decide, Refusal};
use bebop_wasm::decide_abi::{self, bw_amend, bw_pay};
use dowiz_hub::room::{amend, pay, view, Refused};
use dowiz_hub::stock::{StockEvent, StockLog};
use dowiz_hub::Hub;

/// What the hub's typed API decides for the committed inputs -- the answer
/// `BEBOP_WASM_BLESS=1` writes. (status, bytes): 0 and the delta, or the
/// ABI's refusal code and the words.
fn typed(log: &[u8], stock: &[u8], input: &[u8]) -> (i32, Vec<u8>) {
    let input: amend::AmendIn = serde_json::from_slice(input).unwrap();
    let mut h = Hub::load(log).unwrap();
    let mut s = StockLog::load(stock).unwrap();
    let cur = view::current(&h, &input.order_id);
    match amend::decide(&mut h, &mut s, cur.as_ref(), &input) {
        Ok((_, body, _)) => (0, body.into_bytes()),
        Err(r) => {
            let (code, words) = decide_abi::status_of(&Refusal::Refused(r));
            (code, format!("{code} {words}").into_bytes())
        }
    }
}

fn typed_pay(log: &[u8], input: &[u8]) -> Vec<u8> {
    let input: pay::PayIn = serde_json::from_slice(input).unwrap();
    let mut h = Hub::load(log).unwrap();
    let room = pay::Room { open_till: Some("main"), venue_currency: "ALL" };
    let cur = view::current(&h, &input.order_id);
    pay::decide(&mut h, cur.as_ref(), &input, &room).expect("the fixture payment lands").1.into_bytes()
}

/// The committed inputs ARE what `case.rs` builds, and the committed answers
/// are what the typed hub decides for them. Bless only on request.
#[test]
fn the_fixtures_are_what_the_case_builds() {
    let (log, stock) = (case::amend_log(), case::amend_stock());
    let answers = [
        ("amend.delta", typed(&log, &stock, &case::amend_in()).1),
        ("amend_stale.refusal", typed(&log, &stock, &case::amend_stale_in()).1),
        ("pay.delta", typed_pay(&case::pay_log(), &case::pay_in())),
    ];
    let bless = std::env::var("BEBOP_WASM_BLESS").as_deref() == Ok("1");
    for (name, bytes) in case::inputs().into_iter().chain(answers) {
        if bless {
            std::fs::write(format!("{}{name}", case::DIR), &bytes).unwrap();
        }
        assert!(case::read(name) == bytes, "fixtures/decide/{name} is not what case.rs builds");
    }
}

/// THE AGREEMENT, natively: the export and the C surface answer the committed
/// bytes for the committed images and inputs.
#[test]
fn the_export_decides_the_committed_delta_byte_for_byte() {
    let (log, stock) = (case::read("amend.log"), case::read("amend.stock"));
    let got = amend_decide(&log, &stock, &case::read("amend.in.json")).expect("the amendment lands");
    assert_eq!(String::from_utf8_lossy(&got), String::from_utf8_lossy(&case::read("amend.delta")));
    let got = pay_decide(&case::read("pay.log"), &case::read("pay.room.json"), &case::read("pay.in.json")).expect("paid");
    assert_eq!(String::from_utf8_lossy(&got), String::from_utf8_lossy(&case::read("pay.delta")));
}

/// Through `extern "C"`, exactly as node calls it under wasm32.
fn call(f: unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8, usize, *mut usize) -> i32, a: &[u8], b: &[u8], c: &[u8]) -> (i32, Vec<u8>) {
    let mut out = [0usize; 2];
    let status = unsafe { f(a.as_ptr(), a.len(), b.as_ptr(), b.len(), c.as_ptr(), c.len(), out.as_mut_ptr()) };
    let bytes = unsafe { std::slice::from_raw_parts(out[0] as *const u8, out[1]) }.to_vec();
    unsafe { bebop_wasm::abi::bw_free(out[0] as *mut u8, out[1]) };
    (status, bytes)
}

#[test]
fn the_c_surface_answers_the_same_bytes_and_the_same_refusal() {
    let (log, stock) = (case::read("amend.log"), case::read("amend.stock"));
    assert_eq!(call(bw_amend, &log, &stock, &case::read("amend.in.json")), (0, case::read("amend.delta")));
    let (status, words) = call(bw_amend, &log, &stock, &case::read("amend_stale.in.json"));
    assert_eq!(status, decide_abi::CONFLICT, "a stale edit is a conflict");
    assert_eq!(format!("{status} {}", String::from_utf8_lossy(&words)).into_bytes(), case::read("amend_stale.refusal"));
    let (pay_log, room) = (case::read("pay.log"), case::read("pay.room.json"));
    assert_eq!(call(bw_pay, &pay_log, &room, &case::read("pay.in.json")), (0, case::read("pay.delta")));
}

/// THE REPLAY through the export: two offline tablets each sell the ONE cake
/// on the fixture shelf to their own round. Each alone lands; replayed in
/// arrival order against one shelf, the second is `OutOfStock` and the shelf
/// holds the cake once.
#[test]
fn two_offline_tablets_selling_the_last_cake_replay_to_one_sale() {
    let (log, stock) = (case::read("amend.log"), case::read("amend.stock"));
    let sale = |order: &str| {
        let mut v: serde_json::Value = serde_json::from_slice(&case::read("amend.in.json")).unwrap();
        v["order_id"] = order.into();
        v["ops"] = serde_json::json!([{"op": "add", "line": {"product_id": "cake", "quantity": 1, "unit_price": 450}}]);
        v.to_string().into_bytes()
    };
    let (a, b) = (sale("r1"), sale("r2"));
    assert!(amend_images(&log, &stock, &a).is_ok() && amend_images(&log, &stock, &b).is_ok(), "offline, each tablet sold it");

    let first = amend_images(&log, &stock, &a).expect("the first to arrive lands");
    let shelf1 = first.stock.clone().expect("the sale moved the shelf");
    let second = amend_images(&first.log, &shelf1, &b);
    let Err(Refusal::Refused(Refused::Stock(why))) = second else { panic!("the second must be OutOfStock, got {second:?}") };
    assert!(why.starts_with("cake:"), "the refusal names the supply: {why}");

    let s = StockLog::load(&shelf1).unwrap();
    let cakes = s.events().into_iter().filter(|e| matches!(e, StockEvent::Reserved { item, .. } if item == "cake")).count();
    assert_eq!(cakes, 1, "nothing is reserved twice");
    assert_eq!(s.ledger().unwrap().available("cake"), 0);
}

/// Damaged bytes are a refusal with a status, never a trap.
#[test]
fn a_damaged_image_or_input_is_refused_not_trapped() {
    let stock = case::read("amend.stock");
    let input = case::read("amend.in.json");
    assert_eq!(amend_decide(b"not an image", &stock, &input), Err(Refusal::LogImage));
    assert_eq!(amend_decide(&case::read("amend.log"), b"nope", &input), Err(Refusal::StockImage));
    assert!(matches!(amend_decide(&case::read("amend.log"), &stock, b"{"), Err(Refusal::Input(_))));
    assert_eq!(call(bw_amend, b"not an image", &stock, &input).0, decide_abi::LOG_IMAGE);
    // THE TWIN: no log yet is a fresh hub, and the order is then not found.
    assert_eq!(amend_decide(&[], &[], &input), Err(Refusal::Refused(Refused::NotFound)));
}
