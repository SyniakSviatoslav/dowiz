//! W-MG-ATK5: `recheck` AND THE ORDER OF A FILE. `doc_verify` exits 3 on DIFFERS, so an honest DIFFERS
//! is an honest file refused by the auditor, and an honest device that moves a number after the
//! MERGE can also make a lie read `same`. Closed by W-MG-MERGE2, each with its control.
mod atk5_common;

use atk5_common::*;
use einstore::evlog::doc::{merge_body, Doc, MergeView, Trust, STATE_COMMUTES};
use einstore::Store;
use dowiz_hub::doc_seal::merge::Recheck;
use dowiz_hub::doc_seal::{HubIds, MlDsa65, MIN_SEALED_BYTES};

/// HOLE 2. `on_side`: an actor in neither list of rows "wrote before the seal". Not so: a device
/// that first writes AFTER the MERGE has no row either, so its movement joins BOTH sides of a merge
/// that never saw it. C receives five salmon, and the honest oversold MERGE reads DIFFERS, rc 3.
#[test]
fn a_device_that_writes_after_a_merge_does_not_move_its_recheck() {
    let s = genesis(&[received("salmon", 1)]);
    let (m, _) = merged(&offline(&s, 2, &a(), &[served("salmon", 1, "o1")]), &offline(&s, 3, &b(), &[served("salmon", 1, "o2")]), &a(), 4);
    assert_eq!(audit(&m), (vec![Recheck::Same], true), "the control");
    let later = offline(&m, 5, &c(), &[received("salmon", 5)]);
    assert!(holds(&later));
    let (lines, clean) = audit(&later);
    assert!(lines == vec![Recheck::Same] && clean, "HOLE 2: an honest file, a third device's delivery: {lines:?} clean={clean}");
}

/// The other face of HOLE 2: A hid the conflict, DIFFERS caught it, C's later delivery reads it
/// `same`, and the auditor's rc is 0.
#[test]
fn a_later_delivery_does_not_absolve_a_lie() {
    let s = genesis(&[received("salmon", 1)]);
    let (fa, fb) = (offline(&s, 2, &a(), &[served("salmon", 1, "o1")]), offline(&s, 3, &b(), &[served("salmon", 1, "o2")]));
    let (o, t) = (Store::from_bytes(&fa), Store::from_bytes(&fb));
    let mut fresh = Store::create_bytes((fa.len() + fb.len()).max(MIN_SEALED_BYTES)).unwrap();
    let u = Doc::merge_into(&o, &t, &mut fresh, &HubIds, &MlDsa65, Trust::Genesis).unwrap();
    let mut sh = opened(&fresh.to_bytes_trimmed(), 6);
    sh.declare(&merge_body(&MergeView { ours: u.ours, theirs: u.theirs, state: STATE_COMMUTES, seg_root: [0; 32], conflicts: Vec::new() }), &a(), 600).unwrap();
    let lied = trimmed(&sh);
    assert_eq!(audit(&lied).0, vec![Recheck::Differs], "the control");
    let (lines, clean) = audit(&offline(&lied, 5, &c(), &[received("salmon", 5)]));
    assert!(lines == vec![Recheck::Differs] && !clean, "HOLE 2b: a lie absolved by a stranger's delivery: {lines:?} clean={clean}");
}

/// The two files of the order trap: A counted, B sold, each merged the other (two MERGEs), each
/// took the other's MERGE in, so one shared past stands in two file orders.
fn two_orders() -> (Vec<u8>, Vec<u8>) {
    let s = genesis(&[received("salmon", 10)]);
    let (fa, fb) = (offline(&s, 2, &a(), &[counted("salmon", 5, "n1")]), offline(&s, 3, &b(), &[served("salmon", 1, "o1")]));
    let (x, _) = merged(&fa, &fb, &a(), 4);
    let (y, _) = merged(&fb, &fa, &b(), 5);
    let (x2, _) = merged(&x, &y, &a(), 6);
    let (y2, _) = merged(&y, &x, &b(), 7);
    (x2, y2)
}

