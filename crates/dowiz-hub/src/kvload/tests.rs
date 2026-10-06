//! W-CRC: one named test per loader of this crate. Each builds a real image, flips ONE
//! payload byte of one named object, and requires `HubError::BadCrc` naming that object --
//! where before the image loaded and the changed byte was served. No fuzzing (operator
//! rule): the corrupted cell is named.
use crate::catalog::Catalog;
use crate::post::Posts;
use crate::roster::Roster;
use crate::settings::Settings;
use crate::stock::{StockEvent, StockLog};
use crate::table::Table;
use crate::{EventKind, Hub, HubError};
use bebop_store::kv::Kv;
use bebop_store::Store;

/// A KV image the five KV loaders all accept (they read any KV image; what they do with
/// the keys is theirs).
fn kv_bytes() -> Vec<u8> {
    let mut kv = Kv { entries: Vec::new() };
    kv.put("product/maki", b"{\"name\":\"maki\",\"price\":900}");
    kv.put("venue.name", b"sushi");
    kv.compacted_bytes_fit(1 << 20).unwrap()
}

/// The value blob's first payload cell, one byte flipped; returns the blob's index.
fn flip_value_blob(bytes: &[u8]) -> (Vec<u8>, usize) {
    let mut st = Store::from_bytes(bytes);
    let vblob = st.follow(st.root().unwrap(), 4).unwrap();
    st.cells[vblob + 2] ^= 0x100;
    (st.to_bytes(), vblob)
}

/// The newest record's first payload cell (v2 record cell 12), one byte flipped.
fn flip_newest_record(bytes: &[u8]) -> (Vec<u8>, usize) {
    let mut st = Store::from_bytes(bytes);
    let newest = st.follow(st.root().unwrap(), 1).unwrap();
    st.cells[newest + 2 + 12] ^= 0x100;
    (st.to_bytes(), newest)
}

fn named<T>(r: Result<T, HubError>, obj: usize, who: &str) {
    match r {
        Err(HubError::BadCrc(b)) => assert_eq!(b.obj, obj, "{who}: names the object"),
        Err(e) => panic!("{who}: expected BadCrc at {obj}, got {e:?}"),
        Ok(_) => panic!("{who}: a changed byte LOADED"),
    }
}

#[test]
fn catalog_load_refuses_a_flipped_value_byte_by_name() {
    let b = kv_bytes();
    assert!(Catalog::load(&b).is_ok());
    let (bad, obj) = flip_value_blob(&b);
    named(Catalog::load(&bad), obj, "Catalog");
}

#[test]
fn settings_load_refuses_a_flipped_value_byte_by_name() {
    let b = kv_bytes();
    assert!(Settings::load(&b).is_ok());
    let (bad, obj) = flip_value_blob(&b);
    named(Settings::load(&bad), obj, "Settings");
}

#[test]
fn posts_load_refuses_a_flipped_value_byte_by_name() {
    let b = kv_bytes();
    assert!(Posts::load(&b).is_ok());
    let (bad, obj) = flip_value_blob(&b);
    named(Posts::load(&bad), obj, "Posts");
}

#[test]
fn table_load_refuses_a_flipped_value_byte_by_name() {
    let b = kv_bytes();
    assert!(Table::load(&b, 1 << 20).is_ok());
    let (bad, obj) = flip_value_blob(&b);
    named(Table::load(&bad, 1 << 20), obj, "Table");
}

#[test]
fn roster_load_refuses_a_flipped_value_byte_by_name() {
    let b = kv_bytes();
    assert!(Roster::load(&b).is_ok());
    let (bad, obj) = flip_value_blob(&b);
    named(Roster::load(&bad), obj, "Roster");
}

/// The stock log is an APPEND log: since the W-STORE2 merge it quarantines a bad-crc
/// record and folds the rest (operator 2026-10-05), like the order log.
#[test]
fn stocklog_load_quarantines_a_flipped_record_byte() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    for q in [100, 50] {
        log.append(&StockEvent::Received { item: "rice".into(), qty: q }).unwrap();
    }
    let b = log.to_bytes_trimmed();
    assert_eq!(StockLog::load(&b).unwrap().events().len(), 2);
    let (bad, _) = flip_newest_record(&b);
    let log = StockLog::load(&bad).expect("a bad stock record must not refuse the shelf");
    assert_eq!((log.events().len(), log.quarantined()), (1, 1));
    assert_eq!(log.ledger().unwrap().level("rice").on_hand, 100, "folds from the record that is still good");
}

/// D.1 #4 through the gauge: a hub log's dead figure rises with every append (one 10-cell
/// root each), and a compacted KV image reads 0.
#[test]
fn dead_per_mille_rises_with_appends_and_reads_zero_after_compaction() {
    let mut hub = Hub::create_sized(64 * 1024).unwrap();
    let mut last = hub.usage().dead_cells;
    for i in 0..5u64 {
        hub.append(EventKind::Placed, &format!("ord-{i}"), r#"{"status":"new"}"#, i, [0u8; 32]).unwrap();
        let u = hub.usage();
        assert_eq!(u.dead_cells - last, 10, "append {i}: one v2 root and its header");
        assert!(u.dead_per_mille() > 0 && u.dead_per_mille() < 1000, "{u:?}");
        last = u.dead_cells;
    }
    let mut cat = Catalog::load(&kv_bytes()).unwrap();
    assert_eq!(cat.usage().dead_cells, 0, "loaded from a compacted image");
    let back = Catalog::load(&cat.to_bytes().unwrap()).unwrap();
    assert_eq!(back.usage().dead_per_mille(), 0, "and again after a save");
}
