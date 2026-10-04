//! THE VENUE'S ALARM: the object that holds the work says when it is due, and
//! does the work IN ITS OWN TURN.
//!
//! The rules are `cron::timer` (pure); this file reads the object's own images
//! for them and sets, runs and re-arms the one alarm a Durable Object has.
//!
//! NO RUNNER ANY MORE (W-LOOP, docs/research/2026-10-03-hub-cost-recomputed.md
//! §1.3). The jobs (`cron::run`) talk to the venue through `Place`, as every
//! caller does, and a second object (`cron~<venue>`) used to run them so that
//! this one never called itself: alarm -> runner -> about ten reads back = 11.9
//! billed object requests a firing, measured. `alarm()` now hands the jobs an
//! `Env` that answers this venue's name with THIS object, in process
//! (`edge::Own`), so a firing is one alarm and the jobs' reads are function
//! calls. The lease (`outbox/lease.rs`) still guarantees one drain at a time,
//! alarm or retry.
//!
//! THE NIGHT, FANNED OUT (W-LOOP row 3, `timer/night.rs`). The nightly cron
//! asks the platform object once; it pings the venues a batch per alarm, and
//! each venue does its own night in its own alarm -- the cron invocation makes
//! one request whatever the number of venues.
//!
//! THE VENUE'S NAME. An object is addressed by `id_from_name(venue)` but is not
//! reliably told that name, and the alarm needs it to answer its own calls. It
//! is LEARNED -- from the nightly's question, the platform's `id.name`, or the
//! catalogue's own record -- and every candidate is CHECKED: its
//! `id_from_name` must be this object's id, so a catalogue that named another
//! venue could never make this object drain that venue's queue.

use super::{HubImages, CATALOG_IMAGE};
use crate::cron::timer::{self, Arm, Seen};
use dowiz_hub::table::Table;
use worker::*;
// The plain-Rust request/response (W-COV C2): these bodies run under `cargo test`.
use crate::wire::{Call as Request, Reply as Response};

mod night; // the nightly fan-out and the venue's own night (W-LOOP row 3), `hubdo/timer/night.rs`

/// Where the learned name is kept.
const VENUE_KEY: &str = "timer~venue";

fn bad(e: impl std::fmt::Display) -> Error {
    Error::RustError(format!("timer: {e}"))
}

impl HubImages {
    /// When this venue next has work, from its own images; `None` = idle.
    pub(super) async fn timer_next(&self, now_ms: i64) -> Result<Option<i64>> {
        let outbox: Vec<crate::outbox::Entry> = match self.image(crate::outbox::IMAGE_OUTBOX).await? {
            Some((_, b)) => Table::load(&b, crate::outbox::OUTBOX_BYTES)
                .map_err(|_| bad("the outbox image is unreadable"))?
                .all(crate::outbox::KIND)
                .into_iter()
                .filter_map(|(_, j)| serde_json::from_str(&j).ok())
                .collect(),
            None => Vec::new(),
        };
        let ebills = self.ebills_next(now_ms).await?;
        let fiscal = match (crate::fiscal::SEND_ENABLED, self.image(crate::fiscal::wire::IMAGE).await?) {
            (true, Some((_, b))) => Table::load(&b, crate::fiscal::wire::CEILING)
                .map_err(|_| bad("the fiscal image is unreadable"))?
                .all(crate::fiscal::queue::KIND)
                .len(),
            _ => 0,
        };
        let fiscal = timer::fiscal_next(crate::fiscal::SEND_ENABLED, fiscal, now_ms);
        // An offline sale past its 48 h is said once (`room/offline.rs`).
        // Its failure never stops the outbox's alarm: said, and read as nothing due.
        let overdue = self.offline_overdue_next().await.unwrap_or_else(|e| { log_error!("timer: offline overdue unreadable: {e}"); None });
        let night = self.night_next().await?;
        Ok(timer::next_due(&[timer::outbox_next(&outbox), ebills, fiscal, overdue, night]))
    }

