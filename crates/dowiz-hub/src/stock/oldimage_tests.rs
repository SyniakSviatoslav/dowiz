//! P12's OLD-IMAGE RULE, as a golden (W-STORE, 2026-10-05): a log written
//! with no storage key and no move folds -- and is WRITTEN, checkpoints and
//! all -- exactly as before storages existed. The digests below were taken
//! from the code BEFORE `storages.rs` was added (base 2913b5d5 + W-MR0), by
//! running this test with the constants empty; a change that alters one byte
//! of an old venue's image, or one number of its fold, turns this red.

use super::journal::Journal;
use super::meta::Meta;
use super::*;
use sha2::{Digest, Sha256};

/// A deterministic history through the real write doors, every kind of
/// record a venue had before storages: priced receipts with lots and
/// expiries, an order reserved and consumed, a void, a write-off, a count,
/// prep into another supply, and checkpoints every 7 records.
pub(super) fn old_history() -> StockLog {
    let mut log = StockLog::create_sized(16 * 1024).unwrap();
    log.set_checkpoint_every(7);
    for step in 0..40i64 {
        log.set_clock(1_759_000_000_000 + step * 3_600_000);
        let m = Meta {
            unit_cost: Some(900 + step * 10),
            per: Some(1000),
            supplier: Some("Peshku".into()),
            doc: Some(format!("F-{step}")),
            lot: (step % 3 == 0).then(|| format!("L{step}")),
            expiry: Some(20261010 + step % 9),
            ..Meta::default()
        };
        log.receive_with("salmon", 1500 + step, &m).unwrap();
        log.receive_with("rice", 5000, &Meta::default()).unwrap();
        let order = format!("o{step}");
        log.append_all(&reservations_for(&order, &[(r#"{"bom":[{"supply":"salmon","qty":40},{"supply":"rice","qty":90}]}"#.into(), 3)]))
            .unwrap();
        let led = log.ledger().unwrap();
        log.append_all(&settle(&led, &order, step % 4 != 0)).unwrap();
        if step % 5 == 0 {
            log.append(&StockEvent::Wasted { item: "salmon".into(), qty: 30, reason: WasteReason::Spoiled, by: "p1".into() }).unwrap();
        }
        if step % 11 == 0 {
            log.append(&StockEvent::Stocktake { item: "rice".into(), observed: 4000, stocktake_id: format!("c{step}"), by: "p1".into() })
                .unwrap();
        }
        if step % 7 == 3 {
            let into = Some("fillet".to_string());
            log.append(&StockEvent::Produced { item: "salmon".into(), qty: 1000, out: 620, stage: PrepStage::Clean, into, by: "p2".into() })
                .unwrap();
        }
    }
    log
}

fn hex(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

/// Everything a reader of the fold sees, as text.
pub(super) fn fold_text(j: &Journal) -> String {
    format!("{:?}|{:?}|{:?}|{:?}", j.ledger.items(), j.book, j.lots.open(), j.carry)
}

// Re-derived at M0 (2026-10-30): every record this writer emits now carries a 16-byte nonce
// in front of its body, the sha256 flag in cell 11 and a sha256 id, so the IMAGE digest moved
// by design (the pre-M0 value was 863594c1...031d). FOLD_SHA did NOT move -- the fold does not
// see the envelope -- and `m0_a_pre_m0_stock_image_reloads_byte_equal_and_folds_as_the_nonced_one`
// below holds the old bytes themselves to the same fold.
const IMAGE_SHA: &str = "dfacc9ccbc3c08ae5dfa19429ec22ee791a01583b8db9cb34a68388d328909a9";
const FOLD_SHA: &str = "d83a1bd5b0e1f7940cfa78141b3bba92f2777e229d0f14ea7e38636dc45a2a9f";

#[test]
fn an_old_image_writes_and_folds_byte_equal() {
    let log = old_history();
    // W-ES-P0 writes the anchor (Q5); the golden pins every other byte.
    let image = hex(&bebop_store::format::without_anchor(&log.to_bytes_trimmed()));
    let j = log.journal().unwrap();
    let fold = hex(fold_text(&j).as_bytes());
    let checked = log.verify_checkpoints().unwrap();
    println!("GOLDEN IMAGE_SHA={image} FOLD_SHA={fold} checkpoints={checked} records={}", log.len());
    assert!(checked > 10, "the history wrote checkpoints: {checked}");
    assert_eq!(image, IMAGE_SHA, "an old venue's image changed by one byte or more");
    assert_eq!(fold, FOLD_SHA, "an old venue's fold changed");
}

/// M0's OLD-IMAGE RULE for the stock log: the same history as a PRE-M0 image (bare records,
/// legacy FNV ids, flags 0 -- laid down through the verbatim door) reloads byte for byte,
/// every one of its records checks as legacy, and its journal is the nonced image's journal
/// to the byte. What M0 moves is the record envelope, never the fold.
#[test]
fn m0_a_pre_m0_stock_image_reloads_byte_equal_and_folds_as_the_nonced_one() {
    use crate::chain::{verify_id, IdScheme};
    use bebop_store::evlog::EvLog;
    let new = old_history();
    let bodies: Vec<String> = new.since(&[0u8; 32]).unwrap().iter().map(|r| rec_body(r).to_string()).collect();
    assert!(new.since(&[0u8; 32]).unwrap().iter().all(|r| nonced(r)), "every record this build writes is nonced");
    let mut old = StockLog::create_sized(16 * 1024).unwrap();
    old.carry_raw(&bodies.iter().map(String::as_str).collect::<Vec<_>>()).unwrap();
    let bytes = old.to_bytes_trimmed();
    assert_eq!(StockLog::load(&bytes).unwrap().to_bytes_trimmed(), bytes, "a load and a save move no byte of an old image");
    let schemes = |log: &StockLog| -> Vec<Option<IdScheme>> {
        EvLog::walk(&bebop_store::Store::from_bytes(&log.to_bytes())).iter().map(|r| verify_id(r.flags, &r.prev, &r.payload, &r.id)).collect()
    };
    assert!(schemes(&old).iter().all(|s| *s == Some(IdScheme::LegacyChained)), "every old record is a legacy id");
    assert!(schemes(&new).iter().all(|s| *s == Some(IdScheme::Sha256)), "every new record is sha256");
    assert_eq!(old.len(), new.len());
    assert_eq!(fold_text(&old.journal().unwrap()), fold_text(&new.journal().unwrap()), "the fold does not see the envelope");
    assert_eq!(old.verify_checkpoints().unwrap(), new.verify_checkpoints().unwrap());
}
