//! The table sheet (CV1b), the theme button and the venue's ticket-age thresholds, driven the way
//! the host drives them: rows in -> draw -> tap -> intent.
use super::model;
use super::testrig::{Rig, NOW};
use super::*;
use crate::lang::Str;

/// A word's wire number: its place in `Str::ALL` (room/canvas/feed.js `W` sends the same).
fn w(s: Str) -> usize {
    Str::ALL.iter().position(|x| *x == s).unwrap()
}

/// A sheet the way room/canvas/table.js builds one for a two-round table.
fn rows() -> String {
    let mut f = String::new();
    f += &format!("H\t1\t{}\t4\n", w(Str::KTable));
    f += "h\t\t#1\tPREPARING\n";
    f += &format!("i\t2× Salmon nigiri\t1 200 L\t\n");
    f += &format!("i\t1× Miso\t300 L\t{}\n", w(Str::Comped));
    f += &format!("b\t3\t0\t\t−\tn\tround.less\nb\t4\t0\t\t+\tn\tround.more\nb\t5\t0\t{}\t\tn\tround.remove\n", w(Str::Remove));
    f += &format!("k\t{}\t1 500 L\t0\nk\t{}\t1 200 L\t1\n", w(Str::Total), w(Str::Owed));
    f += &format!("B\t7\tr1\t{}\t\tp\tround.pay\n", w(Str::Take));
    f += &format!("p\t1\t{}\t\n", w(Str::GuestRound));
    f += &format!("f\t5\td\t{}\tpay.amount\n", w(Str::Amount));
    f += "-\n";
    f
}

#[test]
fn the_sheet_reads_its_rows_and_counts_the_rest() {
    let mut r = Rig::new(390, 844).signed(false, true);
    assert_eq!(r.b.sheet(rows().as_bytes()), 13);
    assert!(r.b.ts.open);
    assert_eq!((r.b.ts.bad, r.b.ts.dropped), (0, 0));
    assert_eq!(r.b.fields[5].kind, crate::field::Kind::Decimal, "a `d` field gets the decimal keyboard");
    // Nonsense is counted, never drawn; a field outside the sheet's range is refused.
    r.b.sheet(b"H\t1\t\tx\nZ\tq\nf\t2\tt\t\tpay.amount\nb\t1\n");
    assert_eq!((r.b.ts.n, r.b.ts.bad), (1, 3));
    // An empty feed closes it.
    r.b.sheet(b"");
    assert!(!r.b.ts.open);
}

#[test]
fn a_sheet_control_hands_back_its_act_and_argument() {
    let mut r = Rig::new(390, 844).signed(false, true);
    r.b.lang = Lang::En;
    r.b.sheet(rows().as_bytes());
    r.frame();
    assert!(r.cmd.texts.iter().any(|t| t == "Table 4"), "the header: {:?}", &r.cmd.texts[..8]);
    assert!(r.cmd.texts.iter().any(|t| t == "1× Miso · Comped"));
    assert!(r.cmd.texts.iter().any(|t| t == "#1 · Cooking"), "the status word is the board's, in its language");
    assert!(r.cmd.texts.iter().any(|t| t == "Take payment"));
    assert!(r.scene.find_tour("kitchen.board").is_none(), "the board is not drawn under the sheet");
    assert_eq!(r.scene.too_small(crate::ui::TAP), 0, "every sheet control is at least 44 px");
    r.tap_tour("round.pay");
    let Some(Intent::Sheet { act, arg }) = r.b.take_intent() else { panic!() };
    assert_eq!((act, r.b.ts.str(arg)), (7, "r1"));
    r.tap_tour("nav.back");
    assert!(matches!(r.b.take_intent(), Some(Intent::Sheet { act: 1, .. })));
    r.tap_tour("pay.amount");
    assert_eq!(r.b.take_intent(), Some(Intent::Focus(5)));
    r.b.key_enter();
    assert_eq!(r.b.take_intent(), Some(Intent::SheetEnter(5)), "Enter submits the focused field's form");
    r.b.session(false, false, false, Role::Waiter);
    assert!(!r.b.ts.open, "signing out closes the sheet");
}

