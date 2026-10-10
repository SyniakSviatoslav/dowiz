//! THE ROOM / KITCHEN BOARD (CV1): the staff's most-used screen on one canvas.
//!
//! What it shows and does is the existing surfaces' (operator: the same data and actions the staff
//! use most): the kitchen board's three columns of tickets with ONE bump each and the "seen" tap
//! (admin/kitchen.js, `/api/staff/kitchen`, `POST /api/owner/orders/:id/action`, `.../kitchen-ack`),
//! and the room's open tables with what is due (room/screens.js, `/api/staff/room`). Rust keeps the
//! state and decides what a tap means; it never touches the network -- a tap that must reach the
//! hub becomes an `Intent` the host takes (`take_intent`) and performs with the room's own `api()`.

pub mod feed;
pub mod model;
mod cards;
pub mod dirty;
mod input;
mod sheet;
pub mod tsheet;
mod view;

use crate::field::{Field, Kind};
use crate::geom::Rect;
use crate::lang::Lang;
use crate::scene::{Act, Scene};
use feed::Data;
use model::{Bump, Span};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sync {
    Offline,
    Live,
    Polling,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Waiter,
    CounterManager,
    Kitchen,
    Owner,
}

/// What a tap asks of the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Bump { order: Span, bump: Bump },
    Seen { order: Span },
    Lang(Lang),
    Refresh,
    SignOut,
    /// A text field took focus: the host opens the keyboard (EditContext or one transient input).
    Focus(u8),
    Blur,
    Submit { claim: bool },
    /// Open this sitting's table sheet (CV1b): the host builds its rows (room/canvas/table.js).
    Table { sitting: Span },
    /// A table-sheet control was tapped: the act code and argument the host gave its row
    /// (`tsheet.rs`). `arg` is a span of the SHEET's arena.
    Sheet { act: u8, arg: Span },
    /// Enter on a focused table-sheet field (the host submits that field's form).
    SheetEnter(u8),
    /// The theme button: the host stores the next mode (`dw_room_theme`) and answers `set_theme`.
    Theme,
    /// The sound toggle (CV8a): the host stores the new choice and answers `set_sound`.
    Sound,
    /// Reject (before the kitchen accepted) or cancel (after) the ticket in `Board::ask_id`, with
    /// the reason in field `F_REASON` -- the reason the customer reads (kitchen.js `askReason`).
    Stop { cancel: bool },
}

/// View tabs: the three ticket columns, then the tables.
pub const TAB_NEW: u8 = 0;
pub const TAB_PREPARING: u8 = 1;
pub const TAB_READY: u8 = 2;
pub const TAB_TABLES: u8 = 3;

pub const F_EMAIL: u8 = 0;
pub const F_CODE: u8 = 1;
pub const F_PASSWORD: u8 = 2;
pub const F_REASON: u8 = 3;
/// The table sheet's fields: 4..FIELDS (the pay form holds four at once: rate, amount, tip, wallet).
pub const F_SHEET: u8 = 4;
pub const FIELDS: usize = 8;

const QUEUE: usize = 8;
const TOAST: usize = 160;

pub struct Board {
    pub data: Data,
    pub lang: Lang,
    pub dark: bool,
    pub w: i32,
    pub h: i32,
    pub now_ms: i64,
    pub signed_in: bool,
    /// `advance`: the pass (tickets, bumps). `take_orders`: the floor (tables).
    pub can_pass: bool,
    pub can_floor: bool,
    pub role: Role,
    pub sync: Sync,
    /// Has the first read answered? Until then an empty list says "Loading", not "No tickets".
    pub loaded: bool,
    /// A stored session is being restored (the page's network code is still loading): the first
    /// frame says "Loading" instead of showing a signed-in person the login form.
    pub restoring: bool,
    pub tab: u8,
    pub station: u8,
    /// Per tab, then the table sheet's (`tsheet::SCROLL`).
    pub scroll: [i32; 5],
    /// Set by the last draw: the scrolled area and how tall its content was.
    pub list: Rect,
    pub content_h: i32,
    pub fields: [Field; FIELDS],
    /// The open table's sheet (CV1b); `ts.open` = drawn over the board.
    pub ts: tsheet::TSheet,
    /// The stored theme: 0 as the phone, 1 dark, 2 light (room/app.js `dw_room_theme`).
    pub theme: u8,
    /// Whether a new ticket rings (CV8a): on until the staff turns it off; the host holds the choice.
    pub sound: bool,
    /// The venue's ticket-age thresholds in minutes (amber, red): `model::WARN_MIN` /
    /// `LATE_MIN` until the host says (`set_ages`, board.js `ages`).
    pub warn_min: i64,
    pub late_min: i64,
    /// The reject/cancel sheet: open, for which order (its id copied: a new snapshot may move it).
    pub ask: Option<bool>,
    ask_id: [u8; 64],
    ask_len: usize,
    pub focus: Option<u8>,
    pub claiming: bool,
    toast: [u8; TOAST],
    toast_len: usize,
    queue: [Option<Intent>; QUEUE],
    qlen: usize,
    down: Option<(i32, i32)>,
    last_y: i32,
    moved: bool,
}

