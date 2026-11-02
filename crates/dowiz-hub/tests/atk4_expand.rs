//! W-MG-ATK4, ATTACK 1, closed by W-MG-TRANS: THE EXPAND RELEASE. At `SHA256_VENUES = ""` every door
//! writes what prod ac5ff0c4 writes: ONE flagged record on an unflipped log is the failure, because
//! rollback.sh applies no floor before a flip and the old build misreads that record. Each test was
//! RED on the tree the attack ran on; the HOLE numbers are the attack's verdict's.

use einstore::evlog::{EvLog, FLAG_SHA256};
use einstore::Store;
use dowiz_hub::catalog::edits;
use dowiz_hub::consent::{self, Act, Method, State};
use dowiz_hub::glue::{merge::merge, tablet};
use dowiz_hub::logimage::LogImage;
use dowiz_hub::stock::{StockEvent, StockLog, WasteReason};
use dowiz_hub::{EventKind, Hub};

const ACTOR: [u8; 32] = [0xA1; 32];
const KEY: &str = "a1b2c3d4e5f60718";

fn flagged(image: &[u8]) -> usize {
    EvLog::walk(&Store::from_bytes(image)).iter().filter(|r| r.flags & FLAG_SHA256 != 0).count()
}

fn grant() -> Act {
    Act {
        key: KEY.into(),
        purpose: consent::PURPOSE_MARKETING.into(),
        channel: consent::CHANNEL_WHATSAPP.into(),
        state: State::Given,
        at_ms: 10,
        method: Method::OwnerEntered,
        evidence: "paper form, Arben, +355691111111".into(),
        wording_id: consent::wording_id("en"),
        via: "owner_7".into(),
    }
}

fn received(item: &str, qty: i64) -> StockEvent {
    StockEvent::Received { item: item.into(), qty }
}

fn off_stock() -> StockLog {
    let mut s = StockLog::create_sized(64 * 1024).unwrap();
    s.set_sha256(false);
    s.set_clock(1000);
    s
}

/// HOLE 1. `hubdo/forget.rs` arms the consent log, then `consent::forget::redact_acts` does
/// `*log = LogImage::load(..)`: the armed switch goes with the old value, the crate default
/// (sha256) comes back, and the withdrawals an erasure appends are sha256 on an unflipped venue.
/// The old build reads those as junk: the erased person's marketing consent reads GIVEN.
#[test]
fn an_erasure_on_an_unflipped_venue_stays_legacy() {
    let mut c = LogImage::create_sized(64 * 1024).unwrap();
    c.set_sha256(false);
    consent::log::write(&mut c, &grant()).unwrap();
    assert_eq!(flagged(&c.to_bytes()), 0, "control: the grant is legacy");
    let f = consent::forget::forget(&mut c, KEY, 99).unwrap();
    assert!(f.redacted > 0 && f.withdrawn > 0, "control: the erasure did something: {f:?}");
    assert_eq!(flagged(&c.to_bytes()), 0, "HOLE: sha256 records on an unflipped consent log after an erasure");
}

/// Twin: no redaction, so no reload, so the switch survives: the reload is the whole cause.
#[test]
fn twin_withdrawals_with_nothing_to_redact_stay_legacy() {
    let mut c = LogImage::create_sized(64 * 1024).unwrap();
    c.set_sha256(false);
    consent::forget::forget(&mut c, KEY, 99).unwrap();
    assert_eq!(flagged(&c.to_bytes()), 0);
}

