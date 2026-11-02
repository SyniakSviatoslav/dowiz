//! W-MG-ATK4, ATTACK 2, closed by W-MG-TRANS: THE FLIP IS ONE-WAY IN THE BYTES AND A ROTATION MOVES
//! THE BYTES. A rotated image opened with a new checkpoint under the log's scheme, then the kept
//! records verbatim: on a flipped log, legacy records NEWER than a sha256 one, `walk_check`'s forgery
//! shape, and `writes_sha256` reads only the newest record. A rotation in transition now writes it last.

use einstore::evlog::{EvLog, FLAG_SHA256};
use einstore::Store;
use dowiz_hub::{EventKind, Hub};

const ACTOR: [u8; 32] = [0xA1; 32];
const JSON: &str = r#"{"status":"PENDING","subtotal":1800}"#;

fn orders(h: &mut Hub, from: u64, to: u64) {
    (from..to).for_each(|i| drop(h.append(EventKind::Placed, &format!("o{i}"), JSON, i + 1, ACTOR).unwrap()));
}

/// `legacy` old records, then the flip, then `sha` new ones.
fn flipped(legacy: u64, sha: u64) -> Hub {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    h.set_sha256(false);
    orders(&mut h, 0, legacy);
    h.set_entropy([7; 32]);
    h.set_sha256(true);
    orders(&mut h, legacy, legacy + sha);
    h
}

/// HOLE 3. The first night after a flip: every open order and the whole audit trail carried past
/// the checkpoint turns `broken` for good, and nothing was edited.
#[test]
fn the_first_rotation_after_a_flip_breaks_no_record() {
    let mut h = flipped(3, 2);
    let c = h.chain_check();
    assert_eq!((c.chained, c.legacy, c.broken), (2, 3, 0), "control: the flip itself is clean: {c:?}");
    h.rotate(|_| true).unwrap();
    let c = h.chain_check();
    assert_eq!(c.broken, 0, "HOLE: a rotation made honest legacy records broken: {c:?}");
}

/// HOLE 3b, the quiet venue: flipped in the config, no order since, the night rotates first.
#[test]
fn a_quiet_venue_rotated_right_after_the_flip_breaks_no_record() {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    h.set_sha256(false);
    orders(&mut h, 0, 3);
    h.set_sha256(true); // the var now names the venue; not one order yet
    h.rotate(|_| true).unwrap();
    let c = h.chain_check();
    assert_eq!(c.broken, 0, "HOLE: {c:?}");
}

/// HOLE 4. Rollback to the floor (the var is "" again), a night rotation drops the newest order
/// (a delivered one) and keeps a legacy one: the newest record is legacy, so the log writes FNV
/// again, under a sha256 checkpoint. The doc's "one-way in the bytes" holds until the first night.
#[test]
fn one_way_survives_a_rotation() {
    let mut h = flipped(3, 2);
    h.set_sha256(false); // rolled back to the floor
    assert!(h.writes_sha256(), "control: one-way before the rotation");
    h.rotate(|id| id == "o1").unwrap(); // o1 is legacy and open; o3, o4 are done
    assert!(h.writes_sha256(), "HOLE: the rotation walked a flipped log back to legacy");
    orders(&mut h, 5, 6);
    let c = h.chain_check();
    assert_eq!(c.broken, 0, "HOLE: {c:?}");
}

/// Twin: an unflipped log rotates clean and stays legacy (what the expand release does).
#[test]
fn twin_an_unflipped_log_rotates_clean() {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    h.set_sha256(false);
    orders(&mut h, 0, 5);
    h.rotate(|_| true).unwrap();
    let c = h.chain_check();
    assert_eq!((c.broken, c.chained), (0, 0), "{c:?}");
    assert_eq!(EvLog::walk(&Store::from_bytes(&h.to_bytes_trimmed())).iter().filter(|r| r.flags & FLAG_SHA256 != 0).count(), 0);
}

/// Twin: a log wholly on sha256 rotates clean, and a second night too.
#[test]
fn twin_a_wholly_sha256_log_rotates_clean_twice() {
    let mut h = flipped(0, 4);
    h.rotate(|_| true).unwrap();
    orders(&mut h, 4, 6);
    h.rotate(|_| true).unwrap();
    assert_eq!(h.chain_check().broken, 0, "{:?}", h.chain_check());
}

/// Held: two isolates, one armed and one not, take turns on one log: the bytes decide.
#[test]
fn held_two_isolates_one_armed_one_not() {
    let mut a = flipped(2, 1);
    let mut b = Hub::load(&a.to_bytes_trimmed()).unwrap();
    b.set_sha256(false);
    orders(&mut b, 10, 11);
    a = Hub::load(&b.to_bytes_trimmed()).unwrap();
    a.set_sha256(true);
    orders(&mut a, 11, 12);
    let c = a.chain_check();
    assert_eq!((c.chained, c.legacy, c.broken), (3, 2, 0), "{c:?}");
}
