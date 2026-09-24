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
