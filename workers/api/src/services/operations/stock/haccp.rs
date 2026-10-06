//! THE HACCP EXPORT (P13, W-STORE): three CSV files an inspector asks for,
//! over a date range of the VENUE'S local days, owner only.
//!
//!   GET /api/owner/stock/haccp?kind=lots|orders|freezing&from=yyyy-mm-dd&to=yyyy-mm-dd
//!
//!   lots      lot -> orders: which orders ate each lot (recall a lot);
//!   orders    order -> lots: which lots one order ate (a guest fell ill);
//!   freezing  every in-house freezing record (with the 853/2004 rule it
//!             meets, or "none") and every supplier-treated receipt. W-STORE2
//!             APPENDS two columns (old positions unchanged): a `started` column (venue local `yyyy-mm-dd hh:mm`, empty when
//!             the start was not recorded) and, beside the staff id `by`, the
//!             staff member's NAME (`by_name`) -- resolved at export time from
//!             the venue's members; a member since removed shows the id and an
//!             empty name, never a failed export. Then `ended` (the recorded
//!             end, operator 2026-10-05), empty when not recorded.
//!
//! Derived in the venue's object (`/fold/haccp`, `hubdo/reads.rs`) from ONE
//! replay of the stock log (`dowiz_hub::stock::StockLog::trace`); the CSV
//! alone crosses the hop. Quantities are integers in the supply's base unit.
//! A row recorded before records were dated has an empty date and is ALWAYS
//! included: a trace that silently dropped it would hide exactly the lot an
//! inspector asked about.

use dowiz_hub::stock::haccp::Trace;
use dowiz_hub::stock::meta::{day_of_local_ms, parse_day, show_day};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

/// The three files.
pub const KINDS: [&str; 3] = ["lots", "orders", "freezing"];
/// The longest range one export covers, in days.
pub const RANGE_MAX_DAYS: i64 = 400;

/// One CSV cell: quoted when it must be, and a text that a spreadsheet would
/// run as a formula (`=`, `+`, `-`, `@` first) defused with a `'`. Numbers
/// are written by [`num`], never through here.
pub fn cell(s: &str) -> String {
    let s = if s.starts_with(['=', '+', '-', '@']) { format!("'{s}") } else { s.to_string() };
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

fn num(n: i64) -> String {
    n.to_string()
}

/// The asked range as local days `(from, to)`, inclusive, or the 400's words.
pub fn range(from: Option<&str>, to: Option<&str>) -> std::result::Result<(i64, i64), String> {
    let day = |s: Option<&str>, w: &str| s.and_then(parse_day).ok_or(format!("{w}: a day as yyyy-mm-dd"));
    let (f, t) = (day(from, "from")?, day(to, "to")?);
    let span = dowiz_hub::stock::meta::day_number(t) - dowiz_hub::stock::meta::day_number(f);
    if !(0..RANGE_MAX_DAYS).contains(&span) {
        return Err(format!("from is on or before to, at most {RANGE_MAX_DAYS} days apart"));
    }
    Ok((f, t))
}

/// How the export names things: a supply id, a staff id, and an instant in
/// the venue's local time (`yyyy-mm-dd hh:mm`).
pub struct Names<'a> {
    pub supply: &'a dyn Fn(&str) -> String,
    pub staff: &'a dyn Fn(&str) -> String,
    pub local_minute: &'a dyn Fn(i64) -> String,
}

/// The file `kind` over `[from, to]`. `local_day` turns a record's ms into
/// the venue's `yyyymmdd`; `names` the ids into words. PURE.
pub fn csv(kind: &str, t: &Trace, (from, to): (i64, i64), local_day: &dyn Fn(i64) -> i64, names: &Names) -> std::result::Result<String, String> {
    let name = names.supply;
    let date = |at: Option<i64>| at.map(|ms| local_day(ms));
    let within = |d: Option<i64>| d.is_none_or(|d| (from..=to).contains(&d));
    let shown = |d: Option<i64>| d.map(show_day).unwrap_or_default();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let head: &[&str] = match kind {
        "lots" | "orders" => {
            for d in &t.draws {
                let day = date(d.at);
                if !within(day) {
                    continue;
                }
                let (item, lot, order, qty) = (cell(&d.item), cell(&d.lot), cell(&d.order), num(d.qty));
                rows.push(if kind == "lots" {
                    vec![item, cell(&name(&d.item)), lot, shown(day), order, qty]
                } else {
                    vec![order, shown(day), item, cell(&name(&d.item)), lot, qty]
                });
            }
            rows.sort();
            if kind == "lots" {
                &["item", "name", "lot", "date", "order", "qty"]
            } else {
                &["order", "date", "item", "name", "lot", "qty"]
            }
        }
        "freezing" => {
            for f in &t.freezing {
                let day = date(f.at);
                if !within(day) {
                    continue;
                }
                rows.push(vec![
                    shown(day),
                    cell(&f.item),
                    cell(&name(&f.item)),
                    cell(&f.lot),
                    f.how.to_string(),
                    f.hours.map(num).unwrap_or_default(),
                    f.temp_c.map(num).unwrap_or_default(),
                    f.rule.unwrap_or(if f.how == "in_house" { "none" } else { "" }).to_string(),
                    cell(f.doc.as_deref().unwrap_or("")),
                    cell(&f.store),
                    cell(&f.by),
                    cell(&(names.staff)(&f.by)),
                    f.started.map(|ms| (names.local_minute)(ms)).unwrap_or_default(),
                    f.ended.map(|ms| (names.local_minute)(ms)).unwrap_or_default(),
                ]);
            }
            &["date", "item", "name", "lot", "how", "hours", "temp_c", "rule", "doc", "storage", "by", "by_name", "started", "ended"]
        }
        other => return Err(format!("{other:?}: the export is one of lots, orders, freezing")),
    };
    let mut out = head.join(",");
    out.push('\n');
    for r in rows {
        out.push_str(&r.join(","));
        out.push('\n');
    }
    Ok(out)
}

