//! THE LISTS: ticket cards (the pass) and table cards (the floor), virtualised -- only the rows
//! inside the visible list are drawn or registered, so 96 tickets cost what the 5 on screen cost.

use super::model::{age_class, age_min, Bump, Col, Status, Ticket, Where};
use super::view::PAD;
use super::{Board, TAB_PREPARING, TAB_READY};
use crate::geom::Rect;
use crate::lang::Str;
use crate::layout;
use crate::scene::Act;
use crate::ui::{para, Buf, Look, Ui};

const GAP: i32 = 12;
const HEAD: i32 = 48;
const LINE: i32 = 26;
const NOTE: i32 = 20;
const BUMP: i32 = 56;
const STOP: i32 = 48;
const TABLE_H: i32 = 76;
const CARD_MIN: i32 = 340;

fn col_of(tab: u8) -> Col {
    match tab {
        TAB_PREPARING => Col::Preparing,
        TAB_READY => Col::Ready,
        _ => Col::New,
    }
}

/// Does this ticket have a line made at station `s` (0 = every station)?
fn at_station(b: &Board, t: &Ticket, s: u8) -> bool {
    s == 0 || line_range(b, t).any(|k| b.data.lines.get(k).is_some_and(|l| l.station == s))
}

fn line_range(b: &Board, t: &Ticket) -> core::ops::Range<usize> {
    let a = t.line0 as usize;
    a.min(b.data.nl)..(a + t.nlines as usize).min(b.data.nl)
}

/// Open tickets at station `s`, in any column (the station chips' counts).
pub fn station_count(b: &Board, s: u8) -> usize {
    crate::head(&b.data.tickets, b.data.nt).iter().filter(|t| t.status.column().is_some() && at_station(b, t, s)).count()
}

/// Tickets in a tab's column under the current station filter.
pub fn count(b: &Board, tab: u8) -> usize {
    let c = col_of(tab);
    crate::head(&b.data.tickets, b.data.nt).iter().filter(|t| t.status.column() == Some(c) && at_station(b, t, b.station)).count()
}

fn shown_lines(b: &Board, t: &Ticket) -> (i32, i32) {
    let (mut n, mut notes) = (0, 0);
    for k in line_range(b, t) {
        let Some(ln) = b.data.lines.get(k) else { continue };
        if b.station == 0 || ln.station == b.station {
            n += 1;
            if ln.note.len > 0 {
                notes += 1;
            }
        }
    }
    (n, notes)
}

fn ticket_h(b: &Board, t: &Ticket) -> i32 {
    let (n, notes) = shown_lines(b, t);
    let order_note = if t.note.len > 0 { NOTE } else { 0 };
    let bump = if Bump::of(t.status, t.kind).is_some() { 8 + BUMP } else { 0 };
    let stop = if stop_of(t.status).is_some() { 8 + STOP } else { 0 };
    PAD + HEAD + 8 + n * LINE + notes * NOTE + order_note + bump + stop + PAD
}

/// Reject before the kitchen accepts, cancel after (kitchen-logic.js `stopFor`); its label.
fn stop_of(s: Status) -> Option<Str> {
    match s {
        Status::Pending => Some(Str::StopReject),
        Status::Confirmed => Some(Str::StopCancel),
        _ => None,
    }
}

/// Lay out `heights` in rows of `cols` cells; draw the visible ones with `paint(index, rect)`.
/// Returns the content height. One grid for both lists.
fn grid(b: &mut Board, ui: &mut Ui, list: Rect, heights: &[i32], paint: &mut dyn FnMut(&Board, &mut Ui, usize, Rect)) {
    let inner = Rect::new(list.x + PAD, list.y, list.w - 2 * PAD, list.h);
    let cols = ((inner.w + GAP) / (CARD_MIN + GAP)).clamp(1, 4) as usize;
    let mut cells = [Rect::ZERO; 4];
    layout::columns(inner, cols, GAP, &mut cells);
    let mut total = PAD;
    for row in heights.chunks(cols) {
        total += row.iter().copied().max().unwrap_or(0) + GAP;
    }
    b.content_h = total - GAP + PAD + if b.data.dropped > 0 { 40 } else { 0 };
    let tab = b.si();
    b.scroll[tab] = b.scroll[tab].clamp(0, b.max_scroll());
    let mut y = list.y + PAD - b.scroll[tab];
    ui.cmd.clip(list.x, list.y, list.w, list.h);
    for (r, row) in heights.chunks(cols).enumerate() {
        let rh = row.iter().copied().max().unwrap_or(0);
        if y + rh >= list.y && y <= list.bottom() {
            for (c, &h) in row.iter().enumerate() {
                let cell = cells.get(c).copied().unwrap_or_default();
                paint(b, ui, r * cols + c, Rect::new(cell.x, y, cell.w, h));
            }
        }
        y += rh + GAP;
    }
    if b.data.dropped > 0 {
        let mut s: Buf = Buf::new();
        s.push("+").num(b.data.dropped as u64).push(" ").push(b.lang.s(Str::More));
        ui.line(s.as_str(), Rect::new(inner.x, y, inner.w, 32), 15, 600, ui.pal.muted, 2);
    }
    ui.cmd.unclip();
}

