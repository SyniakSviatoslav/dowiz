//! W-SNN: the owner's view of the sheaf network's shadow, and its switch. I/O ONLY: the rule is
//! `taste/snn.rs` (what runs, what is counted) and `dowiz_hub::snn` (the network).
//!
//!   GET  /api/owner/snn?location_id=        the switch, and how the network agrees with the
//!                                           current ranker (counts per venue, never per guest)
//!   POST /api/owner/snn  {mode}             shadow | on | off  (default shadow)
//! `/api/owner/health` carries the same object under `snn` (`health`).

use serde::Deserialize;
use serde_json::json;
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use super::taste::snn::{health_json, mode_of, tally_of, IMAGE_SNN, SETTING, SNN_BYTES};
use crate::hubstore::Place;
use dowiz_hub::snn::shadow::Mode;

/// The object `/api/owner/health` shows under `snn`. An unreadable counter is said, not zeroed.
pub async fn health(place: &Place, settings: Option<&dowiz_hub::settings::Settings>) -> serde_json::Value {
    let mode = settings.map(mode_of).unwrap_or(Mode::Shadow);
    match crate::hubstore::load_table(place, IMAGE_SNN, SNN_BYTES).await {
        Ok(l) => health_json(mode, &tally_of(&l.table)),
        Err(e) => json!({ "mode": mode.as_str(), "error": e.to_string() }),
    }
}

/// `GET /api/owner/snn?location_id=`
pub async fn owner_view(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = Place::of_any(&req, &ctx).await?;
    let (_, _loc, settings) = match crate::owner::owner_beside(&req, &ctx, &place, crate::hubstore::load_settings(&place)).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    Response::from_json(&health(&place, Some(&settings.settings)).await)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModeIn {
    mode: String,
}

/// `POST /api/owner/snn {mode}` -- one of `Mode::ALL`, anything else a 400 by name.
pub async fn owner_set(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: ModeIn = match crate::body::parse(&mut req).await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let loc = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    if !Mode::ALL.contains(&body.mode.as_str()) {
        return Response::error(format!("snn mode {:?} is not one of shadow, on, off", body.mode), 400);
    }
    let place = Place::of_authorised(&ctx, &loc)?;
    let mode = body.mode.clone();
    crate::hubstore::with_settings(&place, move |s| {
        s.set(SETTING, &mode);
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true, "mode": body.mode }))
}
