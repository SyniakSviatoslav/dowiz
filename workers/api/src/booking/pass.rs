//! Entry passes and the venue's pass key. The pass is `dowiz_kernel::pass`'s;
//! this file keeps the key and carries the bytes.

use serde::Deserialize;
use serde_json::json;
use worker::wasm_bindgen::JsCast;
use worker::*;

use dowiz_kernel::pass::{self, PassClaims, PassWindow};
use dowiz_kernel::reservation::ReservationStatus;

use super::{events_of, fold_status, id64, load_bookings, now_min, reservation_of};

/// Fetch the venue's pass key, minting one on first use.
///
/// The key never leaves this Worker: a pass goes out, the key does not.
async fn pass_key(place: &crate::hubstore::Place) -> Result<Vec<u8>> {
    // The venue's settings image, under a key whose LAST SEGMENT IS `key` --
    // which is what `settings::is_secret` matches on, so it is redacted
    // everywhere settings are read back for display. That rule is by shape
    // rather than by a list precisely so a secret added later cannot be
    // forgotten, and this is the first one to rely on it.
    const KEY: &str = "venue.pass.key";
    let loaded = crate::hubstore::load_settings(place).await?;
    if let Some(b64) = loaded.settings.get(KEY).filter(|v| !v.is_empty()) {
        return base64_decode(&b64)
            .ok_or_else(|| Error::RustError("venue pass key is not base64".into()));
    }

    // First use: mint 32 bytes from the runtime's CSPRNG. Not from a hash of the
    // venue id — that would make every deployment's keys derivable from public
    // data.
    let mut key = [0u8; 32];
    getrandom_fill(&mut key)?;
    let b64 = base64_encode(&key);
    // ON CONFLICT DO NOTHING, as a read inside the object's own turn: two
    // requests minting at once must not end with two keys, because a pass
    // signed with one would not verify against the other.
    let mine = b64.clone();
    let settled = crate::hubstore::with_settings(place, move |s| {
        match s.get(KEY).filter(|v| !v.is_empty()) {
            Some(existing) => Ok(existing),
            None => {
                s.set(KEY, &mine);
                Ok(mine.clone())
            }
        }
    })
    .await?;
    base64_decode(&settled)
        .ok_or_else(|| Error::RustError("venue pass key is not base64".into()))
}

fn getrandom_fill(buf: &mut [u8]) -> Result<()> {
    // The Workers runtime exposes the Web Crypto CSPRNG.
    let crypto = worker::js_sys::global()
        .dyn_into::<worker::js_sys::Object>()
        .map_err(|_| Error::RustError("no global object".into()))?;
    let subtle = worker::js_sys::Reflect::get(&crypto, &"crypto".into())
        .map_err(|_| Error::RustError("no crypto".into()))?;
    let arr = worker::js_sys::Uint8Array::new_with_length(buf.len() as u32);
    let f = worker::js_sys::Reflect::get(&subtle, &"getRandomValues".into())
        .map_err(|_| Error::RustError("no getRandomValues".into()))?;
    let f: worker::js_sys::Function = f
        .dyn_into()
        .map_err(|_| Error::RustError("getRandomValues is not callable".into()))?;
    f.call1(&subtle, &arr)
        .map_err(|_| Error::RustError("getRandomValues failed".into()))?;
    arr.copy_to(buf);
    Ok(())
}

fn base64_encode(b: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(b)
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s).ok()
}


/// `GET /api/public/locations/:slug/reservations/:id/pass`
///
/// Only a CONFIRMED booking gets a pass. Minting one for a request the venue has
/// not answered would put a code on a phone that the door will refuse.
pub async fn issue_pass(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let (Some(slug), Some(id)) = (ctx.param("slug").cloned(), ctx.param("id").cloned()) else {
        return Response::error("missing slug or id", 400);
    };
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    // AUTHENTICATED, TO THIS VENUE, AND TO THIS BOOKING: the guest's own
    // token, or the venue. Any other customer of the venue is a 404.
    if let Err(r) = super::reservations::side_at(&req, &ctx, &place, &id).await {
        return Ok(r);
    }

    let t = load_bookings(&place).await?;
    let Some(row) = reservation_of(&t, &id) else {
        return Response::error("not found", 404);
    };
    let events = events_of(&t, &id);
    let status = match fold_status(&events) {
        Ok(s) => s,
        Err(why) => return Response::error(format!("reservation unreadable: {why}"), 500),
    };
    if status != ReservationStatus::Confirmed {
        return Response::error(
            format!(
                "a pass is issued for a confirmed booking; this one is {}",
                status.as_str()
            ),
            409,
        );
    }

    let key = pass_key(&place).await?;
    let claims = PassClaims {
        venue: id64(&place.venue),
        reservation: id64(&row.id),
        slot_min: row.slot_min,
        party: row.party as u16,
        // The event count: a reissued pass after a change differs from the one
        // it replaces, which is what `nonce` is for.
        nonce: events.len() as u64,
    };
    let minted = pass::issue(&key, claims)
        .map_err(|e| Error::RustError(format!("pass: {}", e.message())))?;

    Response::from_json(&json!({
        "code": pass::encode(&minted),
        "reservationId": row.id,
        "slotMin": row.slot_min,
        "party": row.party,
    }))
}

#[derive(Deserialize)]
struct VerifyBody {
    code: String,
}

/// `POST /api/public/locations/:slug/pass/verify`
///
/// The venue's scanner. Every refusal is named: a scanner that can only say
/// "invalid" sends people away without telling them they are simply early.
pub async fn verify_pass(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    let body: VerifyBody = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request: {e}"), 400),
    };

    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    let key = pass_key(&place).await?;

    let decoded = match pass::decode(&body.code) {
        Ok(p) => p,
        Err(e) => {
            return Response::from_json(&json!({ "ok": false, "why": e.message() }));
        }
    };
    match pass::verify(
        &key,
        &decoded,
        id64(&place.venue),
        &PassWindow::default_window(),
        now_min(ctx.data.now_ms),
    ) {
        Ok(()) => Response::from_json(&json!({
            "ok": true,
            "reservation": decoded.claims.reservation,
            "party": decoded.claims.party,
            "slotMin": decoded.claims.slot_min,
        })),
        // 200 with `ok:false`: the request succeeded, the pass did not. A 4xx
        // here would make a scanner treat a wrong code as a broken scanner.
        Err(e) => Response::from_json(&json!({ "ok": false, "why": e.message() })),
    }
}
