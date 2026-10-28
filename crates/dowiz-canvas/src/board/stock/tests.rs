//! The offline stock sheet: open, step to an item, pick the movement, type a quantity, save --
//! one intent naming the item; a quantity that is not one never leaves.

use super::super::testrig::{feed_text, Rig};
use super::super::{Intent, Role, F_QTY};
use crate::scene::Act;

fn rig() -> Rig {
    let mut r = Rig::new(390, 844);
    r.b.session(true, true, false, Role::Kitchen);
    let f = feed_text() + "S\tsalmon\t17\nS\trice\t4 980\n";
    r.b.feed(f.as_bytes());
    r.frame();
    r
}

fn tap_act(r: &mut Rig, want: Act) {
    let n = r.scene.nodes().iter().rev().find(|n| n.act == want).unwrap_or_else(|| panic!("no node {want:?}")).rect;
    r.tap(n.x + n.w / 2, n.y + n.h / 2);
}

#[test]
fn a_counted_item_is_saved_as_one_intent_naming_it() {
    let mut r = rig();
    assert_eq!(r.b.data.ni, 2);
    tap_act(&mut r, Act::StockOpen);
    assert!(r.b.stock_open);
    tap_act(&mut r, Act::StockStep(true));
    tap_act(&mut r, Act::StockKind(1));
    r.b.fields[F_QTY as usize].set(b"3");
    tap_act(&mut r, Act::StockSave);
    let mut got = None;
    while let Some(i) = r.b.take_intent() {
        if let Intent::Stock { kind, item } = i {
            got = Some((kind, r.b.data.str(item).to_string()));
        }
    }
    assert_eq!(got, Some((1, "rice".to_string())), "wasted, the second item");
    assert!(!r.b.stock_open, "the sheet closes on save");
}

#[test]
fn a_quantity_that_is_not_one_never_leaves() {
    let mut r = rig();
    tap_act(&mut r, Act::StockOpen);
    for bad in [&b""[..], b"0", b"2.5", b"-1", b"abc"] {
        r.b.fields[F_QTY as usize].set(bad);
        tap_act(&mut r, Act::StockSave);
        assert!(r.b.stock_open, "{:?} keeps the sheet open", core::str::from_utf8(bad));
        while let Some(i) = r.b.take_intent() {
            assert!(!matches!(i, Intent::Stock { .. }), "no record for {:?}", core::str::from_utf8(bad));
        }
    }
    // The twin: a count of zero is a quantity (an empty shelf).
    tap_act(&mut r, Act::StockKind(2));
    r.b.fields[F_QTY as usize].set(b"0");
    tap_act(&mut r, Act::StockSave);
    assert!(!r.b.stock_open);
}

#[test]
fn every_control_on_the_sheet_is_full_tap_size() {
    let mut r = rig();
    tap_act(&mut r, Act::StockOpen);
    assert_eq!(r.scene.too_small(crate::ui::TAP), 0);
}
