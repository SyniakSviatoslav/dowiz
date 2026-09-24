//! THE ROOM'S DECIDERS OVER BYTES (D7 phase 1; `--features decide`).
//!
//! A tablet that is offline holds the venue's images and must take the same
//! decision the venue's object takes when it is online. "The same" is not a
//! second implementation that agrees in tests: it is `dowiz_hub::room`, the
//! code the Worker re-exports, called here over the images as bytes. What this
//! file adds is only what the object's handler (`hubdo/room.rs`) does around
//! the decider, and it does it in the handler's order:
//!
//!   1. the log image, loaded -- or a fresh 64 KiB hub when there is none,
//!      as `log_hub` creates one;
//!   2. the order the input names, folded from that log (`room::view::current`,
//!      the one-order form of the object's `orders_view`);
//!   3. for amend, the stock image, loaded -- or a fresh 64 KiB shelf, as
//!      `stock_log` does;
//!   4. the decider; and on `Ok` the delta it appended, plus both images
//!      trimmed as `write_both` writes them (the shelf only if it moved).
//!
//! WHAT THE CALLER SUPPLIES THAT THE OBJECT DERIVES. `pay` needs the till that
//! is open and the venue's currency (`room::pay::Room`). The object reads both
//! from its own images; here they arrive as `{"open_till": .., "venue_currency":
//! ..}`, because a tablet holds the same two facts in the state it synced.
//! A wallet payment's debit is NOT decided here (`room`'s header): the wallet
//! ledger is the kernel's, and the object decides that leg on replay.

use dowiz_hub::room::{amend, pay, view, Refused};
use dowiz_hub::stock::StockLog;
use dowiz_hub::Hub;
use serde_json::Value;

/// The size a missing image is created at: the object's own (`log_hub`,
/// `stock_log` in `hubdo/room.rs`).
pub const FRESH_IMAGE_BYTES: usize = 64 * 1024;

/// Why no decision came back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The log image is not a hub image.
    LogImage,
    /// The stock image is not a stock log.
    StockImage,
    /// The input (or the room) is not the JSON the command reads.
    Input(String),
    /// The decider's own refusal, exactly as the object would answer it.
    Refused(Refused),
}

/// A decision: the delta the decider appended, and the images after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decided {
    /// The `Amended` / `Paid` payload, byte for byte what the object appends
    /// and broadcasts.
    pub delta: Vec<u8>,
    /// The log image after the append, trimmed.
    pub log: Vec<u8>,
    /// The stock image after the decision, trimmed; `None` when it did not move.
    pub stock: Option<Vec<u8>>,
}

fn load_log(bytes: &[u8]) -> Result<Hub, Refusal> {
    if bytes.is_empty() {
        return Hub::create_sized(FRESH_IMAGE_BYTES).map_err(|_| Refusal::LogImage);
    }
    Hub::load(bytes).map_err(|_| Refusal::LogImage)
}

fn load_stock(bytes: &[u8]) -> Result<StockLog, Refusal> {
    if bytes.is_empty() {
        return StockLog::create_sized(FRESH_IMAGE_BYTES).map_err(|_| Refusal::StockImage);
    }
    StockLog::load(bytes).map_err(|_| Refusal::StockImage)
}

fn unreadable(e: serde_json::Error) -> Refusal {
    Refusal::Input(e.to_string())
}

/// AMEND A ROUND over images. `stock` may be empty (no shelf yet).
pub fn amend_images(log: &[u8], stock: &[u8], input_json: &[u8]) -> Result<Decided, Refusal> {
    let input: amend::AmendIn = serde_json::from_slice(input_json).map_err(unreadable)?;
    let mut hub = load_log(log)?;
    let mut shelf = load_stock(stock)?;
    let before = shelf.len();
    let current = view::current(&hub, &input.order_id);
    let (_, body, _) = amend::decide(&mut hub, &mut shelf, current.as_ref(), &input).map_err(Refusal::Refused)?;
    let moved = shelf.len() != before;
    Ok(Decided {
        delta: body.into_bytes(),
        log: hub.to_bytes_trimmed(),
        stock: moved.then(|| shelf.to_bytes_trimmed()),
    })
}

/// The delta only: what the card calls `amend_decide`.
pub fn amend_decide(log: &[u8], stock: &[u8], input_json: &[u8]) -> Result<Vec<u8>, Refusal> {
    amend_images(log, stock, input_json).map(|d| d.delta)
}

/// TAKE A PAYMENT over the log image. `room_json` is
/// `{"open_till": "main" | null, "venue_currency": "ALL"}`.
pub fn pay_images(log: &[u8], room_json: &[u8], input_json: &[u8]) -> Result<Decided, Refusal> {
    let input: pay::PayIn = serde_json::from_slice(input_json).map_err(unreadable)?;
    let room: Value = serde_json::from_slice(room_json).map_err(unreadable)?;
    let venue_currency = room
        .get("venue_currency")
        .and_then(Value::as_str)
        .ok_or_else(|| Refusal::Input("the room names the venue's currency".into()))?;
    let room = pay::Room { open_till: room.get("open_till").and_then(Value::as_str), venue_currency };
    let mut hub = load_log(log)?;
    let current = view::current(&hub, &input.order_id);
    let (_, body, _) = pay::decide(&mut hub, current.as_ref(), &input, &room).map_err(Refusal::Refused)?;
    Ok(Decided { delta: body.into_bytes(), log: hub.to_bytes_trimmed(), stock: None })
}

/// The delta only: what the card calls `pay_decide`.
pub fn pay_decide(log: &[u8], room_json: &[u8], input_json: &[u8]) -> Result<Vec<u8>, Refusal> {
    pay_images(log, room_json, input_json).map(|d| d.delta)
}
