//! SHEAF S2: THE KITCHEN TABLET'S WRITER, as a C surface (`--features tablet`).
//!
//! Every rule is `dowiz_hub::glue::tablet`'s -- the same crate the venue's object glues with --
//! so the tablet decides a movement exactly as the hub's write door does, and its tail is the
//! records it chained, byte for byte. This file only moves bytes and JSON across the boundary,
//! in `decide_abi.rs`'s shape: the host copies inputs in with `bw_alloc`, `out` points at two
//! `usize` cells that receive the answer's pointer and length, and the host frees it with
//! `bw_free(ptr, len)`. On a refusal the answer is the refusal's words.
//!
//!   bw_tab_record(image, ctx, record, now_ms, out) -> 0 | 1 (a sale past rights) ; answer = image
//!   bw_tab_tail(image, ctx, base_hex, out)         -> 0 ; answer = the request's JSON
//!                                                    `{"ctx","base","tip","tail":[..]}`
//!   bw_tab_rebase(image, base_hex, from_zero, sent, reply_json, out) -> 0 ; answer = image
//!   bw_tab_items(image, out)                       -> 0 ; answer = `[["item", on_hand], ...]`
//!
//! An empty image (`len == 0`) is a tablet that never synced: a fresh one.

use crate::abi::NULL_ARG;
use dowiz_hub::crypto::{hex, unhex};
use dowiz_hub::glue::merge::Entry;
use dowiz_hub::glue::tablet::{self, TabErr};
use serde_json::{json, Value};

/// Success; `PROVISIONAL` is success too: the sale was recorded past the tablet's rights.
pub const OK: i32 = 0;
pub const PROVISIONAL: i32 = 1;
/// The image is not a stock log (or lost records).
pub const IMAGE: i32 = 40;
/// The record is not a movement the codec reads.
pub const MOVEMENT: i32 = 41;
/// The shelf refused it, as the hub's write door would.
pub const STOCK: i32 = 42;
/// The base is not in the tablet's chain.
pub const BASE: i32 = 43;
/// The reply does not replay onto the hub's tip.
pub const REPLY: i32 = 44;
/// An argument that is not what it should be (a hex id, the reply's JSON).
pub const BAD_INPUT: i32 = 45;

fn status_of(e: &TabErr) -> (i32, String) {
    match e {
        TabErr::Image => (IMAGE, "the tablet's image is not a stock log".into()),
        TabErr::Movement => (MOVEMENT, "not a stock movement".into()),
        TabErr::Stock(s) => (STOCK, s.to_string()),
        TabErr::Base => (BASE, "the base is not in the tablet's chain".into()),
        TabErr::Reply(w) => (REPLY, (*w).to_string()),
    }
}

fn id_of(s: &str) -> Result<[u8; 32], (i32, String)> {
    unhex(s.trim()).and_then(|v| v.try_into().ok()).ok_or((BAD_INPUT, "a record id is 64 hex characters".into()))
}

/// RECORD: the new image, and whether the movement was a sale past the tablet's rights.
pub fn record(image: &[u8], ctx: &str, rec: &str, now_ms: i64) -> Result<(Vec<u8>, bool), (i32, String)> {
    tablet::record(image, ctx, rec, now_ms).map_err(|e| status_of(&e))
}

/// TAIL: the request body the tablet sends (`services/operations/stock/glue.rs`), without the
/// venue, which the host adds.
pub fn tail_json(image: &[u8], ctx: &str, base_hex: &str) -> Result<String, (i32, String)> {
    let base = id_of(base_hex)?;
    let t = tablet::tail(image, ctx, &base).map_err(|e| status_of(&e))?;
    Ok(json!({"ctx": t.ctx, "base": hex(&t.base), "tip": hex(&t.tip), "tail": t.recs}).to_string())
}

