//! The rules of the payroll tab, as functions of their arguments.
//!
//! WHAT IT IS. An owner enters, per employee and date, the hours worked (integer minutes, in
//! quarters of an hour); the pay is minutes x the hourly rate / 60 in integer minor units --
//! never a float. Rates and entries live in the venue's own `payroll` image.
//!
//! ONE ENTRY PER PERSON AND DATE, keyed by `date/staff`, so a double tap replaces rather than
//! double-pays, and an edit is the same write.
//!
//! THE RATE IS SNAPSHOT ON THE ENTRY when it is made and kept when its hours are edited: a raise
//! must not rewrite months already paid.
//!
//! ROUNDED ONCE PER EMPLOYEE-MONTH, half up, on the sum of rate-minutes: two quarter-hours at
//! 250 are 125, not 63 + 63. The grand total is the sum of the rows, so the table adds up.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::services::campaigns::segment::civil;

/// The venue's image: rates and entries, tenant-scoped by where the bytes live.
pub const IMAGE: &str = "payroll";
pub const K_ENTRY: &str = "entry";
pub const K_RATE: &str = "rate";
/// Sized like the venue's other table images; a year of 20 staff is about 1 MB.
pub const BYTES: usize = dowiz_hub::CEILING_BYTES;

pub const QUARTER_MINUTES: i64 = 15;
pub const MAX_MINUTES: i64 = 24 * 60;
/// A bound so a typo cannot reach the sums; far above any real hourly wage in any currency.
pub const MAX_RATE_MINOR: i64 = 100_000_000;
pub const MAX_NOTE: usize = 200;

const DAY_MS: i64 = 86_400_000;

/// One day of one person's work.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub staff_id: String,
    pub date: String,
    pub minutes: i64,
    pub rate_minor: i64,
    #[serde(default)]
    pub note: String,
}

/// One employee, as the staff entity lists them.
#[derive(Clone, Debug)]
pub struct Person {
    pub id: String,
    pub name: String,
    pub role: String,
    pub active: bool,
}

pub fn entry_id(date: &str, staff_id: &str) -> String {
    format!("{date}/{staff_id}")
}

/// The stored rate record.
pub fn rate_record(rate_minor: i64, now_ms: i64) -> String {
    json!({ "rate_minor": rate_minor, "updated_ms": now_ms }).to_string()
}

/// The rate inside a stored rate record.
pub fn rate_of(record: &str) -> Option<i64> {
    serde_json::from_str::<Value>(record).ok()?.get("rate_minor")?.as_i64()
}

fn digits(s: &str) -> Option<i64> {
    (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse().ok()).flatten()
}

/// `YYYY-MM`, a real month.
pub fn check_month(s: &str) -> Result<(), &'static str> {
    let bad = "month must be YYYY-MM";
    if s.len() != 7 || s.as_bytes()[4] != b'-' {
        return Err(bad);
    }
    let (y, m) = (digits(&s[..4]).ok_or(bad)?, digits(&s[5..]).ok_or(bad)?);
    (y >= 2000 && (1..=12).contains(&m)).then_some(()).ok_or(bad)
}

fn days_in(y: i64, m: i64) -> i64 {
    match m {
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// `YYYY-MM-DD`, a day the calendar has.
pub fn check_date(s: &str) -> Result<(), &'static str> {
    let bad = "date must be a real YYYY-MM-DD";
    if s.len() != 10 || s.as_bytes()[7] != b'-' || check_month(&s[..7]).is_err() {
        return Err(bad);
    }
    let (y, m) = (digits(&s[..4]).ok_or(bad)?, digits(&s[5..7]).ok_or(bad)?);
    let d = digits(&s[8..]).ok_or(bad)?;
    (1..=days_in(y, m)).contains(&d).then_some(()).ok_or(bad)
}

pub fn check_minutes(m: i64) -> Result<(), &'static str> {
    if m < QUARTER_MINUTES || m > MAX_MINUTES || m % QUARTER_MINUTES != 0 {
        return Err("hours must be quarters of an hour, at most 24");
    }
    Ok(())
}

pub fn check_rate(r: i64) -> Result<(), &'static str> {
    if r <= 0 || r > MAX_RATE_MINOR {
        return Err("hourly rate must be above zero and within bounds");
    }
    Ok(())
}

pub fn clean_note(s: Option<&str>) -> Result<String, &'static str> {
    let t = s.unwrap_or("").trim();
    if t.chars().count() > MAX_NOTE {
        return Err("note is too long");
    }
    Ok(t.to_string())
}

/// The civil date of a millisecond instant ALREADY SHIFTED to the venue's wall clock.
pub fn date_of(local_ms: i64) -> String {
    let (y, m, d) = civil(local_ms.div_euclid(DAY_MS));
    format!("{y:04}-{m:02}-{d:02}")
}

/// `YYYY-MM` of a valid `YYYY-MM-DD`.
pub fn month_of(date: &str) -> &str {
    &date[..7]
}