fn empty(b: &Board, ui: &mut Ui, list: Rect, word: Str) {
    let w = if b.loaded { word } else { Str::Loading };
    let c = ui.pal.muted;
    para(ui, b.lang.s(w), Rect::new(list.x + 24, list.y + 48, list.w - 48, 0), 17, 500, c, true);
}

pub fn tickets(b: &mut Board, ui: &mut Ui, root: u16, list: Rect) {
    let holder = ui.scene.push(root, "kitchen.board", list, Act::None);
    let c = col_of(b.tab);
    let mut idx = [0u16; super::feed::MAX_TICKETS];
    let mut hs = [0i32; super::feed::MAX_TICKETS];
    let mut n = 0;
    for (i, t) in crate::head(&b.data.tickets, b.data.nt).iter().enumerate() {
        if t.status.column() == Some(c) && at_station(b, t, b.station) {
            if let (Some(x), Some(h)) = (idx.get_mut(n), hs.get_mut(n)) {
                *x = i as u16;
                *h = ticket_h(b, t);
            }
            n += 1;
        }
    }
    if n == 0 {
        b.content_h = 0;
        return empty(b, ui, list, Str::NoTickets);
    }
    grid(b, ui, list, crate::head(&hs, n), &mut |b, ui, k, r| ticket(b, ui, holder, idx.get(k).copied().unwrap_or(0), r));
}

fn ticket(b: &Board, ui: &mut Ui, parent: u16, i: u16, r: Rect) {
    let Some(t) = b.data.tickets.get(i as usize).copied() else { return };
    let l = b.lang;
    ui.fill(r, 14, ui.pal.surface);
    ui.cmd.stroke(r.x, r.y, r.w, r.h, 14, ui.pal.line, 1);
    let age = age_min(t.start_ms, b.now_ms);
    if let Some(cls) = heat_of(b, &t) {
        let c = match cls {
            0 => ui.pal.success,
            1 => ui.pal.warning,
            _ => ui.pal.danger,
        };
        ui.fill(Rect::new(r.x, r.y + 14, 5, r.h - 28), 2, c);
    }
    let card = ui.scene.push(parent, "", r, Act::None);
    let mut head: Buf = Buf::new();
    head.push("#").push(short_id(b.data.str(t.id), &mut [0u8; 4])).push(" · ");
    match t.kind {
        Where::Table => head.push(l.s(Str::KTable)).push(" ").push(b.data.str(t.table)),
        Where::Pickup => head.push(l.s(Str::KPickup)),
        Where::Delivery => head.push(l.s(Str::KDelivery)),
    };
    head.push(" · ");
    if b.data.str(t.when).is_empty() {
        head.num(age as u64).push(" ").push(l.s(Str::KMin));
    } else {
        head.push(l.s(Str::ForTime)).push(" ").push(b.data.str(t.when));
    }
    head.push(" · ").push(l.s(if t.seen { Str::KSeen } else { Str::KUnseen }));
    let hr = Rect::new(r.x + PAD, r.y + PAD, r.w - 2 * PAD, HEAD);
    ui.button(card, hr, head.as_str(), "kitchen.seen", Act::Seen(i), if t.seen { Look::Chip(false) } else { Look::Plain });
    let mut y = hr.bottom() + 8;
    let x = r.x + PAD;
    let w = r.w - 2 * PAD;
    for k in line_range(b, &t) {
        let Some(ln) = b.data.lines.get(k).copied() else { continue };
        if b.station != 0 && ln.station != b.station {
            continue;
        }
        let mut q: Buf = Buf::new();
        q.num(ln.qty as u64).push("×");
        let qw = ui.text(q.as_str(), x, y + 3, 17, 700, ui.pal.fg) + 8;
        let tag = if b.station == 0 && ln.station != 2 { l.s(super::model::STATIONS[ln.station as usize & 3]) } else { "" };
        let tw = if tag.is_empty() { 0 } else { ui.width(tag, 13, 600) + 8 };
        ui.line(b.data.str(ln.name), Rect::new(x + qw, y, w - qw - tw, LINE), 17, 500, ui.pal.fg, 0);
        if tw > 0 {
            ui.line(tag, Rect::new(x + w - tw, y, tw, LINE), 13, 600, ui.pal.muted, 1);
        }
        y += LINE;
        if ln.note.len > 0 {
            ui.line(b.data.str(ln.note), Rect::new(x + qw, y, w - qw, NOTE), 14, 400, ui.pal.muted, 0);
            y += NOTE;
        }
    }
    if t.note.len > 0 {
        ui.line(b.data.str(t.note), Rect::new(x, y, w, NOTE), 14, 500, ui.pal.muted, 0);
        y += NOTE;
    }
    if let Some(bump) = Bump::of(t.status, t.kind) {
        let look = if bump == Bump::Ready { Look::Tone(ui.pal.success) } else { Look::Primary };
        ui.button(card, Rect::new(x, y + 8, w, BUMP), l.s(bump.label()), "kitchen.bump", Act::Bump(i), look);
        y += 8 + BUMP;
    }
    if let Some(word) = stop_of(t.status) {
        ui.button(card, Rect::new(x, y + 8, w, STOP), l.s(word), "kitchen.reject", Act::Stop(i), Look::Plain);
    }
}

