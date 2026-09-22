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
    /// Remove what this bundle does not name.
    ///
    /// Default false, because an incremental seed must stay incremental. Set
    /// when the bundle IS the catalogue -- a venue's real menu replacing the
    /// placeholder one -- and the dishes and categories the bundle leaves out
    /// are deleted rather than left beside it.
    #[serde(default)]
    pub replace: bool,
    /// The first owner. Optional: a hub can be seeded with a menu before anyone
    /// claims it, which is exactly the "shadow org" the old schema allowed for.
    #[serde(default)]
    pub owner: Option<OwnerSeed>,
    /// The people who will carry the orders.
    ///
    /// A hub seeded with a menu and an owner and NO courier cannot complete a
    /// single delivery, so leaving them out made a "seeded" hub one that still
    /// could not run a service. The proper path is the owner's invite flow --
    /// a 16-character code the courier spends once, choosing their own password
    /// -- and that flow lives on the hub's roster, which this Worker does not
    /// use yet. Until it does, seeding is how a courier exists here.
    #[serde(default)]
    pub couriers: Vec<CourierSeed>,
}

#[derive(Deserialize)]
pub struct CourierSeed {
    pub phone: String,
    pub password: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Deserialize)]
pub struct OwnerSeed {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub name: Option<String>,
}

