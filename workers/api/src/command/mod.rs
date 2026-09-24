//! COMMANDS THE OBJECT EXECUTES, instead of decisions the Worker makes at a
//! distance.
//!
//! THE SHAPE THIS REPLACES, and it was the same shape three times. A handler
//! read an image across the hop, asked the kernel, wrote it back under a
//! generation guard, retried five times when the guard fired — and when the
//! change needed a SECOND image, it wrote that one too and hoped. What "hoped"
//! meant in each case:
//!
//! * `storefront::place` reserved the ingredients, appended the order, and on
//!   failure wrote the stock image a third time to release them. If THAT
//!   failed, a `console_error!`. A stranded reservation makes a kitchen believe
//!   it is out of something it has.
//! * `owner::order_action` advanced the order and then settled the shelf, with
//!   the settlement's failure logged and deliberately not failing the
//!   transition — correct, given the order it was in, and an order and a ledger
//!   that disagree either way.
//! * `owner::assign_courier` wrote the assignment and then appended the event,
//!   with NO compensation at all: an assignment could stand while the order
//!   never said it had a courier.
//!
//! WHAT A COMMAND DOES DIFFERENTLY. The object holds every image in `mem`. It
//! can therefore do all the fallible work against copies IN MEMORY and write
//! only once everything has succeeded, so there is no state to undo. Each
//! `decide` below is that work: a pure function over `dowiz_hub` types — bytes
//! in, bytes out, no clock, no I/O — which is why they are the first lines of
//! this path ever to be exercised by `cargo test` rather than by a deployment.
//!
//! WHAT STAYS IN THE WORKER, on purpose: pricing (pure, tested, and a second
//! pricer is exactly what `a18886d4` was), authentication, and any lookup that
//! belongs to a DIFFERENT object — the courier roster lives in the platform
//! object, so `assign_courier` still resolves it before it sends the command.

use serde::de::DeserializeOwned;
use serde::Serialize;
use worker::wasm_bindgen::JsValue;
use worker::*;

pub mod advance;
pub mod aggregator;
pub mod amend;
pub mod assign;
pub mod floor;
pub mod kitchen_ack;
pub mod pay;
pub mod place;
pub mod refund;
pub mod room_rules;
pub mod sitting;
pub mod till;
pub mod tips;
pub mod transfer;

#[cfg(test)]
mod tests;

/// Why a command was refused, and with what status: `dowiz_hub::room::Refused`.
/// MOVED to the hub (D7 phase 1) so the room's deciders, which moved there
/// too, refuse in the one type every command here already speaks.
pub use dowiz_hub::room::Refused;

/// Send a command to the venue's object and read its answer.
///
/// NO GENERATION HEADER, AND THAT IS THE POINT. Every other write on this path
/// carries the generation it read, because the read and the write are separated
/// by a network hop and something could land between them. A command has
/// nothing to guard: the read and the write are two statements inside one
/// object turn.
///
/// THE OBJECT'S OWN WORDS COME BACK. A refusal arrives as its status and its
/// text, so the handler hands the customer "salmon: 80 wanted, 0 available"
/// rather than "something went wrong".
pub(crate) async fn send<I: Serialize, O: DeserializeOwned>(
    place: &crate::hubstore::Place,
    what: &str,
    input: &I,
) -> std::result::Result<O, (u16, String)> {
    let stub = place.stub().map_err(|e| (503, format!("hub unavailable: {e}")))?;
    let body = serde_json::to_string(input).map_err(|e| (500, format!("{what}: {e}")))?;
    let mut req = Request::new_with_init(
        &format!("https://hub/fold/{what}"),
        RequestInit::new().with_method(Method::Post).with_body(Some(JsValue::from_str(&body))),
    )
    .map_err(|e| (500, format!("{what}: {e}")))?;
    req.headers_mut()
        .and_then(|h| h.set("content-type", "application/json"))
        .map_err(|e| (500, format!("{what}: {e}")))?;
    let mut res =
        stub.fetch_with_request(req).await.map_err(|e| (503, format!("hub unavailable: {e}")))?;
    let status = res.status_code();
    if status == 200 {
        return res.json().await.map_err(|e| (500, format!("{what}: unreadable answer: {e}")));
    }
    Err((status, res.text().await.unwrap_or_default()))
}
