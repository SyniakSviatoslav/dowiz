//! The board, driven the way the host drives it: feed -> draw -> tap -> intent.
use super::model::{Bump, Status};
use super::testrig::{feed_text, Rig, NOW};
use super::*;
use crate::ui::{DARK, LIGHT};

#[test]
fn feed_reads_tickets_lines_and_tables() {
    let mut d = Box::new(feed::Data::new());
    assert_eq!(d.apply(feed_text().as_bytes()), 4);
    assert_eq!((d.nt, d.nl, d.ntab, d.bad, d.dropped), (4, 5, 2, 0, 0));
    assert_eq!(d.tickets[0].nlines, 2);
    assert_eq!(d.str(d.lines[1].note), "no onion");
    assert_eq!(d.tables[0].nst, 2);
    assert_eq!(d.str(d.tables[0].due), "1 500 L");
    // A second snapshot REPLACES the first.
    assert_eq!(d.apply(b"T\tx\tPENDING\t0\t0\tp\t\t\t\n"), 1);
    assert_eq!((d.nt, d.nl, d.ntab), (1, 0, 0));
    // Garbage is counted, not drawn.
    d.apply(b"T\tx\tPENDING\tnot-a-number\t0\tp\t\t\t\nL\t1\t2\tno ticket before me\t\nZ\n");
    assert_eq!((d.nt, d.bad), (0, 3));
}

#[test]
fn a_feed_bigger_than_the_pools_is_counted() {
    let mut f = String::new();
    for i in 0..(feed::MAX_TICKETS + 5) {
        f += &format!("T\tid{i}\tPENDING\t{NOW}\t0\tp\t\t\t\nL\t1\t2\tDish\t\n");
    }
    let mut r = Rig::new(390, 844);
    r.b.session(true, true, false, Role::Kitchen);
    r.b.feed(f.as_bytes());
    assert_eq!((r.b.data.nt, r.b.data.dropped), (feed::MAX_TICKETS, 5 + 5));
    r.frame();
    // Virtualised: 96 tickets, but only the visible cards are nodes.
    assert!(r.scene.len() < 60, "{} nodes", r.scene.len());
    r.b.wheel(1_000_000);
    r.frame();
    assert!(r.cmd.texts.iter().any(|t| t.contains("+10")), "the cut is drawn: {:?}", r.cmd.texts.last());
}

#[test]
fn the_pass_shows_columns_and_one_bump_per_ticket() {
    let mut r = Rig::new(390, 844).signed(true, true);
    r.b.lang = Lang::En;
    r.frame();
    assert!(r.cmd.texts.iter().any(|t| t == "New 1"), "{:?}", &r.cmd.texts[..12]);
    assert!(r.cmd.texts.iter().any(|t| t == "Accept"));
    assert!(r.cmd.texts.iter().any(|t| t.starts_with("#1111 · Table 4 · 2 min")));
    r.tap_tour("kitchen.bump");
    let i = r.b.take_intent();
    let Some(Intent::Bump { order, bump }) = i else { panic!("{i:?}") };
    assert_eq!((r.b.data.str(order), bump.action()), ("ord_aaaa1111", "confirm"));
    r.tap_tour("kitchen.seen");
    assert!(matches!(r.b.take_intent(), Some(Intent::Seen { .. })));
}

#[test]
fn ready_hands_over_except_a_delivery() {
    assert_eq!(Bump::of(Status::Ready, model::Where::Table), Some(Bump::Collected));
    assert_eq!(Bump::of(Status::Ready, model::Where::Delivery), None);
    assert_eq!(Bump::of(Status::Delivered, model::Where::Pickup), None);
    let mut r = Rig::new(390, 844).signed(true, false);
    r.b.tab = TAB_READY;
    r.frame();
    let bumps = r.scene.nodes().iter().filter(|n| n.tour == "kitchen.bump").count();
    assert_eq!(bumps, 1, "two READY tickets, the delivery has no button");
}

