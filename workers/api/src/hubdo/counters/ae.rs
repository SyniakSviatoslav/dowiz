//! W-AE: AX0's counters written to Workers Analytics Engine, so a hibernation no longer erases
//! them (operator 2026-10-07, "підключай уже").
//!
//! THE PLATFORM, read 2026-10-07 (citations in `tools/live-proof/contracts/feature-ae-counters.json`):
//! * analytics-engine/pricing (Apr 23 2026): Workers Free includes 100,000 data points written
//!   and 10,000 read queries PER DAY, account-wide; "Currently, you will not be billed".
//! * analytics-engine/limits (Apr 23 2026): 20 blobs, 20 doubles, ONE index (<= 96 bytes) per
//!   `writeDataPoint`; 250 points per invocation; data kept three months.
//! * durable-objects/api/base (Jun 15 2026): "`env` contains the environment bindings available
//!   to this Durable Object" -- so the object writes through its own `env`.
//! * workers/examples/analytics-engine: `writeDataPoint` is non-blocking; nothing is awaited.
//!
//! WHEN A POINT IS WRITTEN. At every flush (`/fold/counters?flush=1`, the nightly's), and on every
//! `NTH` stored write. The card's other moment -- "when an object wakes while the previous window
//! still holds unflushed data" -- CANNOT EXIST: the counters live in memory only (a stored row per
//! write would spend the Free plan's 100,000 rows/day on a gauge), so by the time an object wakes,
//! what the previous wake had not written is already gone. Every `NTH` write bounds that loss
//! instead: an eviction loses at most `NTH - 1` writes' worth of counts (and the reads beside them).
//!
//! A POINT IS A DELTA. It carries what was counted since the PREVIOUS POINT (not since the last
//! health flush), so `SUM(_sample_interval * doubleN)` over any span counts every event once.
//!
//! THE BUDGET, at 1,000 hubs: each object writes at most `CAP_PER_DAY` NTH-points per UTC day
//! (past it, counts wait for the flush, still in memory) plus one nightly flush point:
//! 1,000 x (48 + 1) = 49,000 points/day, 49 % of the Free plan's 100,000. A busy venue's
//! 1,800 writes/day (300 orders x ~6 writes) fills the cap at write 768; a quiet one never reaches it.
//!
//! NO PERSONAL DATA: the venue's own object name, a cause, a clock label and counts. No order,
//! no guest, no phone, no address, no free text.

use std::cell::{Cell, RefCell};

/// The binding in `wrangler.toml` (`[[analytics_engine_datasets]]`, dataset `dowiz_counters`).
pub(crate) const BINDING: &str = "COUNTERS";
/// One point every `NTH` stored writes.
pub(crate) const NTH: u64 = 16;
/// The most NTH-points one object writes in one UTC day (the flush is not capped).
pub(crate) const CAP_PER_DAY: u32 = 48;
const DAY_US: u64 = 86_400_000_000;
/// `Pending` mirrors `counters::NAMES` cell for cell; a new cell must be placed in `DOUBLES` too.
const _: () = assert!(super::NAMES.len() == 12);

/// The doubles, IN THIS ORDER: double1 .. double15 in the SQL API. The collector
/// (`tools/evals/collect/cf.mjs`, `AE_DOUBLES`) reads them by position, so the order is the contract.
pub(crate) const DOUBLES: [&str; 15] = [
    "since_total", "since_none", "wakes_total", "wakes_writing", "reads", "writes", "proj_rows",
    "cold_folds", "cold_fold_us", "cold_fold_p50_us", "cat_writes", "cat_decode_us",
    "cat_decoded_bytes", "journal_us", "journal_bytes",
];

/// One data point: index = venue; blobs = [venue, cause, clock]; doubles in `DOUBLES` order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Point {
    pub index: String,
    pub blobs: [String; 3],
    pub doubles: Vec<f64>,
}

/// What was counted since the last point the platform accepted.
#[derive(Default)]
pub(crate) struct Pending {
    /// `counters::NAMES` order (12 cells).
    cells: RefCell<[u64; 12]>,
    samples: RefCell<Vec<u64>>,
    /// The wake is not in a point yet / the wake's first write is not in a point yet.
    wake: Cell<bool>,
    writing: Cell<u8>,
    writes_since: Cell<u64>,
    day: Cell<u64>,
    today: Cell<u32>,
    pub sent: Cell<u64>,
    pub errors: Cell<u64>,
    pub last_error: RefCell<Option<String>>,
}