#[test]
fn a_long_sheet_scrolls_and_draws_only_what_is_on_screen() {
    let mut r = Rig::new(390, 844).signed(false, true);
    let mut f = format!("H\t1\t\tMenu\n");
    for i in 0..150 {
        f += &format!("b\t20\tp{i}\t\tDish {i} · 900 L\tn\tmenu.dish\n-\n");
    }
    assert_eq!(r.b.sheet(f.as_bytes()), 301);
    r.frame();
    assert!(r.scene.len() < 60, "virtualised: {} nodes", r.scene.len());
    let first = r.scene.find_tour("menu.dish").unwrap().rect;
    r.b.wheel(600);
    r.frame();
    assert_eq!(r.b.scroll[tsheet::SCROLL], 600);
    assert_ne!(first, r.scene.find_tour("menu.dish").unwrap().rect, "the sheet moved");
    assert_eq!(r.b.scroll[r.b.tab as usize & 3], 0, "the board's own scroll is untouched");
    // The same view again (a reload) keeps the place; another view starts at the top.
    r.b.sheet(f.as_bytes());
    assert_eq!(r.b.scroll[tsheet::SCROLL], 600, "a reload of the same view keeps the scroll");
    r.b.sheet(f.replacen("Menu\n", "Menu\tpay\n", 1).as_bytes());
    assert_eq!(r.b.scroll[tsheet::SCROLL], 0, "a new view starts at the top");
}

#[test]
fn a_table_tap_asks_for_its_sheet() {
    let mut r = Rig::new(390, 844).signed(false, true);
    r.frame();
    r.tap_tour("room.table");
    let Some(Intent::Table { sitting }) = r.b.take_intent() else { panic!() };
    assert_eq!(r.b.data.str(sitting), "sit_1");
}

#[test]
fn the_theme_button_asks_and_draws_the_mode() {
    let mut r = Rig::new(390, 844).signed(true, true);
    let system = r.frame();
    r.tap_tour("hud.theme");
    assert_eq!(r.b.take_intent(), Some(Intent::Theme));
    r.b.theme = 1;
    r.b.dark = true;
    let dark = r.frame();
    r.b.theme = 2;
    r.b.dark = false;
    let light = r.frame();
    assert!(system != dark && dark != light, "each mode draws its own button");
    let mut out = Rig::new(390, 844);
    out.frame();
    assert!(out.scene.find_tour("hud.theme").is_some(), "the theme button is there before sign-in too");
}

/// THE VENUE'S THRESHOLDS, not the fixed 10 / 20: a venue that turns amber at 5 and red at 8
/// minutes sees a 6-minute ticket amber and a 9-minute ticket red.
#[test]
fn ticket_age_follows_the_venue_thresholds() {
    let mut r = Rig::new(390, 844);
    r.b.session(true, true, false, Role::Kitchen);
    r.b.set_ages(5, 8);
    r.b.tab = TAB_PREPARING;
    assert_eq!((r.b.warn_min, r.b.late_min), (5, 8));
    let t = |ago: i64| format!("T\tord_{ago}\tPREPARING\t{}\t1\tp\t\t\t\nL\t1\t2\tRamen\t\n", NOW - ago * 60_000);
    let bar = |r: &mut Rig| {
        r.frame();
        // The age bar is a 5 px wide rect at the card's left edge, in the warning or danger colour.
        let p = crate::ui::DARK;
        let mut found = 0u8;
        let ws = &r.cmd.words[..r.cmd.len];
        let mut i = 0;
        while i < ws.len() {
            let n = match ws[i] { 1 | 2 | 5 => 8, 3 => 5, _ => 1 };
            if ws[i] == 1 && ws[i + 3] == 5 {
                found = if ws[i + 6] as u32 == p.danger { 2 } else if ws[i + 6] as u32 == p.warning { 1 } else { found };
            }
            i += n;
        }
        found
    };
    r.b.feed(t(6).as_bytes());
    assert_eq!(bar(&mut r), 1, "6 min at a 5 / 8 venue is amber (the fixed 10 / 20 says nothing yet)");
    r.b.feed(t(9).as_bytes());
    assert_eq!(bar(&mut r), 2, "9 min at a 5 / 8 venue is red");
    assert_eq!(model::age_class(6, r.b.warn_min, r.b.late_min), 1);
    // Unset or nonsense falls back to 10 / 20.
    r.b.set_ages(0, 0);
    assert_eq!((r.b.warn_min, r.b.late_min), (10, 20));
    r.b.set_ages(9, 4);
    assert_eq!((r.b.warn_min, r.b.late_min), (10, 20));
}

