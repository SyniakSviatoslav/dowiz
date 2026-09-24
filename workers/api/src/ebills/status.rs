//! WHAT THE OWNER IS SHOWN about the till link, built from the venue's own
//! images. Pure: a table, the catalogue's products and the floor in, JSON out.
//!
//! NEVER IN THIS ANSWER: the password (only whether one is set), the session
//! cookies (only whether one is live), a waiter's name (the floor keeps none).

use super::state::{self, Config, Mapping, Seen, State, K_CONFIG, K_MAP, K_SEEN, K_STATE, ONE};
use dowiz_hub::table::Table;
use serde_json::{json, Value};

/// `(id, name, price)` of every catalogue product, for names and suggestions.
pub(crate) fn products(cat: &[(String, String)]) -> Vec<(String, String, i64)> {
    cat.iter()
        .filter_map(|(id, j)| {
            let v: Value = serde_json::from_str(j).ok()?;
            let name = v.get("name").and_then(Value::as_str).unwrap_or("").to_string();
            Some((id.clone(), name, v.get("price").and_then(Value::as_i64).unwrap_or(-1)))
        })
        .collect()
}

/// The whole owner view: configuration, the poller's state, the crosswalk,
/// the codes still unmatched with their SUGGESTIONS, and the floor.
pub(crate) fn view(t: &Table, products: &[(String, String, i64)], floor: Value) -> Result<Value, String> {
    let cfg: Config = state::get(t, K_CONFIG, ONE)?.unwrap_or_default();
    let st: State = state::get(t, K_STATE, ONE)?.unwrap_or_default();
    let name_of = |pid: &str| products.iter().find(|p| p.0 == pid).map(|p| p.1.clone());
    let mut mapped = Vec::new();
    for (code, j) in t.all(K_MAP) {
        let m: Mapping = serde_json::from_str(&j).map_err(|e| format!("ebills map/{code}: {e}"))?;
        let seen: Option<Seen> = state::get(t, K_SEEN, &code)?;
        mapped.push(json!({
            "code": code, "product_id": m.product_id, "product": name_of(&m.product_id),
            "name": seen.as_ref().map(|s| s.name.clone()), "price": seen.map(|s| s.price),
        }));
    }
    let mut unmatched = Vec::new();
    for (code, j) in t.all(K_SEEN) {
        if t.has(K_MAP, &code) {
            continue;
        }
        let s: Seen = serde_json::from_str(&j).map_err(|e| format!("ebills seen/{code}: {e}"))?;
        let suggest: Vec<Value> = state::suggest(&s.name, s.price, products)
            .into_iter()
            .map(|(pid, agrees)| json!({ "product_id": pid, "product": name_of(&pid), "price_agrees": agrees }))
            .collect();
        unmatched.push(json!({ "code": code, "name": s.name, "price": s.price, "suggest": suggest }));
    }
    let pending: Vec<Value> = st
        .pending
        .iter()
        .map(|b| json!({ "sale_id": b.sale_id, "table": b.paid.get("table"), "total": b.paid.get("total"), "since_ms": b.since_ms }))
        .collect();
    let mut out = json!({
        "config": { "enabled": cfg.enabled, "pos_id": cfg.pos_id, "user": cfg.user, "secret_set": !cfg.secret.is_empty() },
        "state": {
            "watermark": st.watermark, "backlog": st.backlog, "last_sales_ms": st.last_sales_ms,
            "last_reread_ms": st.last_reread_ms, "last_ok_ms": st.last_ok_ms, "last_error": st.last_error,
            "failures": st.failures, "halted": st.halted, "placed": st.placed, "noted": st.noted,
            "paid": st.paid, "refused": st.refused, "pending": pending, "short": st.short,
            "session_live": st.session.as_ref().is_some_and(|s| s.live()),
        },
        "mapped": mapped, "unmatched": unmatched, "floor": floor,
        "products": products.iter().map(|(id, name, price)| json!({ "id": id, "name": name, "price": price })).collect::<Vec<_>>(),
    });
    out["health"] = health(&out);
    Ok(out)
}

/// The health lines: is it on, when did it last work, what went wrong, how
/// far it has read, and how many codes are waiting for the owner.
pub(crate) fn health(v: &Value) -> Value {
    let st = &v["state"];
    json!({
        "enabled": v["config"]["enabled"], "last_ok_ms": st["last_ok_ms"], "last_error": st["last_error"],
        "failures": st["failures"], "halted": st["halted"], "watermark": st["watermark"],
        "unmatched": v["unmatched"].as_array().map_or(0, Vec::len),
        "pending_bills": st["pending"].as_array().map_or(0, Vec::len),
        "refused": st["refused"].as_array().map_or(0, Vec::len),
        "short": st["short"],
    })
}
