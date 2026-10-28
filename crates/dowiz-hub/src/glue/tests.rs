//! SHEAF S0 pass mark (R-BEBOPDB G.5): finds the X5 conflict; 0 false positives on 1000
//! conflict-free merges -- a FIXED grid, every case named by its (i, j, k), no generator, no
//! seed (memory `no-fuzzers-in-this-repo`); named tests on a corrupted tip; the 2000+20+20
//! measurement (`measure_glue_2000_20_20`, `--ignored`, release).

use super::*;
use crate::stock::{StockEvent, StockLedger, StockLog, WasteReason};
use bebop_store::{crc32_cells, follow_in, root_in, View};
use std::sync::OnceLock;

fn res(item: &str, qty: i64, o: &str) -> StockEvent {
    StockEvent::Reserved { item: item.into(), qty, order_id: o.into() }
}
fn con(item: &str, qty: i64, o: &str) -> StockEvent {
    StockEvent::Consumed { item: item.into(), qty, order_id: o.into() }
}
fn rel(item: &str, qty: i64, o: &str) -> StockEvent {
    StockEvent::Released { item: item.into(), qty, order_id: o.into() }
}
fn rec(item: &str, qty: i64) -> StockEvent {
    StockEvent::Received { item: item.into(), qty }
}

/// `bytes` with `evs` appended, one commit per slice (an order is one commit).
fn branch(bytes: &[u8], commits: &[Vec<StockEvent>]) -> Vec<u8> {
    let mut log = StockLog::load(bytes).unwrap();
    for c in commits {
        log.append_all(c).unwrap();
    }
    log.to_bytes_trimmed()
}

const ITEMS: [&str; 8] = ["salmon", "tuna", "rice", "nori", "avocado", "eel", "shrimp", "cheese"];
/// The X5 shelf: salmon at 17 when the tablets go offline.
const SALMON: i64 = 17;

/// X5's base: 2000 stock records (orders that reserve and consume one portion of the seven
/// other items, in turn) over 8 counted items, checkpoints every 500 as the write door makes.
fn x5_base() -> &'static Vec<u8> {
    static B: OnceLock<Vec<u8>> = OnceLock::new();
    B.get_or_init(|| {
        let mut log = StockLog::create_sized(64 * 1024).unwrap();
        log.append_all(&ITEMS.iter().map(|i| rec(i, if *i == "salmon" { SALMON } else { 5000 })).collect::<Vec<_>>()).unwrap();
        let mut k = 0;
        while log.len() < 2000 {
            let (it, o) = (ITEMS[1 + k % 7], format!("base-{k}"));
            log.append_all(&[res(it, 1, &o), con(it, 1, &o)]).unwrap();
            k += 1;
        }
        log.to_bytes_trimmed()
    })
}

/// X5's two tails, 20 records each: the hub sells 16 salmon, the kitchen 2 (17 on the shelf).
fn x5_pair() -> (Vec<u8>, Vec<u8>) {
    let tail = |side: &str, salmon: i64| -> Vec<Vec<StockEvent>> {
        let mut c = vec![vec![res("salmon", salmon, &format!("{side}-s")), con("salmon", salmon, &format!("{side}-s"))]];
        c.extend((0..9).map(|i| {
            let (it, o) = (ITEMS[1 + i % 7], format!("{side}-{i}"));
            vec![res(it, 1, &o), con(it, 1, &o)]
        }));
        c
    };
    (branch(x5_base(), &tail("h", SALMON - 1)), branch(x5_base(), &tail("k", 2)))
}

#[test]
fn x5_the_last_salmon_sold_on_two_tablets_is_found() {
    let (h, k) = x5_pair();
    let g = glue(&h, &k, None).unwrap();
    assert!(g.hub_tail >= 20 && g.kitchen_tail >= 20, "{} {}", g.hub_tail, g.kitchen_tail);
    assert_eq!(g.state(), State::Obstructed);
    let salmon: Vec<&Conflict> = g.conflicts.iter().filter(|c| matches!(c, Conflict::Oversold { item, .. } if item == "salmon")).collect();
    assert_eq!(g.conflicts.len(), 1, "only salmon: {:?}", g.conflicts);
    match salmon[0] {
        Conflict::Oversold { merged, hub, kitchen, .. } => {
            assert_eq!(merged.on_hand, -1, "17 - 16 - 2");
            assert_eq!((hub.on_hand, kitchen.on_hand), (1, 15), "each branch alone kept the floor");
        }
        c => panic!("{c:?}"),
    }
    // The ancestor is the base's tip: the overlap's digest is one id.
    let base_tip = walk::to_cells(&bebop_store::evlog::EvLog::tip(&bebop_store::Store::from_bytes(x5_base())).unwrap());
    assert_eq!(g.ancestor, base_tip);
    assert!(g.wire_bytes > 0 && g.wire_bytes < 8 * 1024, "{}", g.wire_bytes);
}

