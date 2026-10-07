//! A write cut in the middle leaves the PREVIOUS generation (W-ATOMIC). Every cut is one named
//! `puts_left` budget on the in-memory store, deterministic; the store applies one call, or one
//! group of calls issued together, whole or not at all, as the platform does.

use super::super::host::mem::{Harness, T0};
use super::super::host::MAX_KEYS;
use super::super::CHUNK;
use super::{plan, Plan};
use crate::wire::{Call, Reply};
use serde_json::json;
use worker::{Method, Result};

fn placed(id: &str) -> serde_json::Value {
    json!({
        "kind": dowiz_hub::EventKind::Placed as u8,
        "order_id": id,
        "payload": json!({ "id": id, "status": "PENDING", "total": 1200 }).to_string(),
        "clock": T0 as u64,
    })
}

fn append(h: &Harness, generation: i64, id: &str) -> Result<Reply> {
    h.try_call(
        Call::new("https://hub/fold/append", Method::Post)
            .unwrap()
            .with_header("x-generation", &generation.to_string())
            .with_json(&placed(id)),
    )
}

fn put(h: &Harness, id: &str, generation: i64, bytes: &[u8]) -> Result<Reply> {
    h.try_call(
        Call::new(&format!("https://hub/img/{id}"), Method::Put)
            .unwrap()
            .with_header("x-generation", &generation.to_string())
            .with_body(bytes.to_vec()),
    )
}

/// What a NEW object reads from storage: (generation, bytes), or the refusal.
fn cold(h: &Harness, id: &str) -> Result<(i64, Vec<u8>)> {
    let r = h.cold().try_call(Call::new(&format!("https://hub/img/{id}"), Method::Get).unwrap())?;
    Ok((Harness::gen_of(&r), r.body().to_vec()))
}

fn pattern(chunks: usize, salt: u8) -> Vec<u8> {
    (0..chunks * CHUNK).map(|i| ((i % 251) as u8) ^ salt).collect()
}

/// THE LOG APPEND. The append rewrites the superblock (chunk 0) and the tail; a cut after the
/// first key used to land chunk 0 alone under the old meta.
#[test]
fn a_log_append_cut_after_its_first_key_leaves_the_previous_generation() {
    let h = Harness::new();
    assert_eq!(append(&h, 0, "o1").unwrap().status_code(), 200);
    let before = cold(&h, "log").expect("generation 1 reads");
    assert_eq!(before.0, 1);

    h.host.puts_left.set(Some(1));
    assert!(append(&h, 1, "o2").is_err(), "a cut write is an error, not a 200");
    h.host.puts_left.set(None);

    let after = cold(&h, "log").expect("the previous generation is still readable, not refused");
    assert_eq!(after, before, "byte for byte the previous generation");
    let orders = h.cold().get("/fold/orders").body_value();
    assert_eq!(orders.as_array().map(Vec::len), Some(1), "o1 only: {orders}");

    // Twin: the same append, uncut, lands at generation 2.
    assert_eq!(append(&h, 1, "o2").unwrap().status_code(), 200);
    assert_eq!(cold(&h, "log").unwrap().0, 2);
    assert_eq!(h.cold().get("/fold/orders").body_value().as_array().map(Vec::len), Some(2));
}

/// THE CATALOGUE SAVE. Chunk 0 rewritten in place, the image grown into a new chunk: a cut after
/// chunk 0 used to read back as generation 1 assembled from two generations, with no error at all.
#[test]
fn a_catalogue_save_cut_after_chunk_zero_leaves_the_previous_generation() {
    let h = Harness::new();
    let old = pattern(2, 0);
    assert_eq!(put(&h, "catalog", 0, &old).unwrap().status_code(), 200);
    let mut new = old.clone();
    new[5] ^= 0xff;
    new.extend_from_slice(&[7u8; 100]);

    h.host.puts_left.set(Some(1));
    assert!(put(&h, "catalog", 1, &new).is_err());
    h.host.puts_left.set(None);
    assert_eq!(cold(&h, "catalog").expect("readable"), (1, old.clone()), "the previous generation, not a mix");

    // Twin: the retry lands whole.
    assert_eq!(put(&h, "catalog", 1, &new).unwrap().status_code(), 200);
    assert_eq!(cold(&h, "catalog").unwrap(), (2, new));
}

