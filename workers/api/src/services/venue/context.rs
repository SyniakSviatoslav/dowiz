//! THE MOMENT AT THE VENUE: the band of the day, the weekday and the weather (W-SENSE row 4).
//!
//!   GET /api/public/locations/:slug/context   `{contract, band, weekday, weather | null}`
//!
//! THE VENUE'S PLACE, NEVER THE GUEST'S. The weather is asked for the venue's own coordinates
//! (its record's `lat`/`lng`, rounded to two decimals, ~1 km); the route takes no coordinates and
//! no query at all, so a guest's location cannot reach it by construction (a test pins it).
//! THE CLOCK IS THE VENUE'S: band and weekday from its time zone (`hubstore::zone_of`).
//! IT NEVER FAILS THE PAGE: Open-Meteo unreachable, slow or malformed is `weather: null`, and the
//! storefront ranks without it. Open-Meteo (`/v1/forecast`, no key, CC BY 4.0) is asked at most
//! once an hour per venue per colo: the answer is edge-cached `WEATHER_TTL_S`; the context itself
//! `CONTEXT_TTL_S`, because the band moves within the hour.
//! A placement reads the weather from the cache only (`taste_routes::prepare`): it never waits on a provider.

use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

pub const CONTRACT: &str = "venue.context.v1";
/// The external API this pins (contract `external_api_version`).
pub const OPEN_METEO: &str = "https://api.open-meteo.com/v1/forecast";
pub const WEATHER_TTL_S: i64 = 3600;
pub const CONTEXT_TTL_S: i64 = 300;
pub const WEEKDAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
pub const BANDS: [&str; 5] = ["morning", "midday", "afternoon", "evening", "night"];
pub const WEATHERS: [&str; 4] = ["rain", "cold", "hot", "mild"];

/// PURE. The band of a local minute of the day: 05-11 morning, 11-14 midday, 14-17 afternoon,
/// 17-22 evening, the rest night.
pub fn band(minute: i64) -> &'static str {
    match minute.rem_euclid(1440) / 60 {
        5..=10 => "morning",
        11..=13 => "midday",
        14..=16 => "afternoon",
        17..=21 => "evening",
        _ => "night",
    }
}

/// The moment, as the profile files it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bucket {
    pub band: &'static str,
    /// 0 = Monday.
    pub weekday: usize,
    pub weather: Option<&'static str>,
}

impl Bucket {
    /// PURE. The moment at the venue for a UTC instant, its zone and the weather if known.
    pub fn at(zone: dowiz_hub::tz::Zone, now_ms: i64, weather: Option<&'static str>) -> Bucket {
        let (wd, minute) = dowiz_hub::tz::local_weekday_minute(zone, now_ms);
        Bucket { band: band(minute), weekday: wd.min(6), weather }
    }
    /// The context keys an order is filed under: its band, weekend or weekday, and its weather.
    pub fn keys(&self) -> Vec<String> {
        let day = if self.weekday >= 5 { "weekend" } else { "weekday" };
        let mut k = vec![format!("band:{}", self.band), format!("day:{day}")];
        if let Some(w) = self.weather {
            k.push(format!("wx:{w}"));
        }
        k
    }
    /// "thu:evening": where in the week the guest orders.
    pub fn when(&self) -> String {
        format!("{}:{}", WEEKDAYS[self.weekday.min(6)], self.band)
    }
    pub fn json(&self) -> Value {
        json!({ "band": self.band, "weekday": WEEKDAYS[self.weekday.min(6)], "keys": self.keys() })
    }
}

/// PURE. The provider's URL for the VENUE's coordinates, rounded to two decimals; `None` for a
/// venue with no place (or an impossible one).
pub fn weather_url(lat: f64, lon: f64) -> Option<String> {
    if !lat.is_finite() || !lon.is_finite() || lat.abs() > 90.0 || lon.abs() > 180.0 || (lat == 0.0 && lon == 0.0) {
        return None;
    }
    Some(format!("{OPEN_METEO}?latitude={lat:.2}&longitude={lon:.2}&current=temperature_2m,precipitation,weather_code"))
}

