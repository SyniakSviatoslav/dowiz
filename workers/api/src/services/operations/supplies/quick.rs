//! ADDING MANY INGREDIENTS AT ONCE (W-NOM, 2026-09-28):
//! `POST /api/owner/supplies/bulk {items: [{name, unit?, category?, kind?,
//! code?, barcode?, packs?, ...}]}` -- the list a new person types on a phone,
//! one name per line, written in ONE catalogue write.
//!
//! EACH ITEM IS THE FORM'S BODY WITHOUT AN ID: the id is minted from the name
//! (the importer's slug, a numeric tail on a clash), then the item goes
//! through the same `check` and `record` as `POST /api/owner/supplies`, so a
//! list cannot write what the form could not. A name the venue already has
//! is NOT written again and is answered in `existing`, and in `matched` with
//! the record it is (`{typed, id, name, unit, retired}`): pasting the same list
//! twice adds nothing twice, and a form that typed a name gets the id to
//! receive onto.
//!
//! "ALREADY HAS" IS DECIDED BY [`fold`], the same rule the console's resolver
//! applies (`public/admin/supply-resolve.js`): case, accents and the Ukrainian
//! and Russian spellings of one word are one name, so two people typing on two
//! phones land on one position. Only the exact fold: "the same word with
//! another ending" is a question the form asks a person, never decided here.
//! A category goes the same way: one whose fold the venue already has is
//! stored under the existing spelling.
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

/// A typed name that is a position the venue already has.
#[derive(Debug, Clone, PartialEq)]
pub struct Match {
    pub typed: String,
    pub id: String,
    pub name: String,
    pub unit: String,
    /// The record is retired: it is off the stock list until restored.
    pub retired: bool,
}

/// What one bulk add writes: `(id, record)` per new item, the names skipped,
/// and which record each skipped name is.
#[derive(Debug, Default, PartialEq)]
pub struct Added {
    pub write: Vec<(String, Value)>,
    pub existing: Vec<String>,
    pub matched: Vec<Match>,
}

/// THE FORM NAMES ARE COMPARED IN: lower case, accents off, the Ukrainian and
/// Russian spellings of one word together, soft and hard signs and apostrophes
/// dropped, letters and digits split, everything else one space. The same
/// literal cases pin this and the console's copy (`supply-resolve-cases.json`).
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev = 0u8;
    for ch in s.to_lowercase().chars() {
        if ('\u{300}'..='\u{36f}').contains(&ch) || matches!(ch, 'ь' | 'ъ' | '\'' | 'ʼ' | '’' | '‘' | '`' | '´') {
            continue;
        }
        let c = match ch {
            'ё' | 'є' | 'э' => 'е',
            'і' | 'ї' | 'ы' | 'й' => 'и',
            'ґ' => 'г',
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
            'ç' | 'ć' | 'č' => 'c',
            'ď' | 'đ' => 'd',
            'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ė' | 'ę' | 'ě' => 'e',
            'ğ' => 'g',
            'ì' | 'í' | 'î' | 'ï' | 'ī' | 'ı' => 'i',
            'ł' => 'l',
            'ñ' | 'ń' | 'ň' => 'n',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' => 'o',
            'ř' => 'r',
            'ś' | 'ş' | 'š' => 's',
            'ţ' | 'ť' => 't',
            'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => 'u',
            'ý' | 'ÿ' => 'y',
            'ź' | 'ż' | 'ž' => 'z',
            c => c,
        };
        let kind = if c.is_ascii_digit() { 2 } else if c.is_alphabetic() { 1 } else { 0 };
        if kind != 0 {
            if !out.is_empty() && prev != kind {
                out.push(' ');
            }
            out.push(c);
        }
        prev = kind;
    }
    out
}

/// What a name is compared by: its fold, or (a name of signs only) itself.
fn key(name: &str) -> String {
    let f = fold(name);
    if f.is_empty() { name.to_lowercase() } else { f }
}

/// The plan for `items` against the supplies already stored. PURE.
pub fn plan(items: &[Value], stored: &[(String, String)]) -> std::result::Result<Added, String> {
    if items.is_empty() || items.len() > ITEMS_MAX {
        return Err(format!("1 to {ITEMS_MAX} ingredients at once"));
    }
    let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).map(str::trim).unwrap_or_default().to_string();
    let mut known: Vec<(String, Match)> = Vec::with_capacity(stored.len());
    let mut cats: Vec<String> = Vec::new();
    for (id, j) in stored {
        let Ok(v) = serde_json::from_str::<Value>(j) else { continue };
        let (name, cat) = (text(&v, "name"), text(&v, "category"));
        if !cat.is_empty() && !cats.iter().any(|c| fold(c) == fold(&cat)) {
            cats.push(cat);
        }
        if !name.is_empty() {
            let unit = v.get("unit").and_then(Value::as_str).unwrap_or("g").to_string();
            let retired = v.get("active").and_then(Value::as_bool) == Some(false);
            known.push((key(&name), Match { typed: String::new(), id: id.clone(), name, unit, retired }));
        }
    }
    let mut ids: Vec<String> = stored.iter().map(|(id, _)| id.clone()).collect();
    let mut out = Added::default();
    for (i, it) in items.iter().enumerate() {
        let name = it.get("name").and_then(Value::as_str).map(str::trim).unwrap_or_default().to_string();
        if name.is_empty() || name.chars().count() > NAME_MAX {
            return Err(format!("line {}: a name of 1 to {NAME_MAX} characters", i + 1));
        }
        let k = key(&name);
        if let Some((_, m)) = known.iter().find(|(f, _)| *f == k) {
            out.matched.push(Match { typed: name.clone(), ..m.clone() });
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
        // ONE GROUP, ONE SPELLING: the venue's own, or the first one typed.
        if let Some(typed) = m.get("category").and_then(Value::as_str).map(str::trim).filter(|c| !c.is_empty()).map(str::to_string) {
            match cats.iter().find(|c| fold(c) == fold(&typed)) {
                Some(have) => {
                    m.insert("category".into(), json!(have));
                }
                None => cats.push(typed),
            }
        }
        let body: SupplyIn = serde_json::from_value(v).map_err(|e| format!("line {}: {e}", i + 1))?;
        check(&body).map_err(|e| format!("{name}: {e}"))?;
        let rec = record(&id, &body, &json!({}));
        let unit = rec["unit"].as_str().unwrap_or("g").to_string();
        known.push((k, Match { typed: String::new(), id: id.clone(), name, unit, retired: false }));
        out.write.push((id.clone(), rec));
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
            "added": a.write.iter().map(|(id, r)| json!({ "id": id, "name": r["name"], "unit": r["unit"], "kind": r["kind"], "category": r["category"] })).collect::<Vec<_>>(),
            "existing": a.existing,
            "matched": a.matched.iter().map(|m| json!({ "typed": m.typed, "id": m.id, "name": m.name, "unit": m.unit, "retired": m.retired })).collect::<Vec<_>>(),
        })),
        Err(e) => match e.to_string().split_once("refused: ") {
            Some((_, why)) => Response::error(why.to_string(), 400),
            None => Err(e),
        },
    }
}

#[cfg(test)]
mod tests;
