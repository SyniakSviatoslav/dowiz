//! WHAT CROSSES FROM THE WORKER TO THE VENUE'S OBJECT for the till link:
//! `/fold/ebills/<what>`, one command each, executed in one object turn
//! (`hubdo/ebills.rs`). The Worker does the fetching; the object keeps the
//! state and makes every decision that must not race.

use super::client::Session;
use super::import::Mapped;
use super::map::FloorRow;
use super::state::Noted;
use serde::{Deserialize, Serialize};

/// `tick`: what should this firing do? Answered with a `state::Plan`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TickIn {
    pub(crate) now_ms: i64,
}

/// `import`: the mapped sales, and where the poll got to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ImportIn {
    pub(crate) now_ms: i64,
    pub(crate) sales: Vec<Mapped>,
    /// The new watermark: every id at or below it has been handled.
    pub(crate) watermark: i64,
    pub(crate) backlog: bool,
    /// The five-minute list was read this firing.
    pub(crate) listed: bool,
    /// The week's re-read was done this firing.
    pub(crate) reread: bool,
    /// Sales the Worker could not map, named (`MapError`).
    #[serde(default)]
    pub(crate) refused: Vec<Noted>,
    /// The till's own menu, `(code, name, price)`, when it was read.
    #[serde(default)]
    pub(crate) items: Vec<(String, String, i64)>,
    /// The session, when it changed (a login, a rotated cookie).
    #[serde(default)]
    pub(crate) session: Option<Session>,
    /// The firing finished. `false` imports what was read before a failure
    /// and leaves the failure count and last error for `report` to move.
    pub(crate) complete: bool,
    /// First run: ids below this were read as leads (`state::LEAD_IDS`).
    #[serde(default)]
    pub(crate) lead_below: i64,
    /// The re-check pass after this firing: `(next id, last id)`.
    #[serde(default)]
    pub(crate) recheck: Option<(i64, i64)>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct ImportOut {
    pub(crate) placed: u64,
    pub(crate) noted: u64,
    pub(crate) paid: u64,
    pub(crate) unchanged: u64,
    pub(crate) pending: usize,
    pub(crate) refused: Vec<Noted>,
    pub(crate) short: Vec<(String, i64)>,
    pub(crate) generation: i64,
}

/// `floor`: the tables as the till sees them now.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FloorIn {
    pub(crate) now_ms: i64,
    pub(crate) tables: Vec<FloorRow>,
    #[serde(default)]
    pub(crate) session: Option<Session>,
}

/// `report`: this firing failed, and why. `halt` for what no retry can fix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ReportIn {
    pub(crate) now_ms: i64,
    pub(crate) what: String,
    pub(crate) halt: bool,
    /// Forget the session: it was refused, the next firing logs in afresh.
    pub(crate) drop_session: bool,
}

/// `config`: the owner's settings. `secret: None` keeps the stored one.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfigIn {
    pub(crate) enabled: bool,
    pub(crate) pos_id: i64,
    pub(crate) user: String,
    #[serde(default)]
    pub(crate) password: Option<String>,
}

/// `map`: set (or, with no product, clear) the crosswalk for one code.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MapIn {
    pub(crate) code: String,
    #[serde(default)]
    pub(crate) product_id: Option<String>,
    #[serde(default)]
    pub(crate) now_ms: i64,
}
