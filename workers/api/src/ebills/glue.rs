//! WHAT THE VENUE'S OBJECT DECIDES about its `ebills` image, lifted out of
//! `hubdo/ebills.rs` so it is tested natively: the object reads the images,
//! calls one of these on copies in memory, and writes only on `Ok`. Every
//! refusal here returns BEFORE the table is touched -- its tests hold the
//! bytes before and after equal.

use super::cmd::{ConfigIn, ImportIn, MapIn, ReportIn};
use super::import::{Mapped, Outcome};
use super::map::FloorRow;
use super::state::{self, Config, Mapping, Noted, Plan, Seen, State, K_CONFIG, K_MAP, K_SEEN, K_STATE, ONE};
use crate::command::Refused;
use dowiz_hub::table::Table;
use serde_json::{json, Value};

fn io(e: String) -> Refused {
    Refused::Append(e)
}

/// `tick`: the plan, from the table and the venue's own record (hours, zone).
pub(crate) fn plan_for(t: &Table, loc: &Value, now_ms: i64) -> Result<Plan, Refused> {
    let cfg: Option<Config> = state::get(t, K_CONFIG, ONE).map_err(io)?;
    let st: State = state::get(t, K_STATE, ONE).map_err(io)?.unwrap_or_default();
    let sched = loc.get("hours").map(|h| dowiz_hub::hours::from_json(&h.to_string())).unwrap_or_default();
    let zone = crate::hubstore::zone_of(Some(loc));
    let (weekday, minute) = dowiz_hub::tz::local_weekday_minute(zone, now_ms);
    let open = sched.is_empty() || sched.is_open_at(weekday, minute);
    Ok(state::plan(cfg.as_ref(), &st, now_ms, open, dowiz_hub::tz::local_ms(zone, now_ms)))
}

/// `config`: saving clears a halt and the backoff; a new user, password or
/// point of sale forgets the session. `password: None` keeps the stored one.
pub(crate) fn apply_config(t: &mut Table, input: ConfigIn) -> Result<Value, Refused> {
    // DISCONNECT: switched off with no user is the owner forgetting the link --
    // the stored password and the session go, the crosswalk and the counters
    // stay. Until 2026-09-24 there was no way to remove a password once saved.
    if !input.enabled && input.user.trim().is_empty() {
        let mut st: State = state::get(t, K_STATE, ONE).map_err(io)?.unwrap_or_default();
        (st.halted, st.failures, st.last_error, st.session) = (false, 0, None, None);
        t.remove(K_CONFIG, ONE);
        state::put(t, K_STATE, ONE, &st).map_err(io)?;
        return Ok(json!({ "ok": true, "usable": false, "cleared": true }));
    }
    if input.pos_id < 1 || input.user.trim().is_empty() || input.user.len() > 200 {
        return Err(Refused::Invalid("a point of sale id (1 or more) and a user name".into()));
    }
    let old: Config = state::get(t, K_CONFIG, ONE).map_err(io)?.unwrap_or_default();
    let secret = input.password.filter(|p| !p.is_empty()).unwrap_or(old.secret.clone());
    let cfg = Config { enabled: input.enabled, pos_id: input.pos_id, user: input.user.trim().to_string(), secret };
    let mut st: State = state::get(t, K_STATE, ONE).map_err(io)?.unwrap_or_default();
    (st.halted, st.failures, st.last_error) = (false, 0, None);
    if cfg.user != old.user || cfg.secret != old.secret || cfg.pos_id != old.pos_id {
        st.session = None;
    }
    state::put(t, K_CONFIG, ONE, &cfg).map_err(io)?;
    state::put(t, K_STATE, ONE, &st).map_err(io)?;
    Ok(json!({ "ok": true, "usable": cfg.usable() }))
}

/// `map`: SET BY THE OWNER, to a product this catalogue holds, or cleared.
pub(crate) fn apply_map(t: &mut Table, input: MapIn, has_product: &dyn Fn(&str) -> bool) -> Result<Value, Refused> {
    let code = input.code.trim().to_string();
    if code.is_empty() || code.len() > 64 {
        return Err(Refused::Invalid("an ebills item code".into()));
    }
    match input.product_id.map(|p| p.trim().to_string()).filter(|p| !p.is_empty()) {
        Some(pid) if !has_product(&pid) => return Err(Refused::Invalid(format!("no product {pid} in this venue's catalogue"))),
        Some(pid) => state::put(t, K_MAP, &code, &Mapping { product_id: pid, at_ms: input.now_ms }).map_err(io)?,
        None => {
            t.remove(K_MAP, &code);
        }
    }
    Ok(json!({ "ok": true, "code": code }))
}

