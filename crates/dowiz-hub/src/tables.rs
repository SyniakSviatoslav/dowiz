//! The venue's FLOOR: zones, the tables standing in them, and who may sit at
//! one AT A GIVEN MINUTE.
//!
//! WHAT WAS MISSING. `dowiz_kernel::reservation` decides whether a party size
//! and a time are a legal booking. It does not know a restaurant is a finite
//! number of physical tables, so two guests could both be told yes for the
//! same one and the venue would find out when they both walked in.
//!
//! `occ` IS NOT A PROPERTY OF A TABLE — the one part of the prototype's data
//! model that does not survive a real booking: table 5 is free at 18:00 and
//! taken at 20:00. Nothing here stores occupancy; [`availability`] computes it
//! from the slot asked for and the bookings already holding that table.
//!
//! WHY HERE AND NOT IN THE KERNEL. The kernel owns money and the order FSM. A
//! floor plan is one venue's furniture — hub logic, beside [`crate::hours`].
//! PURE: no clock. Every function that cares about time TAKES the minute, the
//! correction `booking.rs::now_min` made for the same reason.

use crate::minijson::{int_field, objects_in, str_field};

/// The drawing's coordinate space, and the prototype's `viewBox`. A table
/// whose box leaves the room is refused: it could never be tapped.
pub const PLAN_W: i64 = 390;
pub const PLAN_H: i64 = 446;

/// The kernel's `BookingPolicy::default_policy().max_party`. More seats than
/// the largest party the venue seats is a typo, not a banquet.
pub const MAX_SEATS: i64 = 20;

/// The plan is rewritten whole on every owner save, so it is bounded on
/// purpose. Two hundred is larger than any room this product serves.
pub const MAX_TABLES: usize = 200;

/// HOW LONG A TABLE IS HELD, in minutes, from its slot. Two bookings collide
/// when their slots are closer than this. One number, written down, rather
/// than an `==` on the slot — which would let 19:00 and 19:15 both take
/// table 5. A per-zone override is the venue's to add later.
pub const DWELL_MIN: i64 = 90;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape { Rect, Circle }

impl Shape {
    pub fn as_str(&self) -> &'static str {
        match self {
            Shape::Rect => "rect",
            Shape::Circle => "circle",
        }
    }
}

/// One table as the owner drew it. `n` is the number painted on the plan and
/// spoken aloud ("table four"), not a database id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub n: i64,
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
    pub seats: i64,
    pub shape: Shape,
}

impl Table {
    /// SEATS, NOT COMFORT. A four-top seats two; whether the venue WANTS to
    /// give a deuce its only four-top is a revenue policy, and inventing one
    /// here would hide tables from guests who can see them standing empty.
    /// A party of zero is not a party.
    pub fn seats_party(&self, party: i64) -> bool {
        party >= 1 && self.seats >= party
    }
}

/// A room. `name` is the VENUE'S OWN WORD, in its own language — the rule
/// `store/menu.js` follows for dish names, which are never in the storefront's
/// three-language table because they are not the platform's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zone {
    pub id: String,
    pub name: String,
    pub tables: Vec<Table>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    pub zones: Vec<Zone>,
}

impl Plan {
    /// No plan at all. A booking then names no table and the behaviour before
    /// this module is exactly preserved.
    pub fn is_empty(&self) -> bool {
        self.zones.iter().all(|z| z.tables.is_empty())
    }

    pub fn find(&self, zone: &str, n: i64) -> Option<&Table> {
        self.zones
            .iter()
            .find(|z| z.id == zone)?
            .tables
            .iter()
            .find(|t| t.n == n)
    }
}

/// Why a plan was refused. NAMED, NOT COUNTED: `services/venue/zones.rs`
/// reports "3 of 5 could not be read", the right shape but leaving the owner
/// guessing which three. Each of these names the zone, the table and the rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    ZoneIdMissing,
    ZoneIdBadChar { id: String },
    ZoneIdTwice { id: String },
    ZoneNameMissing { id: String },
    TableFieldMissing { zone: String, field: &'static str },
    TableNumberTwice { zone: String, n: i64 },
    TableSeats { zone: String, n: i64, seats: i64 },
    TableSize { zone: String, n: i64 },
    TableOffPlan { zone: String, n: i64 },
    TableNotACircle { zone: String, n: i64 },
    TooManyTables { found: usize },
}

impl PlanError {
    pub fn message(&self) -> String {
        match self {
            PlanError::ZoneIdMissing => "every zone needs an \"id\"".into(),
            PlanError::ZoneIdBadChar { id } => format!(
                "zone id {id:?}: a zone id is a-z, 0-9, '-' and '_' only, because it \
                 becomes part of a storage key"
            ),
            PlanError::ZoneIdTwice { id } => {
                format!("two zones share the id {id:?}; a table would belong to both")
            }
            PlanError::ZoneNameMissing { id } => {
                format!("zone {id:?} has no \"name\" for the guest to read")
            }
            PlanError::TableFieldMissing { zone, field } => {
                format!("zone {zone:?}: a table is missing \"{field}\"")
            }
            PlanError::TableNumberTwice { zone, n } => format!(
                "zone {zone:?} has two tables numbered {n}; the guest and the waiter \
                 would mean different tables"
            ),
            PlanError::TableSeats { zone, n, seats } => format!(
                "zone {zone:?} table {n}: {seats} seats is outside 1..{MAX_SEATS}"
            ),
            PlanError::TableSize { zone, n } => {
                format!("zone {zone:?} table {n}: \"w\" and \"h\" are at least 1")
            }
            PlanError::TableOffPlan { zone, n } => format!(
                "zone {zone:?} table {n} does not fit inside the {PLAN_W}x{PLAN_H} plan; \
                 a table drawn off the edge cannot be tapped"
            ),
            PlanError::TableNotACircle { zone, n } => format!(
                "zone {zone:?} table {n}: a round table has \"w\" equal to \"h\""
            ),
            PlanError::TooManyTables { found } => {
                format!("{found} tables; a floor plan holds at most {MAX_TABLES}")
            }
        }
    }
}

