//! W-MG-ATK5: A CONFLICT THAT READS AS NONE. One HOLE each, closed by W-MG-MERGE2, kept as law; a
//! twin (where one is not in `m2_merge`) is the control. Real ML-DSA-65, no fuzzing.
mod atk5_common;
mod atk_forge;

use atk5_common::*;
use atk_forge::{forged, read, write};
use einstore::evlog::doc::seal::{message, Seal};
use einstore::evlog::doc::{merge_body, Doc, Ids, MergeView, Row, Signer, Trust, ALG_ML_DSA_65, STATE_COMMUTES};
use einstore::Store;
use dowiz_hub::doc_seal::merge::{merge_report, Merged, Recheck};
use dowiz_hub::doc_seal::{DeviceKey, HubIds, MlDsa65, SealedHub, KIND_STOCK, MIN_SEALED_BYTES};

/// The union with NO MERGE yet: what a key holder has before signing whatever it likes.
fn union(fa: &[u8], fb: &[u8]) -> (Vec<u8>, Vec<Row>, Vec<Row>) {
    let (o, t) = (Store::from_bytes(fa), Store::from_bytes(fb));
    let mut fresh = Store::create_bytes((fa.len() + fb.len()).max(MIN_SEALED_BYTES)).unwrap();
    let u = Doc::merge_into(&o, &t, &mut fresh, &HubIds, &MlDsa65, Trust::Genesis).unwrap();
    (fresh.to_bytes_trimmed(), u.ours, u.theirs)
}
fn lie(file: &[u8], ours: Vec<Row>, theirs: Vec<Row>, key: &DeviceKey) -> Vec<u8> {
    let mut sh = opened(file, 7);
    sh.declare(&merge_body(&MergeView { ours, theirs, state: STATE_COMMUTES, seg_root: [0; 32], conflicts: Vec::new() }), key, 600).unwrap();
    trimmed(&sh)
}
/// The last salmon sold on A and on B.
fn last_salmon() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let s = genesis(&[received("salmon", 1), received("rice", 10)]);
    let fa = offline(&s, 2, &a(), &[served("salmon", 1, "o1")]);
    let fb = offline(&s, 3, &b(), &[served("salmon", 1, "o2")]);
    (s, fa, fb)
}

/// THREE SALES OF ONE SALMON. A and B meet and name it; C's sale then meets a side already below
/// zero, and `obstructions` skips `broken(lh)`: "that branch's write door already decided". This
/// path has no write door (`stock.rs`). The second MERGE signs `commutes` as the level goes -1 to -2.
#[test]
fn the_third_sale_of_one_salmon_is_named() {
    let (s, fa, fb) = last_salmon();
    let (ab, said) = merged(&fa, &fb, &a(), 4);
    assert!(named(&said).unwrap().contains("salmon"), "the control");
    let fc = offline(&s, 5, &c(), &[served("salmon", 1, "o3")]);
    let (abc, said) = merged(&ab, &fc, &a(), 6);
    assert_eq!(fold(&abc).unwrap().level("salmon").on_hand, -2);
    assert!(named(&said).unwrap().contains("salmon"), "HOLE 3: a third sale of one salmon reads as no conflict: {said:?}");
}

/// The same skip with two devices: A alone took the shelf to -1, offline, and B's sale is unnamed.
#[test]
fn a_sale_on_a_shelf_the_other_side_broke_is_named() {
    let s = genesis(&[received("salmon", 1), received("rice", 10)]);
    let fa = offline(&s, 2, &a(), &[served("salmon", 2, "o1")]);
    let fb = offline(&s, 3, &b(), &[served("salmon", 1, "o2")]);
    let (_, said) = merged(&fa, &fb, &a(), 4);
    assert!(named(&said).unwrap().contains("salmon"), "HOLE 3b: {said:?}");
}

/// A KEY HOLDER PICKS THE ROWS: both lists name A's head alone, so neither has a tail and a signed
/// `commutes` covers two sales of one salmon. Each list must hold a record the other lacks. PARTIAL
/// BY NECESSITY: `merge.rs` says what one file cannot prove.
#[test]
fn a_signed_merge_with_doctored_rows_reads_differs() {
    let (_, fa, fb) = last_salmon();
    let (u, _, theirs) = union(&fa, &fb);
    let top: Vec<Row> = theirs.iter().filter(|r| r.actor_id == a().id()).copied().collect();
    let f = lie(&u, top.clone(), top, &a());
    assert!(holds(&f), "the file alone cannot say");
    let (lines, clean) = audit(&f);
    assert_eq!(fold(&f).unwrap().level("salmon").on_hand, -1);
    assert!(lines == vec![Recheck::Differs] && !clean, "HOLE 4: a hidden conflict reads clean: {lines:?} clean={clean}");
}

