//! The edit journal (W-PITR): replay to time T rebuilds the catalogue of time T, byte-equal in
//! its compacted form, over three edit sequences (prices; remove + re-add; every key family);
//! a skipped record, an edited record and a changed byte each STOP the replay by name.
use super::*;

/// A catalogue written the way the Worker writes it: load the stored bytes, edit, `to_bytes`
/// (the W-DELTA append path), store (a new generation), journal `before -> after`.
fn write(bytes: &[u8], log: &mut LogImage, at: i64, by: &str, f: impl FnOnce(&mut Catalog)) -> Vec<u8> {
    let mut c = Catalog::load(bytes).unwrap();
    let before = state_of(&c);
    f(&mut c);
    let after = state_of(&c);
    let out = c.to_bytes().unwrap();
    // The generation the journal's newest record names is the one this write read at.
    let read_at = last_gen(log).unwrap_or(0);
    journal(log, &before, &after, at, by, (read_at, read_at + 1)).unwrap();
    out
}

fn dish(id: &str, price: i64) -> String {
    format!(r#"{{"id":"{id}","name":"Dish {id}","price":{price},"available":true}}"#)
}

fn born() -> Vec<u8> {
    let mut c = Catalog::create().unwrap();
    c.set_location(r#"{"name":"Edits","currency":"ALL","menu_version":1}"#);
    c.set_category("rolls", r#"{"id":"rolls","name":"Rolls"}"#);
    for i in 0..6 {
        c.set_product(&format!("d{i}"), &dish(&format!("d{i}"), 500 + i * 100));
    }
    c.to_bytes().unwrap()
}

/// The compacted bytes of a stored image: the canonical form the replay is compared in.
fn canonical(bytes: &[u8]) -> Vec<u8> {
    Catalog::load(bytes).unwrap().compact().unwrap()
}

/// Run `steps` (one write each, at t = 1000, 2000, ...) and check replay-to-T at EVERY T.
fn check_sequence(name: &str, steps: Vec<Box<dyn FnOnce(&mut Catalog)>>) -> LogImage {
    let mut log = LogImage::create().unwrap();
    let mut bytes = born();
    let mut snaps = vec![];
    for (i, f) in steps.into_iter().enumerate() {
        let at = 1000 * (i as i64 + 1);
        bytes = write(&bytes, &mut log, at, "staff-1", f);
        snaps.push((at, canonical(&bytes), Catalog::load(&bytes).unwrap().root()));
    }
    for (at, want, root) in &snaps {
        let got = rebuild(&replay(&log, Cut::Time(*at)).unwrap()).unwrap();
        assert_eq!(&got.root(), root, "{name}: the fold at t={at} differs");
        let mut got = got;
        assert_eq!(&got.compact().unwrap(), want, "{name}: replay to t={at} is not byte-equal to the snapshot");
    }
    assert!(log.quarantined().is_empty(), "{name}: the decoder must read every record it wrote");
    log
}

#[test]
fn replay_to_t_is_byte_equal_to_the_snapshot_over_three_edit_sequences() {
    // 1. Prices and availability, the edits an owner makes daily.
    check_sequence("prices", vec![
        Box::new(|c| c.set_product("d1", &dish("d1", 777))),
        Box::new(|c| c.set_product("d2", &dish("d2", 650).replace("true", "false"))),
        Box::new(|c| { c.set_product("d1", &dish("d1", 800)); c.set_product("d3", &dish("d3", 900)); }),
    ]);
    // 2. Remove a dish, then add it back under the SAME id with a different record.
    let log = check_sequence("remove-readd", vec![
        Box::new(|c| { c.remove_product("d4"); }),
        Box::new(|c| c.set_product("d5", &dish("d5", 1)) ),
        Box::new(|c| c.set_product("d4", &dish("d4", 4444))),
        Box::new(|c| { c.remove_product("d4"); c.remove_product("d0"); }),
        Box::new(|c| c.set_product("d4", &dish("d4", 4500))),
    ]);
    let d4: Vec<Option<String>> = edits(&log).unwrap().into_iter().filter(|e| e.key == "product:d4").map(|e| e.new).collect();
    assert_eq!(d4.len(), 5, "baseline, remove, re-add, remove, re-add: {d4:?}");
    assert_eq!((d4[1].is_none(), d4[3].is_none()), (true, true));
    // 3. Every key family: location, category, supply, promo.
    check_sequence("families", vec![
        Box::new(|c| c.set_location(r#"{"name":"Edits","currency":"ALL","menu_version":2}"#)),
        Box::new(|c| { c.set_category("hot", r#"{"id":"hot"}"#); c.set_supply("rice", r#"{"unit":"g"}"#); }),
        Box::new(|c| { c.set_promo("SAVE10", r#"{"pct":10}"#); c.remove_category("rolls"); }),
        Box::new(|c| { c.remove_promo("SAVE10"); c.remove_supply("rice"); }),
    ]);
}

#[test]
fn the_first_write_records_the_whole_menu_as_its_baseline() {
    let mut log = LogImage::create().unwrap();
    let start = born();
    write(&start, &mut log, 10, "staff-1", |c| c.set_product("d1", &dish("d1", 1)));
    let list = edits(&log).unwrap();
    // location + category + six dishes as the BASELINE (an empty journal), then the edit.
    assert_eq!(list.iter().filter(|e| e.by == BASELINE).count(), 8, "{list:?}");
    assert!(list.iter().all(|e| e.by != UNSEEN), "a first write is not drift");
    assert_eq!(list.last().map(|e| (e.by.as_str(), e.key.as_str())), Some(("staff-1", "product:d1")));
}

/// RED-able: replay must STOP at a record whose predecessor is missing.
#[test]
fn replay_refuses_a_journal_with_one_record_skipped() {
    let mut log = LogImage::create().unwrap();
    let mut bytes = born();
    for p in [610, 620, 630] {
        bytes = write(&bytes, &mut log, p, "staff-1", move |c| c.set_product("d1", &dish("d1", p)));
    }
    let list = edits(&log).unwrap();
    let skip = list.iter().rposition(|e| e.key == "product:d1" && e.new.as_deref() == Some(&dish("d1", 620)[..])).unwrap();
    let mut holed = LogImage::create().unwrap();
    for (i, e) in list.iter().enumerate().filter(|(i, _)| *i != skip) {
        let _ = i;
        holed.append(KIND, &e.key, &record_json(e.at_ms, &e.by, &e.key, &e.old, &e.new, e.gen)).unwrap();
    }
    assert!(matches!(replay(&holed, Cut::All), Err(ReplayError::Gap { ref key, .. }) if key == "product:d1"),
        "a journal missing the 620 record must not replay: {:?}", replay(&holed, Cut::All).map(|s| s.len()));
    // Its positive twin: the whole journal replays to the stored catalogue.
    assert_eq!(replay(&log, Cut::All).unwrap(), state_of(&Catalog::load(&bytes).unwrap()));
}

#[test]
fn a_change_made_without_the_journal_is_recorded_as_unseen_at_the_next_write() {
    let mut log = LogImage::create().unwrap();
    let mut bytes = write(&born(), &mut log, 1, "staff-1", |c| c.set_product("d1", &dish("d1", 1)));
    // A writer that does NOT journal (it moves the image's generation and writes no record):
    let mut c = Catalog::load(&bytes).unwrap();
    c.set_product("d2", &dish("d2", 2));
    bytes = c.to_bytes().unwrap();
    let skipped = last_gen(&log).unwrap() + 1;
    let mut c = Catalog::load(&bytes).unwrap();
    let before = state_of(&c);
    c.set_product("d3", &dish("d3", 3));
    let n = edits(&log).unwrap().len();
    journal(&mut log, &before, &state_of(&c), 3, "staff-2", (skipped, skipped + 1)).unwrap();
    bytes = c.to_bytes().unwrap();
    let tail: Vec<(String, String)> = edits(&log).unwrap()[n..].iter().map(|e| (e.by.clone(), e.key.clone())).collect();
    assert_eq!(tail, vec![(UNSEEN.into(), "product:d2".into()), ("staff-2".into(), "product:d3".into())]);
    assert_eq!(replay(&log, Cut::All).unwrap(), state_of(&Catalog::load(&bytes).unwrap()));
}

#[test]
fn the_decoder_refuses_a_foreign_kind_and_a_changed_byte() {
    let mut log = LogImage::create().unwrap();
    write(&born(), &mut log, 5, "staff-1", |c| c.set_product("d1", &dish("d1", 777)));
    // One named byte: the '7' of the newest record's "price":777 becomes '8'.
    let mut raw = log.to_bytes();
    let at = raw.windows(9).rposition(|w| w == b"price\\\":7").expect("the record's price") + 8;
    raw[at] = b'8';
    let bad = LogImage::load(&raw).expect("a crc failure is quarantined, not refused, at load");
    assert!(matches!(edits(&bad), Err(ReplayError::Unreadable { reason: "crc", .. })), "{:?}", edits(&bad).err());
    // A record of another kind in the journal image.
    let mut foreign = LogImage::create().unwrap();
    foreign.append("note", "x", "{}").unwrap();
    assert!(matches!(replay(&foreign, Cut::All), Err(ReplayError::Unreadable { reason: "foreign-kind", .. })));
}

#[test]
fn before_names_what_a_dish_held_and_a_compaction_keeps_every_kept_point() {
    let mut log = LogImage::create().unwrap();
    let mut bytes = born();
    for p in 1..=12 {
        bytes = write(&bytes, &mut log, p * 10, "staff-1", move |c| c.set_product("d1", &dish("d1", p)));
    }
    let list = edits(&log).unwrap();
    let last = list.last().unwrap().clone();
    let (e, was) = before(&log, last.seq).unwrap().unwrap();
    assert_eq!((e.key.as_str(), was), ("product:d1", Some(dish("d1", 11))));
    let small = compacted(&log, 5).unwrap();
    for e in edits(&small).unwrap().iter().filter(|e| e.by != BASELINE) {
        assert_eq!(replay(&small, Cut::Time(e.at_ms)).unwrap(), replay(&log, Cut::Time(e.at_ms)).unwrap(), "t={}", e.at_ms);
    }
    assert_eq!(replay(&small, Cut::All).unwrap(), state_of(&Catalog::load(&bytes).unwrap()));
    assert!(recent(&log, 3).unwrap().iter().map(|e| e.seq).eq([last.seq, last.seq - 1, last.seq - 2]));
}

// ── W-SPACE: the byte bound ─────────────────────────────────────────────────────────────────

/// A venue record as large as qa-durres's (~1 KB): every menu edit rewrites it (`menu_version`).
fn venue_record(v: i64) -> String {
    format!(r#"{{"name":"Edits","currency":"ALL","menu_version":{v},"about":"{}"}}"#, "x".repeat(1000))
}

/// `journal` WITHOUT its compaction: the same records, for a reference that keeps everything.
fn journal_all(log: &mut LogImage, before: &State, after: &State, at: i64, by: &str, gen: i64) {
    let mut rows: Vec<(&str, (String, Option<String>, Option<String>))> = vec![];
    if log.is_empty() {
        rows.extend(diff(&State::new(), before).into_iter().map(|r| (BASELINE, r)));
    }
    rows.extend(diff(before, after).into_iter().map(|r| (by, r)));
    for (i, (who, (k, o, n))) in rows.iter().enumerate() {
        let g = (i + 1 == rows.len()).then_some(gen);
        log.append(KIND, k, &record_json(at, who, k, o, n, g)).unwrap();
    }
}

/// `writes` price edits of d1, each bumping the venue record, journaled in full: (journal, menu).
fn qa_like(writes: i64) -> (LogImage, State) {
    let mut log = LogImage::create().unwrap();
    let mut c = Catalog::load(&born()).unwrap();
    for w in 1..=writes {
        let before = state_of(&c);
        c.set_product("d1", &dish("d1", 1000 + w));
        c.set_location(&venue_record(w));
        journal_all(&mut log, &before, &state_of(&c), w * 1000, "staff-1", w);
    }
    (log, state_of(&c))
}

/// Every reader of the journal, on the window `small` kept: the newest records (the console),
/// the menu at each kept record's time, and what each kept dish edit restores to.
fn same_on_the_kept_window(big: &LogImage, small: &LogImage) -> usize {
    let (b, s) = (edits(big).unwrap(), edits(small).unwrap());
    let kept = s.iter().rev().take_while(|e| e.by != BASELINE).count();
    let strip = |e: &Edit| (e.at_ms, e.by.clone(), e.key.clone(), e.old.clone(), e.new.clone(), e.gen);
    let (rb, rs) = (recent(big, kept).unwrap(), recent(small, kept).unwrap());
    assert!(rb.iter().map(strip).eq(rs.iter().map(strip)), "the console's newest {kept} differ");
    for r in 0..kept {
        let (eb, es) = (&b[b.len() - 1 - r], &s[s.len() - 1 - r]);
        assert_eq!(strip(eb), strip(es));
        assert_eq!(replay(small, Cut::Time(es.at_ms)).unwrap(), replay(big, Cut::Time(eb.at_ms)).unwrap(), "menu at t={}", es.at_ms);
        assert_eq!(before(small, es.seq).unwrap().map(|x| x.1), before(big, eb.seq).unwrap().map(|x| x.1), "restore of {}", es.key);
    }
    assert_eq!(replay(small, Cut::All).unwrap(), replay(big, Cut::All).unwrap());
    kept
}

/// RED-able: make `compacted_to` ignore `keep_bytes` (the cut stays at `len - keep`) and the
/// image does not shrink below a third.
#[test]
fn a_byte_bounded_compaction_keeps_every_readers_answer_on_its_window_and_drops_the_rest() {
    let (big, menu) = qa_like(250);
    assert!(big.len() < MAX_RECORDS, "this is the byte bound, not the count bound: {}", big.len());
    assert!(over_bytes(&big, &menu), "{} bytes in use", big.usage().used_cells * 8);
    let small = compacted_to(&big, KEEP, byte_budget(&menu).0).unwrap();
    let kept = same_on_the_kept_window(&big, &small);
    assert!(kept >= 30, "the console's default page is inside the window: {kept}");
    assert!(small.to_bytes().len() * 3 < big.to_bytes().len(), "{} -> {}", big.to_bytes().len(), small.to_bytes().len());
    // WHAT IS DROPPED, by name: an edit older than the window -- its signer and its time.
    let oldest_kept = edits(&small).unwrap().iter().find(|e| e.by != BASELINE).unwrap().at_ms;
    assert!(edits(&small).unwrap().iter().all(|e| e.by == BASELINE || e.at_ms >= oldest_kept));
    assert!(!edits(&small).unwrap().iter().any(|e| e.by == "staff-1" && e.at_ms == 1000), "the first edit is gone");
    // And it is under its own bound: the next write does not compact again.
    assert!(!over_bytes(&small, &menu));
}

/// RED-able: drop `|| over_bytes(log, after)` from `journal` and the journal grows to ~400 KB.
#[test]
fn the_writer_compacts_past_the_byte_bound_and_still_replays_to_the_menu() {
    let mut log = LogImage::create().unwrap();
    let mut bytes = born();
    let mut compactions = 0;
    for w in 1..=250 {
        let mut c = Catalog::load(&bytes).unwrap();
        let before = state_of(&c);
        c.set_product("d1", &dish("d1", 1000 + w));
        c.set_location(&venue_record(w));
        let read_at = last_gen(&log).unwrap_or(0);
        compactions += journal(&mut log, &before, &state_of(&c), w * 1000, "staff-1", (read_at, read_at + 1)).unwrap().compacted as usize;
        bytes = c.to_bytes().unwrap();
    }
    assert!(compactions >= 1, "never compacted");
    assert!(log.usage().used_cells as usize * 8 <= MAX_BYTES, "{} bytes in use", log.usage().used_cells * 8);
    assert_eq!(replay(&log, Cut::All).unwrap(), state_of(&Catalog::load(&bytes).unwrap()));
    assert!(log.quarantined().is_empty());
}

/// A menu larger than the bound itself: compaction cannot get under MAX_BYTES, so the bound
/// rises to the baseline it writes -- and it does not compact on every write. RED-able: make
/// `over_bytes` compare against MAX_BYTES alone and every write after the first compacts.
#[test]
fn a_menu_larger_than_the_bound_does_not_compact_on_every_write() {
    let mut c = Catalog::load(&born()).unwrap();
    for i in 0..300 {
        c.set_product(&format!("big{i}"), &format!(r#"{{"id":"big{i}","name":"{}","price":1}}"#, "n".repeat(600)));
    }
    let mut log = LogImage::create().unwrap();
    let mut compactions = 0;
    for w in 1..=40 {
        let before = state_of(&c);
        c.set_product("d1", &dish("d1", 1000 + w));
        let read_at = last_gen(&log).unwrap_or(0);
        compactions += journal(&mut log, &before, &state_of(&c), w, "staff-1", (read_at, read_at + 1)).unwrap().compacted as usize;
    }
    assert!(compactions <= 1, "{compactions} compactions in 40 writes");
    assert_eq!(replay(&log, Cut::All).unwrap(), state_of(&c));
}

/// THE REAL JOURNAL (qa-durres, W-SPACE). Not in the default run: the image is a copy of a
/// venue's data and is never committed. `DOWIZ_SPACE_DIR=<dir holding catalog.edits.img>
/// cargo test --release --lib real_catalog_edits -- --ignored --nocapture`.
#[test]
#[ignore]
fn real_catalog_edits_compaction() {
    let dir = std::env::var("DOWIZ_SPACE_DIR").expect("set DOWIZ_SPACE_DIR");
    let raw = std::fs::read(format!("{dir}/catalog.edits.img")).unwrap();
    let big = LogImage::load(&raw).unwrap();
    let menu = replay(&big, Cut::All).unwrap();
    let small = compacted_to(&big, KEEP, byte_budget(&menu).0).unwrap();
    let kept = same_on_the_kept_window(&big, &small);
    let after = small.to_bytes();
    println!("catalog.edits: {} records, {} bytes -> {} records ({kept} kept + baseline), {} bytes; over_bytes before {} after {}",
        big.len(), raw.len(), small.len(), after.len(), over_bytes(&big, &menu), over_bytes(&small, &menu));
    println!("packed at rest: {} -> {}", after.len(), crate::pack::pack(&after).len());
    // For the wasm32 timing of what an export packs (not `.img`, so the ratio test skips it).
    std::fs::write(format!("{dir}/catalog.edits.compacted.bin"), &after).unwrap();
}
