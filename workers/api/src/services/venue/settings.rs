//! The venue's own configuration: the declared keys and the feature switches.
//!
//! THE KEY SPACE IS CLOSED. An open one would make this a place to stash
//! anything, and nothing would ever be able to say what a venue is configured
//! with.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::owner::owner_and_venue;

/// `GET /api/owner/settings` — the declared keys, their values and their hints.
///
/// The KEY SPACE IS CLOSED. An open one would make this a place to stash
/// arbitrary data that nothing ever reads back, and the console renders the
/// list the hub declares rather than a list of its own.
pub async fn settings(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let loaded = crate::hubstore::load_settings(&place).await?;
    let values: Value = serde_json::from_str(&loaded.settings.as_json()).unwrap_or(json!({}));
    Response::from_json(&json!({
        "values": values,
        "known": dowiz_hub::settings::KNOWN.iter().map(|k| json!({
            "key": k.key, "label": k.label, "hint": k.hint, "default": k.default,
            "secret": dowiz_hub::settings::is_secret(k.key),
        })).collect::<Vec<_>>(),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingIn {
    key: String,
    /// An empty value CLEARS the setting, which is how an owner removes a token
    /// they can no longer see.
    value: String,
}

/// `POST /api/owner/settings`
pub async fn set_setting(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: SettingIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    if !dowiz_hub::settings::KNOWN.iter().any(|k| k.key == body.key) {
        return Response::error(format!("unknown setting {:?}", body.key), 400);
    }
    if body.value.len() > 4096 {
        return Response::error("value too long", 400);
    }
    // An endpoint is checked HERE, while the owner can still fix it, rather
    // than at the first question when they are mid-rush. Plain http is refused
    // outright: a Worker has no loopback, so the native adapter's "only to an
    // address on this machine" exemption cannot apply and would only be a way
    // to send a venue's token in the clear.
    if body.key == "ai.endpoint" && !body.value.trim().is_empty() {
        let v = body.value.trim();
        if !v.starts_with("https://") {
            return Response::error("the endpoint must be https from a Worker", 400);
        }
    }
    // A tax rate is an INTEGER in ppm; `0.20` is refused here, with a reason.
    if let Err(why) = crate::services::ordering::tax_cfg::validate(&body.key, &body.value) {
        return Response::error(why, 400);
    }
    // The stamp card's three keys: 0/1, 2..=20, a reward above 0 (C5).
    if let Err(why) = crate::services::loyalty::stamps::validate(&body.key, &body.value) {
        return Response::error(why, 400);
    }
    let (key, value) = (body.key.clone(), body.value.clone());
    crate::hubstore::with_settings(&place, move |s| {
        s.set(&key, &value);
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true, "key": body.key }))
}

/// `GET /api/owner/features` — what can be switched, and what it costs.
///
/// The list comes from the HUB. A console that held its own copy would show a
/// switch for something that no longer exists, or miss one that does.
pub async fn features(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let s = crate::hubstore::load_settings(&place).await?.settings;
    Response::from_json(&json!({
        "features": dowiz_hub::features::all(&s).into_iter().map(|(f, on)| json!({
            "key": f.key, "label": f.label, "hint": f.hint,
            "surface": f.surface, "on": on, "defaultOn": f.default_on,
        })).collect::<Vec<_>>(),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FeatureIn {
    key: String,
    on: bool,
}

/// `POST /api/owner/features`
pub async fn set_feature(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: FeatureIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    // Only a DECLARED flag. An open key space would make this a way to write
    // arbitrary settings, and nothing would ever read them back.
    if dowiz_hub::features::get(&body.key).is_none() {
        return Response::error(format!("{:?} is not a feature", body.key), 400);
    }
    let (key, value) = (body.key.clone(), if body.on { "1" } else { "0" });
    crate::hubstore::with_settings(&place, move |s| {
        s.set(&key, value);
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true, "key": body.key, "on": body.on }))
}