#[test]
fn gluing_is_symmetric_on_the_x5_pair() {
    let (h, k) = x5_pair();
    let (a, b) = (glue(&h, &k, None).unwrap(), glue(&k, &h, None).unwrap());
    assert_eq!(a.ancestor, b.ancestor);
    assert_eq!((a.hub_tail, a.kitchen_tail), (b.kitchen_tail, b.hub_tail));
    let names = |g: &Glued| g.conflicts.iter().map(|c| format!("{c:?}").split(',').next().unwrap().to_string()).collect::<Vec<_>>();
    assert_eq!(names(&a), names(&b));
    assert_eq!(a.shared.iter().map(|x| x.1).collect::<Vec<_>>(), b.shared.iter().map(|x| x.1).collect::<Vec<_>>());
}

#[test]
fn the_same_image_is_same_and_one_quiet_branch_fast_forwards() {
    let (h, _) = x5_pair();
    assert_eq!(glue(&h, &h, None).unwrap().state(), State::Same);
    let g = glue(&h, x5_base(), None).unwrap();
    assert_eq!(g.state(), State::FastForward { ahead: Side::Hub });
    assert!(g.conflicts.is_empty());
    assert_eq!(glue(x5_base(), &h, None).unwrap().state(), State::FastForward { ahead: Side::Kitchen });
}

// ── the 1000 conflict-free merges ──────────────────────────────────────────

/// A small shelf for the grid: three counted items, a few orders behind them.
fn grid_base() -> &'static Vec<u8> {
    static B: OnceLock<Vec<u8>> = OnceLock::new();
    B.get_or_init(|| {
        let mut log = StockLog::create_sized(64 * 1024).unwrap();
        log.append_all(&[rec("a", 60), rec("b", 70), rec("c", 80)]).unwrap();
        for k in 0..6 {
            let (it, o) = (["a", "b", "c"][k % 3], format!("g-{k}"));
            log.append_all(&[res(it, 1, &o), con(it, 1, &o)]).unwrap();
        }
        log.append_all(&[res("c", 2, "open-1")]).unwrap();
        log.to_bytes_trimmed()
    })
}

/// Case (i, j, k)'s tail for one side: `n` orders of 1..3 portions over the items in turn from
/// `k`, each consumed, released or left open by its index; a delivery when `k % 4 == 0`, a
/// signed write-off when `k % 5 == 0`. An item gets every third order: at most 3 orders x 3
/// portions + 1 written off = 10 per side, 20 together, against 58, 68 and 76 available --
/// conflict-free by construction.
fn grid_tail(side: &str, n: usize, k: usize) -> Vec<Vec<StockEvent>> {
    let mut c = Vec::new();
    for m in 0..n {
        let (it, q, o) = (["a", "b", "c"][(k + m) % 3], 1 + ((k + m) % 3) as i64, format!("{side}-{k}-{m}"));
        let mut order = vec![res(it, q, &o)];
        match m % 3 {
            0 => order.push(con(it, q, &o)),
            1 => order.push(rel(it, q, &o)),
            _ => {}
        }
        c.push(order);
    }
    if k % 4 == 0 {
        c.push(vec![rec(["a", "b", "c"][k % 3], 5)]);
    }
    if k % 5 == 0 {
        c.push(vec![StockEvent::Wasted { item: ["a", "b", "c"][(k + 1) % 3].into(), qty: 1, reason: WasteReason::Spoiled, by: format!("{side}-cook") }]);
    }
    c
}

