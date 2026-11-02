//! The deciders' C surface (`--features decide`), in `abi.rs`'s shape: the
//! host copies bytes in through `bw_alloc`, calls, and reads back.
//!
//! THE ANSWER IS ONE BUFFER THIS SIDE ALLOCATED. `out` points at two `usize`
//! cells (two `u32` under wasm32); on return they hold the pointer and length
//! of the answer, which the host reads and gives back with `bw_free(ptr, len)`.
//! On `OK` the answer is the DELTA the decider appended -- the bytes the object
//! writes and broadcasts. On a refusal it is the refusal's words, and the
//! status says which refusal: a host prints both and never has to parse.
//!
//! An empty image (`len == 0`, pointer may be null) is "none yet": a fresh one
//! is created, as the object does (`decide.rs`, step 1 and 3).

use crate::abi::NULL_ARG;
use crate::decide::{amend_decide, pay_decide, Refusal};
use dowiz_hub::room::Refused;

/// Success: the answer is the delta.
pub const OK: i32 = 0;
/// The log image is not a hub image.
pub const LOG_IMAGE: i32 = 20;
/// The stock image is not a stock log.
pub const STOCK_IMAGE: i32 = 21;
/// The input or the room is not the JSON the command reads.
pub const BAD_INPUT: i32 = 22;
/// The decider refused: 30 + the kind, in `Refused`'s declaration order.
pub const STOCK: i32 = 30;
pub const PROMO: i32 = 31;
pub const APPEND: i32 = 32;
pub const NOT_FOUND: i32 = 33;
pub const CONFLICT: i32 = 34;
pub const INVALID: i32 = 35;
pub const UNTAXED: i32 = 36;

/// The status and the words of a refusal. Total over `Refusal`.
pub fn status_of(r: &Refusal) -> (i32, String) {
    match r {
        Refusal::LogImage => (LOG_IMAGE, "the log image is not a hub image".into()),
        Refusal::StockImage => (STOCK_IMAGE, "the stock image is not a stock log".into()),
        Refusal::Input(m) => (BAD_INPUT, m.clone()),
        Refusal::Refused(r) => {
            let code = match r {
                Refused::Stock(_) => STOCK,
                Refused::Promo(_) => PROMO,
                Refused::Append(_) => APPEND,
                Refused::NotFound => NOT_FOUND,
                Refused::Conflict(_) => CONFLICT,
                Refused::Invalid(_) => INVALID,
                Refused::Untaxed(_) => UNTAXED,
            };
            (code, r.message().to_string())
        }
    }
}

/// `ptr..ptr+len` as a slice; an empty one may be null.
unsafe fn bytes<'a>(ptr: *const u8, len: usize) -> Option<&'a [u8]> {
    if len == 0 {
        return Some(&[]);
    }
    (!ptr.is_null()).then(|| core::slice::from_raw_parts(ptr, len))
}

/// Hand `answer` to the host through `out`, and return `status`.
unsafe fn give(out: *mut usize, answer: Vec<u8>, status: i32) -> i32 {
    // A boxed slice's capacity IS its length, which is what `bw_free` rebuilds.
    let len = answer.len();
    let ptr = if len == 0 { core::ptr::null_mut() } else { Box::into_raw(answer.into_boxed_slice()) as *mut u8 };
    *out = ptr as usize;
    *out.add(1) = len;
    status
}

fn answer(r: Result<Vec<u8>, Refusal>) -> (Vec<u8>, i32) {
    match r {
        Ok(delta) => (delta, OK),
        Err(r) => {
            let (code, words) = status_of(&r);
            (words.into_bytes(), code)
        }
    }
}

/// AMEND A ROUND. `log` and `stock` are the images (either may be empty),
/// `input` is an `AmendIn` as JSON.
///
/// # Safety
/// Each non-empty `ptr..ptr+len` is readable; `out` points at two writable
/// `usize` cells.
#[no_mangle]
pub unsafe extern "C" fn bw_amend(
    log: *const u8,
    log_len: usize,
    stock: *const u8,
    stock_len: usize,
    input: *const u8,
    input_len: usize,
    out: *mut usize,
) -> i32 {
    let (Some(l), Some(s), Some(i)) = (bytes(log, log_len), bytes(stock, stock_len), bytes(input, input_len)) else {
        return NULL_ARG;
    };
    if out.is_null() || i.is_empty() {
        return NULL_ARG;
    }
    let (a, status) = answer(amend_decide(l, s, i));
    give(out, a, status)
}

/// TAKE A PAYMENT. `room` is `{"open_till": .., "venue_currency": ..}`,
/// `input` is a `PayIn` as JSON.
///
/// # Safety
/// As `bw_amend`.
#[no_mangle]
pub unsafe extern "C" fn bw_pay(
    log: *const u8,
    log_len: usize,
    room: *const u8,
    room_len: usize,
    input: *const u8,
    input_len: usize,
    out: *mut usize,
) -> i32 {
    let (Some(l), Some(r), Some(i)) = (bytes(log, log_len), bytes(room, room_len), bytes(input, input_len)) else {
        return NULL_ARG;
    };
    if out.is_null() || i.is_empty() || r.is_empty() {
        return NULL_ARG;
    }
    let (a, status) = answer(pay_decide(l, r, i));
    give(out, a, status)
}
