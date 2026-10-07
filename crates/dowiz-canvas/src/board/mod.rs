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
mod input;
mod sheet;
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
    /// Open the full room page for this sitting (detail screens are not on canvas yet).
    Table { sitting: Span },
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
    pub scroll: [i32; 4],
    /// Set by the last draw: the scrolled area and how tall its content was.
    pub list: Rect,
    pub content_h: i32,
    pub fields: [Field; 4],
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
            tab: TAB_NEW, station: 0, scroll: [0; 4], list: Rect::ZERO, content_h: 0,
            fields: [Field::new(Kind::Email), Field::new(Kind::Code), Field::new(Kind::Password), Field::new(Kind::Text)],
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

    pub fn set_toast(&mut self, b: &[u8]) {
        let s = core::str::from_utf8(b).unwrap_or("");
        let mut n = s.len().min(TOAST);
        while n > 0 && !s.is_char_boundary(n) {
            n -= 1;
        }
        self.toast[..n].copy_from_slice(&s.as_bytes()[..n]);
        self.toast_len = n;
    }
    pub fn toast(&self) -> &str {
        core::str::from_utf8(&self.toast[..self.toast_len]).unwrap_or("")
    }

    /// The order the reject/cancel sheet is about.
    pub fn ask_id(&self) -> &str {
        core::str::from_utf8(&self.ask_id[..self.ask_len]).unwrap_or("")
    }
    fn set_ask_id(&mut self, id: &[u8]) {
        let n = id.len().min(self.ask_id.len());
        self.ask_id[..n].copy_from_slice(&id[..n]);
        self.ask_len = n;
    }

    fn emit(&mut self, i: Intent) {
        if self.qlen < QUEUE {
            self.queue[self.qlen] = Some(i);
            self.qlen += 1;
        }
    }

    /// The oldest intent not yet taken by the host.
    pub fn take_intent(&mut self) -> Option<Intent> {
        if self.qlen == 0 {
            return None;
        }
        let i = self.queue[0];
        self.queue.copy_within(1..self.qlen, 0);
        self.qlen -= 1;
        self.queue[self.qlen] = None;
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
pub(crate) mod testrig;