#[test]
fn a_thousand_conflict_free_merges_report_nothing_and_glue_to_the_real_fold() {
    let base = grid_base();
    let base_events = StockLog::load(base).unwrap().events().len();
    let mut commuted = 0;
    for i in 0..10 {
        for j in 0..10 {
            for k in 0..10 {
                let (h, kt) = (branch(base, &grid_tail("h", i, k)), branch(base, &grid_tail("k", j, k + 1)));
                let g = glue(&h, &kt, None).unwrap_or_else(|r| panic!("case ({i},{j},{k}) refused: {r:?}"));
                assert!(g.conflicts.is_empty(), "case ({i},{j},{k}) false positive: {:?}", g.conflicts);
                // THE GLUED SECTION IS THE FOLD OF BOTH TAILS: the delta arithmetic agrees with
                // `StockLedger::fold` over base + hub tail + kitchen tail on every shared item.
                let mut all = StockLog::load(&h).unwrap().events();
                all.extend(StockLog::load(&kt).unwrap().events().into_iter().skip(base_events));
                let real = StockLedger::fold(&all).unwrap_or_else(|e| panic!("case ({i},{j},{k}) real fold refused: {e}"));
                for (item, lvl) in &g.shared {
                    assert_eq!(real.level(item), *lvl, "case ({i},{j},{k}) item {item}");
                }
                commuted += (g.state() == State::Commutes) as usize;
            }
        }
    }
    // Both tails non-empty in every case with i, j >= 1 or a delivery/write-off: most commute.
    assert!(commuted >= 800, "{commuted}");
}

#[test]
fn a_hundred_named_oversells_are_all_found() {
    let base = grid_base();
    for t in 0..100usize {
        // Shelf "a" has 58 available; the hub draws x, the kitchen 59 - x + t % 3 (> 58 together).
        let x = 3 + (t % 50) as i64;
        let y = 59 - x + (t % 3) as i64;
        let h = branch(base, &[vec![res("a", x, "ho"), con("a", x, "ho")]]);
        let k = branch(base, &[vec![res("a", y, "ko")]]);
        let g = glue(&h, &k, None).unwrap();
        assert!(
            matches!(g.conflicts.as_slice(), [Conflict::Oversold { item, .. }] if item == "a"),
            "case {t} (x={x}, y={y}): {:?}",
            g.conflicts
        );
    }
}

// ── the other two obstructions ─────────────────────────────────────────────

#[test]
fn a_count_on_one_tablet_against_a_sale_on_the_other_asks_for_a_recount() {
    let base = grid_base();
    let h = branch(base, &[vec![StockEvent::Stocktake { item: "b".into(), observed: 30, stocktake_id: "st1".into(), by: "chef".into() }]]);
    let k = branch(base, &[vec![res("b", 2, "ko"), con("b", 2, "ko")]]);
    let g = glue(&h, &k, None).unwrap();
    assert_eq!(g.conflicts, vec![Conflict::Recount { item: "b".into(), absolute_in: Side::Hub }]);
}

#[test]
fn a_hold_released_on_the_hub_and_consumed_in_the_kitchen_is_taken_twice() {
    let base = grid_base();
    let h = branch(base, &[vec![rel("c", 2, "open-1")]]);
    let k = branch(base, &[vec![con("c", 2, "open-1")]]);
    let g = glue(&h, &k, None).unwrap();
    assert!(
        g.conflicts.contains(&Conflict::DoubleTake { order_id: "open-1".into(), item: "c".into(), what: "held", merged: -2 }),
        "{:?}",
        g.conflicts
    );
}

// ── corrupted tips: refused by name, never glued ───────────────────────────

fn cells(b: &[u8]) -> Vec<i64> {
    b.chunks_exact(8).map(|w| i64::from_le_bytes(w.try_into().unwrap())).collect()
}
fn bytes(c: &[i64]) -> Vec<u8> {
    c.iter().flat_map(|x| x.to_le_bytes()).collect()
}
/// Re-seal object `obj`'s crc after an edit: a forger who knows the format.
fn reseal(c: &mut [i64], obj: usize) {
    let len = (c[obj] & 0xFFFF_FFFF) as usize;
    let crc = crc32_cells(c, obj + 2, len) as i64;
    c[obj + 1] = (crc << 32) | (c[obj + 1] & 0xFFFF_FFFF);
}
fn root_obj(b: &[u8]) -> usize {
    root_in(&View::new(b)).unwrap()
}
fn newest_obj(b: &[u8]) -> usize {
    let v = View::new(b);
    follow_in(&v, root_in(&v).unwrap(), 1).unwrap()
}

#[test]
fn the_true_last_glued_tip_glues() {
    let (h, k) = x5_pair();
    let g = glue(&h, &k, None).unwrap();
    assert_eq!(glue(&h, &k, Some(&g.ancestor)).unwrap(), g);
}

