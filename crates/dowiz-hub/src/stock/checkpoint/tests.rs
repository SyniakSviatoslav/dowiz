//! `fold(all) == fold(checkpoint + tail)` (R7's determinism gate), over NAMED
//! histories built through the real write doors -- deterministic, seeded,
//! no fuzzer (the repo's rule). Each history mixes every kind of record.

use super::codec::{body, parse};
use super::*;
use crate::stock::journal::{Journal, Undated};
use crate::stock::{reservations_for, settle, PrepStage, Qty, WasteReason};

const DISHES: [&str; 3] = [
    r#"{"id":"maki","bom":[{"supply":"rice","qty":90},{"supply":"nori","qty":2}]}"#,
    r#"{"id":"sake","bom":[{"supply":"rice","qty":60},{"supply":"salmon","qty":40}]}"#,
    r#"{"id":"odd name","bom":[{"supply":"x \"y\":1 - 3:z","qty":5}]}"#,
];
const SUPPLIES: [&str; 5] = ["rice", "nori", "salmon", "x \"y\":1 - 3:z", "fillet"];

struct Rng(u64);
impl Rng {
    fn next(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % n
    }
}

/// One named history: `steps` actions chosen by `seed`, the clock on or off.
fn history(seed: u64, steps: usize, clock: bool, every: usize) -> StockLog {
    let mut log = StockLog::create_sized(16 * 1024).unwrap();
    log.set_checkpoint_every(every);
    let mut r = Rng(seed);
    let mut orders: Vec<String> = Vec::new();
    for step in 0..steps {
        if clock {
            log.set_clock(1_000 + step as i64 * 60_000);
        }
        let item = SUPPLIES[r.next(4) as usize].to_string();
        let qty = 1 + r.next(400) as Qty;
        // Every result is allowed to be a refusal: a refusal writes nothing.
        let _ = match r.next(10) {
            0 | 1 => {
                let priced = r.next(3) > 0;
                let m = Meta {
                    unit_cost: priced.then(|| 500 + r.next(2000) as i64),
                    per: priced.then_some(1000),
                    lot: (r.next(2) == 0).then(|| format!("L{}", r.next(5))),
                    expiry: Some(20261001 + r.next(20) as i64),
                    supplier: Some("Sea & Co".into()),
                    ..Meta::default()
                };
                log.receive_with(&item, qty * 10, &m)
            }
            2 | 3 => {
                let o = format!("o{step}");
                orders.push(o.clone());
                let lines: Vec<(String, i64)> = vec![(DISHES[r.next(3) as usize].into(), 1 + r.next(3) as i64)];
                log.append_all(&reservations_for(&o, &lines))
            }
            4 | 5 if !orders.is_empty() => {
                let o = orders.remove(r.next(orders.len() as u64) as usize);
                let led = log.ledger().unwrap();
                log.append_all(&settle(&led, &o, r.next(3) > 0))
            }
            6 => log.append(&StockEvent::Wasted { item, qty, reason: WasteReason::Spoiled, by: "p".into() }),
            7 => log.append(&StockEvent::Stocktake { item, observed: qty * 5, stocktake_id: format!("s{step}"), by: "p".into() }),
            8 => log.append(&StockEvent::Produced {
                item, qty, out: qty / 2, stage: PrepStage::Clean, into: Some("fillet".into()), by: "p".into(),
            }),
            _ => {
                let o = format!("till{}", r.next(4));
                if r.next(2) == 0 {
                    log.append(&StockEvent::Served { item, qty, order_id: o })
                } else {
                    log.append(&StockEvent::Unserved { item, qty: 1 + qty / 100, order_id: o })
                }
            }
        };
    }
    log
}

/// The whole state, as a byte string: two folds that print the same agree.
fn state(j: &Journal) -> String {
    body(j)
}

const HISTORIES: [(&str, u64, usize, bool); 4] = [
    ("a quiet week", 0x9E37_79B9, 300, true),
    ("a busy service", 0xC0FF_EE11, 900, true),
    ("before clocks existed", 0x5EED_0003, 400, false),
    ("stocktakes and prep", 0xDEAD_BEEF, 600, true),
];

