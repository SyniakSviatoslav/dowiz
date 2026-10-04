//! ONE VENUE'S NIGHT, in that venue's own alarm turn (W-LOOP row 3).
//!
//! This was the body of `cloud::nightly`'s loop over the registry, run for every
//! venue inside the one cron invocation. It is the same work in the same order --
//! retention sweep, both prunes, the chain and the witness, the rotation, the copy --
//! now asked for by the platform's fan-out (`hubdo/timer/night.rs`) and run by the
//! venue's own alarm, with `env` answering the venue's images in process
//! (`edge::Own`): the night of one venue no longer spends the subrequests of all.

use super::*;

struct Row {
    id: String,
}

/// One venue's night, as of `now` (the cron's clock, carried by the fan-out).
pub async fn night_venue(env: &Env, venue: &str, now: i64) {
    // Read once a night: every venue's copy is sealed to the same platform key.
    let seal_state = seal::state(env);
    let Ok(ns) = env.durable_object("HUB") else { return log_error!("night {venue}: no HUB binding") };
    let place = crate::hubstore::Place { ns, venue: venue.to_string() };
    let r = Row { id: venue.to_string() };
    let configured = match crate::hubstore::load_settings(&place).await {
        Ok(l) => cfg(&l.settings).is_some(),
        Err(_) => false,
    };
    // Idempotency keys past their window. The image is small and the sweep
    // is what keeps it that way; without it a busy venue accumulates a key
    // per order for ever.
    match crate::idempotency::sweep(&place, now).await {
        Ok(0) => {}
        Ok(n) => log_line!("nightly sweep {}: {n} idempotency keys", r.id),
        Err(e) => log_error!("nightly sweep {}: keys refused: {e}", r.id),
    }

    // The venue's own failure log: anything older than a week, and
    // anything past the count, goes. A burst inside one day is exactly when
    // this instrument matters and exactly when an age-only rule keeps
    // everything.
    match place.stub() {
        Ok(s) => match crate::errlog::prune_at(&s, Some(&r.id), now).await {
            Ok(0) => {}
            Ok(n) => log_line!("nightly prune {}: {n} error records", r.id),
            Err(e) => log_error!("nightly prune {}: errors refused: {e}", r.id),
        },
        Err(e) => log_error!("nightly prune {}: no object: {e}", r.id),
    }

    // THE COURIER CHAT OF ORDERS LONG OVER (operator 2026-10-02): a line
    // older than thirty days goes with the order's personal data
    // (`services/orders/chat/store.rs` KEEP_MS). The venue's own thread in
    // the same image is not this prune's.
    match place.stub() {
        Ok(s) => match crate::services::orders::chat::store::prune_at(&s, now).await {
            Ok(0) => {}
            Ok(n) => log_line!("nightly prune {}: {n} courier chat lines", r.id),
            Err(e) => log_error!("nightly prune {}: chat refused: {e}", r.id),
        },
        Err(e) => log_error!("nightly prune {}: no object: {e}", r.id),
    }

    // THE CHAIN AND THE WITNESS, BEFORE ANYTHING TOUCHES THE LOG.
    //
    // ONE LOAD FOR BOTH. The image is the largest thing this job reads and
    // reading it twice would double the night's bill for the same bytes;
    // the two checks are different questions about the same image, not two
    // errands.
    //
    // `Hub::chain_check` shipped in phase 4 and NOTHING IN PRODUCTION HAD
    // EVER CALLED IT. An append-only log whose ids are never recomputed is
    // append-only by assertion; this is the night the assertion is checked.
    // It runs before the rotation so that what it reports is the log as the
    // day left it, and before the backup so that an archive is never the
    // first place a broken chain is noticed.
    //
    // WHAT IT CAN AND CANNOT SEE, stated here because a check whose reach
    // is misunderstood is worse than none: it detects an EDITED record --
    // an id that no longer commits to its payload, and every id after it,
    // because the chain cascades. It cannot detect a record REMOVED from
    // the end, because what is left is a shorter valid chain, nor a log
    // rebuilt from scratch, because whoever can write the image can
    // recompute every id in it. THAT is what the witness beside it is for:
    // the census goes to the platform object -- which is not the venue's --
    // and from there into the night's off-site copy, so a truncation has to
    // contradict a second account kept somewhere the editor cannot reach.
    match futures_util::future::try_join(
        crate::hubstore::load(&place),
        crate::hubstore::load_settings(&place),
    )
    .await
    {
        Ok((l, s)) => {
            let c = l.hub.chain_check();
            if c.intact() {
                log_line!(
                    "nightly chain {}: {} records, {} chained, {} legacy",
                    r.id, c.records, c.chained, c.legacy
                );
            } else {
                // LOUD, and it must stay loud: this is the one condition in
                // this whole job that means somebody edited the ledger.
                crate::loud!(
                    &place.ns, Some(&r.id), "hub.chain",
                    "BROKEN CHAIN: {} of {} records match neither scheme ({} chained, {} legacy)",
                    c.broken, c.records, c.chained, c.legacy
                );
            }
            match crate::witness::nightly(&place, &l.hub, &s.settings, now).await {
                Ok((w, found)) if found.is_empty() => log_line!(
                    "nightly witness {}: {} records, {} archived, tip {}",
                    r.id, w.records, w.archived(), w.tip.as_deref().unwrap_or("-")
                ),
                Ok((_, found)) => {
                    // Said once per contradiction, so the record names what
                    // disagreed rather than that something did.
                    for what in found {
                        crate::loud!(&place.ns, Some(&r.id), "hub.witness", "CONTRADICTED: {what}");
                    }
                }
                Err(e) => {
                    crate::loud!(&place.ns, Some(&r.id), "hub.witness", "not witnessed: {e}")
                }
            }
        }
        // A venue with no log yet is not a failure; an unreadable one is.
        Err(e) => crate::loud!(&place.ns, Some(&r.id), "hub.chain", "not checked: {e}"),
    }

    // ROTATION BEFORE THE COPY. Finished history older than thirty days
    // leaves the hot log and becomes its own image; the backup that runs a
    // moment later carries both, so the night a venue's log is bounded is
    // also the night its archive is first copied off-site.
    //
    // A rotation that fails does not stop the backup: the copy of an
    // unrotated log is still a copy.
    match crate::hubstore::rotate(&place, now).await {
        Ok(v) => {
            if v.get("rotated").and_then(serde_json::Value::as_bool) == Some(true) {
                log_line!("nightly rotate {}: {}", r.id, v);
            }
        }
        Err(e) => crate::loud!(&place.ns, Some(&r.id), "hub.rotate", "nightly: {e}"),
    }
    if !configured {
        return;
    }
    match push_place(&place, now, &seal_state).await {
        Ok(v) => log_line!("nightly backup {}: {}", r.id, v),
        Err(e) => {
            crate::loud!(&place.ns, Some(&r.id), "cloud.nightly", "backup refused: {e}")
        }
    }
}
