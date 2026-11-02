//! T-ES-13/20/21/22/23 over REAL flushes (`testkit::flush` drives `Writer` on a `SimDisk`).

use super::*;
use crate::kv::delta::Op;
use crate::sim::crash::Tear;
use crate::sim::testkit::{applied, delta, flush, kv_image, next, replay, scenarios, Map, CAP};
use crate::sim::SimOp;
use crate::{Durable, Store};

const POWER: Opts = Opts { model: Model::Power(Tear::Half), pick_only: false, old_intact: true };

fn run_map(base: &[i64], ops: &[SimOp], opts: Opts, expect: &[(i64, Map)]) -> Report {
    run(base, ops, opts, expect, &replay, &next)
}

/// T-ES-20 + T-ES-13: the four scenarios, protocol v2, every power-loss state: recovery opens
/// k or k+1, replaying to exactly before or before+ops, the arena walks, nothing of k moved,
/// and the next commit lands. Also with `pick_only` -- the recovery Lean's theorem is about.
#[test]
fn every_crash_point_of_a_v2_commit_recovers_k_or_k_plus_1() {
    for s in scenarios() {
        let f = flush(&s.base, Durability::Data, s.stage);
        assert_eq!(f.gen, s.k + 1, "{}", s.name);
        let expect = [(s.k, s.before.clone()), (s.k + 1, s.after.clone())];
        for opts in [POWER, Opts { pick_only: true, ..POWER }] {
            let r = run_map(&f.base, f.disk.ops(), opts, &expect);
            assert!(r.green(), "{} ({opts:?}): {} of {} states fail; first {:?}", s.name, r.failures.len(), r.states, r.first());
            assert!(r.states >= 8, "{}: {} states", s.name, r.states);
            eprintln!("T-ES-20 {} pick_only={}: {} states green", s.name, opts.pick_only, r.states);
        }
    }
}

/// The ops of the delta scenario, and its durable image.
fn delta_flush(d: Durability) -> (Vec<i64>, Vec<SimOp>, [(i64, Map); 2]) {
    let s = scenarios().into_iter().nth(1).unwrap();
    let f = flush(&s.base, d, s.stage);
    (f.base, f.disk.ops().to_vec(), [(s.k, s.before), (s.k + 1, s.after)])
}

/// Mutant M-SB-EARLY: the barrier between {arena, PartTab} and the superblock removed.
fn m_sb_early(ops: &[SimOp]) -> Vec<SimOp> {
    let first = ops.iter().position(|o| *o == SimOp::Barrier).expect("a barrier");
    ops.iter().enumerate().filter(|(i, _)| *i != first).map(|(_, o)| o.clone()).collect()
}

/// T-ES-21: THE HARNESS CAN GO RED. The named mutant puts the superblock in the same window
/// as what it names; the harness names the exact crash point (the same scenario is green
/// unmutated, above). And `Durability::None` (no barrier at all) is red under power loss,
/// green under a process crash -- exactly its promise.
#[test]
fn the_harness_names_the_crash_point_of_a_mutated_order() {
    let (base, ops, expect) = delta_flush(Durability::Data);
    let r = run_map(&base, &m_sb_early(&ops), POWER, &expect);
    assert!(!r.green());
    eprintln!("T-ES-21 M-SB-EARLY: {} of {} states fail", r.failures.len(), r.states);
    assert_eq!(r.first().unwrap(), "#1 power loss after 0 barrier(s): new [] torn sector 8 first 32 cells new: recover refused: Pick(PartTabBad { at: 512, pt: 528, want: 3585669947, got: Some(1493798410) })");
    let (base, ops, expect) = delta_flush(Durability::None);
    assert!(!ops.contains(&SimOp::Barrier));
    let power = run_map(&base, &ops, POWER, &expect);
    assert!(!power.green(), "None is not power-safe");
    eprintln!("T-ES-21 None: power {} of {} fail", power.failures.len(), power.states);
    let process = run_map(&base, &ops, Opts { model: Model::Process, ..POWER }, &expect);
    assert!(process.green(), "None is process-crash safe: {:?}", process.first());
}

/// T-ES-22: a torn superblock, cut at EVERY cell, always opens the previous generation.
#[test]
fn a_superblock_torn_at_any_cell_opens_the_previous_generation() {
    let (base, ops, expect) = delta_flush(Durability::Data);
    let r = run_map(&base, &ops, Opts { model: Model::Power(Tear::Cells), ..POWER }, &expect);
    assert!(r.green(), "{:?}", r.first());
    // The base's live superblock is A (gen 2), so the new one is B: cells 512..528, sector 8,
    // the only sector window 1 writes. A cut opens k+1 exactly when the superblock's sixteen
    // cells came out equal to the new ones (a cut inside cells the old and new share -- magic,
    // version -- is no tear at all), and k otherwise: crc15 refused every real mix.
    let mut fin = base.clone();
    for op in &ops {
        if let SimOp::Write { off, cells } = op {
            crate::sim::apply(&mut fin, *off, cells);
        }
    }
    let mut torn_sb = 0;
    crate::sim::crash::each_state(&base, &ops, Model::Power(Tear::Cells), |p, img| {
        if let (1, true, Some(t)) = (p.window, p.new.is_empty(), p.torn) {
            assert_eq!(t.sector, 8, "{p}");
            let g = crate::sim::recover(&Store { cells: img.to_vec() }).unwrap().sb.generation;
            let whole = img[512..528] == fin[512..528];
            assert_eq!(g, if whole { expect[1].0 } else { expect[0].0 }, "{p}");
            torn_sb += 1;
        }
    });
    assert_eq!(torn_sb, 126, "every cut of the superblock's sector");
}