/// THE CEILING: a compacted image's full rewrite is one call.
#[test]
fn a_full_rewrite_of_a_ceiling_image_is_one_call() {
    let at_ceiling = dowiz_hub::CEILING_BYTES.div_ceil(CHUNK);
    assert_eq!(at_ceiling, 107, "10 MiB / 96 KiB, rounded up");
    let all: Vec<usize> = (0..at_ceiling).collect();
    assert_eq!(plan(&all, at_ceiling), Plan { ahead: Vec::new(), with_meta: all.clone() });
    assert!(at_ceiling < MAX_KEYS, "chunks + the meta fit one put(entries)");
}

/// PAST 127: what goes ahead of the meta is only what the OLD meta does not name.
#[test]
fn past_127_chunks_only_chunks_the_old_meta_does_not_name_go_ahead() {
    let changed: Vec<usize> = (0..300).collect();
    let p = plan(&changed, 100);
    assert!(p.ahead.iter().flatten().all(|&n| n >= 100), "{:?}", p.ahead);
    assert!(p.ahead.iter().all(|b| b.len() <= MAX_KEYS));
    assert!((0..100).all(|n| p.with_meta.contains(&n)), "every in-place chunk lands with the meta");
    assert_eq!(p.with_meta.len(), MAX_KEYS - 1, "one call: 127 chunks + the meta");
    let mut every: Vec<usize> = p.ahead.iter().flatten().copied().chain(p.with_meta.iter().copied()).collect();
    every.sort_unstable();
    assert_eq!(every, changed, "nothing lost, nothing twice");

    // More than 127 in place: all of them stay with the meta (issued together).
    let p = plan(&changed, 250);
    assert_eq!(p.with_meta, (0..250).collect::<Vec<_>>());
    assert_eq!(p.ahead, vec![(250..300).collect::<Vec<_>>()]);
}

/// THROUGH THE OBJECT, past 127: a cut AFTER the ahead calls landed and before the meta's call
/// leaves the previous generation; a rewrite of 200 chunks in place lands, split under 128 keys
/// per call, and a cut in it lands nothing.
#[test]
fn a_200_chunk_log_writes_split_and_a_cut_anywhere_leaves_the_previous_generation() {
    let h = Harness::new();
    let small = pattern(2, 0);
    assert_eq!(put(&h, "log9", 0, &small).unwrap().status_code(), 200);

    // Grow 2 -> 200 chunks, all 200 changed: chunks 0-1 and 125 fresh ride with the meta, 73 fresh go ahead.
    let big = pattern(200, 0x55);
    h.host.puts_left.set(Some(80)); // the ahead call (73) lands, the meta's call (127 + 1) does not
    assert!(put(&h, "log9", 1, &big).is_err());
    h.host.puts_left.set(None);
    assert!(h.host.keys("c:log9:").len() > 2, "the ahead call did land");
    assert_eq!(cold(&h, "log9").expect("readable"), (1, small.clone()), "the old meta never saw them");

    assert_eq!(put(&h, "log9", 1, &big).unwrap().status_code(), 200, "split under 128 keys per call");
    assert_eq!(cold(&h, "log9").unwrap(), (2, big.clone()));

    // All 200 chunks in place: 200 + the meta, in two calls issued together. A cut lands nothing.
    let again = pattern(200, 0xaa);
    h.host.puts_left.set(Some(150));
    assert!(put(&h, "log9", 2, &again).is_err());
    h.host.puts_left.set(None);
    assert_eq!(cold(&h, "log9").unwrap(), (2, big));
    assert_eq!(put(&h, "log9", 2, &again).unwrap().status_code(), 200);
    assert_eq!(cold(&h, "log9").unwrap(), (3, again));
}