/// Constant-time-ish compare. Length is allowed to leak; the bytes are not.
pub(crate) fn secret_ok(given: &str, want: &str) -> bool {
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
pub async fn seed(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
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
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;

    // ── catalogue ──
    let loc = bundle.location.clone();
    let cats = bundle.categories.clone();
    let prods = bundle.products.clone();
    // A 500 WITH NO MESSAGE IS NOT AN ERROR REPORT. This route is behind a
    // 32-byte secret, so the caller is the operator seeding their own hub, and
    // telling them what actually failed costs nothing and saves a round trip
    // through `wrangler tail` that produced nothing twice.
    let replace = bundle.replace;
    let seeded = crate::hubstore::with_catalog(&place, move |cat| {
        // ── THE VENUE RECORD IS MERGED, NEVER REPLACED WHOLESALE ──
        //
        // This wrote the bundle's `location` straight over the stored one, and
        // a bundle carries the two fields an importer knows: `id` and `slug`.
        // Seeding a catalogue into a LIVE venue therefore erased its name, its
        // currency, its locales, its delivery terms, its opening hours, its
        // theme and its logo -- everything a storefront reads -- and the route
        // reported success. The bundle's keys win where they are present; every
        // other key the venue already had survives.
        let existing: serde_json::Value = cat
            .location()
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        let mut merged = existing;
        if let (Some(m), Some(incoming)) = (merged.as_object_mut(), loc.as_object()) {
            for (k, v) in incoming {
                m.insert(k.clone(), v.clone());
            }
        } else {
            merged = loc.clone();
        }
        cat.set_location(&serde_json::to_string(&merged).unwrap_or_else(|_| "{}".into()));

        // ── REPLACE, when the caller asks for it ──
        //
        // Seeding only ever ADDED, so importing a real 165-dish catalogue over
        // an 18-dish placeholder left 183 dishes and the customer choosing
        // between them. `replace` removes what the bundle does not name; the
        // default stays merge, because that is what an incremental seed means.
        if replace {
            let keep_p: std::collections::BTreeSet<String> = prods
                .iter()
                .filter_map(|p| p.get("id").and_then(|x| x.as_str()).map(String::from))
                .collect();
            let keep_c: std::collections::BTreeSet<String> = cats
                .iter()
                .filter_map(|c| c.get("id").and_then(|x| x.as_str()).map(String::from))
                .collect();
            for (id, _) in cat.products() {
                if !keep_p.contains(&id) {
                    cat.remove_product(&id);
                }
            }
            for (id, _) in cat.categories() {
                if !keep_c.contains(&id) {
                    cat.remove_category(&id);
                }
            }
        }
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
        let now = ctx.data.now_ms;
        let pick = |k: &str, d: &str| {
            bundle.location.get(k).and_then(|x| x.as_str()).unwrap_or(d).to_string()
        };
        let (i2, sl, nm, ph, st) = (
            id.to_string(),
            pick("slug", id),
            pick("name", "Venue"),
            pick("phone", ""),
            pick("status", "closed"),
        );
        // The registry record is a POINTER, not a second copy of the truth:
        // only what a lookup by slug and the platform's listing need.
        // Everything the storefront reads still comes from the image.
        crate::identity_store::with_registry(&ctx.env, move |t| {
            // The upsert kept `created_at_ms` and replaced the rest; so does
            // this, and for the same reason -- re-seeding a venue must not make
            // it look newly created in the platform's list.
            let created = crate::identity_store::rec(t, crate::identity_store::K_LOC, &i2)
                .map(|r| crate::identity_store::i_of(&r, "created_at_ms"))
                .unwrap_or(now);
            let rec = serde_json::json!({
                "id": i2, "slug": sl, "name": nm, "phone": ph, "status": st,
                "created_at_ms": created, "updated_at_ms": now,
            })
            .to_string();
            t.put(
                crate::identity_store::K_LOC,
                &i2,
                &rec,
                &[(crate::identity_store::loc_by_slug(&sl), i2.clone())],
                &[],
            )
            .map_err(|e| Error::RustError(format!("registry: {e}")))
        })
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
        let now = ctx.data.now_ms;
        let email = o.email.trim().to_lowercase();
        let loc_id = bundle
            .location
            .get("id")
            .and_then(|x| x.as_str())
            .unwrap_or("hub")
            .to_string();

        // Idempotent: seeding twice must not mint a second owner for the same
        // email, and must not overwrite a password already in use.
        // ONE TURN: mint the person if the address is free, otherwise adopt the
        // one who holds it, and make them the owner either way. The three
        // statements this replaces were an `ON CONFLICT DO NOTHING`, a read-back
        // to find out which of the two had happened, and a second
        // `ON CONFLICT DO NOTHING` -- a sequence whose correctness depended on
        // nothing else running between them.
        let (u2, e2, n2, h2, l2) = (
            uid.clone(),
            email.clone(),
            o.name.clone().unwrap_or_default(),
            hash,
            loc_id.clone(),
        );
        owner_id = Some(
            crate::identity_store::with_identity(&ctx.env, move |t| {
                let user_id = match crate::identity_store::user_id_for_email(t, &e2) {
                    Some(existing) => existing,
                    None => {
                        let rec = serde_json::json!({
                            "id": u2, "email": e2, "display_name": n2,
                            "password_hash": h2, "created_at_ms": now,
                        })
                        .to_string();
                        t.put(
                            crate::identity_store::K_USER,
                            &u2,
                            &rec,
                            &[(crate::identity_store::user_by_email(&e2), u2.clone())],
                            &[&crate::identity_store::user_by_email(&e2)],
                        )
                        .map_err(|e| Error::RustError(format!("user: {e}")))?;
                        u2.clone()
                    }
                };
                let m = serde_json::json!({
                    "user_id": user_id, "location_id": l2,
                    "role": "owner", "status": "active", "created_at_ms": now,
                })
                .to_string();
                t.put(
                    crate::identity_store::K_MEMBER,
                    &crate::identity_store::member_id(&l2, &user_id),
                    &m,
                    &[
                        (
                            crate::identity_store::member_by_venue(&l2, &user_id),
                            user_id.clone(),
                        ),
                        (
                            crate::identity_store::member_by_user(&user_id, &l2),
                            l2.clone(),
                        ),
                    ],
                    &[],
                )
                .map_err(|e| Error::RustError(format!("membership: {e}")))?;
                Ok(user_id)
            })
            .await?,
        );
    }

    // ── couriers ──
    let mut n_courier = 0usize;
    let loc_id = bundle
        .location
        .get("id")
        .and_then(|x| x.as_str())
        .unwrap_or("hub")
        .to_string();
    for c in &bundle.couriers {
        let Some(cid) = crate::edge_id() else {
            return Response::error("no platform CSPRNG", 500);
        };
        let hash = match hash_password(&c.password) {
            Ok(h) => h,
            Err(e) => return e.into_response(),
        };
        let phone = c.phone.trim().to_string();
        // The email column is NOT NULL and unique, and a courier who signs in by
        // phone has no email. A derived placeholder keeps the constraint honest
        // without inventing an address that might one day reach somebody.
        let email = c
            .email
            .clone()
            .unwrap_or_else(|| format!("{}@courier.invalid", phone.replace(['+', ' '], "")));
        let now = ctx.data.now_ms;
        // THE COURIER AND THEIR ROSTER ROW IN ONE TURN, and the `ON CONFLICT DO
        // NOTHING` becomes what it always meant: seeding twice adopts the
        // person who already holds the address rather than minting a second.
        let (c2, e2, p2, n2, h2, l2) = (
            cid.clone(),
            email.clone(),
            phone.clone(),
            c.name.clone().unwrap_or_default(),
            hash,
            loc_id.clone(),
        );
        let added = crate::identity_store::with_couriers(&ctx.env, move |t| {
            let email_hash = crate::auth::sha256_hex(&e2.to_lowercase());
            let phone_hash = crate::auth::sha256_hex(&p2);
            let id = match crate::identity_store::courier_id_for_email(t, &email_hash)
                .or_else(|| crate::identity_store::courier_id_for_phone(t, &phone_hash))
            {
                Some(existing) => existing,
                None => {
                    let rec = serde_json::json!({
                        "id": c2, "email_encrypted": e2, "email_hash": email_hash,
                        "phone_encrypted": p2, "phone_hash": phone_hash,
                        "full_name_encrypted": n2, "password_hash": h2,
                        "status": "active", "created_at_ms": now,
                    });
                    let index = crate::identity_store::courier_index(&c2, &rec);
                    t.put(
                        crate::identity_store::K_COURIER,
                        &c2,
                        &rec.to_string(),
                        &index,
                        &[],
                    )
                    .map_err(|e| Error::RustError(format!("courier: {e}")))?;
                    c2.clone()
                }
            };
            let roster = serde_json::json!({
                "courier_id": id, "location_id": l2, "role": "courier", "added_at_ms": now,
            })
            .to_string();
            t.put(
                crate::identity_store::K_ROSTER,
                &crate::identity_store::roster_id(&l2, &id),
                &roster,
                &[
                    (crate::identity_store::roster_by_venue(&l2, &id), id.clone()),
                    (crate::identity_store::roster_by_courier(&id, &l2), l2.clone()),
                ],
                &[],
            )
            .map_err(|e| Error::RustError(format!("roster: {e}")))?;
            Ok(true)
        })
        .await?;
        if added {
            n_courier += 1;
        }
    }

    Response::from_json(&json!({
        "ok": true,
        "categories": n_cat,
        "products": n_prod,
        "couriers": n_courier,
        "ownerId": owner_id,
        // The catalogue's content fingerprint. Two hubs seeded from the same
        // bundle produce the same root, which makes a mirror checkable.
        "catalogRoot": crate::hubstore::load_catalog(&place).await?.catalog.root(),
        "secretHash": sha256_hex(&want)[..8].to_string()
    }))
}