/// A ONE-RECORD CUT LAUNDERS THE SAME LIE. `merge_lines` asks `any cut in the file`, so every MERGE
/// reads `unchecked`, rc 0. The control is `m2_merge`'s `a_signed_merge_that_hides_the_conflict`.
#[test]
fn a_one_record_cut_does_not_launder_a_lie() {
    let s = genesis(&[received("salmon", 1)]);
    let (fa, fb) = (offline(&s, 2, &a(), &[served("salmon", 1, "o1")]), offline(&s, 3, &b(), &[served("salmon", 1, "o2")]));
    let (u, ours, theirs) = union(&fa, &fb);
    let f = lie(&u, ours, theirs, &a());
    assert_eq!(audit(&f).0, vec![Recheck::Differs], "the control: it is caught");
    let mut sh = opened(&f, 8);
    let total = read(&f).recs.len();
    sh.cut(total - 1, &a(), 700).unwrap();
    let g = trimmed(&sh);
    assert!(holds(&g));
    let (lines, clean) = audit(&g);
    assert!(!clean, "HOLE 7: the cut of one record turned DIFFERS into {lines:?}, clean={clean}");
    assert!(merge_report(&SealedHub::load(&g).unwrap(), true).0.iter().any(|l| l.contains("не перевірено")), "and it says the words");
    // The auditor's own exit code: 4, the words, never 0. The twin is the same file before the cut.
    let rc = |f: &[u8], name: &str| {
        let path = std::env::temp_dir().join(format!("atk5-{}-{name}", std::process::id()));
        std::fs::write(&path, f).unwrap();
        let o = std::process::Command::new(env!("CARGO_BIN_EXE_doc_verify")).arg(&path).output().unwrap();
        std::fs::remove_file(&path).unwrap();
        (o.status.code(), String::from_utf8_lossy(&o.stdout).lines().last().unwrap_or("").to_string())
    };
    assert_eq!(rc(&g, "cut"), (Some(4), "NOT CHECKED".to_string()));
    assert_eq!(rc(&f, "lie"), (Some(3), "DOES NOT HOLD".to_string()));
}

/// ONE ORDER SERVED ON BOTH. `takes` watches a served line only for going below zero, so a double
/// sale of one order id, on a shelf with room, is `commutes` and the level moves twice.
#[test]
fn one_order_served_on_both_sides_is_named() {
    let s = genesis(&[received("rice", 10)]);
    let fa = offline(&s, 2, &a(), &[served("rice", 2, "o1")]);
    let fb = offline(&s, 3, &b(), &[served("rice", 2, "o1")]);
    let (f, said) = merged(&fa, &fb, &a(), 4);
    assert_eq!(fold(&f).unwrap().level("rice").on_hand, 6);
    assert!(named(&said).unwrap().contains("o1"), "HOLE 6: one order's stock taken twice reads as no conflict: {said:?}");
}

/// A MOVEMENT THIS BUILD CANNOT READ. A newer tablet's kind, or a damaged line, under a real seal:
/// `stock_event` answers `None`, `conflicts_at` skips it, `stock_events` drops it, and the MERGE
/// says commutes over a line nobody read. The line is B's own, sealed by B.
#[test]
fn a_stock_record_this_build_cannot_read_is_not_silence() {
    let s = genesis(&[received("salmon", 1)]);
    let fa = offline(&s, 2, &a(), &[served("salmon", 1, "o1")]);
    let fb = offline(&s, 3, &b(), &[served("salmon", 0, "z")]);
    let k = b();
    let mut side = read(&fb);
    let top = side.chains.iter().flatten().filter(|x| x.actor_id == k.id()).last().unwrap().clone();
    let mut rec = forged(k.id(), top.seq + 1, top.tip, 0x31);
    rec.payload.truncate(16);
    rec.payload.push(KIND_STOCK);
    rec.payload.extend_from_slice(br#"{"k":"transfer","item":"salmon","qty":1,"to":"bar"}"#);
    rec.id = HubIds.chained(&rec);
    let sig = k.sign(&message(&k.id(), rec.actor_seq, &rec.id, 200, &top.tip));
    let new = Seal { at: 0, actor_id: k.id(), seq: rec.actor_seq, tip: rec.id, claimed_ms: 200, prev: None, time: None, alg: ALG_ML_DSA_65, sig };
    side.recs.push(rec);
    side.chains.iter_mut().find(|c| c[0].actor_id == k.id()).unwrap().push(new);
    let fb2 = write(&side);
    assert!(holds(&fb2), "a sealed record of B's own");
    let mut sh = opened(&fa, 4);
    let said = sh.merge(&fb2, &a(), 500);
    let mentions = matches!(&said, Ok(Merged::Sealed { conflicts, .. }) if !conflicts.is_empty()) || said.is_err();
    assert!(mentions, "HOLE 8: a sealed movement read as nothing: {said:?}; {} of {} stock records seen", SealedHub::load(&trimmed(&sh)).unwrap().stock_events().len(), read(&trimmed(&sh)).recs.iter().filter(|r| r.payload.get(16) == Some(&KIND_STOCK)).count());
}