#[test]
fn station_filter_narrows_tickets_and_lines() {
    let mut r = Rig::new(390, 844).signed(true, false);
    r.b.lang = Lang::En;
    r.frame();
    assert_eq!(cards::station_count(&r.b, 0), 4);
    assert_eq!(cards::station_count(&r.b, 3), 1, "only the lemonade is the bar's");
    assert!(r.scene.find_tour("kitchen.station").is_some());
    assert_eq!(r.b.station, 0, "All is the default");
    let chips: Vec<_> = r.scene.nodes().iter().filter(|n| matches!(n.act, Act::Station(_))).map(|n| n.rect).collect();
    r.tap(chips[1].x + 5, chips[1].y + 5);
    assert_eq!(r.b.station, 1);
    assert!(r.cmd.texts.iter().any(|t| t == "Salmon nigiri"));
    assert!(!r.cmd.texts.iter().any(|t| t == "Miso soup"), "the kitchen's line is hidden at the sushi station");
}

#[test]
fn tables_tab_and_caps() {
    let mut r = Rig::new(390, 844).signed(false, true);
    r.frame();
    assert_eq!(r.b.tab, TAB_TABLES, "a waiter without the pass lands on the tables");
    assert!(r.scene.find_tour("kitchen.bump").is_none());
    r.tap_tour("room.table");
    let i = r.b.take_intent();
    let Some(Intent::Table { sitting }) = i else { panic!("{i:?}") };
    assert_eq!(r.b.data.str(sitting), "sit_1");
    let mut none = Rig::new(390, 844).signed(false, false);
    none.frame();
    assert!(none.scene.nodes().iter().all(|n| !matches!(n.act, Act::Tab(_))));
}

#[test]
fn same_state_same_frame_and_language_changes_it() {
    let mut r = Rig::new(390, 844).signed(true, true);
    let a = r.frame();
    assert_eq!(a, r.frame(), "a redraw is byte-identical (the context-loss restore relies on it)");
    r.tap_tour("hud.lang");
    assert_eq!(r.b.take_intent(), Some(Intent::Lang(Lang::En)));
    assert_ne!(a, r.cmd.hash());
    r.b.dark = false;
    assert_ne!(r.cmd.hash(), r.frame(), "the light palette is a different frame");
}

#[test]
fn every_tappable_node_is_at_least_44px_at_every_width() {
    for w in [320, 360, 390, 414, 768, 1024, 1920] {
        for tab in [TAB_NEW, TAB_PREPARING, TAB_READY, TAB_TABLES] {
            let mut r = Rig::new(w, 800).signed(true, true);
            r.b.tab = tab;
            r.frame();
            assert_eq!(r.scene.too_small(crate::ui::TAP), 0, "w={w} tab={tab}");
            assert_eq!((r.cmd.overflow, r.scene.dropped), (0, 0));
        }
        let mut r = Rig::new(w, 800);
        r.frame();
        assert_eq!(r.scene.too_small(crate::ui::TAP), 0, "sign-in at w={w}");
    }
}

#[test]
fn sign_in_focuses_fields_and_submits() {
    let mut r = Rig::new(390, 844);
    r.frame();
    for a in ["login.email", "login.password", "login.submit", "login.claimToggle", "hud.lang", "hud.sync"] {
        assert!(r.scene.find_tour(a).is_some(), "{a}");
    }
    assert!(r.scene.find_tour("login.code").is_none());
    r.tap_tour("login.email");
    assert_eq!(r.b.take_intent(), Some(Intent::Focus(F_EMAIL)));
    assert!(r.b.fields[F_EMAIL as usize].set("elira@venue.al".as_bytes()));
    r.tap_tour("login.password");
    assert_eq!(r.b.take_intent(), Some(Intent::Focus(F_PASSWORD)));
    r.b.fields[F_PASSWORD as usize].set(b"s3cret");
    r.frame();
    assert!(r.cmd.texts.iter().any(|t| t == "elira@venue.al"));
    assert!(r.cmd.texts.iter().any(|t| t == "••••••"), "a password is drawn as bullets");
    assert!(!r.cmd.texts.iter().any(|t| t.contains("s3cret")), "and never as itself");
    r.tap(5, 300);
    assert_eq!(r.b.take_intent(), Some(Intent::Blur), "a tap elsewhere closes the keyboard");
    r.tap_tour("login.claimToggle");
    assert!(r.scene.find_tour("login.code").is_some());
    r.tap_tour("login.submit");
    assert_eq!(r.b.take_intent(), Some(Intent::Submit { claim: true }));
}

