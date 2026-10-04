//! THE NIGHT, ONE VENUE PER OBJECT TURN (W-LOOP row 3; A8/FT5 in
//! docs/research/2026-10-01-*; the wall is §4.2 of 2026-10-03-hub-cost-recomputed.md).
//!
//! THE WALL. The nightly cron walked every venue inside ONE invocation: settings, the
//! idempotency sweep, two prunes, the chain and the witness, the rotation, the backup -- at
//! least four to six object and bucket calls per venue, plus the re-arm sweep's one, against a
//! per-invocation subrequest limit. At some number of venues the night stops part-way, and the
//! venues at the end of the registry are never backed up.
//!
//! THE FAN-OUT. The cron asks the PLATFORM object one question (`/fold/timer?fan=1`). That
//! object reads the registry it holds and keeps a cursor; each of its alarm turns pings
//! `FAN_BATCH` venues (`/fold/timer?night=1`: the lost-alarm check it always was, plus "your
//! night is due") and re-arms itself for the next batch. Each venue books the night and brings
//! its alarm forward; its alarm runs `cloud::night_venue` in ITS OWN turn, with its own budget,
//! its images answered in process. So: the cron makes one request, a platform turn makes at
//! most `FAN_BATCH`, and a venue's night is one alarm whatever the size of the fleet.

use super::super::HubImages;
use crate::cron::timer::{self, Seen, Tally};
use crate::wire::Reply as Response;
use worker::*;

/// Where a venue keeps "my night is due" (the instant the cron asked).
const NIGHT_KEY: &str = "timer~night";
/// Where the platform object keeps the fan-out's cursor.
const FAN_KEY: &str = "timer~fan";
use crate::cron::timer::FAN_BATCH;
/// The pause between two batches.
pub(crate) const FAN_GAP_MS: i64 = 1_000;

#[derive(serde::Serialize, serde::Deserialize)]
struct Fan {
    /// The cron's clock: every venue's night runs "as of" this instant.
    now_ms: i64,
    venues: Vec<String>,
    done: usize,
}

impl HubImages {
    /// When this object's night work is due: a booked venue night, or a fan-out under way.
    pub(super) async fn night_next(&self) -> Result<Option<i64>> {
        let store = self.state.storage();
        let night: Option<i64> = store.get(NIGHT_KEY).await?;
        let fan: Option<Fan> = store.get(FAN_KEY).await?;
        Ok(timer::next_due(&[night, fan.map(|f| f.now_ms)]))
    }

    pub(super) async fn night_due(&self) -> Result<bool> {
        Ok(self.state.storage().get::<i64>(NIGHT_KEY).await?.is_some())
    }

    pub(super) async fn night_book(&self, now_ms: i64) -> Result<()> {
        self.state.storage().put(NIGHT_KEY, now_ms).await
    }

    pub(super) async fn night_done(&self) -> Result<()> {
        self.state.storage().delete(NIGHT_KEY).await.map(|_| ())
    }

    pub(super) async fn night_fan_pending(&self) -> Result<bool> {
        Ok(self.state.storage().get::<Fan>(FAN_KEY).await?.is_some())
    }

    /// THE CRON'S ONE QUESTION, on the platform object: the registry it holds, read here
    /// (no request), into a cursor; the first batch runs on the alarm set for now.
    pub(super) async fn night_fan_start(&self, now_ms: i64) -> Result<Response> {
        use crate::platform_store::{ceiling, REGISTRY};
        let venues: Vec<String> = match self.image(REGISTRY).await? {
            Some((_, b)) => dowiz_hub::table::Table::load(&b, ceiling(REGISTRY))
                .map_err(|_| Error::RustError("night: the registry is unreadable".into()))?
                .all(crate::identity_store::K_LOC)
                .into_iter()
                .map(|(id, _)| id)
                .collect(),
            None => Vec::new(),
        };
        let n = venues.len();
        let store = self.state.storage();
        store.put(FAN_KEY, Fan { now_ms, venues, done: 0 }).await?;
        store.set_alarm_ms(now_ms).await?;
        log_line!("night: {n} venues to fan out, {FAN_BATCH} a turn");
        Response::from_json(&serde_json::json!({ "venues": n, "batch": FAN_BATCH }))
    }

    /// One platform alarm turn of the fan-out: at most `FAN_BATCH` venues pinged, the cursor
    /// moved, and the next turn armed -- or, after the last venue, the platform's own error log
    /// pruned (no venue's night reaches it) and the cursor dropped.
    pub(super) async fn night_fan(&self, base: &crate::edge::Env, at_ms: i64) -> Result<Response> {
        let store = self.state.storage();
        let Some(mut fan) = store.get::<Fan>(FAN_KEY).await? else { return Response::ok("no night") };
        let ns = base.durable_object("HUB")?;
        let end = (fan.done + FAN_BATCH).min(fan.venues.len());
        let mut tally = Tally::default();
        for venue in &fan.venues[fan.done..end] {
            let seen = ping(&ns, venue, fan.now_ms).await.map_err(|e| e.to_string());
            if seen == Ok(Seen::Rearmed) {
                crate::loud!(&ns, Some(venue), "timer.rearmed", "work was due and no alarm was set; re-armed by the nightly");
            }
            tally.add(venue, seen);
        }
        log_line!("night batch {}..{} of {}: {}", fan.done, end, fan.venues.len(), tally.line());
        fan.done = end;
        if fan.done < fan.venues.len() {
            store.put(FAN_KEY, &fan).await?;
            store.set_alarm_ms(at_ms + FAN_GAP_MS).await?;
            return Response::ok("batch");
        }
        store.delete(FAN_KEY).await?;
        // THE PLATFORM'S OWN FAILURE LOG, which no venue's night reaches: in process.
        let own = self.own_env(base, crate::platform_store::PLATFORM);
        match own.durable_object("HUB")?.id_from_name(crate::platform_store::PLATFORM)?.get_stub() {
            Ok(s) => match crate::errlog::prune_at(&s, None, fan.now_ms).await {
                Ok(0) => {}
                Ok(n) => log_line!("nightly prune platform: {n} error records"),
                Err(e) => log_error!("nightly prune platform: errors refused: {e}"),
            },
            Err(e) => log_error!("nightly prune platform: no object: {e}"),
        }
        let want = self.timer_next(at_ms).await?;
        self.timer_apply(timer::after_run(want, at_ms)).await?;
        Response::ok("night fanned out")
    }
}

/// "Your night is due": one request to one venue's object, answered with its timer's state.
async fn ping(ns: &crate::edge::ObjectNamespace, venue: &str, now_ms: i64) -> Result<Seen> {
    let stub = ns.id_from_name(venue)?.get_stub()?;
    let req = crate::wire::Call::new_with_init(&crate::cron::night_path(venue, now_ms), crate::wire::RequestInit::new().with_method(Method::Post))?;
    let mut res = stub.fetch_with_request(req).await?;
    match res.status_code() {
        200 => res.json::<Seen>().await,
        s => Err(Error::RustError(format!("answered {s}"))),
    }
}