#[test]
fn every_named_history_folds_the_same_through_its_checkpoints() {
    for (name, seed, steps, clock) in HISTORIES {
        let log = history(seed, steps, clock, 7);
        let genesis = log.journal().unwrap();
        assert!(genesis.seen > steps / 3, "{name}: the history is mostly accepted records ({})", genesis.seen);
        assert!(log.verify_checkpoints().unwrap() > 10, "{name}: checkpoints were written and each agrees");
        // THE LAW: the fast shelf and book are the genesis fold's.
        assert_eq!(log.ledger().unwrap(), StockLedger::fold(&log.events()).unwrap(), "{name}: ledger");
        assert_eq!(log.cost_book(), genesis.book, "{name}: book");
        // And the tail really is short: the fold started at a checkpoint.
        assert!(log.tail(|_, _| true).recs.len() <= 7 + 3, "{name}: the fold read only the tail");
        // The report's journal: today's state equal, the rows a suffix.
        let since = if clock { 1_000 + (steps as i64 / 2) * 60_000 } else { i64::MAX };
        let j = log.journal_since(since).unwrap();
        assert_eq!(state(&j), state(&genesis), "{name}: journal_since's state");
        let skip = genesis.entries.len() - j.entries.len();
        assert_eq!(j.entries[..], genesis.entries[skip..], "{name}: the rows are the genesis rows' tail");
        assert_eq!(j.before.left(|_| false) + j.entries.iter().filter(|e| e.meta.at.is_none()
            && !matches!(e.ev, StockEvent::Reserved { .. } | StockEvent::Released { .. })).count(),
            genesis.undated.left(|_| false), "{name}: no undatable row lost or counted twice");
        if clock {
            assert!(skip > 0, "{name}: a clocked history's report skips the rows before its window");
            assert!(j.entries.iter().all(|e| e.meta.at.is_some_and(|a| a >= since - 20 * 60_000)), "{name}");
        } else {
            assert_eq!(skip, 0, "{name}: a checkpoint with no clock never stands in for a window");
        }
    }
}

/// An image written before checkpoints folds exactly as it did, and its FIRST
/// write afterwards lays one down.
#[test]
fn an_old_image_without_checkpoints_folds_the_same_and_gains_one() {
    let old = history(0x0123_4567, 300, true, usize::MAX);
    assert_eq!(old.verify_checkpoints(), Ok(0), "none written");
    let mut log = StockLog::load(&old.to_bytes_trimmed()).unwrap();
    // A venue whose history is already longer than the cadence.
    log.set_checkpoint_every(100);
    assert!(log.len() > 100);
    assert_eq!(log.ledger().unwrap(), StockLedger::fold(&log.events()).unwrap());
    assert_eq!(log.tail(|_, _| true).recs.len(), log.len(), "no checkpoint: the whole log is the tail");
    let n = log.len();
    log.append(&StockEvent::Received { item: "rice".into(), qty: 5 }).unwrap();
    assert_eq!(log.len(), n + 2, "the record and the checkpoint the long tail was owed");
    assert_eq!(log.tail(|_, _| true).recs.len(), 0);
    assert_eq!(log.verify_checkpoints(), Ok(1));
    assert_eq!(log.ledger().unwrap(), StockLedger::fold(&log.events()).unwrap());
}

/// A checkpoint that does not parse is walked past; one that parses and lies
/// is caught by `verify_checkpoints`.
#[test]
fn a_broken_checkpoint_is_skipped_and_a_lying_one_is_caught() {
    let mut log = history(0x00C0_FFEE, 120, true, usize::MAX);
    let truth = log.ledger().unwrap();
    log.write_payload(format!("{HEAD}}}\n L 1 nonsense").into_bytes()).unwrap();
    assert_eq!(log.ledger().unwrap(), truth, "an unreadable checkpoint is not a state");
    assert_eq!(log.tail(|_, _| true).recs.len(), log.len(), "and the walk went past it");
    // A parseable checkpoint of the WRONG state: the fold would trust it...
    log.write_payload(format!("{HEAD}}}\n{}", body(&Journal::default())).into_bytes()).unwrap();
    assert_ne!(log.ledger().unwrap(), truth);
    // ...and law 8 names it.
    assert!(log.verify_checkpoints().unwrap_err().contains("disagrees"));
    // Twin: a true one verifies.
    let mut good = history(0x00C0_FFEE, 120, true, usize::MAX);
    good.checkpoint_now().unwrap();
    assert_eq!(good.verify_checkpoints(), Ok(1));
    assert_eq!(good.ledger().unwrap(), truth);
}

