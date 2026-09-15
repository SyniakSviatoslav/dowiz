//! Seeding a hub before it has an owner.
//!
//! A hub has a chicken-and-egg problem at birth: the catalogue import wants owner
//! authority, and the owner does not exist until someone claims the hub. P67's
//! `ClaimReceipt` is the real answer — it enrols the owner's root key into the
//! hub's `AnchorRoster` and mints an owner→hub delegation — and this is NOT that.
//! It is the narrow, honest stand-in until the claim path is wired, and it is
//! named `bootstrap` rather than dressed up as something else.
//!
//! FAILS CLOSED. Without `BOOTSTRAP_SECRET` set on the Worker there is no route:
//! the handler answers 404, so an unconfigured hub has no seeding surface at all
//! rather than one guarded by an empty string. The old platform's dev-guard made
//! exactly this choice and stated the reason — a secret compared against "" lets
//! anyone who sends "" through.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::auth::{hash_password, sha256_hex};

#[derive(Deserialize)]
pub struct Bundle {
    /// The venue, in the shape `LocRow` reads back.
    pub location: Value,
    #[serde(default)]
    pub categories: Vec<Value>,
    #[serde(default)]
    pub products: Vec<Value>,
    /// The first owner. Optional: a hub can be seeded with a menu before anyone
    /// claims it, which is exactly the "shadow org" the old schema allowed for.
    #[serde(default)]
    pub owner: Option<OwnerSeed>,
}

#[derive(Deserialize)]
pub struct OwnerSeed {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub name: Option<String>,
}

/// Constant-time-ish compare. Length is allowed to leak; the bytes are not.
fn secret_ok(given: &str, want: &str) -> bool {
    if given.len() != want.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in given.bytes().zip(want.bytes()) {
        diff |= a ^ b;
    }
    diff == 0
}

