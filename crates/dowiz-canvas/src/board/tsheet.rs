//! THE TABLE SHEET (CV1b): one open table -- its rounds, their lines and money, and every action
//! the old room page offers for it (room/screens.js `renderSitting`, room/sheet.js, pay.js,
//! transfer.js, guest.js, menu.js) -- drawn on the canvas over the board.
//!
//! WHO DECIDES WHAT. The rules of what a person may do with a round are the room's own pure
//! rules (room/logic.js `actionsFor`, `canTransfer`, `canMoveSitting`, `owed`, ...), one copy, in
//! JavaScript; money is formatted by lib/money.js. So the host (room/canvas/table.js) decides
//! WHAT the sheet holds and hands it over as rows, and this module only lays them out, draws them,
//! registers each control in the scene and turns a tap into `Intent::Sheet { act, arg }` -- the
//! code and argument the host gave that row. Rust never does money arithmetic and never decides a
//! permission; the host never holds a word: every label is a `Str` (a number into `Str::ALL`),
//! drawn in the board's language from `lang.rs`, so a missing language is a compile error.
//!
//! THE ROWS (tab-separated, one per line; `w` = a word number or empty, `t` = the host's text):
//! ```text
//! H <act> <w> <t> [view]                   the fixed header: a back button (act), the title, and
//!                                          the view's name -- a new name scrolls back to the top
//! h <w> <t> [STATUS]                       a heading, then the status's word (model.rs Status)
//! p <style 0|1|2> <w> <t>                  a paragraph: 0 muted, 1 warning, 2 success
//! k <w> <value> <strong 0|1>               a label and its value (money: already formatted)
//! i <left> <right> <w>                     an item line: "2× Salmon", its amount, a badge word
//! b <act> <arg> <w> <t> <look> <tour>      a button flowing in a row with the next `b`s
//! B <act> <arg> <w> <t> <look> <tour>      a full-width button
//! f <field 4..7> <kind t|d> <w> <tour>     a labelled text field (its value: `Board::fields`)
//! -                                        a gap
//! ```
//! `look`: p primary, g success, d danger, n plain, c chip, s chip selected. `tour` is a learn
//! anchor of the old page (room.pay, pay.submit, ...): only names in `TOURS` are kept.

use super::model::{Span, Status};
use super::view::{field_box, PAD};
use super::Board;
use crate::field::Kind;
use crate::geom::Rect;
use crate::lang::Str;
use crate::scene::Act;
use crate::ui::{wrap, Buf, Look, Ui, TAP};

pub const ARENA: usize = 16_384;
pub const ROWS: usize = 384;
/// The sheet's own scroll slot in `Board::scroll` (0..3 are the board's tabs).
pub const SCROLL: usize = 4;

/// The old page's anchors the sheet may carry (docs/learn/anchors-room.txt).
pub const TOURS: [&str; 37] = [
    "nav.back", "sitting.round", "sitting.move", "round.line", "round.sums", "round.less", "round.more", "round.remove",
    "round.comp", "round.reason", "round.reasonText", "round.add", "round.pay", "round.transfer", "round.moveSitting",
    "round.tableField", "round.tableMove", "guest.confirm", "guest.reject", "pay.owed", "pay.currency", "pay.method",
    "pay.rate", "pay.amount", "pay.tip", "pay.wallet", "pay.fill", "pay.submit", "transfer.line", "transfer.target",
    "transfer.send", "move.table", "move.submit", "menu.search", "menu.dish", "menu.less", "menu.send",
];

#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub kind: u8,
    pub word: Option<Str>,
    pub text: Span,
    pub aux: Span,
    pub act: u8,
    pub look: u8,
    pub tour: &'static str,
}

const BLANK: Row = Row { kind: 0, word: None, text: Span { off: 0, len: 0 }, aux: Span { off: 0, len: 0 }, act: 0, look: 0, tour: "" };