/// The sheet's words are the room's own (room/i18n.js; the theme words courier/i18n.js): a
/// CHECKED copy, like the status words.
#[test]
fn sheet_words_match_the_room_page() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../workers/api/public");
    let room = std::fs::read_to_string(format!("{root}/room/i18n.js")).unwrap();
    let courier = std::fs::read_to_string(format!("{root}/courier/i18n.js")).unwrap();
    let pairs: &[(Str, &str)] = &[
        (Str::Back, "back"), (Str::Send, "send"), (Str::NoLines, "noLines"), (Str::Subtotal, "subtotal"),
        (Str::Discount, "discount"), (Str::Total, "total"), (Str::Owed, "owed"), (Str::AddItem, "addItem"), (Str::AddN, "addN"),
        (Str::Search, "search"), (Str::NoMatch, "noMatch"), (Str::SoldOut, "unavailable"), (Str::Remove, "remove"),
        (Str::Comp, "comp"), (Str::Comped, "comped"), (Str::MoveTable, "moveTable"), (Str::Move, "move"),
        (Str::WhyRemove, "whyRemove"), (Str::WhyComp, "whyComp"), (Str::RMistake, "reason_mistake"),
        (Str::RGuestChanged, "reason_guest_changed"), (Str::RUnavailable, "reason_unavailable"), (Str::RDropped, "reason_dropped"),
        (Str::ROther, "reason_other"), (Str::OtherText, "otherText"), (Str::NeedReason, "needReason"),
        (Str::ChangedReload, "changedReload"), (Str::Take, "take"), (Str::TakeN, "takeN"), (Str::Amount, "amount"),
        (Str::Method, "method"), (Str::Currency, "currency"), (Str::Rate, "rate"), (Str::MCash, "method_cash"),
        (Str::MCard, "method_card"), (Str::MCheque, "method_cheque"), (Str::MTransfer, "method_transfer"),
        (Str::MGiftCard, "method_gift_card"), (Str::MWallet, "method_wallet"), (Str::RateNeeded, "rateNeeded"),
        (Str::OffTheBill, "offTheBill"), (Str::FillOwed, "fillOwed"), (Str::Taken, "taken"), (Str::PaidInFull, "paidInFull"),
        (Str::BadAmount, "badAmount"), (Str::BadRate, "badRate"), (Str::BadTip, "badTip"), (Str::Tip, "tip"),
        (Str::WalletCode, "walletCode"), (Str::NeedWallet, "needWallet"), (Str::WalletNoTip, "walletNoTip"),
        (Str::MoveLines, "moveLines"), (Str::MoveLinesTo, "moveLinesTo"), (Str::PickLines, "pickLines"),
        (Str::PickRound, "pickRound"), (Str::NoTargets, "noTargets"), (Str::NotAllLines, "notAllLines"),
        (Str::MoveSitting, "moveSitting"), (Str::MoveSittingHint, "moveSittingHint"), (Str::Moved, "moved"),
        (Str::KitchenHasIt, "kitchenHasIt"), (Str::RoundPaid, "roundPaid"), (Str::AlreadyThere, "alreadyThere"),
        (Str::NotHere, "notHere"), (Str::GuestRound, "guestRound"), (Str::GuestConfirm, "guestConfirm"),
        (Str::StopReject, "guestReject"), (Str::GuestConfirmed, "guestConfirmed"), (Str::GuestRejected, "guestRejected"),
        (Str::QueuedSaved, "queuedSaved"), (Str::QueueFull, "queueFull"), (Str::QueueNoStore, "queueNoStore"),
        (Str::MenuFailed, "menuFailed"), (Str::NoSlug, "noSlug"),
    ];
    let theme: &[(Str, &str)] = &[(Str::ThemeSystem, "themeSystem"), (Str::ThemeDark, "themeDark"), (Str::ThemeLight, "themeLight")];
    let block = |src: &str, l: Lang| -> String {
        let start = src.find(&format!("\n  {}: {{", l.code())).unwrap_or_else(|| panic!("no {} block", l.code()));
        let rest = &src[start + 6..];
        rest[..rest.find("\n  },").unwrap_or(rest.len())].to_string()
    };
    let value = |b: &str, key: &str| -> String {
        let at = b.find(&format!(" {key}: '")).or_else(|| b.find(&format!("\t{key}: '"))).unwrap_or_else(|| panic!("no {key}"));
        let v = &b[at + key.len() + 4..];
        let mut out = String::new();
        let mut esc = false;
        for c in v.chars() {
            match (esc, c) {
                (true, c) => { out.push(c); esc = false; }
                (false, '\\') => esc = true,
                (false, '\'') => break,
                (false, c) => out.push(c),
            }
        }
        out.replace(" {n}", "").replace(" {a}", "")
    };
    for l in Lang::ALL {
        let rb = block(&room, l);
        for (s, key) in pairs {
            assert_eq!(l.s(*s), value(&rb, key), "{key} in {}", l.code());
        }
        let cb = block(&courier, l);
        for (s, key) in theme {
            assert_eq!(l.s(*s), value(&cb, key), "{key} in {}", l.code());
        }
    }
}
