use super::*;
use bebop_store::evlog::{EvLog, Record};
use crate::chain::content_id;

const ACTOR: [u8; 32] = [0xA1; 32];
fn order(id: &str, status: &str) -> String {
    format!(r#"{{"id":"{id}","status":"{status}","subtotal":1800}}"#)
}

/// `Amended` IS AN ORDER EVENT: it folds into the order it names, travels
/// on the socket, and survives a reload under its own byte. An unassigned byte is still
/// nobody's, so the reader that predates a kind keeps quarantining it —
/// which is the deploy-order rule: readers before writers.
#[test]
fn an_amendment_is_an_order_event_under_its_own_byte() {
    assert_eq!(EventKind::from_u8(7), Some(EventKind::Amended));
    assert!(EventKind::Amended.is_order());
    assert_eq!(EventKind::from_u8(0x7f), None);
    let mut h = Hub::create_sized(1 << 20).unwrap();
    h.append(EventKind::Placed, "ord_a", &order("ord_a", "PENDING"), 1, ACTOR).unwrap();
    h.append(EventKind::Amended, "ord_a", r#"{"_d":true,"total":900}"#, 2, ACTOR).unwrap();
    let back = Hub::load(&h.to_bytes_trimmed()).unwrap();
    assert!(back.quarantined().is_empty(), "a kind-7 record is readable by this build");
    let hist = back.history("ord_a");
    assert_eq!(hist.len(), 2);
    assert_eq!(hist[1].kind, EventKind::Amended, "oldest first: the amendment is second");
    assert_eq!(back.orders().len(), 1, "one order, not an order and an amendment");
}

#[test]
fn a_fresh_hub_is_empty_and_reloads() {
    let h = Hub::create_sized(1 << 20).unwrap();
    assert!(h.is_empty());
    let back = Hub::load(&h.to_bytes()).unwrap();
    assert!(back.is_empty(), "an empty hub survives the byte round-trip");
}

#[test]
fn load_refuses_something_that_is_not_a_hub() {
    assert!(matches!(Hub::load(&[0u8; 4096]), Err(HubError::NotAHub)),
            "a zeroed image has no superblock and must be refused, not served");
}

/// AND REFUSES A HUB THAT LOST ORDERS ON THE WAY HERE.
///
/// `load` checked the superblock and stopped there -- but a superblock is
/// fifteen cells at the FRONT of the image, so it survives a short read
/// that takes half the orders with it. The hub then answered `len() == 12`
/// while `orders()` listed four, and every reply built from it -- the
/// console, the tracking sheet, the day's takings -- was confidently
/// missing the rest. An image that cannot deliver what it claims is
/// refused, so the caller can re-fetch or alert instead of serving it.
#[test]
fn load_refuses_a_hub_that_cannot_deliver_the_orders_it_claims() {
    let mut h = Hub::create_sized(1 << 20).unwrap();
    for i in 0..12 {
        let id = format!("ord_{i:02}");
        h.append(EventKind::Placed, &id, &order(&id, "PENDING"), i as u64, ACTOR).unwrap();
    }
    let bytes = h.to_bytes_trimmed();
    assert_eq!(Hub::load(&bytes).unwrap().orders().len(), 12, "the whole image is whole");

    for tenth in 1..10usize {
        let keep = bytes.len() * tenth / 10;
        if let Ok(short) = Hub::load(&bytes[..keep]) {
            assert_eq!(
                short.len(),
                short.events().len() + short.quarantined().len(),
                "a hub cut to {keep} bytes loaded and then lost a record to neither list"
            );
        }
    }

    // And the sharpest case: every byte is there, one `next` ref is not.
    let mut st = Store::from_bytes(&bytes);
    let root = st.root().unwrap();
    let newest = st.follow(root, 1).unwrap();
    st.cells[newest + 2 + 2] = 1 << 40;
    assert!(
        matches!(Hub::load(&st.to_bytes()), Err(HubError::Corrupt { claimed: 12, .. })),
        "a chain that stops early must be refused, not served short"
    );
}

/// L5 FROM THE RESILIENCE BLUEPRINT: one bad record must not close the
/// restaurant, and must not vanish either.
///
/// These are the two failures of the same byte. Refusing the whole image
/// because one record is unreadable takes a venue off the air over a single
/// order; skipping it silently takes the order off the books and says
/// nothing. So: the hub LOADS, the record is left out of `events()`, and it
/// is named in `quarantined()` with the promise it broke and an id a human
/// can find in the image. The arithmetic is the whole guarantee —
/// `len() == events().len() + quarantined().len()` — because it is the one
/// statement that cannot be true while a record is quietly missing.
#[test]
fn a_record_this_build_cannot_read_is_quarantined_and_the_venue_still_serves() {
    let mut h = Hub::create_sized(1 << 20).unwrap();
    for i in 0..6 {
        let id = format!("ord_{i:02}");
        h.append(EventKind::Placed, &id, &order(&id, "PENDING"), i as u64, ACTOR).unwrap();
    }
    let bytes = h.to_bytes_trimmed();

    let mut st = Store::from_bytes(&bytes);
    let root = st.root().unwrap();
    let newest = st.follow(root, 1).unwrap();
    // v2 packs eight payload bytes per cell after a twelve-cell header, plus
    // four more when an actor key is present (bit 0 of cell 11 says so).
    let at = if st.get(newest, 11) & 1 != 0 { 16 } else { 12 };
    let cell = st.get(newest, at);
    // Byte 0 of the payload is the event kind, and 9 is not one of the six.
    st.cells[newest + 2 + at] = (cell & !0xFF) | 9;

    let broken = Hub::load(&st.to_bytes()).expect("one bad record must not refuse the image");
    assert_eq!(broken.len(), 6, "the log still holds six records");
    assert_eq!(broken.events().len(), 5, "the unreadable one is not served");
    let q = broken.quarantined();
    assert_eq!(q.len(), 1, "and it is named: {q:?}");
    assert_eq!(q[0].reason, "kind");
    assert_eq!(q[0].at, 0, "it was the newest record");
    assert_eq!(q[0].id.len(), 64, "the id is there for a human to find");
    assert_eq!(
        broken.len(),
        broken.events().len() + broken.quarantined().len(),
        "a record must be in exactly one of the two lists"
    );
    // The orders that are readable are still served, which is the point.
    assert!(broken.order("ord_00").is_ok(), "the venue keeps serving");
}

/// The point of the log: an order's state is the FOLD, so the newest event
/// wins and the earlier one is still there to explain how it got there.
#[test]
fn an_order_reads_back_as_its_newest_event() {
    let mut h = Hub::create_sized(1 << 20).unwrap();
    h.append(EventKind::Placed, "ord_a", &order("ord_a", "PENDING"), 1, ACTOR).unwrap();
    assert!(h.order("ord_a").unwrap().contains("PENDING"));

    h.append(EventKind::Advanced, "ord_a", &order("ord_a", "CONFIRMED"), 2, ACTOR).unwrap();
    assert!(h.order("ord_a").unwrap().contains("CONFIRMED"), "the fold shows the newest state");

    assert_eq!(h.len(), 2, "and the earlier event is NOT overwritten");
    let evs = h.events();
    assert_eq!(evs[0].kind, EventKind::Advanced);
    assert_eq!(evs[1].kind, EventKind::Placed);
}

#[test]
fn orders_lists_each_order_once_at_its_newest_state() {
    let mut h = Hub::create_sized(1 << 20).unwrap();
    h.append(EventKind::Placed, "ord_a", &order("ord_a", "PENDING"), 1, ACTOR).unwrap();
    h.append(EventKind::Placed, "ord_b", &order("ord_b", "PENDING"), 2, ACTOR).unwrap();
    h.append(EventKind::Advanced, "ord_a", &order("ord_a", "READY"), 3, ACTOR).unwrap();

    let list = h.orders();
    assert_eq!(list.len(), 2, "two orders, not three events");
    assert_eq!(list[0].order_id, "ord_a", "newest first");
    assert!(list[0].order_json.contains("READY"));
    assert!(list[1].order_json.contains("PENDING"));
}

#[test]
fn unknown_order_is_an_error_not_an_empty_string() {
    let h = Hub::create_sized(1 << 20).unwrap();
    assert!(matches!(h.order("nope"), Err(HubError::UnknownOrder)));
}

/// ROTATION KEEPS WHAT IS LIVE AND MOVES WHAT IS NOT. The orders that stay
/// must fold to exactly what they folded to before: a venue must not see
/// its kitchen change because the log was tidied.
#[test]
fn a_rotation_keeps_the_live_orders_untouched() {
    let mut h = Hub::create_sized(256 * 1024).unwrap();
    for i in 0..6u64 {
        let id = format!("ord_{i}");
        h.append(EventKind::Placed, &id, &order(&id, "PENDING"), i + 1, ACTOR).unwrap();
        h.append(EventKind::Advanced, &id, &order(&id, "CONFIRMED"), i + 10, ACTOR).unwrap();
    }
    let before: Vec<String> =
        ["ord_4", "ord_5"].iter().map(|id| h.order(id).unwrap()).collect();

    let archived = h.rotate(|id| id == "ord_4" || id == "ord_5").unwrap();

    let after: Vec<String> =
        ["ord_4", "ord_5"].iter().map(|id| h.order(id).unwrap()).collect();
    assert_eq!(before, after, "a kept order must be untouched by the rotation");
    assert_eq!(h.orders().len(), 2, "and nothing else stayed");
    assert!(h.order("ord_0").is_err(), "a moved order is not in the hot log");

    // THE HISTORY IS NOT GONE, it is in the bytes the caller now holds.
    let cold = Hub::load(&archived).unwrap();
    assert_eq!(cold.orders().len(), 6, "every order is in the archive");
    assert!(cold.order("ord_0").unwrap().contains("ord_0"));

    // And the hot image holds much less than what it replaced. MEASURED
    // IN ARENA CELLS, not in bytes of image: every image carries a fixed
    // 1024-cell superblock region -- eight kilobytes that neither side
    // pays for twice -- and at this size that header is most of the file.
    let hot_cells = h.usage().used_cells;
    let cold_cells = Hub::load(&archived).unwrap().usage().used_cells;
    println!("rotation: hot {hot_cells} cells, archive {cold_cells}");
    assert!(hot_cells * 2 < cold_cells, "hot {hot_cells} cells against archive {cold_cells}");
}

/// THE CHECKPOINT IS VISIBLE. An unknown event kind used to be dropped by
/// `decode` without a sound, so a mark in the log would have been a mark
/// nobody could see -- which is the failure this project keeps meeting.
#[test]
fn the_checkpoint_is_an_event_the_log_returns() {
    let mut h = Hub::create_sized(128 * 1024).unwrap();
    h.append(EventKind::Placed, "ord_1", &order("ord_1", "PENDING"), 1, ACTOR).unwrap();
    let tip_before = h.checkpoints().len();
    assert_eq!(tip_before, 0);

    let archived = h.rotate(|_| false).unwrap();

    let events = h.events();
    assert_eq!(events.len(), 1, "the checkpoint is the only thing left: {events:?}");
    assert_eq!(events[0].kind, EventKind::Checkpoint);
    assert!(!events[0].kind.is_order(), "a checkpoint is not an order");
    assert!(h.orders().is_empty(), "and it does not appear as one");

    let marks = h.checkpoints();
    assert_eq!(marks.len(), 1);
    assert!(marks[0].starts_with("tip="), "{}", marks[0]);
    assert!(marks[0].contains("events=1"), "{}", marks[0]);
    assert!(
        marks[0].contains(&format!("bytes={}", archived.len())),
        "the mark names the archive it describes: {}",
        marks[0]
    );
}

/// THE AUDIT TRAIL SURVIVES A ROTATION. It is not an order, so `keep` was
/// never asked about it and the first rotation would have deleted the
/// venue's entire record of who read whose phone number -- out of the hot
/// log, and out of every archive reader, which folds orders.
#[test]
fn a_rotation_keeps_the_audit_trail() {
    let mut h = Hub::create_sized(128 * 1024).unwrap();
    h.append(EventKind::Placed, "ord_1", &order("ord_1", "DELIVERED"), 1, ACTOR).unwrap();
    h.append(EventKind::Revealed, "cust:abc", r#"{"by":"owner_1","at":2}"#, 2, ACTOR).unwrap();

    // Nothing is kept: the strongest version of the test.
    h.rotate(|_| false).unwrap();

    let reveals = h.reveals();
    assert_eq!(reveals.len(), 1, "the reveal must still be in the hot log");
    assert_eq!(reveals[0].order_id, "cust:abc");
    assert!(reveals[0].order_json.contains("owner_1"));
    assert!(h.orders().is_empty(), "and it is still not an order");
}

/// A SECOND ROTATION CHAINS. Each archive names the tip of the one before
/// it, so the archives form a list a reader can walk backwards; the hot
/// image carries exactly one mark, never a pile of them.
#[test]
fn a_second_rotation_chains_to_the_first() {
    let mut h = Hub::create_sized(256 * 1024).unwrap();
    h.append(EventKind::Placed, "ord_1", &order("ord_1", "PENDING"), 1, ACTOR).unwrap();
    let first = h.rotate(|_| false).unwrap();
    let mark_one = h.checkpoints()[0].clone();

    h.append(EventKind::Placed, "ord_2", &order("ord_2", "PENDING"), 2, ACTOR).unwrap();
    let second = h.rotate(|_| false).unwrap();
    let mark_two = h.checkpoints()[0].clone();

    assert_eq!(h.checkpoints().len(), 1, "one mark, not a pile");
    assert_ne!(mark_one, mark_two);
    // The second archive holds the first mark, so the chain is walkable.
    let cold_two = Hub::load(&second).unwrap();
    assert_eq!(cold_two.checkpoints(), vec![mark_one], "the archive carries the older mark");
    let cold_one = Hub::load(&first).unwrap();
    assert!(cold_one.checkpoints().is_empty(), "the first archive predates any mark");
}

/// WHAT A WITNESS IS FOR, and it is the failure `chain_check` is blind to.
///
/// Removing records from the END leaves a shorter chain that verifies
/// perfectly: every id still commits to the one before it, because the
/// cascade only ever looks backwards. What does change is the TIP. So a
/// tip written down somewhere the editor cannot reach — off-site, or in
/// another object — turns a silent truncation into a contradiction.
#[test]
fn a_truncated_log_still_verifies_and_no_longer_holds_its_tip() {
    let mut h = Hub::create_sized(256 * 1024).unwrap();
    for i in 0..6u64 {
        let id = format!("ord_{i}");
        h.append(EventKind::Placed, &id, &order(&id, "PENDING"), i + 1, ACTOR).unwrap();
    }
    let witnessed = h.tip().expect("a log with records has a tip");
    assert!(h.holds(&witnessed), "the tip is in its own log");

    // The truncation: rebuild the log from the first five records only,
    // which is what an editor with write access can do.
    let mut cut = Hub::create_sized(256 * 1024).unwrap();
    for i in 0..5u64 {
        let id = format!("ord_{i}");
        cut.append(EventKind::Placed, &id, &order(&id, "PENDING"), i + 1, ACTOR).unwrap();
    }
    assert!(cut.chain_check().intact(), "a truncated log passes the chain check");
    assert_eq!(cut.len(), 5);
    assert!(!cut.holds(&witnessed), "and the witnessed tip is gone — which is the tell");
    assert_ne!(cut.tip(), Some(witnessed), "the tip moved backwards");
}

/// AND A ROTATION IS NOT A TRUNCATION, which is the distinction that makes
/// the witness usable rather than an alarm every night. The records move
/// verbatim, ids and all, so last night's tip is still held — by the
/// archive. A witness that could not tell these apart would be switched
/// off within a week.
#[test]
fn a_rotation_moves_the_tip_into_the_archive_rather_than_losing_it() {
    let mut h = Hub::create_sized(256 * 1024).unwrap();
    for i in 0..4u64 {
        let id = format!("ord_{i}");
        h.append(EventKind::Placed, &id, &order(&id, "PENDING"), i + 1, ACTOR).unwrap();
    }
    let witnessed = h.tip().expect("tip");
    let archived = h.rotate(|id| id == "ord_3").unwrap();
    let cold = Hub::load(&archived).unwrap();
    assert!(cold.holds(&witnessed), "the archive holds the record verbatim");
    // The hot log holds it too here, because `ord_3` was kept; what
    // matters is that the pair of images between them never loses it.
    assert!(
        h.holds(&witnessed) || cold.holds(&witnessed),
        "a rotation must not lose a record between the two images"
    );
    // An empty log has no tip, and that is not a failure either.
    assert_eq!(Hub::create_sized(1 << 16).unwrap().tip(), None);
}

/// The cascade survives a rotation. Records move verbatim, so each still
/// commits to the id before it -- even where the record before it is now
/// in a different image.
#[test]
fn a_rotated_log_still_verifies() {
    let mut h = Hub::create_sized(256 * 1024).unwrap();
    for i in 0..4u64 {
        let id = format!("ord_{i}");
        h.append(EventKind::Placed, &id, &order(&id, "PENDING"), i + 1, ACTOR).unwrap();
    }
    let archived = h.rotate(|id| id == "ord_3").unwrap();
    let hot = h.chain_check();
    assert!(hot.intact(), "{hot:?}");
    assert_eq!(hot.broken, 0);
    assert_eq!(hot.records, 2, "the checkpoint and the one kept order");
    let cold = Hub::load(&archived).unwrap().chain_check();
    assert!(cold.intact(), "{cold:?}");
    assert_eq!(cold.records, 4);
}

/// An order half of whose events survived would fold to a lie, so `keep` is
/// asked about the ORDER and every event of a kept order travels with it.
#[test]
fn a_kept_order_keeps_all_of_its_events() {
    let mut h = Hub::create_sized(256 * 1024).unwrap();
    for (status, at) in [("PENDING", 1u64), ("CONFIRMED", 2), ("COOKING", 3), ("DELIVERED", 4)]
    {
        h.append(
            if at == 1 { EventKind::Placed } else { EventKind::Advanced },
            "ord_1",
            &order("ord_1", status),
            at,
            ACTOR,
        )
        .unwrap();
    }
    h.rotate(|id| id == "ord_1").unwrap();
    let kept: Vec<Event> = h.history("ord_1");
    assert_eq!(kept.len(), 4, "every event of a kept order travels with it");
    assert!(kept[0].order_json.contains("PENDING"), "oldest first, and the first is the placement");
    assert!(kept[3].order_json.contains("DELIVERED"));
}

/// THE CASCADE IS REAL NOW, and this is the test that would have caught the
/// comment being wrong. Every id commits to the record before it, so a
/// walk can say whether the log has been edited.
#[test]
fn every_id_commits_to_the_record_before_it() {
    let mut h = Hub::create_sized(256 * 1024).unwrap();
    for i in 0..6 {
        h.append(EventKind::Placed, &format!("ord_{i}"), &order(&format!("ord_{i}"), "PENDING"),
                 i as u64 + 1, ACTOR).unwrap();
    }
    let check = h.chain_check();
    assert_eq!(check.records, 6);
    assert_eq!(check.chained, 6, "every record verifies against prev + payload");
    assert_eq!(check.legacy, 0);
    assert!(check.intact());

    // The same payload after a DIFFERENT predecessor is a different id.
    // Under the old scheme these two were identical, which is what made
    // "editing any event changes every id after it" false.
    let one = content_id_chained(&[0u8; 32], b"same bytes");
    let two = content_id_chained(&[9u8; 32], b"same bytes");
    assert_ne!(one, two, "the id has to depend on where in the chain it sits");
}

/// An event edited in the image is found. This is the property the log is
/// FOR: the record keeps its old id, the payload no longer hashes to it,
/// and the walk says so instead of folding the forgery into an order.
#[test]
fn an_edited_event_is_reported_as_broken() {
    let mut h = Hub::create_sized(256 * 1024).unwrap();
    for i in 0..4 {
        h.append(EventKind::Placed, &format!("ord_{i}"), &order(&format!("ord_{i}"), "PENDING"),
                 i as u64 + 1, ACTOR).unwrap();
    }
    assert!(h.chain_check().intact());

    // Edit one byte of one payload in the raw image: "PENDING" -> "PENDINH".
    let mut bytes = h.to_bytes();
    let at = bytes
        .windows(7)
        .position(|w| w == b"PENDING")
        .expect("the status is in the image as text");
    bytes[at + 6] = b'H';
    let tampered = Hub::load(&bytes).unwrap();

    let check = tampered.chain_check();
    assert_eq!(check.records, 4);
    assert_eq!(check.broken, 1, "the edited record must not verify: {check:?}");
    assert!(!check.intact());
}

/// A log written before the cascade reads as LEGACY, not as broken. Every
/// image in production is one of these, and a check that called them
/// tampered with would be an alarm that is always on.
#[test]
fn a_pre_cascade_log_is_legacy_rather_than_broken() {
    // One record written the OLD way, by hand: id over the payload alone.
    let payload = {
        let mut p = vec![EventKind::Placed as u8];
        let id = b"ord_old";
        p.push(id.len() as u8);
        p.extend_from_slice(id);
        p.extend_from_slice(br#"{"id":"ord_old","status":"PENDING"}"#);
        p
    };
    let rec = Record {
        // The OLD scheme: the payload alone.
        id: content_id(&payload),
        prev: [0u8; 32],
        actor_pubkey: [0u8; 32],
        actor_seq: 1,
        payload,
    };
    // Written straight into a store, because no public path writes an old
    // id any more -- which is the point.
    let mut st = Store::create_bytes(64 * 1024).unwrap();
    EvLog::init_bytes(&mut st).unwrap();
    EvLog::append_tip_bytes(&mut st, &rec).unwrap();
    let h = Hub::load(&st.to_bytes()).unwrap();

    let check = h.chain_check();
    assert_eq!(check.records, 1);
    assert_eq!(check.legacy, 1, "an old id is old, not wrong: {check:?}");
    assert_eq!(check.broken, 0);
    assert!(check.intact());
}

/// The whole hub survives being written out and read back — which is the
/// operation a Worker performs on every single request.
#[test]
fn the_log_survives_the_byte_round_trip() {
    let mut h = Hub::create_sized(1 << 20).unwrap();
    for i in 0..5 {
        h.append(EventKind::Placed, &format!("ord_{i}"), &order(&format!("ord_{i}"), "PENDING"),
                 i as u64 + 1, ACTOR).unwrap();
    }
    let bytes = h.to_bytes();
    let back = Hub::load(&bytes).unwrap();
    assert_eq!(back.len(), 5);
    assert_eq!(back.orders().len(), 5);
    assert!(back.order("ord_3").unwrap().contains("ord_3"));
    assert_eq!(back.to_bytes(), bytes, "reloading changes nothing");
}

/// THE TRIMMED IMAGE IS THE SAME HUB. This is the image the object stores
/// and the backup carries, so "the same" has to mean byte-for-byte after a
/// reload, not merely "the orders come back": the padding must land the
/// arena, the capacity and the superblocks exactly where the full image
/// had them, or the next append writes into a different store.
#[test]
fn a_trimmed_log_reloads_into_the_same_hub() {
    let mut h = Hub::create_sized(1 << 20).unwrap();
    for i in 0..5 {
        h.append(EventKind::Placed, &format!("ord_{i}"), &order(&format!("ord_{i}"), "PENDING"),
                 i as u64 + 1, ACTOR).unwrap();
    }
    let full = h.to_bytes();
    let trimmed = h.to_bytes_trimmed();
    assert!(trimmed.len() < full.len() / 4, "a 1 MiB image of five orders is mostly zeros");

    let back = Hub::load(&trimmed).unwrap();
    assert_eq!(back.len(), 5);
    assert_eq!(back.orders().len(), 5);
    assert!(back.order("ord_3").unwrap().contains("ord_3"));
    assert_eq!(back.to_bytes(), full, "the padded image is the full image");
    assert_eq!(back.usage().capacity_cells, h.usage().capacity_cells, "capacity survives");
}

/// A hub reloaded from a trimmed image keeps appending where it left off,
/// and what it writes next is still the same bytes as the one that never
/// left memory. A capacity lost in the round trip would show here as an
/// early refusal or a `grow` at the wrong size.
#[test]
fn a_trimmed_log_keeps_appending() {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    h.append(EventKind::Placed, "ord_1", &order("ord_1", "PENDING"), 1, ACTOR).unwrap();

    let mut back = Hub::load(&h.to_bytes_trimmed()).unwrap();
    h.append(EventKind::Advanced, "ord_1", &order("ord_1", "COOKING"), 2, ACTOR).unwrap();
    back.append(EventKind::Advanced, "ord_1", &order("ord_1", "COOKING"), 2, ACTOR).unwrap();
    assert_eq!(back.len(), 2);
    assert_eq!(back.to_bytes(), h.to_bytes(), "the reloaded hub writes the same image");
    assert_eq!(back.to_bytes_trimmed(), h.to_bytes_trimmed());
}

/// The number this change exists for: a fresh hub is a few hundred bytes on
/// the wire, not its whole arena. It crossed the Worker-to-object hop on
/// every write.
#[test]
fn a_fresh_log_is_tiny_on_the_wire() {
    let h = Hub::create_sized(4 << 20).unwrap();
    let trimmed = h.to_bytes_trimmed();
    assert_eq!(h.to_bytes().len(), 4 << 20);
    assert!(trimmed.len() < 16 * 1024, "a fresh 4 MiB hub trims to {} bytes", trimmed.len());
    assert_eq!(Hub::load(&trimmed).unwrap().len(), 0);
}

#[test]
fn an_over_long_order_id_is_refused_rather_than_truncated() {
    let mut h = Hub::create_sized(1 << 20).unwrap();
    let long = "x".repeat(256);
    assert!(matches!(h.append(EventKind::Placed, &long, "{}", 1, ACTOR),
                     Err(HubError::OrderIdTooLong)),
            "truncating an id would silently merge two different orders");
}