/// Rate-minutes to money: divided by 60 once, half up, in minor units. `None` is a total too
/// large to be money (the bounds on write make it unreachable; it is refused, not wrapped).
pub fn pay_minor(rate_minutes: i128) -> Option<i64> {
    i64::try_from(rate_minutes.checked_add(30)? / 60).ok()
}

/// Every stored entry, or which record is broken: a total that skips a corrupt row is a wrong
/// total that looks right.
pub fn parse_all(rows: Vec<(String, String)>) -> Result<Vec<Entry>, String> {
    rows.into_iter()
        .map(|(id, json)| serde_json::from_str::<Entry>(&json).map_err(|e| format!("payroll entry {id}: {e}")))
        .collect()
}

fn of_month<'a>(entries: &'a [Entry], month: &'a str) -> impl Iterator<Item = &'a Entry> + 'a {
    entries.iter().filter(move |e| month_of(&e.date) == month)
}

/// One employee's rate-minutes and minutes over the given entries.
fn sums<'a>(entries: impl Iterator<Item = &'a Entry>) -> BTreeMap<&'a str, (i64, i128)> {
    let mut by: BTreeMap<&str, (i64, i128)> = BTreeMap::new();
    for e in entries {
        let s = by.entry(e.staff_id.as_str()).or_default();
        s.0 += e.minutes;
        s.1 += i128::from(e.minutes) * i128::from(e.rate_minor);
    }
    by
}

/// THE MONTH'S PAY, in minor units: each employee rounded once, then added. The one rule the
/// table and the profit view both read.
pub fn month_total(entries: &[Entry], month: &str) -> Option<i64> {
    let mut total = 0_i64;
    for (_, (_, rm)) in sums(of_month(entries, month)) {
        total = total.checked_add(pay_minor(rm)?)?;
    }
    Some(total)
}

/// The month as the console draws it: rows = employees, columns = the entered dates.
pub fn month_view(month: &str, people: &[Person], rates: &BTreeMap<String, i64>, entries: &[Entry]) -> Option<Value> {
    let in_month: Vec<&Entry> = of_month(entries, month).collect();
    let totals = sums(in_month.iter().copied());
    let dates: Vec<&str> = {
        let mut d: Vec<&str> = in_month.iter().map(|e| e.date.as_str()).collect();
        d.sort_unstable();
        d.dedup();
        d
    };
    let mut named: Vec<&Person> = people.iter().collect();
    named.sort_by(|a, b| (a.name.as_str(), a.id.as_str()).cmp(&(b.name.as_str(), b.id.as_str())));
    // Everybody active, plus anyone with hours this month. A person whose staff record is gone
    // keeps their hours: money already earned does not vanish with a membership.
    let mut order: Vec<(&str, &str, &str, Option<bool>)> = Vec::new();
    for p in &named {
        if p.active || totals.contains_key(p.id.as_str()) {
            order.push((p.id.as_str(), p.name.as_str(), p.role.as_str(), Some(p.active)));
        }
    }
    for id in totals.keys() {
        if !named.iter().any(|p| p.id == *id) {
            order.push((*id, "", "", None));
        }
    }
    let mut rows = Vec::new();
    let (mut total_minutes, mut total_minor) = (0_i64, 0_i64);
    for (id, name, role, active) in order {
        let (minutes, rm) = totals.get(id).copied().unwrap_or((0, 0));
        let pay = pay_minor(rm)?;
        total_minutes += minutes;
        total_minor = total_minor.checked_add(pay)?;
        let mut days = serde_json::Map::new();
        for e in in_month.iter().filter(|e| e.staff_id == id) {
            days.insert(e.date.clone(), json!({ "minutes": e.minutes, "note": e.note, "rateMinor": e.rate_minor }));
        }
        rows.push(json!({ "staffId": id, "name": name, "role": role, "active": active, "rateMinor": rates.get(id),
            "minutes": minutes, "payMinor": pay, "days": days }));
    }
    let staff: Vec<Value> = named.iter().map(|p| json!({ "id": p.id, "name": p.name, "role": p.role, "active": p.active, "rateMinor": rates.get(&p.id) })).collect();
    Some(json!({ "month": month, "dates": dates, "rows": rows, "staff": staff, "totalMinutes": total_minutes, "totalMinor": total_minor }))
}

/// One employee's pay by month, newest first.
pub fn history(entries: &[Entry], staff_id: &str) -> Option<Value> {
    let mut by: BTreeMap<&str, (i64, i128)> = BTreeMap::new();
    for e in entries.iter().filter(|e| e.staff_id == staff_id) {
        let s = by.entry(month_of(&e.date)).or_default();
        s.0 += e.minutes;
        s.1 += i128::from(e.minutes) * i128::from(e.rate_minor);
    }
    let mut months = Vec::new();
    for (month, (minutes, rm)) in by.into_iter().rev() {
        months.push(json!({ "month": month, "minutes": minutes, "payMinor": pay_minor(rm)? }));
    }
    Some(json!({ "staffId": staff_id, "months": months }))
}

#[cfg(test)]
mod tests;
