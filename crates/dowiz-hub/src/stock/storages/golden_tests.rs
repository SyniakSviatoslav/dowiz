//! W-STORE2's GOLDEN: a venue that never binds a station draws -- through
//! `draws_for`, `append_draws`, the till's `served` and `settle` -- with
//! dishes that DO carry a `"station"`, and its image is the very bytes the
//! code BEFORE W-STORE2 wrote. The digest was taken by running this same file
//! (public paths only) on the lane's base snapshot (40d5dd8d + W-VOICE).

use crate::stock::meta::Meta;
use crate::stock::{draws_for, settle, StockLog};
use sha2::{Digest, Sha256};

const SUSHI: &str = r#"{"id":"roll","station":"sushi","bom":[{"supply":"salmon","qty":100},{"supply":"rice","qty":50}]}"#;
const PLATE: &str = r#"{"id":"plate","bom":[{"supply":"salmon","qty":50},{"supply":"salt","uq":300000}]}"#;
const DRINK: &str = r#"{"id":"spritz","station":"bar","bom":[{"supply":"lemon","qty":1}]}"#;

fn history() -> StockLog {
    let mut log = StockLog::create_sized(16 * 1024).unwrap();
    log.set_checkpoint_every(5);
    for step in 0..30i64 {
        log.set_clock(1_759_000_000_000 + step * 600_000);
        if step % 6 == 0 {
            let m = Meta { store: (step % 12 == 0).then(|| "freezer".to_string()), ..Meta::default() };
            log.receive_with("salmon", 2000, &m).unwrap();
            log.receive_with("rice", 1000, &Meta::default()).unwrap();
            log.receive_with("lemon", 20, &Meta::default()).unwrap();
            log.receive_with("salt", 50, &Meta::default()).unwrap();
        }
        let order = format!("o{step}");
        let lines = vec![(SUSHI.to_string(), 1 + step % 2), (PLATE.to_string(), 1), (DRINK.to_string(), step % 3)];
        if step % 5 == 4 {
            log.append_served_draws(&draws_for(&order, &lines)).unwrap();
            continue;
        }
        log.append_draws(&draws_for(&order, &lines)).unwrap();
        let led = log.ledger().unwrap();
        log.append_all(&settle(&led, &order, step % 7 != 0)).unwrap();
    }
    log
}

// Re-derived at the W-STORE2 merge (2026-10-06): W-CRC counts superseded cells in the superblock,
// and a dump of this history under the pre-W-CRC code differs in 12 cells, ALL inside the two
// superblock pages (7, 8, 15, 17, 25, 26, 519, 520, 527, 529, 537, 538); the arena is byte-equal.
// The pre-W-CRC value, which the unbound pre-STORE2 code also printed, was f76fa117...d8fc.
const IMAGE_SHA: &str = "c7544181e16d25f45205bc0b4aa0665231cc63e5555c1fa44475c7f9e350a9ad";

#[test]
fn an_unbound_venue_writes_the_pre_store2_bytes() {
    let log = history();
    let image: String = Sha256::digest(log.to_bytes_trimmed()).iter().map(|x| format!("{x:02x}")).collect();
    println!("GOLDEN STORE2 IMAGE_SHA={image} records={}", log.len());
    assert!(log.verify_checkpoints().unwrap() > 5);
    assert_eq!(image, IMAGE_SHA, "an unbound venue's image changed by one byte or more");
}