/// CV8a: the ticket's heat (0 fresh, 1 warn, 2 late) -- its age against the venue's own amber and
/// red minutes -- or None while its hour is still ahead (a scheduled ticket is not yet hot).
pub fn heat_of(b: &Board, t: &Ticket) -> Option<u8> {
    b.data.str(t.when).is_empty().then(|| age_class(age_min(t.start_ms, b.now_ms), b.warn_min, b.late_min))
}

/// The last four characters of an id, upper-cased (kitchen-logic.js `shortId`).
pub(super) fn short_id<'a>(id: &str, out: &'a mut [u8; 4]) -> &'a str {
    let b = id.as_bytes();
    let tail = b.get(b.len().saturating_sub(4)..).unwrap_or(&[]);
    if !tail.is_ascii() {
        return "";
    }
    crate::put_at(out, 0, tail);
    out.make_ascii_uppercase();
    core::str::from_utf8(crate::head(out, tail.len())).unwrap_or("")
}

pub fn tables(b: &mut Board, ui: &mut Ui, root: u16, list: Rect) {
    let holder = ui.scene.push(root, "", list, Act::None);
    let n = b.data.ntab;
    if n == 0 {
        b.content_h = 0;
        return empty(b, ui, list, Str::NoTables);
    }
    let hs = [TABLE_H; super::feed::MAX_TABLES];
    grid(b, ui, list, crate::head(&hs, n), &mut |b, ui, k, r| table(b, ui, holder, k, r));
}

fn table(b: &Board, ui: &mut Ui, parent: u16, i: usize, r: Rect) {
    let Some(t) = b.data.tables.get(i).copied() else { return };
    let l = b.lang;
    ui.fill(r, 14, ui.pal.surface);
    ui.cmd.stroke(r.x, r.y, r.w, r.h, 14, ui.pal.line, 1);
    let inner = r.inset(PAD);
    let due = b.data.str(t.due);
    let mut d: Buf = Buf::new();
    d.push(l.s(Str::Due)).push(" ").push(due);
    let dw = ui.width(d.as_str(), 16, 700).min(inner.w / 2);
    let mut title: Buf = Buf::new();
    title.push(l.s(Str::KTable)).push(" ").push(b.data.str(t.table));
    ui.line(title.as_str(), Rect::new(inner.x, inner.y, inner.w - dw - 8, 26), 18, 700, ui.pal.fg, 0);
    ui.line(d.as_str(), Rect::new(inner.right() - dw, inner.y, dw, 26), 16, 700, ui.pal.fg, 1);
    let mut sub: Buf = Buf::new();
    sub.num(t.rounds as u64).push(" ").push(l.s(Str::Rounds));
    for s in crate::head(&t.st, t.nst as usize) {
        if *s != Status::Other {
            sub.push(" · ").push(s.word(l));
        }
    }
    let gw = if t.guest { ui.width(l.s(Str::GuestWaiting), 13, 700) + 20 } else { 0 };
    ui.line(sub.as_str(), Rect::new(inner.x, inner.y + 28, inner.w - gw - 8, 24), 14, 400, ui.pal.muted, 0);
    if t.guest {
        let g = Rect::new(inner.right() - gw, inner.y + 28, gw, 24);
        ui.fill(g, 12, ui.pal.warning);
        ui.line(l.s(Str::GuestWaiting), g.inset(4), 13, 700, ui.pal.on_tone, 2);
    }
    ui.scene.push(parent, "room.table", r, Act::Table(i as u16));
}