#[test]
fn scrolling_is_clamped_to_the_content() {
    let mut f = String::new();
    for i in 0..30 {
        f += &format!("T\tid{i}\tPENDING\t{NOW}\t0\tp\t\t\t\nL\t1\t2\tDish\t\n");
    }
    let mut r = Rig::new(390, 844);
    r.b.session(true, true, false, Role::Kitchen);
    r.b.feed(f.as_bytes());
    r.frame();
    r.b.wheel(-50);
    assert_eq!(r.b.scroll[0], 0);
    r.b.wheel(999_999);
    assert_eq!(r.b.scroll[0], r.b.max_scroll());
    assert!(r.b.max_scroll() > 0);
    let top = r.scene.find_tour("kitchen.bump").unwrap().rect;
    r.frame();
    assert_ne!(top, r.scene.find_tour("kitchen.bump").unwrap().rect, "the list moved");
}

#[test]
fn ages_turn_warn_then_late() {
    assert_eq!((model::age_class(9), model::age_class(10), model::age_class(20)), (0, 1, 2));
    assert_eq!(model::age_min(NOW + 5, NOW), 0, "a clock behind the ticket is not a negative age");
}

/// The order-status words are the console's: a CHECKED copy (admin/i18n.js `st` blocks for
/// sq/en/uk, admin/i18n-ru.js for ru), and the statuses are the kernel's twelve.
#[test]
fn status_words_match_the_console_and_the_kernel() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let admin = std::fs::read_to_string(format!("{root}/workers/api/public/admin/i18n.js")).unwrap();
    let ru = std::fs::read_to_string(format!("{root}/workers/api/public/admin/i18n-ru.js")).unwrap();
    let blocks = |s: &str| -> Vec<String> { s.split("st:{").skip(1).map(|b| b.split('}').next().unwrap().to_string()).collect() };
    let a = blocks(&admin);
    let r = blocks(&ru);
    assert_eq!((a.len(), r.len()), (3, 1));
    for (lang, block) in [(Lang::Sq, &a[0]), (Lang::En, &a[1]), (Lang::Uk, &a[2]), (Lang::Ru, &r[0])] {
        for (key, st) in Status::KNOWN {
            let want = block.split(&format!("{key}:'")).nth(1).and_then(|x| x.split('\'').next()).unwrap_or("<missing>");
            assert_eq!(st.word(lang), want, "{key} in {}", lang.code());
        }
    }
    let fsm = std::fs::read_to_string(format!("{root}/crates/dowiz-core/src/order_machine.rs")).unwrap();
    for (key, _) in Status::KNOWN {
        assert!(fsm.contains(&format!("=> \"{key}\"")), "{key} is not a kernel status");
    }
}

/// Text on its background reads at WCAG AA (4.5:1) in both palettes (the design gate, as a test).
#[test]
fn palettes_meet_contrast() {
    fn lum(c: u32) -> f64 {
        let ch = |s: u32| {
            let v = ((c >> s) & 255) as f64 / 255.0;
            if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * ch(24) + 0.7152 * ch(16) + 0.0722 * ch(8)
    }
    let ratio = |a: u32, b: u32| {
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    };
    for p in [DARK, LIGHT] {
        for (fg, bg, what) in [
            (p.fg, p.surface, "text"), (p.muted, p.surface, "muted"), (p.fg, p.surface2, "plain button"),
            (p.on_accent, p.accent, "primary"), (p.on_tone, p.success, "ready"), (p.on_tone, p.warning, "badge"),
            (p.bg, p.fg, "toast / selected chip"), (p.fg, p.bg, "page"),
        ] {
            assert!(ratio(fg, bg) >= 4.5, "{what}: {:.2}", ratio(fg, bg));
        }
    }
}

#[test]
fn a_stored_session_is_restored_without_the_login_form() {
    let mut r = Rig::new(390, 844);
    r.b.restore();
    let restoring = r.frame();
    assert!(r.b.restoring && r.b.signed_in);
    assert!(r.scene.find_tour("login.email").is_none(), "a signed-in person never sees the login form while the page loads");
    assert!(r.scene.find_tour("room.role").is_none(), "no role is shown before the session says which");
    r.b.session(true, true, true, Role::CounterManager);
    let restored = r.frame();
    assert!(!r.b.restoring, "the real session ends the restoring state");
    assert!(r.scene.find_tour("room.role").is_some());
    assert_ne!(restoring, restored);
    let mut out = Rig::new(390, 844);
    out.b.restore();
    out.b.session(false, false, false, Role::Waiter);
    out.frame();
    assert!(!out.b.restoring && out.scene.find_tour("login.email").is_some(), "an expired session falls back to the login form");
}
