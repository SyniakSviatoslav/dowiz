//! THE BOARD'S FRAME: header, sign-in, tabs and the toast. The lists are `cards.rs`.
//!
//! Tour anchors are the room's and the kitchen board's own words (learn lessons W1, W2 and the
//! owner's kitchen lesson): `hud.sync`, `hud.lang`, `room.refresh`, `room.signout`, `room.role`,
//! `login.*`, `kitchen.station`, `kitchen.board`, `kitchen.seen`, `kitchen.bump`, `room.table`.

use super::cards;
use super::{Board, Role, Sync, F_CODE, F_EMAIL, F_PASSWORD, TAB_NEW, TAB_PREPARING, TAB_READY, TAB_TABLES};
use crate::field::MAX;
use crate::geom::Rect;
use crate::lang::Str;
use crate::layout;
use crate::scene::{Act, ROOT};
use crate::ui::{line_h, para, Buf, Look, Ui, TAP};

pub const HUD: i32 = 56;
pub const PAD: i32 = 12;

pub fn draw(b: &mut Board, ui: &mut Ui) {
    ui.cmd.clear();
    ui.scene.clear();
    let full = Rect::new(0, 0, b.w.max(1), b.h.max(1));
    ui.fill(full, 0, ui.pal.bg);
    let root = ui.scene.push(ROOT, "", full, Act::None);
    let (top, rest) = full.split_top(HUD);
    hud(b, ui, root, top);
    if b.signed_in {
        board(b, ui, root, rest);
    } else {
        login(b, ui, root, rest);
    }
    if b.signed_in && b.ask.is_some() {
        super::sheet::draw(b, ui, root);
    }
    toast(b, ui, root);
}

fn hud(b: &Board, ui: &mut Ui, root: u16, r: Rect) {
    let l = b.lang;
    ui.fill(r, 0, ui.pal.surface);
    ui.fill(Rect::new(r.x, r.bottom() - 1, r.w, 1), 0, ui.pal.line);
    let row = Rect::new(r.x + 6, r.y + 6, r.w - 12, TAP);
    let mut right = row.right();
    let mut place = |ui: &mut Ui, w: i32, label: &str, tour: &'static str, act: Act| {
        let rr = Rect::new(right - w, row.y, w, TAP);
        ui.button(root, rr, label, tour, act, Look::Plain);
        right -= w + 6;
    };
    if b.signed_in {
        let so = l.s(Str::SignOut);
        let w = (ui.width(so, 15, 600) + 24).max(TAP);
        place(ui, w, so, "room.signout", Act::SignOut);
        place(ui, TAP, "↻", "room.refresh", Act::Refresh);
    }
    place(ui, TAP, l.badge(), "hud.lang", Act::Lang);
    let (word, tone) = match b.sync {
        Sync::Live => (Str::Live, ui.pal.success),
        Sync::Polling => (Str::Polling, ui.pal.warning),
        Sync::Offline => (Str::Offline, ui.pal.warning),
    };
    let label = l.s(word);
    let w = (ui.width(label, 13, 600) + 34).min((right - row.x - 6).max(0));
    let chip = Rect::new(row.x, row.y + 8, w, 28);
    ui.fill(chip, 14, ui.pal.surface2);
    ui.fill(Rect::new(chip.x + 10, chip.y + 10, 8, 8), 4, tone);
    ui.line(label, Rect::new(chip.x + 24, chip.y, chip.w - 28, chip.h), 13, 600, ui.pal.fg, 0);
    ui.scene.push(root, "hud.sync", chip, Act::None);
    if b.signed_in && !b.restoring {
        let role = l.s(match b.role {
            Role::Waiter => Str::Waiter,
            Role::CounterManager => Str::CounterManager,
            Role::Kitchen => Str::Kitchen,
            Role::Owner => Str::Owner,
        });
        let x = chip.right() + 8;
        let rw = (ui.width(role, 13, 500) + 20).min(right - x - 6);
        if rw > 30 {
            let rr = Rect::new(x, chip.y, rw, 28);
            ui.cmd.stroke(rr.x, rr.y, rr.w, rr.h, 14, ui.pal.line, 1);
            ui.line(role, rr.inset(4), 13, 500, ui.pal.muted, 2);
            ui.scene.push(root, "room.role", rr, Act::None);
        }
    }
}