/// PURE. Open-Meteo's `current` block as one of four words: rain (any precipitation, or a code of
/// drizzle, rain, showers or thunder), cold (below 10 C, or snow), hot (26 C and over), mild.
/// A missing temperature is no weather.
pub fn weather_of(v: &Value) -> Option<&'static str> {
    let c = v.get("current")?;
    let t = c.get("temperature_2m")?.as_f64()?;
    let p = c.get("precipitation").and_then(Value::as_f64).unwrap_or(0.0);
    let code = c.get("weather_code").and_then(Value::as_i64).unwrap_or(0);
    let wet = p > 0.0 || matches!(code, 51..=67 | 80..=82 | 95..=99);
    let snow = matches!(code, 71..=77 | 85 | 86);
    Some(if wet && !snow { "rain" } else if snow || t < 10.0 { "cold" } else if t >= 26.0 { "hot" } else { "mild" })
}

/// The venue's coordinates from its record (`lat`/`lng`, or the micro-degree pair).
pub fn coords_of(record: &Value) -> Option<(f64, f64)> {
    let f = |a: &str, b: &str| record.get(a).and_then(Value::as_f64).or_else(|| record.get(b).and_then(Value::as_i64).map(|u| u as f64 / 1e6));
    Some((f("lat", "latUdeg")?, f("lng", "lonUdeg")?))
}

fn weather_key(url: &str) -> String {
    format!("https://weather.dowiz/{}", url.split('?').nth(1).unwrap_or(""))
}

/// The venue's weather word: from the edge cache, or (when `ask` and the cache is cold) from
/// Open-Meteo once, then cached. Every failure is `None`.
pub async fn weather(env: &Env, record: &Value, ask: bool) -> Option<&'static str> {
    let (lat, lon) = coords_of(record)?;
    let url = weather_url(lat, lon)?;
    let key = weather_key(&url);
    if let Ok(Some(mut hit)) = crate::edge::cache_get(env, &key).await {
        return hit.json::<Value>().await.ok().as_ref().and_then(weather_of);
    }
    if !ask {
        return None;
    }
    let mut r = crate::edge::fetch(Request::new(&url, Method::Get).ok()?).await.ok()?;
    if r.status_code() != 200 {
        return None;
    }
    let v: Value = r.json().await.ok()?;
    let word = weather_of(&v)?;
    if let Ok(mut res) = Response::from_json(&v) {
        let _ = res.headers_mut().set("cache-control", &format!("public, max-age={WEATHER_TTL_S}"));
        let _ = crate::edge::cache_put(env, &key, &res).await;
    }
    Some(word)
}

/// `GET /api/public/locations/:slug/context`. Takes NO query: a guest's place has no way in.
pub async fn context(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    if req.url().ok().and_then(|u| u.query().map(str::to_string)).is_some_and(|q| !q.is_empty()) {
        return Response::error("context takes no query: it is the venue's moment, never the guest's place", 400);
    }
    let key = format!("https://context.dowiz/{slug}");
    if let Some(hit) = crate::edge::cache_get(&ctx.env, &key).await? {
        return Ok(hit);
    }
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    let rec = crate::hubstore::venue_record(&place).await?.unwrap_or(Value::Null);
    let w = weather(&ctx.env, &rec, true).await;
    let b = Bucket::at(crate::hubstore::zone_of(Some(&rec)), ctx.data.now_ms, w);
    let mut out = b.json();
    out["contract"] = json!(CONTRACT);
    out["weather"] = w.map_or(Value::Null, |w| json!(w));
    out["source"] = json!({ "weather": "open-meteo.com", "at": "venue" });
    let mut res = Response::from_json(&out)?;
    res.headers_mut().set("cache-control", &format!("public, max-age={CONTEXT_TTL_S}"))?;
    if let Ok(copy) = res.cloned() {
        let _ = crate::edge::cache_put(&ctx.env, &key, &copy).await;
    }
    Ok(res)
}

#[cfg(test)]
#[path = "context/tests.rs"]
mod tests;
