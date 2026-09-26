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

use super::{digest, digest_rail, tgrail};

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

/// Put entries into the venue's outbox from a WORKER route (the object's own
/// turns write their image directly). One image write; an entry with the same
/// id replaces its older self, so a retried request is still one message.
pub async fn enqueue(place: &crate::hubstore::Place, entries: &[Entry]) -> Result<()> {
    let recs: Vec<(String, String)> = entries.iter().map(|e| (e.id.clone(), serde_json::to_string(e).unwrap_or_default())).collect();
    crate::hubstore::with_table(place, IMAGE_OUTBOX, OUTBOX_BYTES, move |t| {
        for (id, rec) in &recs {
            t.put(KIND, id, rec, &[], &[]).map_err(|e| Error::RustError(format!("outbox: {e:?}")))?;
        }
        Ok(())
    })
    .await
}

/// Attempt every entry that is due, and apply each verdict.
///
/// RETURNS WHAT HAPPENED rather than logging it: the caller is a cron, and a
/// cron whose outcome is only in a sampled trace is a cron nobody can tell has
/// stopped.
///
/// TELEGRAM (W-TG): routed events are fanned out to the venue's groups first
/// (`tgrail::route_all`), then every Telegram send is paced (15 per chat) and
/// its refusal classified -- a 429 waits without spending a try, a migrated
/// group is re-aimed, a chat the bot was removed from is dropped at once and
/// its groups marked left (`notify::tg`). Every change lands in ONE image write.
pub async fn drain(
    env: &Env,
    place: &crate::hubstore::Place,
    now_ms: i64,
) -> Result<(usize, usize, Vec<String>)> {
    let image = crate::hubstore::load_table(place, IMAGE_OUTBOX, OUTBOX_BYTES).await?.table;
    let entries: Vec<Entry> = image.all(KIND).into_iter().filter_map(|(_, j)| serde_json::from_str::<Entry>(&j).ok()).collect();
    // NOTHING DUE, NOTHING READ: a venue whose only entries are tomorrow's
    // summaries or backed-off retries costs this one read a minute, as before.
    if due(&entries, now_ms).is_empty() {
        return Ok((0, 0, Vec::new()));
    }
    let settings = crate::hubstore::load_settings(place).await?.settings;
    let token = crate::notify::bot_token(env, &settings);
    let wa = crate::channels::whatsapp_cfg(&settings);

    let mut ops = tgrail::Ops::default();
    let mut pace = tgrail::Pace::default();
    let (mut sent, mut kept) = (0usize, 0usize);
    let mut abandoned: Vec<String> = Vec::new();
    let mut verdicts: Vec<(String, Verdict)> = Vec::new();
    // THE GROUPS, and the venue's record for its zone, only when something
    // routed, a summary, or a stored group list is in play.
    let routing = settings.get(crate::notify::route::groups::KEY_GROUPS).is_some()
        || entries.iter().any(|e| e.kind == crate::notify::route::ROUTE_KIND || e.kind == digest::KIND);
    let record = if routing { crate::hubstore::venue_record(place).await.ok().flatten() } else { None };
    let zone = crate::hubstore::zone_of(record.as_ref());
    // `None` = not routing, or the groups are UNREADABLE: then routed events
    // and summaries WAIT (and the error is said), never dropped as "nobody".
    let groups: Option<Vec<crate::notify::route::Group>> = if routing {
        match crate::notify::route::groups_of(&settings, &crate::notify::hook::venue_lang(record.as_ref())) {
            Ok(g) => Some(g.list),
            Err(e) => {
                abandoned.push(format!("telegram groups: {e}"));
                None
            }
        }
    } else {
        None
    };
    let work = match &groups {
        Some(gs) => tgrail::route_all(entries.clone(), gs, zone, now_ms, &mut ops),
        None => entries.clone(),
    };
    let groups = groups.unwrap_or_default();
    let due_now = due(&work, now_ms);
    // CAMPAIGNS RE-ASK THE CONSENT FOLD HERE (G4): read once, only when one
    // is due; `None` = unreadable, and campaign entries wait.
    let acts = crate::services::campaigns::rail::acts_if_due(place, &due_now).await;
    let mut withdrawn: Vec<String> = Vec::new();
    let mut summaries = digest_rail::Folded::default();
    for e in due_now {
        let ok = match e.kind.as_str() {
            // THE PRINTER PULLS ITS OWN (`print_rail.rs`, LAST-MILE §3.1):
            // the cron never sends a ticket and never abandons one -- the
            // printer's DELETE is the delivery and its failures the retries.
            crate::print_rail::KIND => continue,
            // Still routed here only when the groups could not be read: it waits.
            crate::notify::route::ROUTE_KIND => continue,
            "telegram" => {
                // A RAIL THAT IS NOT CONFIGURED HAS NOT FAILED: a missing bot
                // token leaves the entry waiting, visible on the health pane.
                let Some(t) = token.as_deref() else { continue };
                if !pace.admit(&e.to) {
                    continue;
                }
                let r = crate::notify::tg::send(t, &e.to, &e.text).await;
                pace.note(image.get(crate::notify::route::HEALTH_KIND, &e.to), &e.to, &r, now_ms);
                let (s, k, gone) = tgrail::settle(e, &r, now_ms, &mut ops, &mut pace);
                sent += usize::from(s);
                kept += usize::from(k);
                abandoned.extend(gone);
                continue;
            }
            digest::KIND => {
                let (Some(t), false) = (token.as_deref(), groups.is_empty()) else { continue };
                let line = digest_rail::run(place, &image, e, &groups, zone, record.as_ref(), t, now_ms, &mut summaries, &mut ops).await;
                abandoned.extend(line);
                continue;
            }
            "whatsapp" => match &wa {
                Some(cfg) => crate::channels::whatsapp_text(cfg, &e.to, &e.text).await.is_ok(),
                None => continue,
            },
            crate::services::campaigns::send::OUTBOX_KIND => {
                let Some(acts) = acts.as_deref() else { continue };
                match (crate::services::campaigns::send::gate(e, acts), &wa) {
                    // WITHDRAWN SINCE IT WAS QUEUED: removed, never sent, and
                    // filed as `gone` so the report can say so.
                    (None, _) => {
                        withdrawn.push(e.id.clone());
                        verdicts.push((e.id.clone(), Verdict::Abandon { after: 0 }));
                        continue;
                    }
                    (Some(who), Some(cfg)) => crate::services::campaigns::rail::deliver(cfg, &who, e).await,
                    (Some(_), None) => continue,
                }
            }
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
            Verdict::Sent => {
                sent += 1;
                ops.remove(id);
            }
            Verdict::Retry { tries, next_at_ms } => {
                kept += 1;
                if let Some(e) = work.iter().find(|e| e.id == *id) {
                    ops.put(Entry { tries: *tries, next_at_ms: *next_at_ms, ..e.clone() });
                }
            }
            Verdict::Abandon { after } => {
                ops.remove(id);
                // An unknown channel already named itself above.
                if *after > 0 {
                    abandoned.push(format!("{id} after {after} attempts"))
                }
            }
        }
    }
    for (to, h) in &pace.health {
        ops.records.push((crate::notify::route::HEALTH_KIND, to.clone(), Some(serde_json::to_string(h).unwrap_or_default())));
    }
    for (chat, _) in &ops.gone.clone() {
        ops.put(tgrail::gone_notice(chat, &groups, now_ms));
    }
    let nothing = ops.puts.is_empty() && ops.removes.is_empty() && ops.records.is_empty();
    if nothing {
        return Ok((0, entries.len(), abandoned));
    }
    let (gone, moved) = (ops.gone.clone(), ops.moved.clone());
    let dropped = crate::hubstore::with_table(place, IMAGE_OUTBOX, OUTBOX_BYTES, move |t| ops.apply(t).map_err(Error::RustError)).await?;
    abandoned.extend(dropped.into_iter().map(|id| format!("{id}: its chat is gone")));
    if !gone.is_empty() || !moved.is_empty() {
        if let Err(e) = digest_rail::chats_changed(place, &gone, &moved, &crate::notify::hook::venue_lang(record.as_ref())).await {
            abandoned.push(format!("telegram groups not updated: {e}"));
        }
    }
    crate::services::campaigns::rail::record_gone(place, &verdicts, &withdrawn, now_ms).await;
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
            // NOT `(0, 0, _)`: an entry abandoned on its own is neither sent
            // nor kept, and that pattern swallowed its line -- the one thing
            // this sweep promises to say out loud.
            Ok((0, 0, gone)) if gone.is_empty() => {}
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
