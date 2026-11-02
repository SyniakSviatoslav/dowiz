//! W-ES-HARD: every order x cut x crash point of N optimistic commits, v3 and v4, and the RED
//! proofs by name (a section without validation, a replay onto the base, a group without its
//! first barrier).

use super::*;
use crate::sim::SimOp;

/// The full product for N in 2..=4: every pattern, every order, every cut; every group's record
/// under every power-loss state (and, for N = 2, `pick_only` too -- Lean's recovery). Each
/// schedule's verdicts and visible maps are the model's; every state recovers a group boundary.
fn whole_space(img: Image) {
    for n in 2..=4 {
        let (mut schedules, mut groups, mut states) = (0, 0, 0);
        for (pattern, txns) in patterns(n) {
            for order in permutations(n) {
                for cut in cuts(n) {
                    let s = Schedule { pattern, txns: &txns, order: &order, cut: &cut };
                    let variants: &[Opts] = if n == 2 { &[POWER, Opts { pick_only: true, ..POWER }] } else { &[POWER] };
                    for opts in variants {
                        let o = run_schedule(img, &s, *opts).unwrap_or_else(|e| panic!("{img:?} ({opts:?}) {e}"));
                        schedules += 1;
                        groups += o.groups;
                        states += o.states;
                    }
                }
            }
        }
        assert!(groups > 0 && states >= groups * 8, "N={n}: {groups} groups, {states} states");
        eprintln!("W-HARD {img:?} N={n}: {schedules} schedules, {groups} groups flushed, {states} crash states, every one a group boundary");
    }
}

#[test]
fn every_order_cut_and_crash_point_of_n_optimistic_commits_recovers_a_group_boundary_on_v3() {
    whole_space(Image::V3);
}

#[cfg(feature = "tree")]
#[test]
fn every_order_cut_and_crash_point_of_n_optimistic_commits_recovers_a_group_boundary_on_v4() {
    whole_space(Image::V4);
}

/// The model itself, on the named cases: the second of a read-write pair is refused whichever
/// commits first; a 3-cycle refuses only the one that closes it; a remove reads its key.
#[test]
fn the_serial_model_refuses_exactly_the_closing_edge_of_a_cycle() {
    let m0 = map(&KEYS.map(|k| (k, "0")));
    let cycle2 = patterns(2).into_iter().find(|p| p.0 == "read-write cycle of 2").unwrap().1;
    assert_eq!(serial(&m0, &[cycle2[0], cycle2[1]]).accepted, [true, false]);
    assert_eq!(serial(&m0, &[cycle2[1], cycle2[0]]).accepted, [true, false]);
    let cycle3 = patterns(3).into_iter().find(|p| p.0 == "read-write cycle of 3").unwrap().1;
    // T0 (r a, w b), T2 (r c, remove a): T2 read c, T0 wrote b -> accepted; then T1 (r b, w c): b was written -> refused.
    assert_eq!(serial(&m0, &[cycle3[0], cycle3[2], cycle3[1]]).accepted, [true, true, false]);
    // T0 wrote b, then T1 (r b) refused, then T2 (r c, removes a): c untouched -> accepted.
    assert_eq!(serial(&m0, &[cycle3[0], cycle3[1], cycle3[2]]).accepted, [true, false, true]);
    // The remove reads: "disjoint" T1 removes b; a writer of b before it refuses it.
    let rb = t(&[], &[("b", Some("X"))]);
    let disjoint = patterns(2).into_iter().next().unwrap().1;
    assert_eq!(serial(&m0, &[rb, disjoint[1]]).accepted, [true, false]);
    assert_eq!(serial(&m0, &[disjoint[1], rb]).accepted, [true, true], "a blind put after the remove");
    assert_eq!(permutations(4).len(), 24);
    assert_eq!(cuts(4), vec![vec![4], vec![1, 3], vec![2, 2], vec![1, 1, 2], vec![3, 1], vec![1, 2, 1], vec![2, 1, 1], vec![1, 1, 1, 1]]);
}

/// RED (a): a section that validates NOTHING is caught on the first read-write cycle: the model
/// `serial_blind` expects the second commit accepted, the real section refuses it, and the
/// harness names the commit.
#[test]
fn a_section_without_validation_is_caught_by_the_first_read_write_cycle() {
    let txns = patterns(2).into_iter().find(|p| p.0 == "read-write cycle of 2").unwrap().1;
    let s = Schedule { pattern: "read-write cycle of 2", txns: &txns, order: &[0, 1], cut: &[2] };
    let e = run_schedule_with(Image::V3, &s, POWER, &serial_blind, &|o| o.to_vec()).unwrap_err();
    eprintln!("W-HARD RED no-validation: {e}");
    assert!(e.contains("commit #1: the model says accepted=true, the section Err(Conflict { with: 2 })"), "{e}");
    assert!(run_schedule(Image::V3, &s, POWER).is_ok(), "the real model passes the same schedule");
}

/// RED (b): a replay onto the transaction's BASE (the earlier commit's key lost) is caught by the
/// first disjoint schedule, at the second commit, by the visible map.
#[test]
fn a_replay_onto_the_base_is_caught_by_the_first_disjoint_schedule() {
    let txns = patterns(2).into_iter().next().unwrap().1;
    let s = Schedule { pattern: "disjoint", txns: &txns, order: &[0, 1], cut: &[2] };
    let e = run_schedule_with(Image::V3, &s, POWER, &serial_onto_base, &|o| o.to_vec()).unwrap_err();
    eprintln!("W-HARD RED lost update: {e}");
    assert!(e.starts_with("disjoint N=2 order [0, 1] groups [2]: after commit #1 readers see Some({\"a\": [65]"), "{e}");
    assert!(e.contains("the model says Some({\"a\": [48]"), "the mutant model lost T0's put of a: {e}");
}

/// RED (c), as T-ES-23's: a group flushed WITHOUT barrier 1 (its superblock in the window of the
/// arena and PartTab it names) fails the crash axis on every cut of a 3-writer schedule, and the
/// failure names the crash point.
#[test]
fn a_group_without_its_first_barrier_is_red_under_every_cut() {
    let m_sb_early = |ops: &[SimOp]| -> Vec<SimOp> {
        let first = ops.iter().position(|o| *o == SimOp::Barrier).expect("a barrier");
        ops.iter().enumerate().filter(|(i, _)| *i != first).map(|(_, o)| o.clone()).collect()
    };
    let txns = patterns(3).into_iter().find(|p| p.0 == "one shared read").unwrap().1;
    for cut in cuts(3) {
        let s = Schedule { pattern: "one shared read", txns: &txns, order: &[2, 0, 1], cut: &cut };
        let e = run_schedule_with(Image::V3, &s, Opts { old_intact: false, ..POWER }, &serial, &m_sb_early).unwrap_err();
        assert!(e.contains("states fail; first Some(\"#"), "{e}");
        eprintln!("W-HARD RED M-SB-EARLY {e}");
    }
}
