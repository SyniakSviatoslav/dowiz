//! The one read `health.fiscal` needs: the venue's `fiscal` image, through
//! `platform_store::load_at` -- a READ, which never writes an image back and
//! never creates one for a venue that has not fiscalised. The rules and the
//! answer's shape are `wire::health_json`, pure and tested there.

use serde_json::{json, Value};

use super::queue::KIND;
use super::wire::{config, health_json, Config, CEILING, IMAGE, SETTING};
use crate::outbox::Entry;

/// `health.fiscal` for `/api/owner/health`. `settings` is `None` when the
/// settings image could not be read, and that is said, never read as "off".
pub async fn health(place: &crate::hubstore::Place, settings: Option<&dowiz_hub::settings::Settings>, now_ms: i64) -> Value {
    let Some(s) = settings else {
        return json!({ "configured": null, "error": format!("the settings image is unreadable, so {SETTING} is unknown") });
    };
    let cfg = config(&s.known(SETTING));
    if !matches!(cfg, Config::From(_)) {
        return health_json(&cfg, Ok(&[]), now_ms);
    }
    let mut v = queued(place, &cfg, now_ms).await;
    if v.get("sender").is_some() && super::ebills_arm::arming(&|k| s.get(k)).armed {
        v["sender"] = serde_json::json!("ebills");
    }
    v
}

async fn queued(place: &crate::hubstore::Place, cfg: &Config, now_ms: i64) -> Value {
    let cfg = cfg.clone();
    let read = match place.stub() {
        Ok(stub) => crate::platform_store::load_at(&stub, IMAGE, CEILING).await,
        Err(e) => Err(e),
    };
    match read {
        Ok(loaded) => {
            let mut es: Vec<Entry> = Vec::new();
            for (id, j) in loaded.table.all(KIND) {
                match serde_json::from_str::<Entry>(&j) {
                    Ok(e) => es.push(e),
                    // AN UNREADABLE ENTRY IS NOT A MISSING ONE: its order would
                    // read as "no document" and the cause would be lost.
                    Err(e) => return health_json(&cfg, Err(format!("fiscal entry {id} is unreadable: {e}")), now_ms),
                }
            }
            health_json(&cfg, Ok(&es), now_ms)
        }
        Err(e) => health_json(&cfg, Err(format!("the fiscal image is unreadable: {e}")), now_ms),
    }
}

/// THE MINUTE CRON'S eBills FIRING (card L70), after the till link's poll so
/// the two never share a minute's session. Per venue: the object's gate and
/// claim (`fiscal_plan` -- nothing unless ARMED), the firing over the
/// allow-listed client (`ebills_fire::fire`, ≤ `ebills_sender::BATCH`
/// documents), and the answer back in one command. Every failure is LOUD,
/// in the venue's error log; none is read as "sent".
pub async fn sweep(env: &worker::Env, now_ms: i64) {
    if !super::SEND_ENABLED {
        return;
    }
    use super::ebills_cmd::{AnswerIn, PlanIn, PlanOut};
    use super::ebills_sender::Outcome;
    let registry = match crate::identity_store::registry(env).await {
        Ok(t) => t,
        Err(e) => return worker::console_error!("fiscal sweep: registry unreadable: {e}"),
    };
    for (venue, _) in registry.all(crate::identity_store::K_LOC) {
        let Ok(ns) = env.durable_object("HUB") else { return worker::console_error!("fiscal sweep: no HUB binding") };
        let place = crate::hubstore::Place { ns, venue: venue.clone() };
        let plan: PlanOut = match crate::command::send(&place, "ebills/fiscal_plan", &PlanIn { now_ms }).await {
            Ok(p) => p,
            Err((s, m)) => {
                crate::loud!(&place.ns, Some(&venue), "fiscal.plan", "{s}: {m}");
                continue;
            }
        };
        if !plan.ready || plan.batch.items.is_empty() {
            continue;
        }
        let mut c = crate::ebills::fetch::Client::new(&plan.user, &plan.secret, plan.session.clone());
        let outcomes = super::ebills_fire::fire(&mut c, &plan.batch).await;
        if let Some(why) = outcomes.iter().find_map(|(_, o)| if let Outcome::NotSent(w) = o { Some(w.clone()) } else { None }) {
            crate::loud!(&place.ns, Some(&venue), "fiscal.ebills", "not sent this firing: {why}");
        }
        let answer = AnswerIn { now_ms, outcomes, session: c.changed.then(|| c.session.clone()) };
        match crate::command::send::<_, serde_json::Value>(&place, "ebills/fiscal_answer", &answer).await {
            Ok(v) => {
                for x in v["exceptions"].as_array().into_iter().flatten() {
                    crate::loud!(&place.ns, Some(&venue), "fiscal.ebills", "order {}: {}", x["order_id"].as_str().unwrap_or("?"), x["reason"].as_str().unwrap_or(""));
                }
                if v["sent"].as_u64().unwrap_or(0) > 0 {
                    worker::console_log!("fiscal {venue}: {} registered at ebills", v["sent"]);
                }
            }
            // THE SENDS HAPPENED AND THEIR ANSWERS WERE NOT RECORDED: the
            // intents say `Sending`, so the next firing reconciles before any POST.
            Err((s, m)) => crate::loud!(&place.ns, Some(&venue), "fiscal.answer", "{s}: {m} (the next firing reconciles)"),
        }
    }
}
