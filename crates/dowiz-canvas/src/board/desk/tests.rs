//! The "decision needed" strip: drawn from the feed's D lines, one Done each, and nothing cut
//! silently.

use super::super::testrig::{feed_text, Rig};
use super::super::{Intent, Role};
use crate::scene::Act;

fn with_decisions(n: usize) -> Rig {
    let mut r = Rig::new(390, 844);
    r.b.session(true, true, true, Role::Kitchen);
    let mut f = feed_text();
    for i in 0..n {
        f += &format!("D\tab12-{i}\toversold\tsalmon\t-1\tkitchen\n");
    }
    r.b.feed(f.as_bytes());
    r.frame();
    r
}

fn done_buttons(r: &Rig) -> Vec<(u16, crate::geom::Rect)> {
    r.scene.nodes().iter().filter_map(|n| if let Act::Decided(i) = n.act { Some((i, n.rect)) } else { None }).collect()
}

#[test]
fn a_decision_is_drawn_and_done_closes_that_one() {
    let mut r = with_decisions(2);
    assert_eq!(r.b.data.nd, 2);
    let done = done_buttons(&r);
    assert_eq!(done.len(), 2);
    let (_, rect) = done[1];
    r.tap(rect.x + rect.w / 2, rect.y + rect.h / 2);
    match r.b.take_intent() {
        Some(Intent::Decided { did }) => assert_eq!(r.b.data.str(did), "ab12-1"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn no_decision_no_strip_and_a_long_list_says_how_many_more() {
    assert!(done_buttons(&with_decisions(0)).is_empty(), "the twin: nothing to decide, nothing drawn");
    let r = with_decisions(5);
    assert_eq!(done_buttons(&r).len(), super::SHOWN, "three rows");
    assert_eq!(r.b.data.nd, 5, "all five read, two counted as more");
    let bad = {
        let mut r = Rig::new(390, 844);
        r.b.feed(b"D\t\toversold\tsalmon\t-1\tkitchen\n");
        r.b.data.bad
    };
    assert_eq!(bad, 1, "a decision with no id is a bad line, not a row");
}