/// REBASE: the hub's reply (`{"tip", "tail": [record | index]}`) replayed onto the image.
pub fn rebase_json(image: &[u8], base_hex: &str, from_zero: bool, sent: usize, reply: &str) -> Result<Vec<u8>, (i32, String)> {
    let base = id_of(base_hex)?;
    let v: Value = serde_json::from_str(reply).map_err(|e| (BAD_INPUT, e.to_string()))?;
    let tip = id_of(v.get("tip").and_then(Value::as_str).unwrap_or(""))?;
    let mut entries = Vec::new();
    for e in v.get("tail").and_then(Value::as_array).ok_or((BAD_INPUT, "the reply has no tail".to_string()))? {
        entries.push(match (e.as_u64(), e.as_str()) {
            (Some(i), _) => Entry::Echo(i as usize),
            (_, Some(s)) => Entry::Rec(s.to_string()),
            _ => return Err((BAD_INPUT, "a tail entry is a record or an index".into())),
        });
    }
    tablet::rebase(image, &base, from_zero, sent, &entries, &tip).map_err(|e| status_of(&e))
}

/// ITEMS: the tablet's shelf as JSON, for the offline stock sheet.
pub fn items_json(image: &[u8]) -> Result<String, (i32, String)> {
    let v = tablet::items(image).map_err(|e| status_of(&e))?;
    Ok(json!(v).to_string())
}

/// `ptr..ptr+len` as a slice; an empty one may be null.
unsafe fn bytes<'a>(ptr: *const u8, len: usize) -> Option<&'a [u8]> {
    if len == 0 {
        return Some(&[]);
    }
    (!ptr.is_null()).then(|| core::slice::from_raw_parts(ptr, len))
}

unsafe fn text<'a>(ptr: *const u8, len: usize) -> Option<&'a str> {
    core::str::from_utf8(bytes(ptr, len)?).ok()
}

/// Hand `answer` to the host through `out`, and return `status`.
unsafe fn give(out: *mut usize, r: Result<(Vec<u8>, i32), (i32, String)>) -> i32 {
    let (answer, status) = match r {
        Ok(v) => v,
        Err((code, words)) => (words.into_bytes(), code),
    };
    let len = answer.len();
    let ptr = if len == 0 { core::ptr::null_mut() } else { Box::into_raw(answer.into_boxed_slice()) as *mut u8 };
    *out = ptr as usize;
    *out.add(1) = len;
    status
}

/// # Safety
/// Each non-empty `ptr..ptr+len` is readable; `out` points at two writable `usize` cells.
#[no_mangle]
pub unsafe extern "C" fn bw_tab_record(img: *const u8, img_len: usize, ctx: *const u8, ctx_len: usize, rec: *const u8, rec_len: usize, now_ms: i64, out: *mut usize) -> i32 {
    let (Some(i), Some(c), Some(r)) = (bytes(img, img_len), text(ctx, ctx_len), text(rec, rec_len)) else { return NULL_ARG };
    if out.is_null() {
        return NULL_ARG;
    }
    give(out, record(i, c, r, now_ms).map(|(img, prov)| (img, if prov { PROVISIONAL } else { OK })))
}

/// # Safety
/// As `bw_tab_record`.
#[no_mangle]
pub unsafe extern "C" fn bw_tab_tail(img: *const u8, img_len: usize, ctx: *const u8, ctx_len: usize, base: *const u8, base_len: usize, out: *mut usize) -> i32 {
    let (Some(i), Some(c), Some(b)) = (bytes(img, img_len), text(ctx, ctx_len), text(base, base_len)) else { return NULL_ARG };
    if out.is_null() {
        return NULL_ARG;
    }
    give(out, tail_json(i, c, b).map(|s| (s.into_bytes(), OK)))
}

/// # Safety
/// As `bw_tab_record`.
#[no_mangle]
pub unsafe extern "C" fn bw_tab_rebase(img: *const u8, img_len: usize, base: *const u8, base_len: usize, from_zero: i32, sent: usize, reply: *const u8, reply_len: usize, out: *mut usize) -> i32 {
    let (Some(i), Some(b), Some(r)) = (bytes(img, img_len), text(base, base_len), text(reply, reply_len)) else { return NULL_ARG };
    if out.is_null() {
        return NULL_ARG;
    }
    give(out, rebase_json(i, b, from_zero != 0, sent, r).map(|img| (img, OK)))
}

/// # Safety
/// As `bw_tab_record`.
#[no_mangle]
pub unsafe extern "C" fn bw_tab_items(img: *const u8, img_len: usize, out: *mut usize) -> i32 {
    let Some(i) = bytes(img, img_len) else { return NULL_ARG };
    if out.is_null() {
        return NULL_ARG;
    }
    give(out, items_json(i).map(|s| (s.into_bytes(), OK)))
}
