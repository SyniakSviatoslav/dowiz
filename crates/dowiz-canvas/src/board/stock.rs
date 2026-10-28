//! SHEAF S2: THE OFFLINE STOCK SHEET (operator rule "no backend without UI": the tablet's writer
//! `bw_tab_record` gets its screen). Maximally simple: which movement (received, wasted, counted),
//! which item (stepped through the tablet's own shelf -- no typing a name that is not a supply),
//! how many. Save hands the host one intent; the host writes it to the tablet's log with no
//! network (`tablet.js` `record`) and syncs through the glue when it can.

use super::model::MOVES;
use super::view::{field_box, PAD};
use super::{Board, Intent, F_QTY};
use crate::geom::Rect;
use crate::lang::Str;
use crate::scene::Act;
use crate::ui::{para, Buf, Look, Ui, TAP};

/// The "+ Stock" row above the list.
pub(super) fn opener(b: &Board, ui: &mut Ui, root: u16, area: Rect) -> Rect {
    let (row, rest) = area.split_top(TAP + 8);
    let mut label: Buf = Buf::new();
    label.push("+ ").push(b.lang.s(Str::Stock));
    ui.button(root, Rect::new(row.x + 8, row.y, row.w - 16, TAP), label.as_str(), "", Act::StockOpen, Look::Plain);
    rest
}

pub(super) fn draw(b: &Board, ui: &mut Ui, root: u16) {
    if !b.stock_open {
        return;
    }
    let l = b.lang;
    let full = Rect::new(0, 0, b.w, b.h);
    ui.fill(full, 0, 0x0000_0099);
    ui.scene.push(root, "", full, Act::SheetClose);
    let ph = 380.min(b.h);
    let panel = Rect::new(0, b.h - ph, b.w, ph + 16);
    ui.fill(panel, 16, ui.pal.surface);
    let p = ui.scene.push(root, "", panel, Act::Block);
    let x = PAD + 4;
    let w = b.w - 2 * x;
    let mut y = panel.y + 16;
    let fg = ui.pal.fg;
    ui.line(l.s(Str::Stock), Rect::new(x, y, w, 28), 20, 700, fg, 0);
    y += 36;
    let third = (w - 12) / 3;
    for (k, word) in MOVES.iter().enumerate() {
        let r = Rect::new(x + k as i32 * (third + 6), y, third, TAP);
        ui.button(p, r, l.s(*word), "", Act::StockKind(k as u8), Look::Chip(b.stock_kind == k as u8));
    }
    y += TAP + 12;
    if b.data.ni == 0 {
        let muted = ui.pal.muted;
        para(ui, l.s(Str::NoItems), Rect::new(x, y, w, 0), 15, 500, muted, false);
    } else {
        let it = b.data.items[(b.stock_item as usize).min(b.data.ni - 1)];
        ui.button(p, Rect::new(x, y, TAP, TAP), "‹", "", Act::StockStep(false), Look::Plain);
        ui.button(p, Rect::new(x + w - TAP, y, TAP, TAP), "›", "", Act::StockStep(true), Look::Plain);
        let mut name: Buf = Buf::new();
        name.push(b.data.str(it.item)).push(" · ").push(b.data.str(it.on_hand));
        ui.line(name.as_str(), Rect::new(x + TAP + 8, y, w - 2 * TAP - 16, TAP), 16, 600, fg, 1);
    }
    y += TAP + 12;
    let muted = ui.pal.muted;
    ui.line(l.s(Str::Qty), Rect::new(x, y, w, 20), 14, 600, muted, 0);
    y += 24;
    field_box(b, ui, p, F_QTY, Rect::new(x, y, w, 52), "");
    y += 52 + 16;
    let half = (w - 8) / 2;
    ui.button(p, Rect::new(x, y, half, TAP + 8), l.s(Str::Close), "", Act::SheetClose, Look::Plain);
    ui.button(p, Rect::new(x + half + 8, y, w - half - 8, TAP + 8), l.s(Str::Save), "", Act::StockSave, Look::Primary);
}

impl Board {
    /// What a tap on the sheet means.
    pub(super) fn stock_tap(&mut self, act: Act) {
        match act {
            Act::StockOpen => {
                self.stock_open = true;
                self.fields[F_QTY as usize].clear();
                if self.stock_item as usize >= self.data.ni {
                    self.stock_item = 0;
                }
            }
            Act::StockKind(k) => self.stock_kind = k.min(MOVES.len() as u8 - 1),
            Act::StockStep(next) if self.data.ni > 0 => {
                let n = self.data.ni as u16;
                self.stock_item = if next { (self.stock_item + 1) % n } else { (self.stock_item + n - 1) % n };
            }
            Act::StockSave => self.stock_save(),
            _ => {}
        }
    }

    /// Save: a whole quantity (more than zero, or zero for a count) on an item of the shelf.
    pub(super) fn stock_save(&mut self) {
        let q = self.fields[F_QTY as usize].value().trim();
        let whole = !q.is_empty() && q.len() <= 9 && q.bytes().all(|c| c.is_ascii_digit());
        let ok = whole && (self.stock_kind == 2 || q.bytes().any(|c| c != b'0'));
        if !ok || self.data.ni == 0 {
            let w = self.lang.s(Str::QtyNeeded);
            self.set_toast(w.as_bytes());
            return;
        }
        let item = self.data.items[(self.stock_item as usize).min(self.data.ni - 1)].item;
        self.stock_open = false;
        if self.focus.take().is_some() {
            self.emit(Intent::Blur);
        }
        self.emit(Intent::Stock { kind: self.stock_kind, item });
    }
}

#[cfg(test)]
#[path = "stock/tests.rs"]
mod tests;