impl Pending {
    pub(crate) fn woke() -> Self {
        let p = Pending::default();
        p.wake.set(true);
        p
    }
    pub(crate) fn bump(&self, i: usize, n: u64) {
        let mut c = self.cells.borrow_mut();
        c[i] = c[i].saturating_add(n);
    }
    pub(crate) fn sample(&self, us: u64) {
        let mut s = self.samples.borrow_mut();
        if s.len() < 16 {
            s.push(us);
        }
    }
    /// A stored write; `first` = the wake's first. True when an NTH-point is due (and allowed today).
    pub(crate) fn wrote(&self, first: bool, now_us: u64) -> bool {
        if first {
            self.writing.set(1);
        }
        self.writes_since.set(self.writes_since.get() + 1);
        let day = now_us / DAY_US;
        if day != self.day.get() {
            self.day.set(day);
            self.today.set(0);
        }
        self.writes_since.get() >= NTH && self.today.get() < CAP_PER_DAY
    }
    fn empty(&self) -> bool {
        !self.wake.get() && self.writing.get() == 0 && self.cells.borrow().iter().all(|&v| v == 0)
    }
    /// The point for what is pending, or None when nothing is.
    pub(crate) fn point(&self, venue: &str, cause: &str, clock: &str) -> Option<Point> {
        if self.empty() {
            return None;
        }
        let c = self.cells.borrow();
        let mut s = self.samples.borrow().clone();
        s.sort_unstable();
        let p50 = s.get(s.len() / 2).copied().unwrap_or(0);
        // NAMES: since_total 0, since_none 1, reads 2, writes 3, proj_rows 4, cold_folds 5,
        // cold_fold_us 6, cat_writes 7, cat_decode_us 8, cat_decoded_bytes 9, journal_us 10, journal_bytes 11.
        let v: [u64; DOUBLES.len()] = [
            c[0], c[1], u64::from(self.wake.get()), u64::from(self.writing.get()), c[2], c[3], c[4],
            c[5], c[6], p50, c[7], c[8], c[9], c[10], c[11],
        ];
        Some(Point {
            index: venue.to_string(),
            blobs: [venue.to_string(), cause.to_string(), clock.to_string()],
            doubles: v.iter().map(|&n| n as f64).collect(),
        })
    }
    /// The platform took the point: start the next delta.
    pub(crate) fn accepted(&self, nth: bool) {
        *self.cells.borrow_mut() = [0; 12];
        self.samples.borrow_mut().clear();
        self.wake.set(false);
        self.writing.set(0);
        self.writes_since.set(0);
        self.sent.set(self.sent.get() + 1);
        if nth {
            self.today.set(self.today.get() + 1);
        }
    }
    /// The platform did not: KEEP the delta (it is bounded) and say why, once per reason.
    pub(crate) fn refused(&self, why: String) {
        self.errors.set(self.errors.get() + 1);
        let mut last = self.last_error.borrow_mut();
        if last.as_deref() != Some(why.as_str()) {
            log_error!("counters -> analytics engine: {why}; counts kept in memory, the request goes on");
            *last = Some(why);
        }
    }
}

/// Where a point goes: the platform's binding, or (tests) a list that records it.
pub(crate) enum Sink<'a> {
    Live(&'a worker::Env),
    #[cfg(test)]
    Mem(&'a RefCell<Option<Vec<Point>>>),
}

/// Write one point. An absent binding is an `Err`, never a panic or a failed request.
pub(crate) fn send(sink: Sink<'_>, p: &Point) -> Result<(), String> {
    match sink {
        Sink::Live(env) => {
            let ds = env.analytics_engine(BINDING).map_err(|e| e.to_string())?;
            worker::AnalyticsEngineDataPointBuilder::new()
                .indexes([p.index.as_str()])
                .blobs(p.blobs.iter().map(String::as_str))
                .doubles(p.doubles.iter().copied())
                .write_to(&ds)
                .map_err(|e| e.to_string())
        }
        #[cfg(test)]
        Sink::Mem(fake) => match fake.borrow_mut().as_mut() {
            Some(points) => {
                points.push(p.clone());
                Ok(())
            }
            None => Err(format!("Binding `{BINDING}` is undefined.")),
        },
    }
}

#[cfg(test)]
mod tests;
