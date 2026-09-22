//! `POST /api/owner/place` — where this venue actually is.
//!
//! A COORDINATE IS REFUSED, NOT CLAMPED. A latitude of 91 is a bug in whatever
//! sent it, and a venue quietly moved to the pole is worse than a refusal
//! somebody has to read.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::owner::owner_and_venue;

/// `POST /api/owner/place`
///
/// WHERE THE VENUE IS, WHEN IT OPENS, AND WHAT ITS NEIGHBOURS SAY.
///
/// Three of these fields were readable and unwritable. `hours` has been read on
/// every menu request since the schedule landed -- `is_open_at` decides whether
/// the storefront says "open" -- and NOTHING HAS EVER WRITTEN IT, so every
/// venue fell through to the empty schedule that is open forever. `address` was
/// served and never set. Coordinates did not exist at all, which is why a
/// storefront could not draw a map of the place it sells from.
///
/// The BODY IS ALREADY NORMALISED: seven arrays of minute windows, Monday
/// first, exactly as `dowiz_hub::hours::from_json` reads them. The conversion
/// from "пʼятниця 11:00–23:00" happens in the harvester, where the language of
/// the source is known; a Worker that guessed at weekday names in an unknown
/// locale would open a venue on the wrong day.
///
/// THE GOOGLE BLOCK IS SOMEBODY ELSE'S MATERIAL and is stored as such: the
/// listing's URL travels with it so every surface that shows a rating can say
/// where it came from and link back to it.
pub async fn set_place(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    #[derive(Deserialize)]
    struct Win {
        open: i64,
        close: i64,
    }
    #[derive(Deserialize)]
    struct In {
        #[serde(default)]
        address: Option<String>,
        #[serde(default)]
        lat: Option<f64>,
        #[serde(default)]
        lng: Option<f64>,
        #[serde(default)]
        hours: Option<Vec<Vec<Win>>>,
        #[serde(default)]
        google: Option<Value>,
    }
    let body: In = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;

    // A COORDINATE IS REFUSED, NOT CLAMPED. A latitude of 91 is a bug in
    // whatever sent it, and clamping it to 90 puts the venue at the North Pole
    // with no error anybody will see.
    if let Some(v) = body.lat {
        if !(-90.0..=90.0).contains(&v) {
            return Response::error("a latitude is between -90 and 90", 400);
        }
    }
    if let Some(v) = body.lng {
        if !(-180.0..=180.0).contains(&v) {
            return Response::error("a longitude is between -180 and 180", 400);
        }
    }
    if body.lat.is_some() != body.lng.is_some() {
        return Response::error("a latitude without a longitude is not a place", 400);
    }

    let hours_json = match &body.hours {
        None => None,
        Some(days) => {
            if days.len() != 7 {
                return Response::error("a week has seven days", 400);
            }
            for d in days {
                for w in d {
                    if !(0..1440).contains(&w.open) || !(0..=1440).contains(&w.close) {
                        return Response::error("a window is minutes from 0 to 1440", 400);
                    }
                    if w.open == w.close {
                        return Response::error("a window of zero length is not a window", 400);
                    }
                }
            }
            Some(json!(days
                .iter()
                .map(|d| d.iter().map(|w| json!({"open": w.open, "close": w.close})).collect::<Vec<_>>())
                .collect::<Vec<_>>()))
        }
    };

    let (addr, lat, lng, goog) = (body.address.clone(), body.lat, body.lng, body.google.clone());
    crate::hubstore::with_catalog(&place, move |cat| {
        let raw = cat.location().ok_or_else(|| Error::RustError("no venue".into()))?;
        let mut l: Value = serde_json::from_str(&raw).unwrap_or(json!({}));
        // Each field is written only when it was sent. A harvest that could not
        // read the telephone number must not erase the one the owner typed.
        if let Some(a) = &addr {
            l["address"] = if a.trim().is_empty() { Value::Null } else { json!(a.trim()) };
        }
        if let (Some(la), Some(ln)) = (lat, lng) {
            l["lat"] = json!(la);
            l["lng"] = json!(ln);
        }
        if let Some(h) = &hours_json {
            l["hours"] = h.clone();
        }
        if let Some(g) = &goog {
            l["google"] = g.clone();
        }
        cat.set_location(&serde_json::to_string(&l).unwrap_or(raw));
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true }))
}