    /// The till link's next firing (`timer::ebills_next`), with the venue's
    /// polling hours (`ebills::cadence`) and the latest change it has seen:
    /// the floor image's `at_ms` is written only when a table moved.
    async fn ebills_next(&self, now_ms: i64) -> Result<Option<i64>> {
        use crate::ebills::state::{self, Config, State, FLOOR_IMAGE, K_CONFIG, K_STATE, ONE};
        let Some((_, b)) = self.image(state::IMAGE).await? else { return Ok(None) };
        let t = Table::load(&b, state::CEILING).map_err(|_| bad("the ebills image is unreadable"))?;
        let cfg: Option<Config> = state::get(&t, K_CONFIG, ONE).map_err(bad)?;
        if !cfg.as_ref().is_some_and(Config::usable) {
            return Ok(None);
        }
        let st: State = state::get(&t, K_STATE, ONE).map_err(bad)?.unwrap_or_default();
        let loc: serde_json::Value = match self.image(CATALOG_IMAGE).await? {
            Some((_, b)) => dowiz_hub::catalog::Catalog::load(&b)
                .ok()
                .and_then(|c| c.location())
                .and_then(|j| serde_json::from_str(&j).ok())
                .unwrap_or_default(),
            None => serde_json::Value::Null,
        };
        let floor_at = match self.image(FLOOR_IMAGE).await? {
            Some((_, b)) => serde_json::from_slice::<serde_json::Value>(&b).ok().and_then(|v| v["at_ms"].as_i64()).unwrap_or(0),
            None => 0,
        };
        let sched = crate::ebills::cadence::schedule(&loc);
        let zone = crate::hubstore::zone_of(Some(&loc));
        let open_at = |t: i64| crate::ebills::cadence::open_at(&sched, zone, t);
        Ok(timer::ebills_next(true, &st, now_ms, &open_at, floor_at))
    }

    async fn timer_apply(&self, arm: Arm) -> Result<()> {
        let store = self.state.storage();
        match arm {
            Arm::Set(at) => store.set_alarm_ms(at).await,
            Arm::Clear => store.delete_alarm().await,
            Arm::Keep => Ok(()),
        }
    }

    /// Is `venue` this object's name? `id_from_name` says, and nothing else.
    fn is_me(&self, venue: &str) -> bool {
        self.state.is_me(venue)
    }

    /// This object's venue: the stored name, the one it was just `told`, the
    /// platform's, or its catalogue's -- the first that checks. Stored once.
    async fn own_venue(&self, told: Option<&str>) -> Option<String> {
        let store = self.state.storage();
        let stored: Option<String> = store.get(VENUE_KEY).await.ok().flatten();
        let catalogue = || async {
            match self.image(CATALOG_IMAGE).await.ok().flatten() {
                Some((_, b)) => dowiz_hub::catalog::Catalog::load(&b)
                    .ok()
                    .and_then(|c| c.location())
                    .and_then(|j| serde_json::from_str::<serde_json::Value>(&j).ok())
                    .and_then(|v| v.get("id").and_then(|i| i.as_str()).map(str::to_string)),
                None => None,
            }
        };
        let mut found = [stored.clone(), told.map(str::to_string), self.state.own_name()]
            .into_iter()
            .flatten()
            .find(|v| self.is_me(v));
        if found.is_none() {
            found = catalogue().await.filter(|v| self.is_me(v));
        }
        if let Some(v) = &found {
            if stored.as_deref() != Some(v) {
                let _ = store.put(VENUE_KEY, v.clone()).await;
            }
        }
        found
    }

    /// After a write to a `timer::TIMED` image: bring the alarm forward to the
    /// work. Never fails the write it follows -- a lost arm is the nightly's
    /// to find (`timer::rearm`), and it is said here.
    pub(super) async fn timer_after_write(&self, now_ms: i64) {
        if self.in_alarm.get() {
            return; // the run re-arms when it ends (`timer_alarm`)
        }
        let armed = async {
            let want = self.timer_next(now_ms).await?;
            if want.is_some() {
                // Learned while a request is here, so the alarm has it later.
                let _ = self.own_venue(None).await;
            }
            let have = self.state.storage().get_alarm().await?;
            self.timer_apply(timer::on_write(have, want, now_ms)).await
        }
        .await;
        if let Err(e) = armed {
            log_error!("timer: the alarm was not set after a write: {e}");
        }
    }

    /// `alarm()` on the platform: its `Env` is the one the jobs get.
    pub(super) async fn timer_alarm(&self, now_ms: i64) -> Result<Response> {
        let Some(env) = self.state.env() else {
            return Response::error("an alarm outside the platform", 500);
        };
        self.timer_alarm_in(&crate::edge::Env::Live(env.clone()), now_ms).await
    }

