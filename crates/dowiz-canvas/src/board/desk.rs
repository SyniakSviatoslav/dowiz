//! SHEAF S2: THE "DECISION NEEDED" STRIP above the kitchen's list. Each row is one decision a
//! kitchen tablet's sync left (`dowiz_hub::glue::desk`: the last salmon sold twice, a count
//! against a sale, a movement the hub could not apply) with ONE button, Done, which closes it
//! (`Intent::Decided`). The fix itself -- an amend, a refund, a recount -- goes through the doors
//! that already exist; this strip only makes sure a person saw it. Nothing is cut silently: past
//! `SHOWN` rows the strip says how many more there are.

use super::model::decision_word;
use super::Board;
use crate::geom::Rect;
use crate::lang::Str;
use crate::scene::Act;
use crate::ui::{Buf, Look, Ui, TAP};

/// Rows drawn; the rest are counted.
pub const SHOWN: usize = 3;
const ROW: i32 = TAP + 6;

/// Draw the strip at the top of `area` and answer what is left below it.
pub(super) fn strip(b: &Board, ui: &mut Ui, root: u16, area: Rect) -> Rect {
    let n = b.data.nd.min(SHOWN);
    let more = b.data.nd - n;
    let h = 8 + 28 + n as i32 * ROW + if more > 0 { 24 } else { 0 } + 8;
    let (top, rest) = area.split_top(h);
    let inner = top.inset(8);
    ui.fill(inner, 12, ui.pal.surface2);
    ui.cmd.stroke(inner.x, inner.y, inner.w, inner.h, 12, ui.pal.warning, 2);
    let holder = ui.scene.push(root, "", inner, Act::None);
    let l = b.lang;
    let mut title: Buf = Buf::new();
    title.push(l.s(Str::Decide)).push(" ").num(b.data.nd as u64);
    ui.line(title.as_str(), Rect::new(inner.x + 12, inner.y + 4, inner.w - 24, 24), 15, 700, ui.pal.fg, 0);
    let done = l.s(Str::Done);
    let bw = (ui.width(done, 15, 600) + 28).max(TAP);
    let mut y = inner.y + 32;
    for i in 0..n {
        let d = b.data.decisions[i];
        let row = Rect::new(inner.x + 12, y, inner.w - 24, TAP);
        let mut text: Buf = Buf::new();
        text.push(l.s(decision_word(b.data.str(d.kind)))).push(": ").push(b.data.str(d.item));
        for (sep, s) in [(" ", b.data.str(d.qty)), (" · ", b.data.str(d.ctx))] {
            if !s.is_empty() {
                text.push(sep).push(s);
            }
        }
        ui.line(text.as_str(), Rect::new(row.x, row.y, (row.w - bw - 8).max(0), row.h), 14, 500, ui.pal.fg, 0);
        ui.button(holder, Rect::new(row.right() - bw, row.y, bw, TAP), done, "", Act::Decided(i as u16), Look::Plain);
        y += ROW;
    }
    if more > 0 {
        let mut s: Buf = Buf::new();
        s.push("+").num(more as u64).push(" ").push(l.s(Str::More));
        ui.line(s.as_str(), Rect::new(inner.x + 12, y, inner.w - 24, 24), 13, 600, ui.pal.muted, 0);
    }
    rest
}

#[cfg(test)]
#[path = "desk/tests.rs"]
mod tests;