/// Names with every character the codec has to survive, and every list.
#[test]
fn the_codec_round_trips_hostile_names_and_every_list() {
    let log = history(0xFEED_F00D, 500, true, usize::MAX);
    let j = log.journal().unwrap();
    assert!(j.ledger.levels.iter().any(|(i, _)| i.contains('"') && i.contains(' ')), "a hostile name is in it");
    assert!(!j.lots.lots.is_empty() && !j.ledger.served.is_empty() && !j.book.pools.is_empty());
    let text = format!("{HEAD},\"at\":7}}\n{}", body(&j));
    let (back, at) = parse(text.as_bytes()).expect("parses");
    assert_eq!((state(&back), at), (state(&j), Some(7)));
    assert_eq!((back.ledger, back.book, back.lots), (j.ledger.clone(), j.book.clone(), j.lots.clone()));
    assert_eq!(back.before, j.undated, "the carried rows are the checkpoint's own");
    // Refusals, each beside the text above that parses: a truncated body, a
    // trailing token, a record that is not a checkpoint at all.
    assert!(parse(&text.as_bytes()[..text.len() - 1]).is_none());
    assert!(parse(format!("{text} 1").as_bytes()).is_none());
    assert!(parse(br#"{"k":"received","item":"rice","qty":1}"#).is_none());
    assert_eq!(decode(&text), None, "a checkpoint never folds as an event");
}

/// A grown image is a rewritten one: it gets a checkpoint with its write.
#[test]
fn a_rewritten_image_writes_a_checkpoint() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_checkpoint_every(1_000_000);
    let cap = log.to_bytes().len();
    let mut i = 0;
    while log.to_bytes().len() == cap {
        assert_eq!(log.verify_checkpoints(), Ok(0), "none before the image is rewritten");
        log.append(&StockEvent::Received { item: format!("item{i}"), qty: 1 }).unwrap();
        i += 1;
    }
    assert_eq!(log.verify_checkpoints(), Ok(1), "the grow wrote one, and it agrees");
    assert!(!log.grew, "and the flag was spent");
}

/// The rows a checkpoint hides from a report are CARRIED, not dropped: the
/// ones no clock dated, and per order the ones only a placement can date.
#[test]
fn undatable_rows_behind_a_checkpoint_are_carried_to_the_report() {
    let mut log = StockLog::create_sized(16 * 1024).unwrap();
    log.set_checkpoint_every(4);
    // Before clocks: two receipts (never datable) and an order's draw.
    log.append(&StockEvent::Received { item: "rice".into(), qty: 500 }).unwrap();
    log.append(&StockEvent::Received { item: "nori".into(), qty: 50 }).unwrap();
    log.append_all(&reservations_for("o1", &[(DISHES[0].into(), 1)])).unwrap();
    let led = log.ledger().unwrap();
    log.append_all(&settle(&led, "o1", true)).unwrap();
    log.set_clock(10_000);
    for _ in 0..4 {
        log.append(&StockEvent::Received { item: "rice".into(), qty: 1 }).unwrap();
    }
    let j = log.journal_since(50_000).unwrap();
    assert!(j.entries.iter().all(|e| e.meta.at.is_some()), "the report reads only rows after the checkpoint");
    assert_eq!(j.before.plain, 2, "the two unclocked receipts");
    assert_eq!(j.before.by_order, vec![("o1".to_string(), 2)], "o1's two draws; its holds are not movements");
    assert_eq!(j.before.left(|o| o == "o1"), 2, "a reader who knows o1's placement dates its draws");
    assert_eq!(j.before.left(|_| false), 4);
    assert_eq!(log.journal().unwrap().before, Undated::default(), "a fold from genesis carries nothing");
    // Twin: a window that starts BEFORE the checkpoint cannot use it.
    assert_eq!(log.journal_since(10_000).unwrap().entries.len(), log.journal().unwrap().entries.len());
}

/// R4's door: the book it answers is the one the shelf was decided against.
#[test]
fn the_costed_door_answers_the_book_of_the_same_fold() {
    let mut log = history(0x1357_9BDF, 200, true, 7);
    let (before, at) = (log.cost_book(), log.len());
    let evs = reservations_for("new", &[(DISHES[0].into(), 1)]);
    let (book, stamped_at) = log.append_all_costed(&evs).unwrap();
    assert_eq!((book, stamped_at), (before, at));
    // Twin: refused, nothing is written and nothing is answered.
    let mut empty = StockLog::create_sized(16 * 1024).unwrap();
    empty.append(&StockEvent::Received { item: "rice".into(), qty: 1 }).unwrap();
    assert!(empty.append_all_costed(&reservations_for("o", &[(DISHES[0].into(), 1)])).is_err());
    assert_eq!(empty.len(), 1);
}

/// MEASURED, not asserted: `ledger()` over 5k and 50k records, from genesis
/// and through checkpoints. `cargo test --release --lib measure_ -- --ignored --nocapture`.
#[test]
#[ignore]
fn measure_ledger_fold() {
    for n in [5_000usize, 50_000] {
        for every in [usize::MAX, CHECKPOINT_EVERY] {
            let log = service(n, every);
            let t = std::time::Instant::now();
            let runs = 20;
            for _ in 0..runs {
                std::hint::black_box(log.ledger().unwrap());
            }
            let per = t.elapsed().as_micros() / runs;
            let t = std::time::Instant::now();
            std::hint::black_box(log.journal_since(i64::MAX).unwrap());
            let jr = t.elapsed().as_micros();
            println!(
                "records={n} every={} image={}B ledger()={per}us journal_since={jr}us tail={}",
                if every == usize::MAX { "none".into() } else { every.to_string() },
                log.to_bytes_trimmed().len(),
                log.tail(|_, _| true).recs.len()
            );
        }
    }
}

/// A service-shaped log: priced receipts, then orders that reserve and are
/// consumed, laid down through `write_payload` (no decide: the shape is known
/// valid), with a checkpoint every `every` records as `commit` would write.
fn service(n: usize, every: usize) -> StockLog {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_checkpoint_every(every);
    let items = ["rice", "nori", "salmon", "tuna", "avocado", "cucumber"];
    let (mut written, mut since, mut k) = (0usize, 0usize, 0usize);
    while written < n {
        log.set_clock(1_790_000_000_000 + k as i64 * 60_000);
        let evs: Vec<(StockEvent, Meta)> = if k % 40 == 0 {
            // What the next forty orders draw of each (800), and a little
            // more: old deliveries run out first-in-first-out, as in a kitchen.
            items.iter().map(|i| (StockEvent::Received { item: (*i).into(), qty: 900 },
                Meta { unit_cost: Some(1500), per: Some(1000), supplier: Some("Sea".into()), ..Meta::default() })).collect()
        } else {
            let o = format!("ord_{k}");
            let three: Vec<&str> = (0..3).map(|j| items[(k + j) % items.len()]).collect();
            let mut v: Vec<(StockEvent, Meta)> = three.iter().map(|i| (StockEvent::Reserved { item: (*i).into(), qty: 40, order_id: o.clone() }, Meta::default())).collect();
            v.extend(three.iter().map(|i| (StockEvent::Consumed { item: (*i).into(), qty: 40, order_id: o.clone() }, Meta::default())));
            v
        };
        for (ev, m) in &evs {
            let m = log.stamped(m);
            log.write_payload(with_meta(&encode(ev), &m).into_bytes()).unwrap();
        }
        written += evs.len();
        since += evs.len();
        if since >= every {
            log.checkpoint_now().unwrap();
            since = 0;
        }
        k += 1;
    }
    log
}