pub struct TSheet {
    pub open: bool,
    arena: [u8; ARENA],
    used: usize,
    pub rows: [Row; ROWS],
    pub n: usize,
    /// Rows or bytes that did not fit, and records the reader did not understand: counted.
    pub dropped: u32,
    pub bad: u32,
    /// FNV of the header's view name: when it changes, the board scrolls the sheet to the top.
    pub view: u32,
}

fn num(b: &[u8]) -> Option<usize> {
    if b.is_empty() || b.len() > 4 || !b.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some(b.iter().fold(0usize, |a, &c| a * 10 + (c - b'0') as usize))
}

impl TSheet {
    pub const fn new() -> TSheet {
        TSheet { open: false, arena: [0; ARENA], used: 0, rows: [BLANK; ROWS], n: 0, dropped: 0, bad: 0, view: 0 }
    }

    pub fn str(&self, s: Span) -> &str {
        let (a, b) = (s.off as usize, (s.off + s.len) as usize);
        self.arena.get(a..b.min(self.used)).and_then(|x| core::str::from_utf8(x).ok()).unwrap_or("")
    }

    fn put(&mut self, b: &[u8]) -> Span {
        if core::str::from_utf8(b).is_err() {
            self.bad += 1;
            return Span::default();
        }
        if self.used + b.len() > ARENA {
            self.dropped += 1;
            return Span::default();
        }
        let s = Span { off: self.used as u32, len: b.len() as u32 };
        crate::put_at(&mut self.arena, self.used, b);
        self.used += b.len();
        s
    }

    /// Replace the sheet with `feed`. Returns the rows read; `kinds` receives each field row's
    /// (field, kind) so the board can tell the keyboard what it types.
    pub fn apply(&mut self, feed: &[u8], kinds: &mut dyn FnMut(u8, Kind)) -> usize {
        self.used = 0;
        self.n = 0;
        self.dropped = 0;
        self.bad = 0;
        for rec in feed.split(|&c| c == b'\n').filter(|r| !r.is_empty()) {
            let (f, k) = super::feed::fields(rec);
            let word = |b: &[u8]| num(b).and_then(Str::at);
            let tour = |b: &[u8]| TOURS.iter().copied().find(|t| t.as_bytes() == b).unwrap_or("");
            let first = f[0].first().copied().unwrap_or(0);
            let mut r = Row { kind: first, ..BLANK };
            match (first, k) {
                (b'H', 4 | 5) => {
                    r.act = num(f[1]).unwrap_or(0) as u8;
                    r.word = word(f[2]);
                    r.text = self.put(f[3]);
                    self.view = f[4].iter().fold(0x811c_9dc5u32, |h, &c| (h ^ c as u32).wrapping_mul(0x0100_0193));
                }
                (b'h', 3 | 4) => {
                    r.word = word(f[1]);
                    r.text = self.put(f[2]);
                    // 1 + the status's place in Status::KNOWN; 0 = none.
                    r.look = Status::KNOWN.iter().position(|(k, _)| k.as_bytes() == f[3]).map_or(0, |i| i as u8 + 1);
                }
                (b'p', 4) => {
                    r.look = num(f[1]).unwrap_or(0) as u8;
                    r.word = word(f[2]);
                    r.text = self.put(f[3]);
                }
                (b'k', 4) => {
                    r.word = word(f[1]);
                    r.aux = self.put(f[2]);
                    r.look = (f[3] == b"1") as u8;
                }
                (b'i', 4) => {
                    r.text = self.put(f[1]);
                    r.aux = self.put(f[2]);
                    r.word = word(f[3]);
                }
                (b'b' | b'B', 7) => {
                    r.act = num(f[1]).unwrap_or(0) as u8;
                    r.aux = self.put(f[2]);
                    r.word = word(f[3]);
                    r.text = self.put(f[4]);
                    r.look = f[5].first().copied().unwrap_or(b'n');
                    r.tour = tour(f[6]);
                }
                (b'f', 5) => {
                    let Some(field) = num(f[1]).filter(|&x| (super::F_SHEET as usize..super::FIELDS).contains(&x)) else {
                        self.bad += 1;
                        continue;
                    };
                    r.act = field as u8;
                    kinds(field as u8, if f[2] == b"d" { Kind::Decimal } else { Kind::Text });
                    r.word = word(f[3]);
                    r.tour = tour(f[4]);
                }
                (b'-', 1) => {}
                _ => {
                    self.bad += 1;
                    continue;
                }
            }
            if self.n == ROWS {
                self.dropped += 1;
                continue;
            }
            if let Some(x) = self.rows.get_mut(self.n) {
                *x = r;
            }
            self.n += 1;
        }
        self.n
    }
}