/// `report`: a failed firing, named and counted; `halt` for MFA and the like.
pub(crate) fn apply_report(t: &mut Table, input: ReportIn) -> Result<Value, Refused> {
    let mut st: State = state::get(t, K_STATE, ONE).map_err(io)?.unwrap_or_default();
    st.last_error = Some(Noted { at_ms: input.now_ms, sale_id: 0, why: input.what });
    st.failures = st.failures.saturating_add(1);
    st.halted |= input.halt;
    if input.drop_session {
        st.session = None;
    }
    state::put(t, K_STATE, ONE, &st).map_err(io)?;
    Ok(json!({ "failures": st.failures, "halted": st.halted }))
}

/// `import`, the part after the log and the shelf: the poller's state moves
/// with what the import did. Errors and the failure count are cleared only
/// by a COMPLETE firing (`report` owns them otherwise).
pub(crate) fn record_import(t: &mut Table, input: &ImportIn, out: &Outcome, short: Option<Vec<(String, i64)>>) -> Result<State, Refused> {
    let mut st: State = state::get(t, K_STATE, ONE).map_err(io)?.unwrap_or_default();
    st.watermark = st.watermark.max(input.watermark);
    st.backlog = input.backlog;
    if st.lead_below == 0 && input.lead_below > 0 {
        st.lead_below = input.lead_below;
    }
    if let Some(c) = input.recheck {
        (st.recheck_from, st.recheck_until) = c;
    }
    if input.listed {
        st.last_sales_ms = input.now_ms;
    }
    if input.reread {
        st.last_reread_ms = input.now_ms;
    }
    if input.complete {
        st.last_ok_ms = input.now_ms;
        (st.last_error, st.failures) = (None, 0);
    }
    (st.placed, st.noted, st.paid) = (st.placed + out.placed, st.noted + out.noted, st.paid + out.paid);
    // A SALE TAKEN NOW is no longer refused (`state::transient`'s retry), and
    // a refusal already on the list is not added again by the daily re-read.
    let taken = |id: i64| input.sales.iter().any(|m| matches!(m, Mapped::Order { sale_id, .. } | Mapped::Bill { sale_id, .. } if *sale_id == id));
    st.refused.retain(|n| !taken(n.sale_id));
    for n in input.refused.iter().chain(out.refused.iter()) {
        if !st.refused.iter().any(|o| o.sale_id == n.sale_id && o.why == n.why) {
            st.refused.push(n.clone());
        }
    }
    st.pending = out.pending.clone();
    st.leads = out.leads.clone();
    if let Some(s) = short {
        st.short = s;
    }
    if let Some(s) = input.session.clone() {
        st.session = Some(s);
    }
    st.trim();
    for (code, name, price) in out.seen.iter().chain(input.items.iter()) {
        state::put(t, K_SEEN, code, &Seen { name: name.clone(), price: *price, at_ms: input.now_ms }).map_err(io)?;
    }
    state::put(t, K_STATE, ONE, &st).map_err(io)?;
    Ok(st)
}

/// `floor`: the bytes to write, or `None` when no table changed -- the floor
/// is read every minute and rewritten only when the room moves.
pub(crate) fn floor_bytes(old: Option<&[u8]>, rows: &[FloorRow], now_ms: i64) -> Option<Vec<u8>> {
    let rows = json!(rows);
    let was = old.and_then(|b| serde_json::from_slice::<Value>(b).ok());
    if was.as_ref().and_then(|o| o.get("tables")) == Some(&rows) {
        return None;
    }
    Some(json!({ "at_ms": now_ms, "tables": rows }).to_string().into_bytes())
}

/// `floor`'s session side: a rotated cookie is kept, nothing else moves.
pub(crate) fn keep_session(t: &mut Table, s: super::client::Session) -> Result<(), Refused> {
    let mut st: State = state::get(t, K_STATE, ONE).map_err(io)?.unwrap_or_default();
    st.session = Some(s);
    state::put(t, K_STATE, ONE, &st).map_err(io)
}

#[cfg(test)]
mod tests;
