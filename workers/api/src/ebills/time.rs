//! An ISO-8601 UTC instant to Unix milliseconds, with no clock and no crate.
//!
//! WHY NOT A DATE CRATE. The Worker's decision path takes time as an argument
//! (`json_api::place_order_at(…, created_at_ms, …)`) and the hub is "no
//! clock" by contract; the one thing this border needs is to read the
//! platform's `"2026-09-21T09:01:25.857Z"` the same way on every node. Thirty
//! lines of arithmetic do that; a dependency would also do time zones,
//! locales and `now()`, none of which may enter an order record.

use super::MapError;

/// `YYYY-MM-DDTHH:MM:SS[.frac]Z` to Unix milliseconds. The fraction is
/// TRUNCATED to milliseconds, never rounded, so `…19.404634Z` is `…404` on
/// every node. Days from the civil date by Howard Hinnant's algorithm.
pub(super) fn epoch_ms(ts: &str) -> Result<i64, MapError> {
    let bad = || MapError::Timestamp(ts.to_string());
    let body = ts.strip_suffix('Z').ok_or_else(bad)?;
    let (date, time) = body.split_once('T').ok_or_else(bad)?;
    let (clock, frac) = time.split_once('.').unwrap_or((time, ""));
    let mut parts = date
        .split('-')
        .chain(clock.split(':'))
        .map(|p| p.parse::<i64>());
    let mut next = || parts.next().and_then(Result::ok).ok_or_else(bad);
    let (y, m, day, h, mi, s) = (next()?, next()?, next()?, next()?, next()?, next()?);
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&day) {
        return Err(bad());
    }
    if h > 23 || mi > 59 || s > 60 || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    let ms: i64 = format!("{frac}000")[..3].parse().map_err(|_| bad())?;
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Ok(((days * 86_400 + h * 3_600 + mi * 60 + s) * 1_000) + ms)
}

/// The civil date `YYYY-MM-DD` of a millisecond instant ALREADY SHIFTED to
/// the venue's wall clock (`dowiz_hub::tz::local_ms`). The inverse of the day
/// arithmetic above, Hinnant's `civil_from_days`; no clock, no crate.
pub(crate) fn day_of(local_ms: i64) -> String {
    let z = local_ms.div_euclid(86_400_000) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}
