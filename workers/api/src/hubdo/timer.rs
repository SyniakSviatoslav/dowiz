//! THE VENUE'S ALARM: the object that holds the work says when it is due.
//!
//! The rules are `cron::timer` (pure); this file reads the object's own images
//! for them and sets, runs and re-arms the one alarm a Durable Object has.
//!
//! WHY THE ALARM LIVES HERE AND THE WORK IN THE RUNNER. This object holds the
//! outbox, the till link's state and the fiscal queue in memory, so it can say
//! when work is due from a write it is already making -- no request, no
//! registry read. But the jobs (`cron::run`) talk to the venue through its
//! object like every other caller does, so running them HERE would have this
//! object call itself; the runner `cron~<venue>` exists so that it never does
//! (`crate::cron`'s header). So `alarm()` asks the runner for one run -- one
//! request -- and the lease (`outbox/lease.rs`) still guarantees one drain at a
//! time, alarm or retry.
//!
//! THE VENUE'S NAME. An object is addressed by `id_from_name(venue)` but is not
//! reliably told that name, and the alarm needs it to address the runner. It
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
        Ok(timer::next_due(&[timer::outbox_next(&outbox), ebills, fiscal]))
    }

    /// The till link's next firing (`timer::ebills_next`), with the venue's
    /// opening hours from its catalogue record.
    async fn ebills_next(&self, now_ms: i64) -> Result<Option<i64>> {
        use crate::ebills::state::{self, Config, State, K_CONFIG, K_STATE, ONE};
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
        let sched = loc.get("hours").map(|h| dowiz_hub::hours::from_json(&h.to_string())).unwrap_or_default();
        let zone = crate::hubstore::zone_of(Some(&loc));
        let open_at = |t: i64| {
            let (weekday, minute) = dowiz_hub::tz::local_weekday_minute(zone, t);
            sched.is_empty() || sched.is_open_at(weekday, minute)
        };
        Ok(timer::ebills_next(true, &st, now_ms, &open_at))
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

    /// `alarm()`: one run of the venue's jobs in its runner, then the next
    /// alarm -- or none. AN ERROR IS RETURNED, not swallowed: the platform
    /// retries a failed alarm (at-least-once), and every job is idempotent.
    pub(super) async fn timer_alarm(&self, now_ms: i64) -> Result<Response> {
        let Some(venue) = self.own_venue(None).await else {
            // Nothing can be run without the name; the nightly tells it.
            log_error!("timer: an alarm fired in an object that does not know its venue");
            return Response::ok("no venue");
        };
        self.in_alarm.set(true);
        let ran = async {
            match self.state.call_runner(&venue, now_ms).await? {
                200 => Ok(()),
                s => Err(bad(format!("the runner answered {s}"))),
            }
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

    /// `POST /fold/timer?venue=&now=`: the nightly's safety net. Re-arms a
    /// lost alarm; answers what it found (`timer::Seen`).
    pub(super) async fn timer_route(&self, req: Request) -> Result<Response> {
        let url = req.url()?;
        let Some((venue, now_ms)) = crate::cron::parse_runner_query(url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())))
        else {
            return Response::error("timer needs a venue and a clock", 400);
        };
        if !self.is_me(&venue) {
            return Response::error("that venue is not this object", 409);
        }
        let _ = self.own_venue(Some(&venue)).await;
        let want = self.timer_next(now_ms).await?;
        let have = self.state.storage().get_alarm().await?;
        let arm = timer::rearm(have, want, now_ms);
        self.timer_apply(arm).await?;
        Response::from_json(&Seen::of(arm, have))
    }
}