impl Board {
    pub const fn new() -> Board {
        Board {
            data: Data::new(), lang: Lang::Sq, dark: true, w: 390, h: 844, now_ms: 0, signed_in: false,
            can_pass: false, can_floor: false, role: Role::Waiter, sync: Sync::Offline, loaded: false, restoring: false,
            tab: TAB_NEW, station: 0, scroll: [0; 5], list: Rect::ZERO, content_h: 0,
            fields: [
                Field::new(Kind::Email), Field::new(Kind::Code), Field::new(Kind::Password), Field::new(Kind::Text),
                Field::new(Kind::Text), Field::new(Kind::Text), Field::new(Kind::Text), Field::new(Kind::Text),
            ],
            ts: tsheet::TSheet::new(), theme: 0, sound: true, warn_min: model::WARN_MIN, late_min: model::LATE_MIN,
            ask: None, ask_id: [0; 64], ask_len: 0,
            focus: None, claiming: false, toast: [0; TOAST], toast_len: 0, queue: [None; QUEUE], qlen: 0,
            down: None, last_y: 0, moved: false,
        }
    }

    /// A stored session will be restored: no tabs, no role, "Loading" until `session` answers.
    pub fn restore(&mut self) {
        self.session(true, false, false, Role::Waiter);
        self.restoring = true;
    }

    /// A session began (or ended: `signed` false). Caps decide which tabs exist.
    pub fn session(&mut self, signed: bool, pass: bool, floor: bool, role: Role) {
        self.signed_in = signed;
        self.can_pass = signed && pass;
        self.can_floor = signed && floor;
        self.role = role;
        self.loaded = false;
        self.restoring = false;
        self.focus = None;
        self.ask = None;
        self.ts.open = false;
        if !signed {
            self.data.apply(b"");
            for f in self.fields.iter_mut() {
                f.clear();
            }
        }
        if !self.tab_allowed(self.tab) {
            self.tab = if self.can_pass { TAB_NEW } else { TAB_TABLES };
        }
    }

    pub fn tab_allowed(&self, t: u8) -> bool {
        if t == TAB_TABLES {
            self.can_floor
        } else {
            self.can_pass && t < TAB_TABLES
        }
    }

    pub fn feed(&mut self, b: &[u8]) -> usize {
        self.loaded = true;
        self.data.apply(b)
    }

    /// The table sheet's rows (`tsheet.rs`); an empty feed closes it. Returns the rows read.
    pub fn sheet(&mut self, b: &[u8]) -> usize {
        if b.is_empty() {
            self.ts.open = false;
            if self.focus.is_some_and(|f| f >= F_SHEET) {
                self.focus = None;
                self.emit(Intent::Blur);
            }
            return 0;
        }
        let was = (self.ts.open, self.ts.view);
        let fields = &mut self.fields;
        let n = self.ts.apply(b, &mut |f, k| {
            if let Some(x) = fields.get_mut(f as usize) {
                x.kind = k;
            }
        });
        self.ts.open = true;
        if was != (true, self.ts.view) {
            self.scroll[tsheet::SCROLL] = 0;
        }
        n
    }

    /// The scroll slot in use: the sheet's when it is open, else the tab's.
    pub fn si(&self) -> usize {
        if self.ts.open {
            tsheet::SCROLL
        } else {
            self.tab as usize & 3
        }
    }

    /// The venue's amber / red minutes; nonsense (0, or red before amber) keeps the defaults.
    pub fn set_ages(&mut self, warn: i64, late: i64) {
        let ok = warn > 0 && late > warn;
        self.warn_min = if ok { warn } else { model::WARN_MIN };
        self.late_min = if ok { late } else { model::LATE_MIN };
    }

    pub fn set_toast(&mut self, b: &[u8]) {
        let s = core::str::from_utf8(b).unwrap_or("");
        let mut n = s.len().min(TOAST);
        while n > 0 && !s.is_char_boundary(n) {
            n -= 1;
        }
        crate::put_at(&mut self.toast, 0, crate::head(s.as_bytes(), n));
        self.toast_len = n;
    }
    pub fn toast(&self) -> &str {
        core::str::from_utf8(crate::head(&self.toast, self.toast_len)).unwrap_or("")
    }

    /// The order the reject/cancel sheet is about.
    pub fn ask_id(&self) -> &str {
        core::str::from_utf8(crate::head(&self.ask_id, self.ask_len)).unwrap_or("")
    }
    fn set_ask_id(&mut self, id: &[u8]) {
        let n = id.len().min(self.ask_id.len());
        crate::put_at(&mut self.ask_id, 0, crate::head(id, n));
        self.ask_len = n;
    }

    fn emit(&mut self, i: Intent) {
        if self.qlen < QUEUE {
            if let Some(x) = self.queue.get_mut(self.qlen) {
                *x = Some(i);
            }
            self.qlen += 1;
        }
    }

    /// The oldest intent not yet taken by the host.
    pub fn take_intent(&mut self) -> Option<Intent> {
        if self.qlen == 0 {
            return None;
        }
        let i = self.queue[0];
        for k in 1..QUEUE {
            self.queue[k - 1] = self.queue[k];
        }
        self.qlen -= 1;
        self.queue[QUEUE - 1] = None;
        i
    }


    /// Build this frame: the scene and the command buffer, from state.
    pub fn draw(&mut self, ui: &mut crate::ui::Ui) {
        view::draw(self, ui);
    }

    /// The rectangle of a focused field (the host places IME bounds / the transient input there).
    pub fn field_rect(&self, scene: &Scene, f: u8) -> Option<Rect> {
        scene.nodes().iter().find(|n| n.act == Act::Field(f)).map(|n| n.rect)
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tsheet_tests;
#[cfg(test)]
pub(crate) mod testrig;
