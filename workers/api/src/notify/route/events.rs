//! THE EVENT CATALOGUE: every kind of message the bot can send, once.
//!
//! The console's matrix is BUILT from this list (`GET /api/owner/telegram`),
//! so a row can never exist in the console that the hub does not know, and a
//! new event is one line here. `live` is honest: a row whose producer is not
//! in this build is shown as "coming", never as a switch that does nothing.

/// The matrix's row groups, in the owner's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    Orders,
    Stock,
    Analytics,
}

impl Area {
    pub fn as_str(self) -> &'static str {
        match self {
            Area::Orders => "orders",
            Area::Stock => "stock",
            Area::Analytics => "analytics",
        }
    }
}

/// How an event treats the group's quiet hours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// An order waiting to be cooked: rings through quiet hours.
    Urgent,
    /// Waits for the end of the quiet hours.
    Normal,
    /// Is itself a summary, sent at the group's time: now or off, never "digest".
    Scheduled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub key: &'static str,
    pub area: Area,
    pub class: Class,
    /// A producer exists in this build.
    pub live: bool,
}

const fn ev(key: &'static str, area: Area, class: Class, live: bool) -> Event {
    Event { key, area, class, live }
}

pub const EVENTS: &[Event] = &[
    ev("order.placed", Area::Orders, Class::Urgent, true),
    ev("order.amended", Area::Orders, Class::Urgent, true),
    ev("order.status", Area::Orders, Class::Urgent, false),
    ev("order.late", Area::Orders, Class::Urgent, false),
    ev("order.exception", Area::Orders, Class::Normal, true),
    ev("inbox.message", Area::Orders, Class::Normal, true),
    ev("stock.low", Area::Stock, Class::Normal, false),
    ev("stock.received", Area::Stock, Class::Normal, false),
    ev("stock.expiring", Area::Stock, Class::Normal, false),
    ev("stock.wasted", Area::Stock, Class::Normal, false),
    ev("stocktake.variance", Area::Stock, Class::Normal, false),
    ev("digest.daily", Area::Analytics, Class::Scheduled, true),
    ev("digest.weekly", Area::Analytics, Class::Scheduled, true),
    ev("system.alert", Area::Analytics, Class::Normal, true),
];

pub fn get(key: &str) -> Option<&'static Event> {
    EVENTS.iter().find(|e| e.key == key)
}

/// Quiet hours hold it back.
pub fn holds_in_quiet(key: &str) -> bool {
    get(key).map_or(true, |e| e.class != Class::Urgent)
}
