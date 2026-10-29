use super::*;
use crate::format::{without_anchor, PickError, SB_ANC};
use crate::kv::delta::Op;
use crate::sim::testkit::{delta, kv_image, CAP};
use crate::Store;

/// Generation 2 = a v2 base (gen 1) + one delta (gen 2), with the anchor written.
fn gen2() -> Store {
    let mut st = kv_image(&[("a", "1"), ("b", "22")], CAP);
    delta(&mut st, &[Op::Put("c", b"333")]);
    st
}

fn mark_of(st: &Store) -> usize {
    let sb = st.pick().unwrap();
    (st.cells[sb.at + SB_ANC] as u64 >> 32) as usize
}

/// T-ES-17, Rust half: an image this crate wrote opens through its ANCHOR, walking nothing.
#[test]
fn a_rust_image_opens_by_its_anchor_walking_no_object() {
    let st = gen2();
    let r = recover(&st).unwrap();
    assert_eq!((r.sb.generation, r.hidden), (2, None));
    let used = r.sb.arena_used as usize;
    assert_eq!(r.via, Via::Anchor { cells: used - mark_of(&st) });
}

/// The twin: the same image with `anc` = 0 is WALKED, from the other superblock's cursor.
#[test]
fn an_image_without_an_anchor_is_walked_from_the_other_cursor() {
    let st = Store::from_bytes(&without_anchor(&gen2().to_bytes()));
    let r = recover(&st).unwrap();
    assert_eq!(r.sb.generation, 2);
    assert!(matches!(r.via, Via::Walk { from, objects: 2 } if from == mark_of(&gen2())), "{:?}", r.via);
    let fresh = Store::create_bytes(CAP).unwrap();
    assert_eq!(recover(&fresh).unwrap().via, Via::Walk { from: ARENA, objects: 0 });
}

/// A torn LAST COMMIT under a crc-valid superblock: hidden in memory, the older generation
/// opened after a whole-arena walk -- st_reopen_verify's rule, without writing.
#[test]
fn a_torn_last_commit_falls_back_to_the_previous_generation() {
    let mut st = gen2();
    let live = st.pick().unwrap().at;
    let m = mark_of(&st);
    st.cells[m + 3] ^= 1;
    let before = st.cells.clone();
    let r = recover(&st).unwrap();
    assert_eq!((r.sb.generation, r.hidden), (1, Some(live)));
    assert!(matches!(r.via, Via::Fallback { from_gen: 2, why: Why::Anchor(Anchor::Torn { .. }), .. }), "{:?}", r.via);
    assert_eq!(st.cells, before, "recover writes nothing");
}

/// ... and when the older generation's arena is torn too, nothing opens, and both reasons
/// are named.
#[test]
fn a_torn_fallback_is_refused_naming_both() {
    let mut st = gen2();
    let m = mark_of(&st);
    st.cells[m + 3] ^= 1;
    st.cells[ARENA + 3] ^= 1;
    let e = recover(&st).unwrap_err();
    assert!(matches!(e, RecoverError::Both { live: Why::Anchor(_), other: Why::Walk { at: ARENA } }), "{e:?}");
}

/// A STRUCTURAL refusal stays a refusal (FORMAT §3.1 item 3): a bad PartTab under the newer
/// valid superblock is not a reason to read the older generation.
#[test]
fn a_bad_parttab_is_a_refusal_not_a_fallback() {
    let mut st = gen2();
    let at = st.pick().unwrap().at;
    st.cells[at + 16 + 19] += 8;
    assert!(matches!(recover(&st), Err(RecoverError::Pick(PickError::PartTabBad { .. }))));
}

/// `Hide` hides exactly one cell.
#[test]
fn hide_reads_one_cell_as_zero() {
    let st = gen2();
    let h = Hide { inner: &st, at: 512 };
    assert_eq!((h.cell_at(512), h.cell_at(513), h.n_cells()), (0, st.cells[513], st.cells.len()));
}