/// The N-th op of the group scenario, and the group's base.
fn group_ops() -> Vec<Vec<Op<'static>>> {
    vec![
        vec![Op::Put("b", b"2")],
        vec![Op::Remove("a")],
        vec![Op::Put("c", b"3")],
        vec![Op::Put("d", b"4"), Op::Put("e", b"5")],
        vec![Op::Remove("c")],
        vec![Op::Put("a", b"back")],
        vec![Op::Put("f", b"6")],
        vec![Op::Remove("b"), Op::Put("g", b"7")],
    ]
}

/// One group of the first `n` ops flushed with `d`: the flush, and the maps of k and k+n.
fn group(n: usize, d: Durability) -> (crate::sim::testkit::Flushed, [(i64, Map); 2]) {
    let base = kv_image(&[("a", "1")], CAP);
    let ops = group_ops();
    let f = flush(&base, d, |st| ops[..n].iter().for_each(|o| delta(st, o)));
    let before = replay(&base);
    let after = ops[..n].iter().fold(before.clone(), |m, o| applied(&m, o));
    (f, [(1, before), (1 + n as i64, after)])
}

/// T-ES-23 (AC-ES-18, W-ES-P11): a GROUP of N staged commits flushed once costs exactly TWO
/// barriers for every N in 1..8 (the flip: an even group's live superblock is moved to the
/// page opposite the durable one, FORMAT §8), and every power-loss state recovers k or k+N
/// with the exact map -- never a generation between (the `expect` list holds only those two,
/// so a state opening k+j, 0<j<N, would fail "not one of").
#[test]
fn a_group_flush_is_two_barriers_and_recovers_k_or_k_plus_n() {
    for n in 1..=8usize {
        let (f, expect) = group(n, Durability::Data);
        assert_eq!(f.gen, 1 + n as i64, "N={n}");
        let barriers = f.disk.ops().iter().filter(|o| **o == SimOp::Barrier).count();
        assert_eq!(barriers, 2, "N={n}: {barriers} barriers");
        for opts in [Opts { old_intact: true, ..POWER }, Opts { pick_only: true, ..POWER }] {
            let r = run_map(&f.base, f.disk.ops(), opts, &expect);
            assert!(r.green(), "N={n} ({opts:?}): {} of {} states fail; first {:?}", r.failures.len(), r.states, r.first());
            eprintln!("T-ES-23 N={n} pick_only={}: {} states green, 2 barriers", opts.pick_only, r.states);
        }
        // What the device holds after the group: the durable page as it was, k+N opposite.
        let fin = f.disk.cache();
        let d = Durable::of(&Store { cells: f.base.clone() }).unwrap();
        assert_eq!(fin[d.at..d.at + 39], f.base[d.at..d.at + 39], "N={n}: the old durable page is untouched");
        assert_eq!(Durable::of(&Store { cells: fin }).unwrap().at, crate::SB_B - d.at, "N={n}: k+N sits opposite");
    }
}

/// T-ES-23 RED (W-ES-P11): a group that skips barrier 1 (superblock in the same window as
/// the arena and PartTab it names) fails the harness, for an even and an odd N.
#[test]
fn a_group_that_skips_barrier_1_is_red() {
    for n in [2usize, 5] {
        let (f, expect) = group(n, Durability::Data);
        let r = run_map(&f.base, &m_sb_early(f.disk.ops()), Opts { old_intact: false, ..POWER }, &expect);
        assert!(!r.green(), "N={n}: a group without barrier 1 passed");
        eprintln!("T-ES-23 RED N={n} M-SB-EARLY: {} of {} states fail; first {:?}", r.failures.len(), r.states, r.first());
    }
}

/// ... and the group §3.5 sketched -- N `Durability::None` commits, then one sync -- is NOT
/// one: a crash can open LESS than the generation that was durable before the group.
#[test]
fn n_unsynced_commits_and_one_sync_are_not_a_group_commit() {
    let base = kv_image(&[("a", "1")], CAP);
    let mut w = Writer::open(SimDisk::new(base.cells.clone())).unwrap();
    let mut m = replay(&base);
    let mut expect = vec![(1, m.clone())];
    for (i, o) in [Op::Put("b", "2".as_bytes()), Op::Remove("a"), Op::Put("c", "3".as_bytes())].into_iter().enumerate() {
        delta(w.stage().unwrap(), &[o]);
        w.flush(Durability::None).unwrap();
        m = applied(&m, &[o]);
        expect.push((2 + i as i64, m.clone()));
    }
    w.sync().unwrap();
    let disk = w.into_device();
    let r = run_map(&base.cells, disk.ops(), Opts { old_intact: false, ..POWER }, &expect);
    assert!(!r.green());
    let lost = r.failures.iter().filter(|f| f.why.starts_with("recovered generation 0 ")).count();
    eprintln!("naive group: {} of {} states fail, {lost} open generation 0 < the durable 1", r.failures.len(), r.states);
    assert!(lost > 0, "the claim: a crash can open LESS than the durable generation");
    assert_eq!(r.first().unwrap(), "#1 power loss after 0 barrier(s): new [] torn sector 0 first 32 cells new: recover refused: Pick(PartTabBad { at: 0, pt: 16, want: 3349604515, got: Some(1505123122) })");
}
