//! SHEAF S2 through the C surface's own functions (`--features tablet`): a tablet that never
//! synced bootstraps from the hub, sells offline, sends its tail, and replays the hub's answer
//! onto the hub's tip -- with the hub played by `dowiz_hub::glue::merge`, the code the venue's
//! object runs. Then the same through the `extern "C"` exports, as a host calls them.
#![cfg(feature = "tablet")]

use bebop_wasm::tablet::{self, bw_tab_record, bw_tab_tail, BAD_INPUT, PROVISIONAL, STOCK};
use dowiz_hub::crypto::hex;
use dowiz_hub::glue::merge::{merge, Entry, Reply};
use dowiz_hub::glue::wire::Tail;
use dowiz_hub::stock::{StockEvent, StockLog};
use serde_json::{json, Value};

const ZERO: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// The hub's answer as the Worker serialises it (`hubdo/glue_turn.rs` `reply_json`).
fn wire(r: &Reply) -> String {
    let tail: Vec<Value> = r.entries.iter().map(|e| match e { Entry::Rec(s) => json!(s), Entry::Echo(i) => json!(i) }).collect();
    json!({"tip": hex(&r.tip), "tail": tail}).to_string()
}

/// The request as the tablet sends it, glued by the hub.
fn hub_glue(hub: &mut StockLog, body: &str) -> Reply {
    let v: Value = serde_json::from_str(body).unwrap();
    let id = |k: &str| -> [u8; 32] { dowiz_hub::crypto::unhex(v[k].as_str().unwrap()).unwrap().try_into().unwrap() };
    let recs = v["tail"].as_array().unwrap().iter().map(|r| r.as_str().unwrap().to_string()).collect();
    let t = Tail { ctx: v["ctx"].as_str().unwrap().into(), base: id("base"), tip: id("tip"), recs };
    let bytes = hub.to_bytes_trimmed();
    merge(&bytes, hub, &t).unwrap()
}

#[test]
fn a_tablet_bootstraps_sells_offline_and_lands_on_the_hubs_tip() {
    let mut hub = StockLog::create_sized(64 * 1024).unwrap();
    hub.append_all(&[StockEvent::Received { item: "salmon".into(), qty: 8 }, StockEvent::Received { item: "rice".into(), qty: 900 }]).unwrap();
    let r0 = hub_glue(&mut hub, &tablet::tail_json(&[], "kitchen", ZERO).unwrap());
    let img = tablet::rebase_json(&[], ZERO, false, 0, &wire(&r0)).unwrap();
    let base = hex(&r0.tip);
    // 8 salmon: the kitchen holds 6. It sells 6, then 1 more -- recorded, flagged.
    let (img, p1) = tablet::record(&img, "kitchen", r#"{"k":"served","item":"salmon","qty":6,"order":"k-1"}"#, 1).unwrap();
    let (img, p2) = tablet::record(&img, "kitchen", r#"{"k":"served","item":"salmon","qty":1,"order":"k-2"}"#, 2).unwrap();
    assert_eq!((p1, p2), (false, true));
    let body = tablet::tail_json(&img, "kitchen", &base).unwrap();
    let r1 = hub_glue(&mut hub, &body);
    assert_eq!((r1.merged, r1.held.len()), (2, 0));
    let after = tablet::rebase_json(&img, &base, false, 2, &wire(&r1)).unwrap();
    assert_eq!(tablet::items_json(&after).unwrap(), r#"[["rice",900],["salmon",1]]"#, "the shelf the sheet steps through");
    assert_eq!(StockLog::load(&after).unwrap().tip_id(), hub.tip_id());
    assert_eq!(StockLog::load(&after).unwrap().ledger().unwrap(), hub.ledger().unwrap());
}

#[test]
fn refusals_come_back_as_codes_with_words() {
    assert_eq!(tablet::tail_json(&[], "kitchen", "zz").unwrap_err().0, BAD_INPUT);
    assert_eq!(tablet::record(&[], "kitchen", r#"{"k":"consumed","item":"x","qty":1,"order":"none"}"#, 0).unwrap_err().0, STOCK);
    assert_eq!(tablet::rebase_json(&[], ZERO, false, 0, "{}").unwrap_err().0, BAD_INPUT);
}

#[test]
fn the_exports_answer_through_out_as_a_host_reads_them() {
    let mut hub = StockLog::create_sized(64 * 1024).unwrap();
    hub.append_all(&[StockEvent::Received { item: "salmon".into(), qty: 4 }]).unwrap();
    let r0 = hub_glue(&mut hub, &tablet::tail_json(&[], "kitchen", ZERO).unwrap());
    let img = tablet::rebase_json(&[], ZERO, false, 0, &wire(&r0)).unwrap();
    let (ctx, rec) = ("kitchen", r#"{"k":"served","item":"salmon","qty":4,"order":"k-1"}"#);
    let mut out = [0usize; 2];
    let st = unsafe { bw_tab_record(img.as_ptr(), img.len(), ctx.as_ptr(), ctx.len(), rec.as_ptr(), rec.len(), 5, out.as_mut_ptr()) };
    assert_eq!(st, PROVISIONAL, "4 on the shelf, 3 granted: the sale is past the rights");
    let img2 = unsafe { Box::from_raw(core::slice::from_raw_parts_mut(out[0] as *mut u8, out[1])) }.to_vec();
    let base = hex(&r0.tip);
    let st = unsafe { bw_tab_tail(img2.as_ptr(), img2.len(), ctx.as_ptr(), ctx.len(), base.as_ptr(), base.len(), out.as_mut_ptr()) };
    assert_eq!(st, 0);
    let body = unsafe { Box::from_raw(core::slice::from_raw_parts_mut(out[0] as *mut u8, out[1])) }.to_vec();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["tail"].as_array().unwrap().len(), 1);
    assert!(v["tail"][0].as_str().unwrap().contains("\"prov\":1"));
}