/// HOLE 5. A INTO B AND B INTO A DISAGREE. Same two files, same records, and the level of the shared
/// past is 4 in one file's order and 5 in the other's: two more sales are oversold one way and not
/// the other. The Recount of round one was named, but this round's MERGE depends on who pressed it.
#[test]
fn a_into_b_and_b_into_a_name_the_same_conflict() {
    let (x, y) = two_orders();
    let (xa, yb) = (offline(&x, 8, &a(), &[served("salmon", 2, "a2")]), offline(&y, 9, &b(), &[served("salmon", 3, "b2")]));
    let (_, one) = merged(&xa, &yb, &a(), 10);
    let (_, two) = merged(&yb, &xa, &b(), 11);
    let n = |m: &dowiz_hub::doc_seal::merge::Merged| named(m).unwrap().contains("salmon");
    assert_eq!(n(&one), n(&two), "HOLE 5: {one:?} against {two:?}");
}

/// The converged file of the same run, read by the auditor, for both orders.
#[test]
fn a_converged_honest_file_passes_its_audit_in_either_order() {
    let (x, y) = two_orders();
    let (xa, yb) = (offline(&x, 8, &a(), &[served("salmon", 2, "a2")]), offline(&y, 9, &b(), &[served("salmon", 3, "b2")]));
    let (mx, _) = merged(&xa, &yb, &a(), 10);
    let (my, _) = merged(&yb, &xa, &b(), 11);
    let (ex, _) = merged(&mx, &my, &a(), 12);
    let (ey, _) = merged(&my, &mx, &b(), 13);
    let (ax, ay) = (audit(&ex), audit(&ey));
    assert!(holds(&ex) && holds(&ey));
    assert!(ax.1 && ay.1, "HOLE 5b: an honest converged file fails doc_verify: x={ax:?} y={ay:?}");
}

/// THE ONE ORDER RESPECTS WHAT IS SIGNED. A device's first record chains on what it saw, so a sale folds
/// after the delivery it sold from, whichever device sorts first. Sorted by `(actor, seq)` alone,
/// one arrangement of these two keys folds the sale first, the first delivery clips, and the last
/// salmon sold twice reads as no conflict. The first record of `seller` and of C both chain on it.
#[test]
fn a_sale_folds_after_the_delivery_it_sold_from() {
    for (deliverer, seller) in [(a(), b()), (b(), a())] {
        let s = genesis(&[received("rice", 1)]);
        let fresh = key(0xD4);
        let mut sh = opened(&s, 2);
        sh.commit_keyop(&einstore::evlog::doc::keys::KeyOp::Admit(fresh.key()), &a(), 3).unwrap();
        let s = trimmed(&sh);
        let d = offline(&s, 3, &deliverer, &[received("salmon", 1)]);
        let first = offline(&d, 4, &fresh, &[served("salmon", 1, "o1")]);
        let second = offline(&d, 5, &seller, &[served("salmon", 1, "o2")]);
        let (_, said) = merged(&first, &second, &fresh, 6);
        assert!(named(&said).unwrap().contains("salmon"), "{said:?}");
    }
}

/// THE SAME CUT OF TWO ORDERS IS UNREACHABLE: two file orders exist only above the second device's
/// ADMIT, and no cut crosses an ADMIT. Held, and it stops holding the day cuts may cross one.
#[test]
fn a_cut_cannot_reach_the_records_two_files_order_differently() {
    let (x, _) = two_orders();
    let keep = dowiz_hub::doc_seal::SealedHub::load(&x).unwrap().verify().unwrap().counters.records - 2;
    let got = opened(&x, 8).cut(keep, &a(), 700).map(|_| ()).map_err(|e| format!("{e:?}"));
    assert!(got.is_err_and(|e| e.contains("ADMIT")));
}
