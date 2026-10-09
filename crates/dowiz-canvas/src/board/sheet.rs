//! THE REJECT / CANCEL SHEET (admin/kitchen.js `askReason`): the reason is required, and it is
//! the one the customer reads. A scrim over the board closes it; the panel swallows its own taps.

use super::view::{field_box, PAD};
use super::{Board, F_REASON};
use crate::geom::Rect;
use crate::lang::Str;
use crate::scene::Act;
use crate::ui::{para, Buf, Look, Ui, TAP};

pub fn draw(b: &Board, ui: &mut Ui, root: u16) {
    let Some(cancel) = b.ask else { return };
    let l = b.lang;
    let full = Rect::new(0, 0, b.w, b.h);
    ui.fill(full, 0, 0x0000_0099);
    ui.scene.push(root, "", full, Act::SheetClose);
    let ph = 300.min(b.h);
    let panel = Rect::new(0, b.h - ph, b.w, ph + 16);
    ui.fill(panel, 16, ui.pal.surface);
    let p = ui.scene.push(root, "", panel, Act::Block);
    let x = PAD + 4;
    let w = b.w - 2 * x;
    let mut y = panel.y + 16;
    let word = l.s(if cancel { Str::StopCancel } else { Str::StopReject });
    let mut title: Buf = Buf::new();
    title.push(word).push(" · #").push(super::cards::short_id(b.ask_id(), &mut [0u8; 4]));
    let fg = ui.pal.fg;
    ui.line(title.as_str(), Rect::new(x, y, w, 28), 20, 700, fg, 0);
    y += 36;
    let muted = ui.pal.muted;
    y += para(ui, l.s(Str::KReasonHint), Rect::new(x, y, w, 0), 15, 400, muted, false) + 8;
    ui.line(l.s(Str::KReason), Rect::new(x, y, w, 20), 14, 600, muted, 0);
    y += 24;
    field_box(b, ui, p, F_REASON, Rect::new(x, y, w, 52), "kitchen.reason");
    y += 52 + 16;
    let half = (w - 8) / 2;
    ui.button(p, Rect::new(x, y, half, TAP + 8), l.s(Str::Close), "kitchen.reasonClose", Act::SheetClose, Look::Plain);
    let danger = ui.pal.danger;
    ui.button(p, Rect::new(x + half + 8, y, w - half - 8, TAP + 8), word, "kitchen.reasonSend", Act::SheetSend, Look::Tone(danger));
}