fn login(b: &Board, ui: &mut Ui, root: u16, area: Rect) {
    let l = b.lang;
    let w = (area.w - 32).clamp(0, 420);
    let x = area.x + (area.w - w) / 2;
    let mut y = area.y + 24;
    ui.text("dowiz", x, y, 28, 700, ui.pal.fg);
    y += 44;
    let muted = ui.pal.muted;
    y += para(ui, l.s(Str::LoginLine), Rect::new(x, y, w, 0), 15, 400, muted, false) + 16;
    let fields: &[(u8, Str, &'static str)] = if b.claiming {
        &[(F_EMAIL, Str::Email, "login.email"), (F_CODE, Str::ClaimCode, "login.code"), (F_PASSWORD, Str::Password, "login.password")]
    } else {
        &[(F_EMAIL, Str::Email, "login.email"), (F_PASSWORD, Str::Password, "login.password")]
    };
    for &(f, word, tour) in fields {
        ui.line(l.s(word), Rect::new(x, y, w, 20), 14, 600, ui.pal.muted, 0);
        y += 24;
        field_box(b, ui, root, f, Rect::new(x, y, w, 52), tour);
        y += 52 + 16;
    }
    let submit = l.s(if b.claiming { Str::Claim } else { Str::SignIn });
    ui.button(root, Rect::new(x, y, w, 56), submit, "login.submit", Act::Submit, Look::Primary);
    y += 68;
    let toggle = l.s(if b.claiming { Str::HaveAccount } else { Str::HaveCode });
    ui.button(root, Rect::new(x, y, w, 48), toggle, "login.claimToggle", Act::ClaimToggle, Look::Plain);
}

/// A text field: its box, its value (bullets for a password), a caret when focused, its node.
pub(super) fn field_box(b: &Board, ui: &mut Ui, parent: u16, f: u8, r: Rect, tour: &'static str) {
    let focused = b.focus == Some(f);
    ui.fill(r, 12, ui.pal.surface);
    let (c, lw) = if focused { (ui.pal.accent, 2) } else { (ui.pal.line, 1) };
    ui.cmd.stroke(r.x, r.y, r.w, r.h, 12, c, lw);
    let mut buf = [0u8; MAX * 3];
    let shown = b.fields[f as usize].shown(&mut buf);
    let inner = r.inset(14);
    ui.line(shown, inner, 17, 400, ui.pal.fg, 0);
    if focused {
        let tw = ui.width(shown, 17, 400).min(inner.w);
        ui.fill(Rect::new(inner.x + tw + 1, inner.y + (inner.h - line_h(17)) / 2, 2, line_h(17)), 0, ui.pal.accent);
    }
    ui.scene.push(parent, tour, r, Act::Field(f));
}

fn board(b: &mut Board, ui: &mut Ui, root: u16, area: Rect) {
    let l = b.lang;
    if !b.can_pass && !b.can_floor {
        let c = ui.pal.muted;
        para(ui, l.s(if b.restoring { Str::Loading } else { Str::NoAccess }), Rect::new(area.x + 24, area.y + 48, area.w - 48, 0), 17, 500, c, true);
        return;
    }
    let mut tabs = [0u8; 4];
    let mut n = 0;
    for t in [TAB_NEW, TAB_PREPARING, TAB_READY, TAB_TABLES] {
        if b.tab_allowed(t) {
            tabs[n] = t;
            n += 1;
        }
    }
    let (row, rest) = area.split_top(TAP + 2 * 8);
    let mut cells = [Rect::ZERO; 4];
    layout::columns(row.inset(8), n, 6, &mut cells);
    for (k, &t) in tabs[..n].iter().enumerate() {
        let (word, count) = match t {
            TAB_NEW => (Str::ColNew, cards::count(b, t)),
            TAB_PREPARING => (Str::ColPreparing, cards::count(b, t)),
            TAB_READY => (Str::ColReady, cards::count(b, t)),
            _ => (Str::Tables, b.data.ntab),
        };
        let mut label: Buf<64> = Buf::new();
        label.push(l.s(word)).push(" ").num(count as u64);
        ui.button(root, cells[k], label.as_str(), "", Act::Tab(t), Look::Chip(b.tab == t));
    }
    let list = if b.tab == TAB_TABLES {
        rest
    } else {
        let (srow, list) = rest.split_top(TAP + 8);
        stations(b, ui, root, srow);
        list
    };
    b.list = list;
    if b.tab == TAB_TABLES {
        cards::tables(b, ui, root, list);
    } else {
        cards::tickets(b, ui, root, list);
    }
}

fn stations(b: &Board, ui: &mut Ui, root: u16, r: Rect) {
    let row = Rect::new(r.x + 8, r.y, r.w - 16, TAP);
    let holder = ui.scene.push(root, "kitchen.station", row, Act::None);
    let mut cells = [Rect::ZERO; 4];
    layout::columns(row, 4, 6, &mut cells);
    for (s, word) in super::model::STATIONS.iter().enumerate() {
        let mut label: Buf<48> = Buf::new();
        label.push(b.lang.s(*word)).push(" ").num(cards::station_count(b, s as u8) as u64);
        ui.button(holder, cells[s], label.as_str(), "", Act::Station(s as u8), Look::Chip(b.station == s as u8));
    }
}

fn toast(b: &Board, ui: &mut Ui, root: u16) {
    let s = b.toast();
    if s.is_empty() {
        return;
    }
    let r = Rect::new(PAD, b.h - 16 - 52, b.w - 2 * PAD, 52);
    ui.fill(r, 14, ui.pal.fg);
    ui.line(s, r.inset(14), 15, 600, ui.pal.bg, 2);
    ui.scene.push(root, "", r, Act::None);
}
