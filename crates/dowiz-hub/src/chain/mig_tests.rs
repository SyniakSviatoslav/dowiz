//! THE ROLLBACK-SAFE PATH FOR M0 (W-MG-MIG, 2026-10-31): what `set_sha256` guarantees, each
//! as a kill test. Measured first with the real old binary (ac5ff0c4, prod) on an image this
//! code wrote -- `docs/design/MIGRATION-sha256-ids.md` §1 quotes the lines: the old build does
//! not refuse a sha256 record, it QUARANTINES it (`reason=kind`: the nonce's first byte is
//! read as the event kind), writes FNV after it, and its `grow` strips the flag. So the one
//! safe shape is expand/contract: this build reads both schemes and WRITES legacy until a
//! venue is flipped, and a flip is one-way in the bytes.

use bebop_store::evlog::{EvLog, Record, FLAG_SHA256};
use bebop_store::Store;

use crate::chain::{chain_id, legacy_id_chained, NONCE_LEN};
use crate::logimage::LogImage;
use crate::stock::StockLog;
use crate::{EventKind, Hub};

const ACTOR: [u8; 32] = [0xA1; 32];

fn order(id: &str) -> String {
    format!(r#"{{"id":"{id}","status":"PENDING","subtotal":1800}}"#)
}

fn body(id: &str) -> Vec<u8> {
    let mut p = vec![EventKind::Placed as u8, id.len() as u8];
    p.extend_from_slice(id.as_bytes());
    p.extend_from_slice(order(id).as_bytes());
    p
}

/// A prod-shaped pre-M0 image: `n` records exactly as ac5ff0c4 writes them (FNV over
/// `prev ‖ payload`, flags 0, no nonce), written by hand into a 64 KiB v2 store.
fn old_image(n: usize) -> Vec<u8> {
    let mut st = Store::create_bytes(64 * 1024).unwrap();
    EvLog::init_bytes(&mut st).unwrap();
    let mut prev = [0u8; 32];
    for i in 0..n {
        let payload = body(&format!("ord_{i}"));
        let id = legacy_id_chained(&prev, &payload);
        EvLog::append_tip_bytes(&mut st, &Record { id, prev, actor_pubkey: ACTOR, actor_seq: i as u64 + 1, flags: 0, payload }).unwrap();
        prev = id;
    }
    st.to_bytes_trimmed()
}

fn walk(bytes: &[u8]) -> Vec<Record> {
    EvLog::walk(&Store::from_bytes(bytes))
}

/// FLAG OFF WRITES BYTE-IDENTICAL OLD RECORDS. An old image plus one record appended with the
/// switch off is, byte for byte, the image the old writer makes for the same four records:
/// same FNV id, flags 0, the body as the payload, no nonce. The rollback target reads it
/// because it is what the rollback target writes.
#[test]
fn switch_off_appends_exactly_what_the_old_writer_writes() {
    let mut h = Hub::load(&old_image(3)).unwrap();
    h.set_sha256(false);
    h.append(EventKind::Placed, "ord_3", &order("ord_3"), 4, ACTOR).unwrap();
    assert_eq!(h.to_bytes_trimmed(), old_image(4), "one appended record = the old writer's fourth record, byte for byte");
    let newest = &walk(&h.to_bytes())[0];
    assert_eq!(newest.flags, 0);
    assert_eq!(newest.id, legacy_id_chained(&newest.prev, &newest.payload));
    assert_eq!(newest.payload, body("ord_3"), "no nonce in front");
    let c = h.chain_check();
    assert_eq!((c.records, c.chained, c.legacy, c.broken), (4, 0, 4, 0), "{c:?}");
    assert_eq!(h.events().len(), 4);
}

/// THE FLIP IS ONE-WAY, IN THE BYTES. Once a log's newest record is sha256, asking for legacy
/// changes nothing: the next record is sha256 too, and the chain stays whole. This is what
/// makes a rollback to THIS build safe after a flip -- it keeps writing what the log is on --
/// and it is why `walk_check`'s "a legacy record newer than a sha256 one is broken" can never
/// be triggered by a config.
#[test]
fn switch_off_is_ignored_once_the_log_is_sha256() {
    let mut h = Hub::load(&old_image(2)).unwrap();
    h.append(EventKind::Placed, "new_0", &order("new_0"), 3, ACTOR).unwrap();
    h.set_sha256(false);
    h.append(EventKind::Placed, "new_1", &order("new_1"), 4, ACTOR).unwrap();
    let w = walk(&h.to_bytes());
    assert_eq!(w[0].flags & FLAG_SHA256, FLAG_SHA256, "flipped: the switch is ignored");
    assert_eq!(w[0].id, chain_id(&w[0].prev, &w[0].payload));
    assert_eq!(w[1].flags & FLAG_SHA256, FLAG_SHA256);
    let c = h.chain_check();
    assert_eq!((c.records, c.chained, c.legacy, c.broken), (4, 2, 2, 0), "{c:?}");
    assert_eq!(h.events().len(), 4);
}

/// A MIXED CHAIN KEEPS THE OLD PART BYTE-IDENTICAL. Flipping an old image appends sha256
/// records above the legacy ones and moves no byte of a legacy record: id, prev, flags and
/// payload of every old record are what they were, and the fold reads every order.
#[test]
fn a_flip_moves_no_byte_of_the_old_records() {
    let before = walk(&old_image(3));
    let mut h = Hub::load(&old_image(3)).unwrap();
    h.set_sha256(true);
    h.set_entropy([9u8; 32]);
    h.append(EventKind::Placed, "new_0", &order("new_0"), 4, ACTOR).unwrap();
    h.append(EventKind::Advanced, "new_0", &order("new_0"), 5, ACTOR).unwrap();
    let after = walk(&h.to_bytes());
    assert_eq!(&after[2..], &before[..], "the three legacy records are untouched");
    assert_eq!(after[0].prev, after[1].id);
    assert_eq!(after[1].prev, before[0].id, "the first sha256 record chains onto the legacy tip");
    assert_eq!(after[1].payload.len(), NONCE_LEN + body("new_0").len());
    let c = h.chain_check();
    assert_eq!((c.records, c.chained, c.legacy, c.broken, c.gaps), (5, 2, 3, 0, 0), "{c:?}");
    assert_eq!((h.events().len(), h.orders().len()), (5, 4));
}

/// THE SHAPE A ROLLBACK TO ac5ff0c4 LEAVES, read by this build: a legacy record written AFTER
/// a sha256 one. It is READ (every event decodes, nothing is quarantined) and it is NAMED
/// (`broken: 1`), never silently folded as healthy. `tools/deploy/rollback.sh` refuses the
/// rollback that makes this shape once a venue is flipped; this is what it protects.
#[test]
fn a_legacy_record_after_a_flip_is_read_and_named_broken() {
    let mut h = Hub::load(&old_image(2)).unwrap();
    h.append(EventKind::Placed, "new_0", &order("new_0"), 3, ACTOR).unwrap();
    let mut st = Store::from_bytes(&h.to_bytes());
    let prev = EvLog::tip(&st).unwrap();
    let payload = body("old_after");
    let id = legacy_id_chained(&prev, &payload);
    EvLog::append_tip_bytes(&mut st, &Record { id, prev, actor_pubkey: ACTOR, actor_seq: 4, flags: 0, payload }).unwrap();
    let back = Hub::load(&st.to_bytes()).unwrap();
    assert_eq!(back.events().len(), 4, "a proven read: every record decodes");
    assert!(back.quarantined().is_empty());
    let c = back.chain_check();
    assert_eq!((c.records, c.chained, c.legacy, c.broken), (4, 1, 2, 1), "named, not misread: {c:?}");
}

/// THE ENTRY LOG AND THE STOCK LOG HONOUR THE SWITCH AND ITS ONE-WAY RULE, like the order log.
#[test]
fn logimage_and_stocklog_honour_the_switch_one_way() {
    let mut l = LogImage::create_sized(64 * 1024).unwrap();
    l.set_sha256(false);
    l.append("e", "v", r#"{"a":1}"#).unwrap();
    let w = walk(&l.to_bytes());
    assert_eq!((w[0].flags, w[0].id), (0, legacy_id_chained(&w[0].prev, &w[0].payload)), "legacy entry");
    l.set_sha256(true);
    l.append("e", "v", r#"{"a":2}"#).unwrap();
    l.set_sha256(false);
    l.append("e", "v", r#"{"a":3}"#).unwrap();
    let w = walk(&l.to_bytes());
    assert_eq!((w[0].flags & FLAG_SHA256, w[1].flags & FLAG_SHA256, w[2].flags), (FLAG_SHA256, FLAG_SHA256, 0));
    let c = l.chain_check();
    assert_eq!((c.chained, c.legacy, c.broken), (2, 1, 0), "{c:?}");
    assert_eq!(l.entries().len(), 3);

    let mut s = StockLog::create_sized(64 * 1024).unwrap();
    s.set_sha256(false);
    s.receive_with("rice", 5000, &crate::stock::meta::Meta::default()).unwrap();
    let w = walk(&s.to_bytes());
    assert_eq!(w[0].flags, 0);
    assert_eq!(w[0].payload.first(), Some(&b'{'), "a bare JSON object, as ac5ff0c4 writes it");
    assert_eq!(w[0].id, legacy_id_chained(&w[0].prev, &w[0].payload));
    s.set_sha256(true);
    s.receive_with("rice", 100, &crate::stock::meta::Meta::default()).unwrap();
    s.set_sha256(false);
    s.receive_with("rice", 200, &crate::stock::meta::Meta::default()).unwrap();
    let w = walk(&s.to_bytes());
    assert_eq!((w[0].flags & FLAG_SHA256, w[1].flags & FLAG_SHA256, w[2].flags), (FLAG_SHA256, FLAG_SHA256, 0), "one-way");
    assert_eq!(s.events().len(), 3, "every record still folds");
}

/// `keep` RE-SEALS UNDER THE LOG'S OWN SCHEME: a legacy entry log trimmed by this build stays
/// legacy, so a rollback target still reads the error log it trimmed.
#[test]
fn keep_reseals_a_legacy_log_as_legacy() {
    let mut l = LogImage::create_sized(64 * 1024).unwrap();
    l.set_sha256(false);
    for i in 0..5 {
        l.append("e", "v", &format!(r#"{{"i":{i}}}"#)).unwrap();
    }
    assert_eq!(l.keep(3).unwrap(), 2);
    let w = walk(&l.to_bytes());
    assert_eq!(w.len(), 3);
    assert!(w.iter().all(|r| r.flags == 0 && r.id == legacy_id_chained(&r.prev, &r.payload)), "all legacy after the trim");
    let c = l.chain_check();
    assert_eq!((c.chained, c.legacy, c.broken), (0, 3, 0), "{c:?}");
}
