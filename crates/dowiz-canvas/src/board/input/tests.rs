//! Reject / cancel with a reason (kitchen.js `askReason`), and the sheet's tap handling.
use crate::board::testrig::Rig;
use crate::board::{Intent, F_REASON, TAB_NEW};
use crate::lang::Lang;
use crate::scene::Act;

fn open_sheet(r: &mut Rig) {
    r.b.lang = Lang::En;
    r.b.tab = TAB_NEW;
    r.frame();
    r.tap_tour("kitchen.reject");
    assert_eq!(r.b.ask, Some(false), "a PENDING ticket is rejected, not cancelled");
    assert_eq!(r.b.ask_id(), "ord_aaaa1111");
}

#[test]
fn a_reason_is_required_and_sent_with_the_order() {
    let mut r = Rig::new(390, 844).signed(true, true);
    open_sheet(&mut r);
    for a in ["kitchen.reason", "kitchen.reasonClose", "kitchen.reasonSend"] {
        assert!(r.scene.find_tour(a).is_some(), "{a}");
    }
    assert!(r.cmd.texts.iter().any(|t| t == "Reject · #1111"), "{:?}", r.cmd.texts.iter().rev().take(8).collect::<Vec<_>>());
    r.tap_tour("kitchen.reasonSend");
    assert_eq!(r.b.take_intent(), None, "an empty reason sends nothing");
    assert_eq!(r.b.toast(), "Write the reason.");
    assert_eq!(r.b.ask, Some(false), "and the sheet stays");
    r.tap_tour("kitchen.reason");
    assert_eq!(r.b.take_intent(), Some(Intent::Focus(F_REASON)));
    r.b.fields[F_REASON as usize].set("out of salmon".as_bytes());
    r.tap_tour("kitchen.reasonSend");
    assert_eq!(r.b.take_intent(), Some(Intent::Blur));
    assert_eq!(r.b.take_intent(), Some(Intent::Stop { cancel: false }));
    assert_eq!(r.b.ask, None);
    assert_eq!(r.b.ask_id(), "ord_aaaa1111", "the id outlives the sheet until the host has read it");
    assert_eq!(r.b.fields[F_REASON as usize].value(), "out of salmon");
}

#[test]
fn a_confirmed_ticket_is_cancelled_and_enter_sends() {
    let mut f = String::from("T\tord_zz99\tCONFIRMED\t0\t1\tp\t\t\t\nL\t1\t2\tRamen\t\n");
    f.push('\n');
    let mut r = Rig::new(390, 844);
    r.b.session(true, true, false, crate::board::Role::Kitchen);
    r.b.feed(f.as_bytes());
    r.frame();
    r.tap_tour("kitchen.reject");
    assert_eq!(r.b.ask, Some(true));
    r.b.fields[F_REASON as usize].set(b"closed early");
    r.b.key_enter();
    assert_eq!(r.b.take_intent(), Some(Intent::Blur));
    assert_eq!(r.b.take_intent(), Some(Intent::Stop { cancel: true }));
}

#[test]
fn the_scrim_closes_and_the_panel_swallows_taps() {
    let mut r = Rig::new(390, 844).signed(true, true);
    open_sheet(&mut r);
    let panel = r.scene.nodes().iter().find(|n| n.act == Act::Block).unwrap().rect;
    r.tap(panel.x + 3, panel.y + 3);
    assert_eq!(r.b.ask, Some(false), "a tap on the panel's own background keeps it open");
    r.tap(195, 100);
    assert_eq!(r.b.ask, None, "the scrim closes it");
    assert_eq!(r.b.take_intent(), None, "and nothing was sent");
    // While open, the board under it cannot be tapped or scrolled.
    open_sheet(&mut r);
    let before = r.b.scroll;
    r.b.wheel(500);
    assert_eq!(r.b.scroll, before);
    assert_eq!(r.scene.too_small(crate::ui::TAP), 0);
}