/// Read a plan, or REFUSE IT WHOLE. A HALF-READ PLAN IS THE DEFECT, not the
/// safe fallback: `zone::from_json` drops what it cannot parse and its caller
/// compares counts to notice. A dropped TABLE is worse than a dropped delivery
/// zone — the table is still physically there, with guests at it, while the
/// hub believes the room is smaller. The first thing that does not read stops
/// the parse and says what it was.
pub fn from_json(doc: &str) -> Result<Plan, PlanError> {
    let mut zones: Vec<Zone> = Vec::new();
    let mut total = 0usize;
    for zj in objects_in(doc, "zones") {
        let id = str_field(&zj, "id").filter(|s| !s.is_empty()).ok_or(PlanError::ZoneIdMissing)?;
        if !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        {
            return Err(PlanError::ZoneIdBadChar { id });
        }
        if zones.iter().any(|z| z.id == id) {
            return Err(PlanError::ZoneIdTwice { id });
        }
        let name = str_field(&zj, "name")
            .filter(|s| !s.is_empty())
            .ok_or(PlanError::ZoneNameMissing { id: id.clone() })?;
        let mut tables: Vec<Table> = Vec::new();
        for tj in objects_in(&zj, "tables") {
            let need = |field: &'static str| -> Result<i64, PlanError> {
                int_field(&tj, field).ok_or(PlanError::TableFieldMissing { zone: id.clone(), field })
            };
            let (n, x, y) = (need("n")?, need("x")?, need("y")?);
            let (w, h, seats) = (need("w")?, need("h")?, need("seats")?);
            if tables.iter().any(|t| t.n == n) {
                return Err(PlanError::TableNumberTwice { zone: id.clone(), n });
            }
            if !(1..=MAX_SEATS).contains(&seats) {
                return Err(PlanError::TableSeats { zone: id.clone(), n, seats });
            }
            if w < 1 || h < 1 {
                return Err(PlanError::TableSize { zone: id.clone(), n });
            }
            // `x`,`y` are the table's CENTRE, the way the prototype draws it.
            if x - w / 2 < 0 || y - h / 2 < 0 || x + w / 2 > PLAN_W || y + h / 2 > PLAN_H {
                return Err(PlanError::TableOffPlan { zone: id.clone(), n });
            }
            let shape = match str_field(&tj, "shape").as_deref() {
                Some("circle") => Shape::Circle,
                // `rect` is the default and an UNKNOWN shape is a rectangle,
                // not a refusal: a plan drawn by a future owner surface that
                // learns "booth" must still open every table it names.
                _ => Shape::Rect,
            };
            if shape == Shape::Circle && w != h {
                return Err(PlanError::TableNotACircle { zone: id.clone(), n });
            }
            tables.push(Table { n, x, y, w, h, seats, shape });
            total += 1;
            if total > MAX_TABLES {
                return Err(PlanError::TooManyTables { found: total });
            }
        }
        zones.push(Zone { id, name, tables });
    }
    Ok(Plan { zones })
}

/// A table a booking is holding: which one, and for when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    pub zone: String,
    pub n: i64,
    pub slot_min: i64,
    /// The reservation doing the holding, so a refusal can be traced.
    pub reservation: String,
}

/// Does a booking in this status still hold its table?
///
/// `REQUESTED` HOLDS IT — a deliberate departure from the kernel's sentence
/// "the guest asked, nothing is held yet". That sentence is about the VENUE'S
/// COMMITMENT. The FLOOR is another question: if a request held nothing, two
/// guests could both request table 5 at 19:00 and the double booking would
/// surface only when the venue confirmed the second. A terminal status
/// releases it, `COMPLETED` included — they ate and left.
pub fn holds_table(status: &str) -> bool {
    matches!(status, "REQUESTED" | "CONFIRMED" | "SEATED")
}

/// The booking already holding this table at this minute, if there is one.
/// THE ANSWER IS THE BOOKING, not a boolean: the refusal has to name the
/// table and say which slot took it.
pub fn holder<'a>(
    held: &'a [Held],
    zone: &str,
    n: i64,
    slot_min: i64,
    dwell_min: i64,
) -> Option<&'a Held> {
    held.iter()
        .find(|h| h.zone == zone && h.n == n && (h.slot_min - slot_min).abs() < dwell_min)
}

/// One table's standing for one slot and one party size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    pub zone: String,
    pub n: i64,
    pub occupied: bool,
    pub too_small: bool,
}

/// Every table on the plan, said to be free or taken FOR THIS SLOT. The taken
/// and the too-small ones appear too: a guest who cannot see the tables they
/// cannot have reads an empty room as a closed restaurant.
pub fn availability(
    plan: &Plan,
    held: &[Held],
    party: i64,
    slot_min: i64,
    dwell_min: i64,
) -> Vec<Standing> {
    let mut out = Vec::new();
    for z in &plan.zones {
        for t in &z.tables {
            out.push(Standing {
                zone: z.id.clone(),
                n: t.n,
                occupied: holder(held, &z.id, t.n, slot_min, dwell_min).is_some(),
                too_small: !t.seats_party(party),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests;
