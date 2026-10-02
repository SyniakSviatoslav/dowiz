//! ADDING MANY INGREDIENTS AT ONCE (W-NOM, 2026-09-28):
//! `POST /api/owner/supplies/bulk {items: [{name, unit?, category?, kind?,
//! code?, barcode?, packs?, ...}]}` -- the list a new person types on a phone,
//! one name per line, written in ONE catalogue write.
//!
//! EACH ITEM IS THE FORM'S BODY WITHOUT AN ID: the id is minted from the name
//! (the importer's slug, a numeric tail on a clash), then the item goes
//! through the same `check` and `record` as `POST /api/owner/supplies`, so a
//! list cannot write what the form could not. A name the venue already has
//! (any case) is NOT written again and is answered in `existing`: pasting the
//! same list twice adds nothing twice.
//!
//! WHO: the owner, or staff holding `catalog` OR `stock` -- the kitchen that
//! receives the goods is who names them (operator, 2026-09-28).

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use super::{check, record, SupplyIn};
use crate::auth::Cap;

/// Who may add to the nomenclature: the menu's word or the shelf's.
pub(crate) const ADD: [Cap; 2] = [Cap::Catalog, Cap::Stock];
pub const ITEMS_MAX: usize = 200;
const NAME_MAX: usize = 80;
const TAILS: usize = 99;

/// What one bulk add writes: `(id, record)` per new item, and the names skipped.
#[derive(Debug, Default, PartialEq)]
pub struct Added {
    pub write: Vec<(String, Value)>,
    pub existing: Vec<String>,
}

/// The plan for `items` against the supplies already stored. PURE.
pub fn plan(items: &[Value], stored: &[(String, String)]) -> std::result::Result<Added, String> {
    if items.is_empty() || items.len() > ITEMS_MAX {
        return Err(format!("1 to {ITEMS_MAX} ingredients at once"));
    }
    let name_of = |j: &str| serde_json::from_str::<Value>(j).ok().and_then(|v| v.get("name").and_then(Value::as_str).map(str::to_lowercase));
    let mut names: Vec<String> = stored.iter().filter_map(|(_, j)| name_of(j)).collect();
    let mut ids: Vec<String> = stored.iter().map(|(id, _)| id.clone()).collect();
    let mut out = Added::default();
    for (i, it) in items.iter().enumerate() {
        let name = it.get("name").and_then(Value::as_str).map(str::trim).unwrap_or_default().to_string();
        if name.is_empty() || name.chars().count() > NAME_MAX {
            return Err(format!("line {}: a name of 1 to {NAME_MAX} characters", i + 1));
        }
        if names.contains(&name.to_lowercase()) {
            out.existing.push(name);
            continue;
        }
        let base: String = dowiz_hub::import::slug(&name).chars().take(60).collect();
        let Some(id) = std::iter::once(base.clone()).chain((2..=TAILS).map(|n| format!("{base}-{n}"))).find(|c| !ids.contains(c)) else {
            return Err(format!("line {}: no id is free for {name}", i + 1));
        };
        let mut v = it.clone();
        let Some(m) = v.as_object_mut() else { return Err(format!("line {}: not an ingredient", i + 1)) };
        m.insert("id".into(), json!(id));
        m.insert("name".into(), json!(name));
        let body: SupplyIn = serde_json::from_value(v).map_err(|e| format!("line {}: {e}", i + 1))?;
        check(&body).map_err(|e| format!("{name}: {e}"))?;
        out.write.push((id.clone(), record(&id, &body, &json!({}))));
        names.push(name.to_lowercase());
        ids.push(id);
    }
    Ok(out)
}

/// `POST /api/owner/supplies/bulk` -- many new ingredients, one write.
pub async fn add_supplies(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct In {
        items: Vec<Value>,
        #[serde(rename = "location_id")]
        _location_id: Option<String>,
    }
    // AUTHORITY BEFORE THE BODY (W-FIX O9).
    let loc = match crate::services::identity::staff::guard::staff_venue(&req, &ctx, &ADD).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let body: In = match crate::body::parse(&mut req).await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let out = crate::hubstore::with_catalog(&place, move |cat| {
        let added = plan(&body.items, &cat.supplies()).map_err(|e| Error::RustError(format!("refused: {e}")))?;
        for (id, rec) in &added.write {
            cat.set_supply(id, &rec.to_string());
        }
        Ok(added)
    })
    .await;
    match out {
        Ok(a) => Response::from_json(&json!({
            "added": a.write.iter().map(|(id, r)| json!({ "id": id, "name": r["name"] })).collect::<Vec<_>>(),
            "existing": a.existing,
        })),
        Err(e) => match e.to_string().split_once("refused: ") {
            Some((_, why)) => Response::error(why.to_string(), 400),
            None => Err(e),
        },
    }
}

#[cfg(test)]
mod tests;
