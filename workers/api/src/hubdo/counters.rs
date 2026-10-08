//! AX0-COUNTERS: the numbers AX2 and AX4 may not be built without (plan
//! `docs/research/2026-10-02-ax-implementation-plan.md` §A0), counted IN the object and read by
//! `/api/owner/health` as `counters`.
//!
//!   (a) `since_total` / `since_none`: `?since=` catch-ups, and how many the window could not
//!       answer (`changes_since` = `None`, "ask for the list") -- AX2's premise.
//!   (b) `wakes_total` / `wakes_readonly`: this window holds the object's wake (its constructor
//!       ran) and, so far, that wake has served no write -- AX4's gate.
//!   (c) `cold_fold_us`: the wall of an `orders_view` that folded from bytes (the memo missed),
//!       the image read included; `cold_fold_first_us` is the first one after the wake.
//!   (d) `proj_rows`: chunks a write stored (`changed.len()`), per `writes`.
//!   (e) per catalogue write: the two `Catalog::load` decodes (`cat_decode_us`,
//!       `cat_decoded_bytes`) and the journal's load + append (`journal_us`, `journal_bytes`).
//!
//! HIBERNATION ERASES THEM. Nothing here is stored: a row written per write to keep a counter
//! would cost the Free plan's 100,000 rows/day for a gauge. So the counters live for one wake,
//! and the honesty is in the `window`: `cause: "wake"` says the window began when the object
//! woke, so whatever an earlier wake counted after the last flush is GONE -- the collector,
//! which remembers its previous flush, sees the gap instead of a clean zero. `?flush=1` (the
//! nightly's `/api/owner/health?counters=flush`) answers and zeroes, so summed reads never count
//! twice; the owner's own health pane reads without flushing.
//!
//! THE CLOCK: `otel::wall_us`, which on the platform only advances after I/O (Spectre). A live
//! `cat_decode_us` is therefore ~0 by construction and `journal_us` is the journal's storage read;
//! natively they are real µs. `clock` in the snapshot says which.
//!
//! W-AE: every count is ALSO kept as a delta for Workers Analytics Engine (`counters/ae.rs`), written
//! at each flush and every `ae::NTH` stored writes, so the platform keeps what a hibernation erases.

use super::HubImages;
use crate::wire::{Call, Reply};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use worker::Result;

/// The flushed cells, in snapshot order.
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    SinceTotal,
    SinceNone,
    Reads,
    Writes,
    ProjRows,
    ColdFolds,
    ColdFoldUs,
    CatWrites,
    CatDecodeUs,
    CatDecodedBytes,
    JournalUs,
    JournalBytes,
}
const NAMES: [&str; 12] = [
    "since_total", "since_none", "reads", "writes", "proj_rows", "cold_folds", "cold_fold_us",
    "cat_writes", "cat_decode_us", "cat_decoded_bytes", "journal_us", "journal_bytes",
];
/// The cold folds kept one by one, for a p50 across venues (`cf.cold_fold_us_p50`).
const SAMPLES: usize = 16;

pub(crate) struct Counters {
    cells: RefCell<[u64; NAMES.len()]>,
    samples: RefCell<Vec<u64>>,
    woke_at_us: u64,
    /// Where the window the cells cover began: the wake, or the last flush.
    from_us: Cell<u64>,
    flushes: Cell<u32>,
    /// The wake's own, never flushed: what it served so far.
    wake_reads: Cell<u64>,
    wake_writes: Cell<u64>,
    first_fold_us: Cell<Option<u64>>,
    /// W-AE: the delta since the last Analytics Engine point (`counters/ae.rs`).
    ae: ae::Pending,
    /// Tests: the fake dataset; `None` = the binding is absent.
    #[cfg(test)]
    pub(crate) fake_ae: RefCell<Option<Vec<ae::Point>>>,
}

impl Counters {
    /// The constructor's hook: one wake, now.
    pub(crate) fn woke() -> Self {
        let now = crate::otel::wall_us();
        Counters {
            cells: RefCell::new([0; NAMES.len()]),
            samples: RefCell::new(Vec::new()),
            woke_at_us: now,
            from_us: Cell::new(now),
            flushes: Cell::new(0),
            wake_reads: Cell::new(0),
            wake_writes: Cell::new(0),
            first_fold_us: Cell::new(None),
            ae: ae::Pending::woke(),
            #[cfg(test)]
            fake_ae: RefCell::new(Some(Vec::new())),
        }
    }

    pub(crate) fn bump(&self, k: Kind, n: u64) {
        let mut c = self.cells.borrow_mut();
        c[k as usize] = c[k as usize].saturating_add(n);
        self.ae.bump(k as usize, n);
    }

    /// (a): a catch-up's answer, counted and handed straight back.
    pub(crate) fn since<T>(&self, answer: Option<T>) -> Option<T> {
        self.bump(Kind::SinceTotal, 1);
        if answer.is_none() {
            self.bump(Kind::SinceNone, 1);
        }
        answer
    }

    /// A read the object served (every `GET /fold/...`).
    pub(crate) fn read(&self) {
        self.bump(Kind::Reads, 1);
        self.wake_reads.set(self.wake_reads.get() + 1);
    }

    /// (b) + (d): a stored write of `rows` chunks. The first one ends the wake's read-only run.
    /// True when an Analytics Engine point is due (`ae::NTH`).
    pub(crate) fn wrote(&self, rows: usize) -> bool {
        self.bump(Kind::Writes, 1);
        self.bump(Kind::ProjRows, rows as u64);
        self.wake_writes.set(self.wake_writes.get() + 1);
        self.ae.wrote(self.wake_writes.get() == 1, crate::otel::wall_us())
    }