#[test]
fn a_last_glued_tip_with_one_flipped_bit_is_refused_not_glued() {
    let (h, k) = x5_pair();
    let mut tip = glue(&h, &k, None).unwrap().ancestor;
    tip[2] ^= 1 << 33;
    assert_eq!(glue(&h, &k, Some(&tip)), Err(Refusal { side: Side::Hub, why: Why::LastGluedNotInLog }));
}

#[test]
fn the_kitchen_root_tip_with_one_flipped_bit_is_refused_by_its_crc() {
    let (h, k) = x5_pair();
    let mut c = cells(&k);
    c[root_obj(&k) + 2 + 3] ^= 1;
    let r = glue(&h, &bytes(&c), None).unwrap_err();
    assert_eq!(r.side, Side::Kitchen);
    assert!(matches!(r.why, Why::Unreadable(_)), "{r:?}");
}

#[test]
fn the_kitchen_root_tip_flipped_and_resealed_is_a_tip_mismatch() {
    let (h, k) = x5_pair();
    let mut c = cells(&k);
    let r = root_obj(&k);
    c[r + 2 + 3] ^= 1;
    reseal(&mut c, r);
    assert_eq!(glue(&h, &bytes(&c), None).unwrap_err().why, Why::TipMismatch);
}

#[test]
fn the_kitchen_newest_record_with_one_flipped_payload_bit_is_a_bad_crc() {
    let (h, k) = x5_pair();
    let mut c = cells(&k);
    c[newest_obj(&k) + 2 + 12] ^= 1 << 9;
    assert_eq!(glue(&h, &bytes(&c), None), Err(Refusal { side: Side::Kitchen, why: Why::BadCrc { pos: 0 } }));
}

#[test]
fn the_hub_with_one_flipped_payload_bit_below_the_ancestor_is_quarantined_and_refused() {
    let (h, k) = x5_pair();
    let mut c = cells(&h);
    // The oldest record: far below the ancestor, read only by the hub's whole load.
    let v = View::new(&h);
    let mut o = newest_obj(&h);
    while let Some(next) = follow_in(&v, o, 2) {
        o = next;
    }
    c[o + 2 + 12] ^= 1 << 9;
    assert_eq!(glue(&bytes(&c), &k, None), Err(Refusal { side: Side::Hub, why: Why::Quarantined(1) }));
}

#[test]
fn the_kitchen_newest_record_forged_and_resealed_is_a_broken_link() {
    let (h, k) = x5_pair();
    let mut c = cells(&k);
    let o = newest_obj(&k);
    c[o + 2 + 12] ^= 1 << 9;
    reseal(&mut c, o);
    assert_eq!(glue(&h, &bytes(&c), None), Err(Refusal { side: Side::Kitchen, why: Why::BrokenLink { pos: 0 } }));
}

#[test]
fn two_unrelated_logs_share_no_ancestor() {
    let (h, _) = x5_pair();
    let other = branch(grid_base(), &[vec![res("a", 1, "x")]]);
    assert_eq!(glue(&h, &other, None).unwrap_err().why, Why::NoCommonAncestor);
}

// ── the measurement (release, `--ignored --nocapture`) ─────────────────────

#[test]
#[ignore]
fn measure_glue_2000_20_20() {
    let (h, k) = x5_pair();
    let reps = 9;
    let time = |f: &dyn Fn() -> usize| -> (u128, Vec<u128>) {
        let mut v: Vec<u128> = (0..reps)
            .map(|_| {
                let t = std::time::Instant::now();
                std::hint::black_box(f());
                t.elapsed().as_micros()
            })
            .collect();
        v.sort();
        (v[reps / 2], v)
    };
    let (glue_us, all) = time(&|| glue(&h, &k, None).unwrap().conflicts.len());
    let (load_us, _) = time(&|| StockLog::load(&h).unwrap().len());
    let hub = StockLog::load(&h).unwrap();
    let (ledger_us, _) = time(&|| hub.ledger().unwrap().items().len());
    let g = glue(&h, &k, None).unwrap();
    println!(
        "glue 2000+{}+{} median={glue_us}us of {reps} {all:?}; inside it: hub StockLog::load={load_us}us, hub ledger()={ledger_us}us; image={}B; wire={}B; conflicts={:?}",
        g.hub_tail,
        g.kitchen_tail,
        h.len(),
        g.wire_bytes,
        g.conflicts
    );
    assert!(glue_us <= 1000, "S0 pass mark: <= 1 ms, got {glue_us} us");
}
