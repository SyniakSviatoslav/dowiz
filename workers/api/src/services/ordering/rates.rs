//! `GET /api/rates` — what a price in lek is in somebody else's currency.
//!
//! IT NEVER FAILS THE PAGE. If the upstream is unreachable the answer is the
//! identity rate and `stale: true`: a storefront that cannot render a price
//! because a currency API is down is worse than one that shows lek.

use serde_json::{json, Value};
use worker::*;

/// `GET /api/public/rates` — what one unit of the venue's currency is worth.
///
/// DISPLAY ONLY, AND THE DISTINCTION IS THE WHOLE DESIGN. An order is priced,
/// totalled, charged and refunded in the VENUE'S currency; the kernel's money
/// law is exact integer arithmetic in one currency and a second one inside it
/// would be a rounding error with a customer's name on it. What a customer
/// switching to EUR gets is a READING of a price that is still in lek -- so the
/// figure that is authoritative and the figure that is convenient are never the
/// same number, and the surface says which is which.
///
/// INTEGER PARTS PER MILLION, not a float. The rate arrives as `0.0109` and is
/// stored and shipped as `10900`, so every consumer converts with
/// `amount * ppm / 10_000` into minor units and no surface has to agree with
/// another about rounding. MANIFESTO C2 keeps floats out of anything replayed;
/// this is not replayed, and it costs nothing to hold the line anyway.
///
/// CACHED FOR AN HOUR AT THE EDGE. A reference rate moves in fractions of a
/// percent over a day and a menu is read thousands of times, so fetching per
/// request would add a third-party round trip to the most-requested path in the
/// product for a number that had not changed.
///
/// IT NEVER FAILS THE PAGE. If the upstream is unreachable the answer is the
/// identity rate and `stale: true`, because a storefront that cannot render a
/// price because a currency API is down is worse than one that shows lek.
pub async fn rates(req: Request, _ctx: RouteContext<crate::Req>) -> Result<Response> {
    let base = req
        .url()
        .ok()
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "base").map(|(_, v)| v.to_string()))
        .unwrap_or_else(|| "ALL".to_string())
        .to_ascii_uppercase();
    // A currency code is three letters. Anything else is not one, and it would
    // otherwise be pasted into a third-party URL.
    if base.len() != 3 || !base.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Response::error("base must be a three-letter currency code", 400);
    }

    let key = format!("https://rates.dowiz/{base}");
    let cache = Cache::default();
    if let Some(hit) = cache.get(&key, false).await? {
        return Ok(hit);
    }

    let identity = |stale: bool| {
        json!({
            "base": base,
            "ppm": { base.clone(): 1_000_000 },
            "decimals": { "ALL": 0, "EUR": 2, "USD": 2 },
            "stale": stale,
        })
    };

    let fetched = Fetch::Url(
        format!("https://open.er-api.com/v6/latest/{base}")
            .parse()
            .map_err(|e| Error::RustError(format!("rate url: {e}")))?,
    )
    .send()
    .await;

    let body = match fetched {
        Ok(mut r) if r.status_code() == 200 => r.json::<Value>().await.ok(),
        _ => None,
    };

    let mut out = identity(true);
    if let Some(v) = body {
        if v.get("result").and_then(Value::as_str) == Some("success") {
            let mut ppm = serde_json::Map::new();
            // Only the three this product renders. A map of 160 currencies is
            // 160 numbers nothing reads, on the hottest path in the service.
            for code in ["ALL", "EUR", "USD"] {
                if let Some(r) = v.get("rates").and_then(|m| m.get(code)).and_then(Value::as_f64) {
                    // The ONE float in this path, and it ends here: the wire
                    // format is a decimal and the ledger is integers, so the
                    // conversion happens once, at the boundary, and is rounded
                    // rather than truncated.
                    ppm.insert(code.to_string(), json!((r * 1_000_000.0).round() as i64));
                }
            }
            if !ppm.is_empty() {
                out = json!({
                    "base": base,
                    "ppm": ppm,
                    "decimals": { "ALL": 0, "EUR": 2, "USD": 2 },
                    "asOf": v.get("time_last_update_utc").cloned().unwrap_or(Value::Null),
                    "stale": false,
                });
            }
        }
    }

    let mut res = Response::from_json(&out)?;
    res.headers_mut().set("cache-control", "public, max-age=3600")?;
    let to_cache = res.cloned()?;
    cache.put(&key, to_cache).await?;
    Ok(res)
}