    /// (c): one fold from bytes, `us` from the start of `orders_view`.
    pub(crate) fn cold_fold(&self, us: u64) {
        self.bump(Kind::ColdFolds, 1);
        self.bump(Kind::ColdFoldUs, us);
        if self.first_fold_us.get().is_none() {
            self.first_fold_us.set(Some(us));
        }
        let mut s = self.samples.borrow_mut();
        if s.len() < SAMPLES {
            s.push(us);
        }
        self.ae.sample(us);
    }

    /// (e): one catalogue write's decode and journal cost.
    pub(crate) fn catalogue_write(&self, decode_us: u64, decoded: usize, journal_us: u64, journal: usize) {
        self.bump(Kind::CatWrites, 1);
        self.bump(Kind::CatDecodeUs, decode_us);
        self.bump(Kind::CatDecodedBytes, decoded as u64);
        self.bump(Kind::JournalUs, journal_us);
        self.bump(Kind::JournalBytes, journal as u64);
    }

    /// The window, its cells and the wake; `flush` zeroes the cells and starts the next window.
    pub(crate) fn snapshot(&self, flush: bool) -> Value {
        let now = crate::otel::wall_us();
        let from = self.from_us.get();
        // The wake is counted in the ONE window that begins with it, so summed flushes count it once.
        let woke_here = from == self.woke_at_us && self.flushes.get() == 0;
        let readonly = woke_here && self.wake_writes.get() == 0;
        let mut out = serde_json::Map::new();
        for (name, v) in NAMES.iter().zip(self.cells.borrow().iter()) {
            out.insert((*name).into(), json!(v));
        }
        out.insert("wakes_total".into(), json!(u64::from(woke_here)));
        out.insert("wakes_readonly".into(), json!(u64::from(readonly)));
        out.insert("cold_fold_samples".into(), json!(*self.samples.borrow()));
        out.insert("cold_fold_first_us".into(), json!(self.first_fold_us.get()));
        out.insert("window".into(), json!({
            "fromMs": from / 1000, "toMs": now / 1000, "wokeAtMs": self.woke_at_us / 1000,
            "cause": if woke_here { "wake" } else { "flush" }, "flushes": self.flushes.get(), "flushed": flush,
        }));
        out.insert("wake".into(), json!({
            "atMs": self.woke_at_us / 1000, "reads": self.wake_reads.get(), "writes": self.wake_writes.get(),
        }));
        out.insert("clock".into(), json!(clock()));
        out.insert("ae".into(), json!({
            "binding": ae::BINDING, "points": self.ae.sent.get(), "errors": self.ae.errors.get(), "lastError": *self.ae.last_error.borrow(),
        }));
        if flush {
            *self.cells.borrow_mut() = [0; NAMES.len()];
            self.samples.borrow_mut().clear();
            self.from_us.set(now.max(from + 1));
            self.flushes.set(self.flushes.get() + 1);
        }
        Value::Object(out)
    }
}

fn clock() -> &'static str {
    if cfg!(target_arch = "wasm32") { "platform-ms-advances-on-io-only" } else { "native-us" }
}

impl HubImages {
    /// `GET /fold/counters[?flush=1]`. A flush writes the window's Analytics Engine point first.
    pub(super) fn counters_route(&self, req: &Call) -> Result<Reply> {
        let flush = req.url()?.query_pairs().any(|(k, v)| k == "flush" && v == "1");
        if flush {
            self.ae_point("flush");
        }
        Reply::from_json(&self.counters.snapshot(flush))
    }

    /// AX0 (b) + (d) for a stored write, and every `ae::NTH`th one's Analytics Engine point.
    pub(super) fn count_write(&self, rows: usize) {
        if self.counters.wrote(rows) {
            self.ae_point("nth");
        }
    }

    /// W-AE: write what is pending as ONE point. A refusal (no binding, a platform error) is logged
    /// and kept in memory; it never fails the request that caused it.
    fn ae_point(&self, cause: &str) {
        let venue = self.state.own_name().or_else(|| self.test_name()).unwrap_or_default();
        let Some(point) = self.counters.ae.point(&venue, cause, clock()) else { return };
        let sink = match self.state.env() {
            Some(env) => ae::Sink::Live(env),
            #[cfg(test)]
            None => ae::Sink::Mem(&self.counters.fake_ae),
            #[cfg(not(test))]
            None => return,
        };
        match ae::send(sink, &point) {
            Ok(()) => self.counters.ae.accepted(cause == "nth"),
            Err(why) => self.counters.ae.refused(why),
        }
    }

    /// The test host's venue name (the live one is the platform's `ctx.id.name`).
    fn test_name(&self) -> Option<String> {
        #[cfg(test)]
        if let super::host::Host::Mem(m) = &self.state {
            return m.name.clone();
        }
        None
    }
}

/// The Worker's side, for `/api/owner/health`: the object's counters, or the error said out loud
/// (an unreadable gauge is not a zero one).
pub async fn ask(place: &crate::hubstore::Place, flush: bool) -> Value {
    let got = async {
        let req = Call::new(&format!("https://hub/fold/counters?flush={}", u8::from(flush)), worker::Method::Get)?;
        let mut res = place.stub()?.fetch_with_request(req).await?;
        if res.status_code() != 200 {
            return Err(worker::Error::RustError(format!("the object answered {}", res.status_code())));
        }
        res.json::<Value>().await
    };
    got.await.unwrap_or_else(|e| json!({ "error": e.to_string() }))
}

pub(crate) mod ae;

#[cfg(test)]
mod tests;
