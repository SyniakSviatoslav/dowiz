//! INPUT: pointer, wheel and Enter, and what a tap on each kind of node means.
//!
//! A tap is a down and an up within 8 px; anything longer that started in the list scrolls it.
//! The meaning of a tap is decided HERE, from the scene the last frame built; what must reach the
//! hub leaves as an `Intent`.

use super::model::{Bump, Status};
use super::{Board, Intent, F_REASON};
use crate::lang::Str;
use crate::scene::{Act, Scene};

impl Board {
    /// 0 down, 1 move, 2 up, 3 cancel. Returns true when a redraw is wanted.
    pub fn pointer(&mut self, scene: &Scene, kind: u32, x: i32, y: i32) -> bool {
        match kind {
            0 => {
                self.down = Some((x, y));
                self.last_y = y;
                self.moved = false;
                false
            }
            1 => {
                let Some((dx, dy)) = self.down else { return false };
                if (x - dx).abs() + (y - dy).abs() > 8 {
                    self.moved = true;
                }
                let step = y - self.last_y;
                self.last_y = y;
                if self.moved && self.ask.is_none() && self.list.contains(dx, dy) {
                    self.scroll_by(-step);
                    return true;
                }
                false
            }
            2 => {
                let tap = self.down.is_some() && !self.moved;
                self.down = None;
                if tap {
                    self.tap(scene, x, y);
                }
                tap
            }
            _ => {
                self.down = None;
                false
            }
        }
    }

    pub fn wheel(&mut self, dy: i32) {
        if self.ask.is_none() {
            self.scroll_by(dy);
        }
    }

    fn scroll_by(&mut self, dy: i32) {
        let t = self.si();
        self.scroll[t] = (self.scroll[t] + dy).clamp(0, self.max_scroll());
    }

    pub fn max_scroll(&self) -> i32 {
        (self.content_h - self.list.h).max(0)
    }

    /// Enter on the keyboard: submit the sign-in form, send the reason, or a sheet field's form.
    pub fn key_enter(&mut self) {
        if !self.signed_in {
            self.emit(Intent::Submit { claim: self.claiming });
        } else if self.ask.is_some() {
            self.send_stop();
        } else if let Some(f) = self.focus.filter(|&f| self.ts.open && f >= super::F_SHEET) {
            self.emit(Intent::SheetEnter(f));
        }
    }

    fn send_stop(&mut self) {
        let Some(cancel) = self.ask else { return };
        if self.fields[F_REASON as usize].value().trim_ascii().is_empty() {
            let w = self.lang.s(Str::KReasonNeeded);
            self.set_toast(w.as_bytes());
            return;
        }
        self.ask = None;
        self.focus = None;
        self.emit(Intent::Blur);
        self.emit(Intent::Stop { cancel });
    }

    pub(super) fn tap(&mut self, scene: &Scene, x: i32, y: i32) {
        let act = scene.hit(x, y).map_or(Act::None, |n| n.act);
        if self.focus.is_some() && !matches!(act, Act::Field(_) | Act::Submit | Act::SheetSend) {
            self.focus = None;
            self.emit(Intent::Blur);
        }
        let ticket = |b: &Board, i: u16| crate::head(&b.data.tickets, b.data.nt).get(i as usize).copied();
        match act {
            Act::None | Act::Block => {}
            Act::Tab(t) if self.tab_allowed(t) => self.tab = t,
            Act::Tab(_) => {}
            Act::Station(s) => self.station = s.min(3),
            Act::Bump(i) => {
                if let Some(t) = ticket(self, i) {
                    if let Some(b) = Bump::of(t.status, t.kind) {
                        self.emit(Intent::Bump { order: t.id, bump: b });
                    }
                }
            }
            Act::Seen(i) => {
                if let Some(t) = ticket(self, i) {
                    if !t.seen {
                        self.emit(Intent::Seen { order: t.id });
                    }
                }
            }
            Act::Stop(i) => {
                if let Some(t) = ticket(self, i) {
                    // kitchen-logic.js `stopFor`: reject before the kitchen accepts, cancel after.
                    let cancel = match t.status {
                        Status::Pending => Some(false),
                        Status::Confirmed => Some(true),
                        _ => None,
                    };
                    if let Some(c) = cancel {
                        let mut buf = [0u8; 64];
                        let id = crate::head(self.data.str(t.id).as_bytes(), 64);
                        crate::put_at(&mut buf, 0, id);
                        let n = id.len();
                        self.set_ask_id(crate::head(&buf, n));
                        self.fields[F_REASON as usize].clear();
                        self.ask = Some(c);
                    }
                }
            }
            Act::SheetClose => {
                self.ask = None;
                if self.focus.take().is_some() {
                    self.emit(Intent::Blur);
                }
            }
            Act::SheetSend => self.send_stop(),
            Act::Sheet(i) => {
                let r = crate::head(&self.ts.rows, self.ts.n).get(i as usize).copied();
                if let Some(r) = r.filter(|r| matches!(r.kind, b'b' | b'B' | b'H')) {
                    self.emit(Intent::Sheet { act: r.act, arg: r.aux });
                }
            }
            Act::Theme => self.emit(Intent::Theme),
            Act::Sound => self.emit(Intent::Sound),
            Act::Table(i) => {
                if let Some(t) = crate::head(&self.data.tables, self.data.ntab).get(i as usize).copied() {
                    self.emit(Intent::Table { sitting: t.sitting });
                }
            }
            Act::Field(f) => {
                self.focus = Some(f);
                self.emit(Intent::Focus(f));
            }
            Act::Submit => {
                self.focus = None;
                self.emit(Intent::Submit { claim: self.claiming });
            }
            Act::ClaimToggle => self.claiming = !self.claiming,
            Act::Lang => {
                self.lang = self.lang.next();
                self.emit(Intent::Lang(self.lang));
            }
            Act::Refresh => self.emit(Intent::Refresh),
            Act::SignOut => self.emit(Intent::SignOut),
        }
    }
}

#[cfg(test)]
mod tests;
