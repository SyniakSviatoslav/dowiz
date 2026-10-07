//! The board's DATA: what the hub said, kept in fixed pools with its strings in one arena.
//!
//! The vocabulary is the kernel's: twelve order statuses (`crates/dowiz-core/src/order_machine.rs`),
//! the three columns and the one bump per status of the kitchen board (admin/kitchen-logic.js
//! `COLUMNS`, `bumpFor`), three stations (`STATIONS`). The hub's FSM still decides every edge --
//! a bump here only NAMES the intent the console's own button sends.

use crate::lang::{Lang, Str};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Pending, Confirmed, Preparing, Ready, InDelivery, Delivered, Rejected, Cancelled, Scheduled,
    PickedUp, Refunding, CompensatedRefund,
    /// A status this build does not know: drawn as nothing rather than as a wrong word.
    Other,
}

impl Status {
    pub const KNOWN: [(&'static str, Status); 12] = [
        ("PENDING", Status::Pending), ("CONFIRMED", Status::Confirmed), ("PREPARING", Status::Preparing),
        ("READY", Status::Ready), ("IN_DELIVERY", Status::InDelivery), ("DELIVERED", Status::Delivered),
        ("REJECTED", Status::Rejected), ("CANCELLED", Status::Cancelled), ("SCHEDULED", Status::Scheduled),
        ("PICKED_UP", Status::PickedUp), ("REFUNDING", Status::Refunding),
        ("COMPENSATED_REFUND", Status::CompensatedRefund),
    ];

    pub fn parse(b: &[u8]) -> Status {
        Status::KNOWN.iter().find(|(k, _)| k.as_bytes() == b).map_or(Status::Other, |(_, s)| *s)
    }

    /// The console's word (admin/i18n.js / i18n-ru.js `st`; `board/tests.rs` holds them equal).
    pub fn word(self, l: Lang) -> &'static str {
        let p = |sq, en, uk, ru| match l {
            Lang::Sq => sq,
            Lang::En => en,
            Lang::Uk => uk,
            Lang::Ru => ru,
        };
        match self {
            Status::Pending => p("E re", "New", "Нове", "Новый"),
            Status::Confirmed => p("Pranuar", "Accepted", "Прийнято", "Принят"),
            Status::Preparing => p("Po gatuhet", "Cooking", "Готується", "Готовится"),
            Status::Ready => p("Gati", "Ready", "Готове", "Готов"),
            Status::InDelivery => p("Në rrugë", "On the way", "У дорозі", "В пути"),
            Status::Delivered => p("Dorëzuar", "Delivered", "Доставлено", "Доставлен"),
            Status::Rejected => p("Refuzuar", "Rejected", "Відхилено", "Отклонён"),
            Status::Cancelled => p("Anuluar", "Cancelled", "Скасовано", "Отменён"),
            Status::Scheduled => p("Planifikuar", "Scheduled", "Заплановано", "Запланирован"),
            Status::PickedUp => p("Marrë", "Picked up", "Забрано", "Забран"),
            Status::Refunding => p("Po kthehet", "Refunding", "Повертаємо", "Возвращаем"),
            Status::CompensatedRefund => p("U kthye", "Refunded", "Повернено", "Возвращено"),
            Status::Other => "",
        }
    }

    /// The board column (kitchen-logic.js `COLUMNS`), or None when it is off the pass.
    pub fn column(self) -> Option<Col> {
        match self {
            Status::Pending | Status::Confirmed => Some(Col::New),
            Status::Preparing => Some(Col::Preparing),
            Status::Ready => Some(Col::Ready),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Col {
    New,
    Preparing,
    Ready,
}

/// The one button a ticket offers (kitchen-logic.js `bumpFor`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bump {
    Confirm,
    Preparing,
    Ready,
    Collected,
}

impl Bump {
    /// A delivery at READY leaves the pass (the courier's); a pickup or a table is handed over.
    pub fn of(s: Status, w: Where) -> Option<Bump> {
        match s {
            Status::Pending => Some(Bump::Confirm),
            Status::Confirmed => Some(Bump::Preparing),
            Status::Preparing => Some(Bump::Ready),
            Status::Ready if w != Where::Delivery => Some(Bump::Collected),
            _ => None,
        }
    }
    /// The `action` word `POST /api/owner/orders/:id/action` takes.
    pub const fn action(self) -> &'static str {
        match self {
            Bump::Confirm => "confirm",
            Bump::Preparing => "preparing",
            Bump::Ready => "ready",
            Bump::Collected => "collected",
        }
    }
    pub const fn label(self) -> Str {
        match self {
            Bump::Confirm => Str::BumpConfirm,
            Bump::Preparing => Str::BumpPreparing,
            Bump::Ready => Str::BumpReady,
            Bump::Collected => Str::BumpCollected,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Where {
    Table,
    Pickup,
    Delivery,
}

/// Station filter: 0 = all, then kitchen-logic.js `STATIONS` order (sushi, kitchen, bar).
pub const STATIONS: [Str; 4] = [Str::All, Str::StSushi, Str::StKitchen, Str::StBar];

/// A string in the arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Span {
    pub off: u32,
    pub len: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Ticket {
    pub id: Span,
    pub status: Status,
    pub start_ms: i64,
    pub seen: bool,
    pub kind: Where,
    pub table: Span,
    pub note: Span,
    /// "for 19:00" when the ticket's hour is still ahead (formatted by the host in the venue's
    /// time zone -- memory dowiz-venue-timezone); empty otherwise.
    pub when: Span,
    pub line0: u16,
    pub nlines: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct Line {
    pub qty: i32,
    /// 1 sushi, 2 kitchen, 3 bar (STATIONS index); absent IS the kitchen.
    pub station: u8,
    pub name: Span,
    pub note: Span,
}

pub const ROUND_ST: usize = 6;

#[derive(Clone, Copy, Debug)]
pub struct Table {
    pub sitting: Span,
    pub table: Span,
    pub rounds: u16,
    /// What is due, ALREADY FORMATTED by the host's one money formatter (lib/money.js): no money
    /// arithmetic happens in this crate (memory kit-had-a-fourth-money-copy).
    pub due: Span,
    pub guest: bool,
    pub st: [Status; ROUND_ST],
    pub nst: u8,
}

pub const BLANK_TICKET: Ticket = Ticket {
    id: Span { off: 0, len: 0 }, status: Status::Other, start_ms: 0, seen: false, kind: Where::Pickup,
    table: Span { off: 0, len: 0 }, note: Span { off: 0, len: 0 }, when: Span { off: 0, len: 0 }, line0: 0, nlines: 0,
};
pub const BLANK_LINE: Line = Line { qty: 0, station: 2, name: Span { off: 0, len: 0 }, note: Span { off: 0, len: 0 } };
pub const BLANK_TABLE: Table = Table {
    sitting: Span { off: 0, len: 0 }, table: Span { off: 0, len: 0 }, rounds: 0, due: Span { off: 0, len: 0 },
    guest: false, st: [Status::Other; ROUND_ST], nst: 0,
};

/// Whole minutes waited; never negative (kitchen-logic.js `ageMin`).
pub fn age_min(start_ms: i64, now_ms: i64) -> i64 {
    ((now_ms - start_ms) / 60_000).max(0)
}

/// 0 ok, 1 warn, 2 late (kitchen-logic.js `ageClass` with its default 10 / 20 minutes).
pub fn age_class(m: i64) -> u8 {
    if m >= 20 {
        2
    } else if m >= 10 {
        1
    } else {
        0
    }
}