/// `POST /api/bootstrap` — seed the catalogue, and optionally the first owner.
pub async fn seed(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // No secret configured => the route does not exist. 404, not 401: a 401
    // confirms there is something here to guess at.
    let Ok(want) = ctx.env.secret("BOOTSTRAP_SECRET") else {
        return Response::error("not found", 404);
    };
    let want = want.to_string();
    if want.len() < 32 {
        return Response::error("bootstrap secret is too short to be one", 503);
    }
    let given = req
        .headers()
        .get("x-dowiz-bootstrap")
        .ok()
        .flatten()
        .unwrap_or_default();
    if !secret_ok(&given, &want) {
        return Response::error("not found", 404);
    }

    let bundle: Bundle = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad bundle: {e}"), 400),
    };
    let db = ctx.d1("DB")?;

    // ── catalogue ──
    let loc = bundle.location.clone();
    let cats = bundle.categories.clone();
    let prods = bundle.products.clone();
    // A 500 WITH NO MESSAGE IS NOT AN ERROR REPORT. This route is behind a
    // 32-byte secret, so the caller is the operator seeding their own hub, and
    // telling them what actually failed costs nothing and saves a round trip
    // through `wrangler tail` that produced nothing twice.
    let seeded = crate::hubstore::with_catalog(&db, move |cat| {
        cat.set_location(&serde_json::to_string(&loc).unwrap_or_else(|_| "{}".into()));
        let mut nc = 0;
        for c in &cats {
            let Some(id) = c.get("id").and_then(|x| x.as_str()) else { continue };
            cat.set_category(id, &serde_json::to_string(c).unwrap_or_else(|_| "{}".into()));
            nc += 1;
        }
        let mut np = 0;
        for p in &prods {
            let Some(id) = p.get("id").and_then(|x| x.as_str()) else { continue };
            cat.set_product(id, &serde_json::to_string(p).unwrap_or_else(|_| "{}".into()));
            np += 1;
        }
        Ok((nc, np))
    })
    .await;
    let (n_cat, n_prod) = match seeded {
        Ok(v) => v,
        Err(e) => return Response::error(format!("catalogue seed failed: {e}"), 500),
    };

    // ── THE VENUE ROW THAT `memberships` POINTS AT ──
    //
    // The venue itself lives in the catalogue IMAGE, which is the data model.
    // But `memberships.location_id` is a foreign key into the `locations`
    // table, so seeding an owner against a venue that exists only in the image
    // failed the constraint -- and the failure surfaced as a bare 500. This
    // route has therefore never been able to seed an owner.
    //
    // The row is a POINTER, not a second copy of the truth: only the columns
    // the constraint and the identity queries need. Everything the storefront
    // reads still comes from the image.
    if let Some(id) = bundle.location.get("id").and_then(|x| x.as_str()) {
        let now = Date::now().as_millis() as i64;
        let pick = |k: &str, d: &str| {
            bundle.location.get(k).and_then(|x| x.as_str()).unwrap_or(d).to_string()
        };
        let _ = db
            .prepare(
                "INSERT INTO locations (id,slug,name,phone,status,created_at_ms,updated_at_ms) \
                 VALUES (?1,?2,?3,?4,?5,?6,?6) ON CONFLICT(id) DO UPDATE SET \
                 slug=excluded.slug, name=excluded.name, updated_at_ms=excluded.updated_at_ms",
            )
            .bind(&[
                id.into(),
                pick("slug", id).into(),
                pick("name", "Venue").into(),
                pick("phone", "").into(),
                pick("status", "closed").into(),
                worker::wasm_bindgen::JsValue::from_f64(now as f64),
            ])?
            .run()
            .await?;
    }

    // ── first owner, if one was supplied ──
    let mut owner_id: Option<String> = None;
    if let Some(o) = &bundle.owner {
        let Some(uid) = crate::edge_id() else {
            return Response::error("no platform CSPRNG", 500);
        };
        let hash = match hash_password(&o.password) {
            Ok(h) => h,
            Err(e) => return e.into_response(),
        };
        let now = Date::now().as_millis() as i64;
        let email = o.email.trim().to_lowercase();
        let loc_id = bundle
            .location
            .get("id")
            .and_then(|x| x.as_str())
            .unwrap_or("hub")
            .to_string();

        // Idempotent: seeding twice must not mint a second owner for the same
        // email, and must not overwrite a password already in use.
        db.prepare(
            "INSERT INTO users (id,email,display_name,password_hash,created_at_ms) \
             VALUES (?1,?2,?3,?4,?5) ON CONFLICT(email) DO NOTHING",
        )
        .bind(&[
            uid.clone().into(),
            email.clone().into(),
            o.name.clone().unwrap_or_default().into(),
            hash.into(),
            worker::wasm_bindgen::JsValue::from_f64(now as f64),
        ])?
        .run()
        .await?;

        #[derive(Deserialize)]
        struct U {
            id: String,
        }
        let u: Option<U> = db
            .prepare("SELECT id FROM users WHERE email = ?1")
            .bind(&[email.into()])?
            .first(None)
            .await?;
        if let Some(u) = u {
            let Some(mid) = crate::edge_id() else {
                return Response::error("no platform CSPRNG", 500);
            };
            db.prepare(
                "INSERT INTO memberships (id,user_id,location_id,role,status,created_at_ms) \
                 VALUES (?1,?2,?3,'owner','active',?4) \
                 ON CONFLICT(user_id,location_id,role) DO NOTHING",
            )
            .bind(&[
                mid.into(),
                u.id.clone().into(),
                loc_id.into(),
                worker::wasm_bindgen::JsValue::from_f64(now as f64),
            ])?
            .run()
            .await?;
            owner_id = Some(u.id);
        }
    }

    Response::from_json(&json!({
        "ok": true,
        "categories": n_cat,
        "products": n_prod,
        "ownerId": owner_id,
        // The catalogue's content fingerprint. Two hubs seeded from the same
        // bundle produce the same root, which makes a mirror checkable.
        "catalogRoot": crate::hubstore::load_catalog(&db).await?.catalog.root(),
        "secretHash": sha256_hex(&want)[..8].to_string()
    }))
}