/// HOLE 2 (the moment tablet.wasm is rebuilt: `glue::tablet::open` never hears the switch).
/// `adopt_from` writes a nonced record verbatim, so a tablet flips an unflipped stock log.
#[test]
fn a_tablet_cannot_flip_an_unflipped_hub() {
    let mut hub = off_stock();
    hub.append(&received("salmon", 100)).unwrap();
    let base = hub.tip_id();
    let (img, _) = tablet::record(&hub.to_bytes_trimmed(), "kitchen", r#"{"k":"wasted","item":"salmon","qty":1,"reason":"spoiled","by":"ana"}"#, 2000).unwrap();
    let t = tablet::tail(&img, "kitchen", &base).unwrap();
    assert!(!dowiz_hub::stock::nonced(&t.recs[0]), "the tablet writes its log's scheme: {}", t.recs[0]);
    let bytes = hub.to_bytes_trimmed();
    let r = merge(&bytes, &mut hub, &t).unwrap();
    assert_eq!(r.merged, 1, "control: {r:?}");
    assert_eq!(flagged(&hub.to_bytes_trimmed()), 0, "HOLE: sha256 records on an unflipped stock log, written by a tablet");
    assert!(!hub.writes_sha256(), "HOLE: and the venue's stock log is flipped for good");
}

/// H8, the hub's half: a crafted nonced record does not flip a legacy log either.
#[test]
fn a_nonced_record_on_a_legacy_hub_goes_in_bare() {
    let mut hub = off_stock();
    hub.append(&received("salmon", 100)).unwrap();
    let base = hub.tip_id();
    let mut armed = StockLog::load(&hub.to_bytes_trimmed()).unwrap();
    armed.set_clock(2000);
    armed.append(&received("salmon", 1)).unwrap();
    let t = dowiz_hub::glue::wire::Tail { ctx: "kitchen".into(), base, tip: armed.tip_id(), recs: armed.since(&base).unwrap() };
    assert!(dowiz_hub::stock::nonced(&t.recs[0]), "control: the tail is nonced");
    let bytes = hub.to_bytes_trimmed();
    let r = merge(&bytes, &mut hub, &t).unwrap();
    assert_eq!(r.merged, 1, "{r:?}");
    assert_eq!(flagged(&hub.to_bytes_trimmed()), 0);
}

/// Twin: what the tablet in prod sends today (bare JSON, FNV chain) stays legacy on the hub.
#[test]
fn twin_a_bare_tail_stays_legacy() {
    let mut hub = off_stock();
    hub.append(&received("salmon", 100)).unwrap();
    let base = hub.tip_id();
    let mut bare = StockLog::load(&hub.to_bytes_trimmed()).unwrap();
    bare.set_sha256(false);
    bare.set_clock(2000);
    bare.append(&StockEvent::Wasted { item: "salmon".into(), qty: 1, reason: WasteReason::Spoiled, by: "ana".into() }).unwrap();
    let t = dowiz_hub::glue::wire::Tail { ctx: "kitchen".into(), base, tip: bare.tip_id(), recs: bare.since(&base).unwrap() };
    let bytes = hub.to_bytes_trimmed();
    let r = merge(&bytes, &mut hub, &t).unwrap();
    assert_eq!(r.merged, 1, "control: {r:?}");
    assert_eq!(flagged(&hub.to_bytes_trimmed()), 0);
}

/// HOLE 1b. The catalogue's edit journal compacts into `LogImage::create()` (`edits/history.rs`):
/// default entropy, sha256 on, no seed. An unflipped venue's journal is rewritten whole as sha256
/// the first time it passes its budget, and a menu write is all it takes.
#[test]
fn a_compacted_catalogue_journal_stays_legacy() {
    let mut log = LogImage::create_sized(64 * 1024).unwrap();
    log.set_sha256(false);
    let (mut before, mut done) = (edits::State::new(), false);
    for i in 0..20_000i64 {
        let mut after = before.clone();
        after.insert("product/d1".to_string(), format!(r#"{{"price":{i}}}"#));
        let j = edits::journal(&mut log, &before, &after, i, "owner", (i, i + 1)).unwrap();
        before = after;
        if j.compacted {
            done = true;
            break;
        }
        assert_eq!(flagged(&log.to_bytes()), 0, "control: legacy until the compaction");
    }
    assert!(done, "control: the journal compacted");
    assert_eq!(flagged(&log.to_bytes()), 0, "HOLE: the compaction rewrote an unflipped journal as sha256");
    assert!(!log.writes_sha256(), "HOLE: and the journal is flipped for good");
}

/// Every other door, each with the switch off: the ones that held.
#[test]
fn twin_every_other_door_writes_legacy() {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    h.set_sha256(false);
    for i in 0..600u64 {
        h.append(EventKind::Placed, &format!("o{i}"), r#"{"status":"PENDING","phone":"+355691111111"}"#, i + 1, ACTOR).unwrap();
    }
    assert_eq!(flagged(&h.to_bytes_trimmed()), 0, "appends, past a grow");
    h.rotate(|id| id == "o599").unwrap();
    assert_eq!(flagged(&h.to_bytes_trimmed()), 0, "the rotation's checkpoint");
    h.redact(|e| (e.order_id == "o599").then(|| "{}".to_string())).unwrap();
    assert_eq!(flagged(&h.to_bytes_trimmed()), 0, "a redaction");

    let mut l = LogImage::create_sized(64 * 1024).unwrap();
    l.set_sha256(false);
    (0..5).for_each(|i| l.append("x", "s", &format!("{{\"i\":{i}}}")).unwrap());
    l.keep(2).unwrap();
    assert_eq!(flagged(&l.to_bytes()), 0, "LogImage::keep");

    let mut s = off_stock();
    s.append(&received("salmon", 100)).unwrap();
    s.append_note("glued", r#"{"ctx":"kitchen"}"#).unwrap();
    s.checkpoint_now().unwrap();
    s.grant("kitchen", "salmon", 5, None).unwrap();
    (0..900).for_each(|i| s.append(&received(&format!("i{}", i % 7), 1)).unwrap());
    assert_eq!(flagged(&s.to_bytes_trimmed()), 0, "stock movement, note, checkpoint (explicit and due), grant, grow");
}
