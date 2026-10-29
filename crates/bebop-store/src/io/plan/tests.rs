use super::*;
use crate::kv::delta::Op;
use crate::sim::testkit::{delta, kv_image, CAP};

fn w(off: usize, n: usize) -> Step {
    Step::Write { off, n }
}

/// One staged commit: arena, PartTab, barrier, superblock, barrier -- Lean's `docOps`.
#[test]
fn one_commit_is_two_barriers_superblock_last() {
    let base = kv_image(&[("a", "1")], CAP);
    let d = Durable::of(&base).unwrap();
    let mut st = Store { cells: base.cells.clone() };
    delta(&mut st, &[Op::Put("b", b"2")]);
    let used = st.pick().unwrap().arena_used as usize;
    let o = SB_B - d.at;
    assert_eq!(plan(&d, &st).unwrap(), vec![w(d.cursor as usize, used - d.cursor as usize), w(o + 16, 21), Step::Barrier, w(o, 16), Step::Barrier]);
}

/// Two staged commits: the second superblock sits on the durable page, so the first is
/// published (made durable) before that page is touched.
#[test]
fn two_commits_publish_the_inner_one_first() {
    let base = kv_image(&[("a", "1")], CAP);
    let d = Durable::of(&base).unwrap();
    let mut st = Store { cells: base.cells.clone() };
    delta(&mut st, &[Op::Put("b", b"2")]);
    delta(&mut st, &[Op::Put("c", b"3")]);
    let o = SB_B - d.at;
    let p = plan(&d, &st).unwrap();
    assert_eq!(&p[1..], &[w(o + 16, 21), Step::Barrier, w(o, 16), Step::Barrier, w(d.at + 16, 21), Step::Barrier, w(d.at, 16), Step::Barrier]);
}

/// Nothing staged: nothing to write. An image that does not extend the durable one: refused.
#[test]
fn nothing_staged_is_no_step_and_an_older_image_is_refused() {
    let base = kv_image(&[("a", "1")], CAP);
    let d = Durable::of(&base).unwrap();
    assert_eq!(plan(&d, &base).unwrap(), vec![]);
    let ahead = Durable { gen: d.gen + 5, ..d };
    assert!(matches!(plan(&ahead, &base), Err(IoError::Plan(_))));
    let behind = Durable { cursor: i64::MAX, ..d };
    let mut st = Store { cells: base.cells.clone() };
    delta(&mut st, &[Op::Put("b", b"2")]);
    assert!(matches!(plan(&behind, &st), Err(IoError::Plan(_))));
}