    /// One run of the venue's jobs IN THIS OBJECT'S TURN, then the next alarm
    /// -- or none. `base` is the platform (or, in tests, memory); the jobs get
    /// it wrapped so that this venue's own name is answered in process. An
    /// error is returned, not swallowed: the platform retries a failed alarm
    /// (at-least-once), and every job is idempotent.
    pub(crate) async fn timer_alarm_in(&self, base: &crate::edge::Env, now_ms: i64) -> Result<Response> {
        // THE PLATFORM OBJECT'S NIGHT: a batch of venues pinged (`timer/night.rs`).
        if self.night_fan_pending().await? {
            return self.night_fan(base, now_ms).await;
        }
        let Some(venue) = self.own_venue(None).await else {
            // Nothing can be run without the name; the nightly tells it.
            log_error!("timer: an alarm fired in an object that does not know its venue");
            return Response::ok("no venue");
        };
        let env = self.own_env(base, &venue);
        self.in_alarm.set(true);
        // BEFORE the runner, so the alert it queues is drained in this run.
        self.offline_overdue(&venue, now_ms).await;
        let ran = async {
            if self.night_due().await? {
                crate::cloud::night_venue(&env, &venue, now_ms).await;
                self.night_done().await?;
            }
            crate::cron::run(&env, &venue, now_ms).await;
            Ok::<(), Error>(())
        }
        .await;
        self.in_alarm.set(false);
        ran?;
        let arm = match self.timer_next(now_ms).await {
            Ok(want) => timer::after_run(want, now_ms),
            // UNREADABLE IS NOT IDLE: try again in a minute rather than never.
            Err(e) => {
                log_error!("timer {venue}: {e}");
                Arm::Set(now_ms + timer::RUN_GAP_MS)
            }
        };
        self.timer_apply(arm).await?;
        Response::ok("ran")
    }

    /// `base`, with this venue's own name answered by this object (`edge::Own`).
    fn own_env(&self, base: &crate::edge::Env, venue: &str) -> crate::edge::Env {
        // SAFETY: the `'static` the platform's own dispatch already takes for
        // every `alarm()` and `fetch()` (worker-macros `durable_object.rs`: "Durable
        // Object will never be destroyed while there is still a running promise
        // inside of it"). The `Env` built here lives in this call's future only:
        // nothing it reaches is spawned past it.
        let me: &'static super::HubImages = unsafe { &*(self as *const super::HubImages) };
        crate::edge::Env::Own(std::rc::Rc::new(crate::edge::Own { me, venue: venue.to_string(), base: base.clone() }))
    }

    /// `POST /fold/timer?venue=&now=[&night=1|&fan=1]`: the nightly's safety
    /// net. Re-arms a lost alarm; answers what it found (`timer::Seen`).
    /// `night=1` (from the platform's fan-out) also books this venue's night
    /// and brings the alarm forward to it; `fan=1`, asked of the platform
    /// object by the cron, starts the fan-out (`timer/night.rs`).
    pub(super) async fn timer_route(&self, req: Request) -> Result<Response> {
        let url = req.url()?;
        let pairs: Vec<(String, String)> = url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        let Some((venue, now_ms)) = crate::cron::parse_runner_query(pairs.iter().cloned()) else {
            return Response::error("timer needs a venue and a clock", 400);
        };
        if !self.is_me(&venue) {
            return Response::error("that venue is not this object", 409);
        }
        let flag = |k: &str| pairs.iter().any(|(a, b)| a == k && b == "1");
        if flag("fan") {
            return self.night_fan_start(now_ms).await;
        }
        let _ = self.own_venue(Some(&venue)).await;
        let want = self.timer_next(now_ms).await?;
        let have = self.state.storage().get_alarm().await?;
        let arm = timer::rearm(have, want, now_ms);
        self.timer_apply(arm).await?;
        // The answer is about the work that was due BEFORE the night was booked:
        // a lost alarm is still named as one.
        let seen = Seen::of(arm, have);
        if flag("night") {
            self.night_book(now_ms).await?;
            let have = self.state.storage().get_alarm().await?;
            self.timer_apply(timer::on_write(have, Some(now_ms), now_ms)).await?;
        }
        Response::from_json(&seen)
    }
}

/// The alarm, the jobs in its own turn and the re-arm, natively (W-INT2 #44, W-LOOP).
#[cfg(test)]
mod tests;
