//! THE STORAGE AND THE DRAIN.
//!
//! THE ENQUEUE IS IN THE OBJECT'S TURN AND THE DRAIN IS IN THE WORKER'S, and the
//! split is the whole design. An effect is written beside the event that caused
//! it, inside the one turn that already writes the log — so "the order landed"
//! and "the message is owed" are the same fact and cannot come apart. Sending is
//! a `fetch` to a third party, which belongs nowhere near that turn: it is done
//! later, by the cron, which has the `Env` the rails need and can take as long
//! as it takes.

use super::*;
use worker::*;

/// Every entry waiting for this venue.
pub async fn waiting(place: &crate::hubstore::Place) -> Result<Vec<Entry>> {
    let mut out: Vec<Entry> = Vec::new();
    crate::hubstore::with_table(place, IMAGE_OUTBOX, OUTBOX_BYTES, |t| {
        out = t
            .all(KIND)
            .into_iter()
            .filter_map(|(_, j)| serde_json::from_str::<Entry>(&j).ok())
            .collect();
        Ok(())
    })
    .await?;
    Ok(out)
}

/// Attempt every entry that is due, and apply each verdict.
///
/// RETURNS WHAT HAPPENED rather than logging it: the caller is a cron, and a
/// cron whose outcome is only in a sampled trace is a cron nobody can tell has
/// stopped.
pub async fn drain(
    env: &Env,
    place: &crate::hubstore::Place,
    now_ms: i64,
) -> Result<(usize, usize, Vec<String>)> {
    let entries = waiting(place).await?;
    if entries.is_empty() {
        return Ok((0, 0, Vec::new()));
    }
    let settings = crate::hubstore::load_settings(place).await?.settings;
    let token = crate::notify::bot_token(env, &settings);
    let wa = crate::channels::whatsapp_cfg(&settings);

    let (mut sent, mut kept) = (0usize, 0usize);
    let mut abandoned: Vec<String> = Vec::new();
    let mut verdicts: Vec<(String, Verdict)> = Vec::new();
    for e in due(&entries, now_ms) {
        let ok = match e.kind.as_str() {
            // THE PRINTER PULLS ITS OWN (`print_rail.rs`, LAST-MILE §3.1):
            // the cron never sends a ticket and never abandons one -- the
            // printer's DELETE is the delivery and its failures the retries.
            crate::print_rail::KIND => continue,
            "telegram" => match token.as_deref() {
                Some(t) => crate::notify::telegram(t, &e.to, &e.text).await.is_ok(),
                // A RAIL THAT IS NOT CONFIGURED HAS NOT FAILED. A venue whose
                // bot token is briefly missing must not burn six attempts and
                // lose the message; it is left where it is, and the next drain
                // finds it. `/api/owner/health` shows it waiting, which is the
                // honest state.
                None => continue,
            },
            "whatsapp" => match &wa {
                Some(cfg) => crate::channels::whatsapp_text(cfg, &e.to, &e.text).await.is_ok(),
                None => continue,
            },
            // AN UNKNOWN KIND IS ABANDONED, NOT GUESSED AT. Guessing means
            // sending a stranger something; the record names it on the way out.
            other => {
                abandoned.push(format!("{}: unknown channel {other:?}", e.id));
                verdicts.push((e.id.clone(), Verdict::Abandon { after: e.tries }));
                continue;
            }
        };
        verdicts.push((e.id.clone(), after_attempt(e, ok, now_ms)));
    }
    for (id, v) in &verdicts {
        match v {
            Verdict::Sent => sent += 1,
            Verdict::Retry { .. } => kept += 1,
            Verdict::Abandon { after } if *after > 0 => {
                abandoned.push(format!("{id} after {after} attempts"))
            }
            // An unknown channel already named itself above.
            Verdict::Abandon { .. } => {}
        }
    }
    if verdicts.is_empty() {
        return Ok((0, entries.len(), Vec::new()));
    }
    let apply = verdicts.clone();
    crate::hubstore::with_table(place, IMAGE_OUTBOX, OUTBOX_BYTES, move |t| {
        for (id, v) in &apply {
            match v {
                Verdict::Sent | Verdict::Abandon { .. } => {
                    // `remove` answers whether it was there; it not being
                    // there is not an error -- two drains overlapping is a
                    // second delivery avoided, not a failure.
                    t.remove(KIND, id);
                }
                Verdict::Retry { tries, next_at_ms } => {
                    let Some(mut e) =
                        t.get(KIND, id).and_then(|j| serde_json::from_str::<Entry>(&j).ok())
                    else {
                        continue;
                    };
                    e.tries = *tries;
                    e.next_at_ms = *next_at_ms;
                    let rec = serde_json::to_string(&e).unwrap_or_default();
                    t.put(KIND, id, &rec, &[], &[])
                        .map_err(|e| Error::RustError(format!("outbox: {e:?}")))?;
                }
            }
        }
        Ok(())
    })
    .await?;
    Ok((sent, kept, abandoned))
}

/// The minute cron: deliver what every venue is owed.
///
/// IT WALKS THE REGISTRY, one platform read plus one object read per venue,
/// and a venue with no outbox image answers 204 -- which is most of them, most
/// of the time, and is the cheapest answer a Durable Object can give.
///
/// THE BOUND IS WRITTEN DOWN RATHER THAN DISCOVERED: at roughly forty venues
/// this becomes forty object wakes a minute whether or not anything is queued,
/// and the answer at that point is a platform-level set of venues with
/// something waiting, written by the enqueue. It is NOT built now, because a
/// set that can disagree with the outboxes is a second source of truth, and
/// two venues do not need one.
///
/// LOUD ON ANYTHING IT ABANDONS. A message that has failed six times is a
/// broken integration, and the venue's own error log is where an owner looks.
pub async fn sweep(env: &Env, now_ms: i64) {
    let Ok(ns) = env.durable_object("HUB") else {
        console_error!("outbox sweep: no HUB binding");
        return;
    };
    let registry = match crate::identity_store::registry(env).await {
        Ok(t) => t,
        Err(e) => {
            console_error!("outbox sweep: registry unreadable: {e}");
            return;
        }
    };
    for (venue, _) in registry.all(crate::identity_store::K_LOC) {
        let Ok(ns) = env.durable_object("HUB") else { continue };
        let place = crate::hubstore::Place { ns, venue: venue.clone() };
        match drain(env, &place, now_ms).await {
            Ok((0, 0, _)) => {}
            Ok((sent, kept, gone)) => {
                console_log!("outbox {venue}: {sent} sent, {kept} waiting");
                for what in gone {
                    crate::loud!(&place.ns, Some(&venue), "outbox.abandoned", "{what}");
                }
            }
            Err(e) => console_error!("outbox {venue}: drain refused: {e}"),
        }
    }
    let _ = ns;
}