/// `{id: name}` as the Worker sends it (`names=`); anything unreadable is no
/// names at all -- the ids still print. PURE.
pub fn staff_map(raw: Option<&str>) -> std::collections::BTreeMap<String, String> {
    raw.and_then(|r| serde_json::from_str(r).ok()).unwrap_or_default()
}

/// What the venue's object answers for `/fold/haccp`. PURE over its images.
/// `staff`: the Worker's `names=` (W-STORE2).
pub fn fold(cat: &dowiz_hub::catalog::Catalog, log: &dowiz_hub::stock::StockLog, kind: &str, from: Option<&str>, to: Option<&str>, staff: Option<&str>) -> std::result::Result<String, (u16, String)> {
    let span = range(from, to).map_err(|e| (400, e))?;
    if !KINDS.contains(&kind) {
        return Err((400, format!("{kind:?}: the export is one of lots, orders, freezing")));
    }
    let t = log.trace().map_err(|e| (500, e.to_string()))?;
    let zone = crate::hubstore::zone_of(cat.location().and_then(|j| serde_json::from_str::<serde_json::Value>(&j).ok()).as_ref());
    let local_day = move |ms: i64| day_of_local_ms(dowiz_hub::tz::local_ms(zone, ms));
    let name = |id: &str| {
        cat.supply(id)
            .and_then(|j| serde_json::from_str::<serde_json::Value>(&j).ok())
            .and_then(|v| v.get("name").and_then(serde_json::Value::as_str).map(str::to_string))
            .unwrap_or_default()
    };
    let people = staff_map(staff);
    let staff = |id: &str| people.get(id).cloned().unwrap_or_default();
    let local_minute = move |ms: i64| {
        let l = dowiz_hub::tz::local_ms(zone, ms);
        let m = l.rem_euclid(86_400_000) / 60_000;
        format!("{} {:02}:{:02}", show_day(day_of_local_ms(l)), m / 60, m % 60)
    };
    let names = Names { supply: &name, staff: &staff, local_minute: &local_minute };
    csv(kind, &t, span, &local_day, &names).map_err(|e| (400, e))
}

/// `GET /api/owner/stock/haccp` -- the OWNER's (it names who froze what and
/// every order's lots); a member of staff is refused.
pub async fn export(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let url = req.url()?;
    let q = |k: &str| url.query_pairs().find(|(n, _)| n == k).map(|(_, v)| v.to_string()).unwrap_or_default();
    let (kind, from, to) = (q("kind"), q("from"), q("to"));
    // THE NAMES, at export time (W-STORE2): every member of this venue, so a
    // record's signer prints as id AND name. A failed lookup is no names --
    // the ids still print; the export never fails for a name.
    let work = async {
        let names = staff_names(&ctx.env, &place.venue).await.unwrap_or_default();
        let ask = format!(
            "https://hub/fold/haccp?kind={}&from={}&to={}&names={}",
            crate::mcp::enc(&kind),
            crate::mcp::enc(&from),
            crate::mcp::enc(&to),
            crate::mcp::enc(&serde_json::to_string(&names).unwrap_or_default())
        );
        crate::fold::ask::text(&place, &ask).await
    };
    let (_, _loc, (status, text)) = match crate::owner::owner_beside(&req, &ctx, &place, work).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    if status != 200 {
        return Response::error(text, status);
    }
    let mut out = Response::ok(text)?;
    out.headers_mut().set("content-type", "text/csv; charset=utf-8")?;
    out.headers_mut().set("content-disposition", &format!("attachment; filename=\"haccp-{kind}-{from}-{to}.csv\""))?;
    Ok(out)
}

/// `{member id: display name}` of every member of `venue` (owner and staff)
/// with a name; a user who is not a member of THIS venue is never named.
async fn staff_names(env: &Env, venue: &str) -> Result<std::collections::BTreeMap<String, String>> {
    let t = crate::identity_store::identity(env).await?;
    let mut out = std::collections::BTreeMap::new();
    for (_, uid) in t.scan(&format!("member.venue/{venue}/")) {
        let name = crate::identity_store::rec(&t, crate::identity_store::K_USER, &uid)
            .map(|u| crate::identity_store::s_of(&u, "display_name"))
            .unwrap_or_default();
        if !name.trim().is_empty() {
            out.insert(uid, name);
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "haccp/tests.rs"]
mod tests;