const FLOW_GAP: i32 = 8;
const BLOCK: i32 = 52;

/// A row's label: its word, then the host's text.
fn label<'a>(b: &Board, r: &Row, out: &'a mut Buf) -> &'a str {
    if let Some(w) = r.word {
        out.push(b.lang.s(w));
        if r.text.len > 0 {
            out.push(" ");
        }
    }
    out.push(b.ts.str(r.text));
    out.as_str()
}

fn look(ui: &Ui, c: u8) -> Look {
    match c {
        b'p' => Look::Primary,
        b'g' => Look::Tone(ui.pal.success),
        b'd' => Look::Tone(ui.pal.danger),
        b'c' => Look::Chip(false),
        b's' => Look::Chip(true),
        _ => Look::Plain,
    }
}

/// Draw the sheet over `area` (everything under the header bar). Rows outside the visible body
/// are laid out (their height counts) but neither drawn nor registered.
pub fn draw(b: &mut Board, ui: &mut Ui, root: u16, area: Rect) {
    ui.fill(area, 0, ui.pal.bg);
    let (top, body) = area.split_top(TAP + 16);
    ui.fill(top, 0, ui.pal.surface);
    ui.fill(Rect::new(top.x, top.bottom() - 1, top.w, 1), 0, ui.pal.line);
    let panel = ui.scene.push(root, "", area, Act::Block);
    let rows = crate::head(&b.ts.rows, b.ts.n);
    if let Some(h) = rows.iter().find(|r| r.kind == b'H').copied() {
        let back = b.lang.s(Str::Back);
        let bw = (ui.width(back, 15, 600) + 40).max(TAP);
        let row = rows.iter().position(|r| r.kind == b'H').unwrap_or(0);
        let mut s: Buf = Buf::new();
        s.push("‹ ").push(back);
        ui.button(panel, Rect::new(top.x + 8, top.y + 8, bw, TAP), s.as_str(), "nav.back", Act::Sheet(row as u16), Look::Plain);
        let mut t: Buf = Buf::new();
        let title = label(b, &h, &mut t);
        let fg = ui.pal.fg;
        ui.line(title, Rect::new(top.x + bw + 20, top.y, top.w - bw - 28, top.h), 18, 700, fg, 0);
    }
    b.list = body;
    let inner = Rect::new(body.x + PAD, body.y, body.w - 2 * PAD, body.h);
    let holder = ui.scene.push(panel, "", body, Act::None);
    let s = SCROLL;
    let mut y = body.y + PAD - b.scroll[s];
    let mut flow_x = inner.x;
    let mut in_flow = false;
    ui.cmd.clip(body.x, body.y, body.w, body.h);
    let vis = |y: i32, h: i32| y + h >= body.y && y <= body.bottom();
    for i in 0..b.ts.n {
        let Some(r) = b.ts.rows.get(i).copied() else { break };
        if r.kind != b'b' && in_flow {
            y += TAP + FLOW_GAP;
            in_flow = false;
            flow_x = inner.x;
        }
        let mut t: Buf = Buf::new();
        match r.kind {
            b'h' => {
                if vis(y, 34) {
                    let fg = ui.pal.fg;
                    label(b, &r, &mut t);
                    if let Some((_, st)) = Status::KNOWN.get((r.look as usize).wrapping_sub(1)) {
                        t.push(" · ").push(st.word(b.lang));
                    }
                    ui.line(t.as_str(), Rect::new(inner.x, y, inner.w, 34), 18, 700, fg, 0);
                }
                y += 38;
            }
            b'p' => {
                let (fg, bg) = match r.look {
                    1 => (ui.pal.on_tone, Some(ui.pal.warning)),
                    2 => (ui.pal.on_tone, Some(ui.pal.success)),
                    _ => (ui.pal.muted, None),
                };
                let pad = if bg.is_some() { 12 } else { 0 };
                let text = label(b, &r, &mut t);
                let tr = Rect::new(inner.x + pad, y + pad, inner.w - 2 * pad, 0);
                let h = wrap(ui, text, tr, 15, 500, fg, false, false) + 2 * pad;
                if vis(y, h) {
                    if let Some(c) = bg {
                        ui.fill(Rect::new(inner.x, y, inner.w, h), 12, c);
                    }
                    wrap(ui, text, tr, 15, 500, fg, false, true);
                }
                y += h + 8;
            }
            b'k' => {
                let h = if r.look == 1 { 40 } else { 30 };
                if vis(y, h) {
                    let (muted, fg) = (ui.pal.muted, ui.pal.fg);
                    let v = b.ts.str(r.aux);
                    let px = if r.look == 1 { 22 } else { 16 };
                    let vw = ui.width(v, px, 700).min(inner.w / 2);
                    ui.line(r.word.map_or("", |w| b.lang.s(w)), Rect::new(inner.x, y, inner.w - vw - 8, h), 15, 500, muted, 0);
                    ui.line(v, Rect::new(inner.right() - vw, y, vw, h), px, 700, fg, 1);
                }
                y += h;
            }
            b'i' => {
                if vis(y, 32) {
                    let c = if r.word.is_some() { ui.pal.muted } else { ui.pal.fg };
                    let v = b.ts.str(r.aux);
                    let vw = ui.width(v, 16, 600).min(inner.w / 3);
                    t.push(b.ts.str(r.text));
                    if let Some(w) = r.word {
                        t.push(" · ").push(b.lang.s(w));
                    }
                    ui.line(t.as_str(), Rect::new(inner.x, y, inner.w - vw - 8, 32), 16, 500, c, 0);
                    ui.line(v, Rect::new(inner.right() - vw, y, vw, 32), 16, 600, c, 1);
                    ui.scene.push(holder, "round.line", Rect::new(inner.x, y, inner.w, 32), Act::None);
                }
                y += 32;
            }
            b'b' => {
                let text = label(b, &r, &mut t);
                // max then min, not clamp: clamp panics when the screen is narrower than one tap target.
                let w = (ui.width(text, 15, 600) + 28).max(TAP).min(inner.w.max(TAP));
                if in_flow && flow_x + w > inner.right() {
                    y += TAP + FLOW_GAP;
                    flow_x = inner.x;
                }
                if vis(y, TAP) {
                    let lk = look(ui, r.look);
                    ui.button(holder, Rect::new(flow_x, y, w, TAP), text, r.tour, Act::Sheet(i as u16), lk);
                }
                flow_x += w + FLOW_GAP;
                in_flow = true;
            }
            b'B' => {
                if vis(y, BLOCK) {
                    let lk = look(ui, r.look);
                    let text = label(b, &r, &mut t);
                    ui.button(holder, Rect::new(inner.x, y, inner.w, BLOCK), text, r.tour, Act::Sheet(i as u16), lk);
                }
                y += BLOCK + FLOW_GAP;
            }
            b'f' => {
                if vis(y, 80) {
                    let muted = ui.pal.muted;
                    ui.line(r.word.map_or("", |w| b.lang.s(w)), Rect::new(inner.x, y, inner.w, 20), 14, 600, muted, 0);
                    field_box(b, ui, holder, r.act, Rect::new(inner.x, y + 24, inner.w, 52), r.tour);
                }
                y += 88;
            }
            b'-' => y += 12,
            _ => {}
        }
    }
    if in_flow {
        y += TAP + FLOW_GAP;
    }
    b.content_h = y + b.scroll[s] - body.y + PAD;
    b.scroll[s] = b.scroll[s].clamp(0, b.max_scroll());
    ui.cmd.unclip();
}
