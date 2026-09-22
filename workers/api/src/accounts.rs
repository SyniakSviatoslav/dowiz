//! Login, refresh, logout — for owners and couriers.
//!
//! The refresh design is the old platform's, kept because it solves a problem
//! naive rotation gets wrong. Tokens rotate inside a FAMILY. Replaying a spent
//! one is theft and revokes the whole family. But two tabs refreshing at the
//! same instant is NOT theft, and the old code learned that the hard way: it
//! distinguishes the two by whether a sibling rotation happened seconds ago, and
//! answers 409 instead of logging every device out.

use serde::{Deserialize, Serialize};
use serde_json::json;
use worker::*;

use crate::auth::{
    self, hash_password, sha256_hex, verify_password_constant_work, Claims, COURIER_TTL_MS,
    OWNER_TTL_MS,
};

const REFRESH_TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;
const COURIER_REFRESH_TTL_MS: i64 = 30 * 24 * 60 * 60 * 1000;
/// Two tabs refreshing together is not an attack. The old service picked five
/// seconds after a bug where honest double-fires logged people out everywhere.
const CONCURRENT_REFRESH_GRACE_MS: i64 = 5_000;

#[derive(Deserialize)]
pub struct LoginIn {
    pub email: String,
    pub password: String,
    /// WHICH VENUE, when the caller owns more than one.
    ///
    /// The token carries `active_location_id`, and `Place::of_any` trusts that
    /// claim BEFORE the host: an owner of two venues signing in without saying
    /// which one got a token for the OLDEST membership, and every write they
    /// then made -- a photograph, a logo, an address -- landed on that venue
    /// however the request was addressed. Two hundred writes went to the wrong
    /// restaurant that way, each answering 200.
    ///
    /// It is checked against this caller's OWN memberships, so naming a venue
    /// grants nothing: an id they do not own finds no row and the login is
    /// refused exactly as if they had no membership at all.
    #[serde(default)]
    pub location_id: Option<String>,
}

#[derive(Deserialize)]
pub struct RefreshIn {
    pub refresh_token: String,
}

#[derive(Serialize)]
struct TokenPair {
    access_token: String,
    refresh_token: String,
}

fn now_ms() -> i64 {
    Date::now().as_millis() as i64
}

fn opaque_token() -> Option<String> {
    // Two UUIDs of platform CSPRNG. Fail closed if it is unreachable rather than
    // reach for a weaker source.
    Some(format!("{}{}", crate::edge_id()?, crate::edge_id()?).replace('-', ""))
}

async fn issue_owner_refresh(env: &Env, user_id: &str, family_id: &str) -> Result<Option<String>> {
    let Some(tok) = opaque_token() else { return Ok(None) };
    let now = now_ms();
    // THE TOKEN HASH IS THE KEY. The table had a surrogate `id` and an index on
    // `token_hash`; the hash is what every read looks up by, so it is the
    // record's id and the index is the record.
    let hash = sha256_hex(&tok);
    let (h, u, f) = (hash.clone(), user_id.to_string(), family_id.to_string());
    crate::identity_store::with_sessions(env, move |t| {
        let rec = serde_json::json!({
            "user_id": u, "family_id": f, "used": false,
            "expires_at_ms": now + REFRESH_TTL_MS, "created_at_ms": now,
        })
        .to_string();
        let index = vec![(
            crate::identity_store::refresh_family(&f, now),
            h.clone(),
        )];
        t.put(crate::identity_store::K_REFRESH, &h, &rec, &index, &[])
            .map_err(|e| Error::RustError(format!("refresh: {e}")))
    })
    .await?;
    Ok(Some(tok))
}

/// Rewrite a password hash that was stored at a cost this Worker can no longer
/// afford — see `auth::needs_rehash`.
///
/// ON LOGIN, because that is the only moment the plaintext exists. A migration
/// cannot do this: the whole point of the stored value is that the password is
/// not recoverable from it, so the rehash has to ride along with someone
/// actually signing in.
///
/// IT NEVER FAILS THE LOGIN. The caller has already been authenticated; if the
/// write does not land they simply pay the old cost again on their next visit
/// and we try again then. Turning a successful authentication into a 500
/// because of an optimisation would be the worse trade by a wide margin.
///
/// WHICH SET, chosen at the two call sites. This used to be a table name
/// interpolated into SQL, with a comment explaining that it was a `&'static
/// str` for exactly that reason. There is no SQL to interpolate into now, and
/// the parameter is a record kind rather than a fragment of a statement.
async fn upgrade_hash(
    env: &Env,
    what: Who,
    id: &str,
    password: &str,
    stored: Option<&str>,
) {
    let Some(stored) = stored else { return };
    if !auth::needs_rehash(stored) {
        return;
    }
    let Ok(fresh) = hash_password(password) else { return };
    let id = id.to_string();
    let _ = match what {
        Who::Owner => {
            crate::identity_store::with_identity(env, move |t| {
                if let Some(mut u) =
                    crate::identity_store::rec(t, crate::identity_store::K_USER, &id)
                {
                    u["password_hash"] = serde_json::json!(fresh);
                    let email = crate::identity_store::s_of(&u, "email");
                    let index =
                        vec![(crate::identity_store::user_by_email(&email), id.clone())];
                    t.put(crate::identity_store::K_USER, &id, &u.to_string(), &index, &[])
                        .map_err(|e| Error::RustError(format!("user: {e}")))?;
                }
                Ok(())
            })
            .await
        }
        Who::Courier => {
            crate::identity_store::with_couriers(env, move |t| {
                if let Some(mut c) =
                    crate::identity_store::rec(t, crate::identity_store::K_COURIER, &id)
                {
                    c["password_hash"] = serde_json::json!(fresh);
                    let index = crate::identity_store::courier_index(&id, &c);
                    t.put(crate::identity_store::K_COURIER, &id, &c.to_string(), &index, &[])
                        .map_err(|e| Error::RustError(format!("courier: {e}")))?;
                }
                Ok(())
            })
            .await
        }
    };
}

/// Whose password is being rehashed.
#[derive(Clone, Copy)]
enum Who {
    Owner,
    Courier,
}

/// `POST /api/auth/login` — owner, email + password.
pub async fn owner_login(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: LoginIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let email = body.email.trim().to_lowercase();

    #[derive(Deserialize)]
    struct U {
        id: String,
        password_hash: Option<String>,
        display_name: Option<String>,
    }
    let ident = crate::identity_store::identity(&ctx.env).await?;
    let user: Option<U> = crate::identity_store::user_id_for_email(&ident, &email)
        .and_then(|id| crate::identity_store::rec(&ident, crate::identity_store::K_USER, &id))
        .map(|u| U {
        id: crate::identity_store::s_of(&u, "id"),
        password_hash: u
            .get("password_hash")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        display_name: u
            .get("display_name")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        });

    // Same work on a miss as on a hit: "no such account" and "wrong password"
    // must not be distinguishable by how long the answer takes.
    let stored = user.as_ref().and_then(|u| u.password_hash.as_deref());
    if !verify_password_constant_work(&body.password, stored) {
        return Response::error("invalid credentials", 401);
    }
    let user = user.expect("verified above");
    upgrade_hash(&ctx.env, Who::Owner, &user.id, &body.password, user.password_hash.as_deref())
        .await;

    // Authority comes from memberships, not from the request.
    #[derive(Deserialize)]
    struct M {
        location_id: String,
    }
    // ── AND THE HOST NAMES THE VENUE HERE TOO ──
    //
    // This owner owns two venues, and the fallback below is `ORDER BY
    // created_at_ms LIMIT 1`: the OLDEST membership, whichever host the login
    // arrived on. Measured against the deployed Worker: signing in at
    // `dubin-sushi.dowiz.org` returned a token whose `active_location_id` was
    // `sushi-durres`.
    //
    // That is not a cosmetic mismatch, because `owner_and_venue` was fixed to
    // resolve the venue from the TOKEN's claim -- so every writer that trusts
    // it inherits the coin toss one level up. `invite_courier` is one of them:
    // a code created from the dubin console mints a courier attached to sushi.
    // Five of the six couriers on this platform sit on one venue and one on
    // the other, which is what that looks like after a few weeks.
    //
    // Same rule as the courier's, and the same function.
    let host_venue = venue_of_host(&req, &ctx).await?;
    let wanted = match venue_for_login(host_venue.as_deref(), body.location_id.as_deref()) {
        Ok(v) => v,
        Err(VenueContradiction) => {
            return Response::error("that location is not this venue", 403)
        }
    };
    let m: Option<M> = match wanted.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(l) => crate::identity_store::membership(&ident, l, &user.id)
            .filter(|x| crate::identity_store::s_of(x, "role") == "owner")
            .map(|_| M { location_id: l.to_string() }),
        // Reached where the host names no venue: the apex, `*.workers.dev`.
        // For everyone who owns one venue this is still the only membership.
        //
        // IT IS NO LONGER `ORDER BY created_at_ms LIMIT 1`. That fallback was
        // the oldest membership whichever host the login arrived on, and it is
        // what sent an owner of two restaurants a token for the other one. Here
        // the venue is named by the host above; when the host names none, the
        // only defensible answer for someone with several is "say which", so
        // the pick is the sorted-first one and is stable rather than an
        // accident of when a row was written.
        None => {
            let mut owned: Vec<String> = crate::identity_store::memberships_of(&ident, &user.id)
                .into_iter()
                .filter(|(_, role)| role == "owner")
                .map(|(loc, _)| loc)
                .collect();
            owned.sort();
            owned.into_iter().next().map(|location_id| M { location_id })
        }
    };
    // A PLATFORM ADMINISTRATOR OWNS NO RESTAURANT, and that is the point of
    // them. Authority still comes from a record and not from the request -- it
    // just comes from a different one. Without this, the only way to sign in to
    // the main hub was to first make its administrator the owner of somebody's
    // venue, which would put a platform account inside a tenant's data.
    //
    // THE TOKEN NAMES NO VENUE. `active_location_id` stays `None`, so
    // `claimed_venue` finds nothing and this token cannot be used to read a
    // hub: every venue route resolves its venue from the claim first. A
    // platform admin can create hubs and cannot read one.
    let admin: Option<M> = if m.is_none() {
        ident
            .get(crate::identity_store::K_ADMIN, &user.id)
            .map(|_| M { location_id: String::new() })
    } else {
        None
    };
    if m.is_none() && admin.is_none() {
        return Response::error("no active owner membership", 403);
    }
    let venue = m.as_ref().map(|m| m.location_id.clone());

    let now = now_ms();
    let claims = Claims::Owner {
        sub: user.id.clone(),
        user_id: user.id.clone(),
        active_location_id: venue.clone(),
        iat: now,
        exp: now + OWNER_TTL_MS,
    };
    let access = match auth::sign(&ctx.env, &claims) {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    let Some(family) = crate::edge_id() else {
        return Response::error("no platform CSPRNG", 500);
    };
    let refresh = issue_owner_refresh(&ctx.env, &user.id, &family).await?;

    Response::from_json(&json!({
        "access_token": access,
        "refresh_token": refresh,
        "user": { "id": user.id, "name": user.display_name, "locationId": venue,
                  "platformAdmin": m.is_none() }
    }))
}

/// `POST /api/auth/refresh` — rotate within the family, detect reuse.
pub async fn owner_refresh(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: RefreshIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let hash = sha256_hex(&body.refresh_token);
    let now = now_ms();

    #[derive(Deserialize)]
    struct R {
        user_id: String,
        family_id: String,
    }
    // ── THE WHOLE ROTATION IS ONE WRITE ──
    //
    // It was four statements: a read, a count, a conditional delete, and a
    // conditional update whose `changes` count stood in for atomicity. Inside
    // the object's own turn none of them can interleave, so the decision and
    // the write are the same operation -- and "whoever flips `used` first owns
    // the rotation" stops being a race the database happened to settle.
    enum Verdict {
        Ok { user_id: String, family_id: String },
        Missing,
        Expired,
        Concurrent,
        Reused,
    }
    let h = hash.clone();
    let verdict = crate::identity_store::with_sessions(&ctx.env, move |t| {
        let Some(r) = crate::identity_store::rec(t, crate::identity_store::K_REFRESH, &h) else {
            return Ok(Verdict::Missing);
        };
        if crate::identity_store::i_of(&r, "expires_at_ms") <= now {
            return Ok(Verdict::Expired);
        }
        let family_id = crate::identity_store::s_of(&r, "family_id");
        let user_id = crate::identity_store::s_of(&r, "user_id");
        if r.get("used").and_then(serde_json::Value::as_bool).unwrap_or(false) {
            // Spent already. Either an honest race or a stolen token; the two
            // are told apart by whether the family rotated moments ago. The
            // count is a prefix scan of the family's own keys, which is why
            // they carry a zero-padded time.
            let recent = t
                .scan(&format!("refresh.family/{family_id}/"))
                .into_iter()
                .filter(|(key, _)| {
                    key.rsplit('/')
                        .next()
                        .and_then(|n| n.parse::<i64>().ok())
                        .is_some_and(|at| at > now - CONCURRENT_REFRESH_GRACE_MS)
                })
                .count();
            if recent > 0 {
                return Ok(Verdict::Concurrent);
            }
            // THE WHOLE FAMILY GOES. Every key in the prefix, and the records
            // they name -- `remove` drops a record's own index entries, so
            // dropping the record is enough.
            for (_, id) in t.scan(&format!("refresh.family/{family_id}/")) {
                t.remove(crate::identity_store::K_REFRESH, &id);
            }
            return Ok(Verdict::Reused);
        }
        let mut r = r;
        r["used"] = serde_json::json!(true);
        let created = crate::identity_store::i_of(&r, "created_at_ms");
        let index = vec![(
            crate::identity_store::refresh_family(&family_id, created),
            h.clone(),
        )];
        t.put(crate::identity_store::K_REFRESH, &h, &r.to_string(), &index, &[])
            .map_err(|e| Error::RustError(format!("refresh: {e}")))?;
        Ok(Verdict::Ok { user_id, family_id })
    })
    .await?;
    let row = match verdict {
        Verdict::Ok { user_id, family_id } => R { user_id, family_id },
        Verdict::Missing => return Response::error("invalid refresh token", 401),
        Verdict::Expired => return Response::error("refresh token expired", 401),
        Verdict::Concurrent => return Response::error("concurrent refresh", 409),
        Verdict::Reused => {
            return Response::error("token reuse detected; family revoked", 401)
        }
    };

    // Re-derive authority. A revoked owner does not roll forward on a refresh.
    //
    // AND NEITHER DOES THE VENUE MOVE. This was the third place with the
    // oldest-membership guess, and the worst of them, because it changes a
    // session that was already correct. Measured against the deployed Worker:
    // signing in at `dubin-sushi.dowiz.org` gave a `dubin-durres` token, and
    // ONE refresh on the same host gave `sushi-durres`.
    //
    // The console reaches this on its own — it refreshes on the first 401, so
    // any tab left open past the access TTL flips — and it keeps sending the
    // venue it is showing in the BODY. `owner_and_venue` then authorises that
    // venue while `Place::of_any` resolves the image from the CLAIM, so the
    // owner reads an empty queue for their own restaurant and their writes
    // land in the other one. That is the "two hundred writes to the wrong
    // restaurant" failure this module's header describes, re-entering through
    // the back door.
    //
    // The refresh family stores no venue (migration 0003), so the host is
    // asked, exactly as the login does — same helper, same rule, so the two
    // agree by construction rather than by coincidence.
    #[derive(Deserialize)]
    struct M {
        location_id: String,
    }
    let host_venue = venue_of_host(&req, &ctx).await?;
    let ident = crate::identity_store::identity(&ctx.env).await?;
    let m: Option<M> = match host_venue.as_deref() {
        Some(v) => crate::identity_store::membership(&ident, v, &row.user_id)
            .filter(|x| crate::identity_store::s_of(x, "role") == "owner")
            .map(|_| M { location_id: v.to_string() }),
        // A host that names no venue: the apex, `*.workers.dev`. The caller has
        // said nothing, and for an owner of one venue this is still that venue.
        // Sorted rather than oldest-first, for the reason the login gives.
        None => {
            let mut owned: Vec<String> =
                crate::identity_store::memberships_of(&ident, &row.user_id)
                    .into_iter()
                    .filter(|(_, role)| role == "owner")
                    .map(|(loc, _)| loc)
                    .collect();
            owned.sort();
            owned.into_iter().next().map(|location_id| M { location_id })
        }
    };
    let Some(m) = m else {
        return Response::error("owner access revoked", 401);
    };

    let claims = Claims::Owner {
        sub: row.user_id.clone(),
        user_id: row.user_id.clone(),
        active_location_id: Some(m.location_id),
        iat: now,
        exp: now + OWNER_TTL_MS,
    };
    let access = match auth::sign(&ctx.env, &claims) {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    let refresh = issue_owner_refresh(&ctx.env, &row.user_id, &row.family_id).await?;
    Response::from_json(&TokenPair {
        access_token: access,
        refresh_token: refresh.unwrap_or_default(),
    })
}

/// `POST /api/auth/logout` — every device, like the old service.
pub async fn owner_logout(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let p = match auth::authenticate(&req, &ctx.env, &db, now_ms()).await {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };
    let auth::Principal::Owner { user_id, .. } = p else {
        return Response::error("forbidden role", 403);
    };
    // EVERY DEVICE. A scan of the records rather than of an index, because
    // "all of this user's tokens" is not an access path any other reader wants
    // -- and inventing an index for one logout would be a key maintained on
    // every refresh to save one walk on the rarest call in the file.
    crate::identity_store::with_sessions(&ctx.env, move |t| {
        let mine: Vec<String> = t
            .all(crate::identity_store::K_REFRESH)
            .into_iter()
            .filter(|(_, j)| {
                serde_json::from_str::<serde_json::Value>(j)
                    .ok()
                    .is_some_and(|v| crate::identity_store::s_of(&v, "user_id") == user_id)
            })
            .map(|(id, _)| id)
            .collect();
        for id in mine {
            t.remove(crate::identity_store::K_REFRESH, &id);
        }
        Ok(())
    })
    .await?;
    // The access token itself outlives this, up to its 24h exp. The old service
    // recorded that as an accepted risk rather than pretending otherwise.
    Response::from_json(&json!({ "ok": true }))
}

/// `POST /api/courier/auth/login`
/// The venue this request's Host header names, if it names one.
///
/// `None` on the apex, on `www.`, and on `*.workers.dev` -- the hosts that
/// belong to the platform rather than to a venue -- and on a subdomain that
/// matches no venue's slug.
async fn venue_of_host(req: &Request, ctx: &RouteContext<()>) -> Result<Option<String>> {
    let Some(slug) = crate::hubstore::Place::slug_of_host(req, ctx) else {
        return Ok(None);
    };
    let venue = crate::hubstore::Place::of_slug(ctx, &slug).await?.venue;
    Ok((venue != crate::hubstore::UNNAMED_VENUE).then_some(venue))
}

/// The body named a venue and the host named a different one.
pub struct VenueContradiction;

/// WHICH VENUE A SESSION IS FOR, as a rule that can be read.
///
/// Used by BOTH logins. An owner may own two venues and a courier may carry
/// bags for two, and in each case the request has already named one -- in the
/// host it arrived on -- long before any table is consulted.
///
/// `None` means "the caller has named no venue and the host names none
/// either" -- the single-membership fallback is then honest. Everything else
/// is a name the membership lookup must confirm.
pub fn venue_for_login(
    host_venue: Option<&str>,
    body_location: Option<&str>,
) -> std::result::Result<Option<String>, VenueContradiction> {
    match (host_venue, body_location) {
        (Some(h), Some(b)) if h != b => Err(VenueContradiction),
        (Some(h), _) => Ok(Some(h.to_string())),
        (None, Some(b)) => Ok(Some(b.to_string())),
        (None, None) => Ok(None),
    }
}

pub async fn courier_login(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    #[derive(Deserialize)]
    struct In {
        #[serde(default)]
        email: Option<String>,
        #[serde(default)]
        phone: Option<String>,
        password: String,
        #[serde(default)]
        location_id: Option<String>,
    }
    let body: In = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };

    // Email and phone are looked up under SEPARATE PREFIXES. The old schema
    // matched both against one shared hash space, where a phone could in
    // principle resolve an email's record; two key prefixes cannot.
    let crew = crate::identity_store::couriers(&ctx.env).await?;
    let found = match (&body.email, &body.phone) {
        (Some(e), _) if !e.trim().is_empty() => crate::identity_store::courier_id_for_email(
            &crew,
            &sha256_hex(&e.trim().to_lowercase()),
        ),
        (_, Some(p)) if !p.trim().is_empty() => {
            crate::identity_store::courier_id_for_phone(&crew, &sha256_hex(p.trim()))
        }
        _ => return Response::error("email or phone required", 400),
    };

    struct C {
        id: String,
        password_hash: String,
        status: String,
    }
    let c: Option<C> = found
        .and_then(|id| {
            crate::identity_store::rec(&crew, crate::identity_store::K_COURIER, &id)
                .map(|r| (id, r))
        })
        .map(|(id, r)| C {
            id,
            password_hash: crate::identity_store::s_of(&r, "password_hash"),
            status: crate::identity_store::s_of(&r, "status"),
        });
    let stored = c.as_ref().map(|c| c.password_hash.as_str());
    if !verify_password_constant_work(&body.password, stored) {
        return Response::error("invalid credentials", 401);
    }
    let c = c.expect("verified above");
    upgrade_hash(&ctx.env, Who::Courier, &c.id, &body.password, Some(c.password_hash.as_str()))
        .await;
    if c.status != "active" {
        return Response::error("courier account is not active", 403);
    }

    struct L {
        location_id: String,
    }
    // ── WHICH VENUE, AND WHY THE HOST DECIDES IT ──
    //
    // This was the courier half of the bug that `owner_and_venue` had: with no
    // `location_id` in the body it fell to `ORDER BY added_at_ms LIMIT 1`, the
    // courier's FIRST venue, whatever host they were standing on. The courier
    // app never sends the field, so that branch is the one every real login
    // takes -- and a courier of `dubin-durres` signing in at
    // `sushi-durres.dowiz.org` was handed a dubin session on the sushi domain.
    // It looked like it worked: the login succeeded, the shift opened, and the
    // app then showed the OTHER venue's pool for ever while `/api/live`, the
    // one place that does compare the principal to the host, answered 404 to
    // every handshake. A courier waiting for orders that are being placed two
    // streets away has no way to tell that from a quiet evening.
    //
    // So the host is asked FIRST, exactly as it is for an anonymous read in
    // `Place::of_any`: a venue's own subdomain names its venue, and a session
    // minted there is for that venue or it is refused. A body `location_id`
    // that disagrees with the host is a contradiction, not a preference, and
    // is refused rather than silently resolved one way -- fail closed, the
    // same stance the socket takes. The old guess survives only where it was
    // ever true: a host that names no venue (the apex, `*.workers.dev`), where
    // the caller genuinely has not said, and a single membership is an answer
    // rather than a coin toss.
    let host_venue = venue_of_host(&req, &ctx).await?;
    let want = match venue_for_login(host_venue.as_deref(), body.location_id.as_deref()) {
        Ok(v) => v,
        Err(VenueContradiction) => {
            return Response::error("that location is not this venue", 403)
        }
    };
    let loc: Option<L> = match &want {
        Some(want) => crew
            .get(
                crate::identity_store::K_ROSTER,
                &crate::identity_store::roster_id(want, &c.id),
            )
            .map(|_| L { location_id: want.clone() }),
        // The old guess survives only where it was ever true: a host that names
        // no venue, where the caller genuinely has not said. Sorted rather than
        // `ORDER BY added_at_ms LIMIT 1`, so a courier on two rosters gets a
        // stable answer instead of an accident of when a row was written.
        None => {
            let mut on: Vec<String> = crew
                .scan(&format!("roster.courier/{}/", c.id))
                .into_iter()
                .filter_map(|(key, _)| key.rsplit('/').next().map(str::to_string))
                .collect();
            on.sort();
            on.into_iter().next().map(|location_id| L { location_id })
        }
    };
    let Some(loc) = loc else {
        return Response::error("not assigned to this location", 403);
    };

    let now = now_ms();
    let (Some(session_id), Some(family_id), Some(secret)) =
        (crate::edge_id(), crate::edge_id(), opaque_token())
    else {
        return Response::error("no platform CSPRNG", 500);
    };
    // The session secret is argon2-hashed, which is why the refresh token has to
    // carry the row id as a prefix: a hash lookup is impossible by design.
    let token_hash = auth::hash_opaque(&secret);
    let (sid, cid, fid, lid) = (
        session_id.clone(),
        c.id.clone(),
        family_id.clone(),
        loc.location_id.clone(),
    );
    crate::identity_store::with_sessions(&ctx.env, move |t| {
        let rec = serde_json::json!({
            "courier_id": cid, "family_id": fid, "token_hash": token_hash,
            "active_location_id": lid,
            "issued_at_ms": now, "expires_at_ms": now + COURIER_REFRESH_TTL_MS,
            "revoked_at_ms": serde_json::Value::Null,
        })
        .to_string();
        t.put(crate::identity_store::K_CSESSION, &sid, &rec, &[], &[])
            .map_err(|e| Error::RustError(format!("courier session: {e}")))
    })
    .await?;

    // NEITHER OF THESE MAY FAIL THE LOGIN. A courier standing in the rain must
    // not be refused because a bookkeeping write lost a generation guard, which
    // is why both were `let _ =` before and stay that way.
    let cid = c.id.clone();
    let _ = crate::identity_store::with_couriers(&ctx.env, move |t| {
        if let Some(mut r) = crate::identity_store::rec(t, crate::identity_store::K_COURIER, &cid) {
            r["last_login_at_ms"] = serde_json::json!(now);
            let index = crate::identity_store::courier_index(&cid, &r);
            t.put(crate::identity_store::K_COURIER, &cid, &r.to_string(), &index, &[])
                .map_err(|e| Error::RustError(format!("courier: {e}")))?;
        }
        Ok(())
    })
    .await;
    // The audit trail goes in THE VENUE'S OWN log, beside its other failures
    // and reveals -- it is a fact about that restaurant, and the venue's
    // console is what reads it.
    if let Ok(place) = crate::hubstore::Place::of(&req, &ctx, Some(&loc.location_id)) {
        let who = c.id.clone();
        let entry = serde_json::json!({
            "action": "login.success", "actor_kind": "courier",
            "actor_id": who, "courier_id": who, "created_at_ms": now,
        })
        .to_string();
        let _ = crate::hubstore::with_log(&place, crate::hubstore::IMAGE_AUDIT, move |log| {
            log.append("courier", &who, &entry)
                .map_err(|e| Error::RustError(format!("audit: {e:?}")))
        })
        .await;
    }

    let claims = Claims::Courier {
        sub: c.id.clone(),
        active_location_id: loc.location_id.clone(),
        jti: session_id.clone(),
        iat: now,
        exp: now + COURIER_TTL_MS,
    };
    let jwt = match auth::sign(&ctx.env, &claims) {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    Response::from_json(&json!({
        "jwt": jwt,
        "refreshToken": format!("{session_id}.{secret}"),
        "courier": { "id": c.id, "locationId": loc.location_id }
    }))
}


#[derive(Deserialize)]
pub struct ClaimIn {
    pub phone: String,
    pub code: String,
    pub password: String,
}

/// `POST /api/courier/auth/claim` — turn an invite into an account.
///
/// PUBLIC BY NECESSITY: the courier has no credentials yet, which is the whole
/// point. What stands in for authentication is the code, and the code is stored
/// hashed and compared in constant time, so this route cannot be used to
/// discover which phone numbers a venue has invited.
///
/// "No invite for this phone" and "wrong code" are ONE answer, reached after the
/// same lookup. Expiry is told apart in the answer, because a courier whose code
/// ran out needs a new one rather than another attempt.
///
/// The courier chooses their own password. An owner who set it for them would
/// know it.
pub async fn courier_claim(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: ClaimIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    // Eight characters. Short enough to type at a door, long enough not to be
    // the four-digit PIN everyone would otherwise pick. The cost of a weak one
    // here is somebody else's shift.
    if body.password.chars().count() < 8 {
        return Response::error("choose a password of at least 8 characters", 400);
    }
    let phone = body.phone.trim().to_string();
    let phone_hash = auth::sha256_hex(&phone);
    let code_hash = auth::sha256_hex(body.code.trim());
    let now = now_ms();

    #[derive(Deserialize)]
    struct Inv {
        id: String,
        location_id: String,
        invited_name: Option<String>,
        expires_at_ms: i64,
    }
    let crew = crate::identity_store::couriers(&ctx.env).await?;
    // The invite is found by the PHONE it was sent to, and the code is checked
    // against the record rather than being part of the lookup: two prefixes
    // would mean an invite findable by a code alone, and a code is guessable in
    // a way a phone number the venue chose is not.
    let inv: Option<Inv> = crew
        .lookup(&crate::identity_store::invite_by_phone(&phone_hash))
        .and_then(|id| {
            crate::identity_store::rec(&crew, crate::identity_store::K_INVITE, &id)
                .map(|r| (id, r))
        })
        .filter(|(_, r)| crate::identity_store::s_of(r, "code_hash") == code_hash)
        .filter(|(_, r)| r.get("used_at_ms").map_or(true, |v| v.is_null()))
        .filter(|(_, r)| r.get("revoked_at_ms").map_or(true, |v| v.is_null()))
        .map(|(id, r)| Inv {
            id,
            location_id: crate::identity_store::s_of(&r, "location_id"),
            invited_name: r
                .get("invited_name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            expires_at_ms: crate::identity_store::i_of(&r, "expires_at_ms"),
        });
    let Some(inv) = inv else {
        return Response::error("that code does not match", 400);
    };
    if now >= inv.expires_at_ms {
        return Response::error("that code has expired -- ask for a new one", 400);
    }

    let (Some(cid), Some(session_id), Some(family_id), Some(secret)) =
        (crate::edge_id(), crate::edge_id(), crate::edge_id(), opaque_token())
    else {
        return Response::error("no platform CSPRNG", 500);
    };
    let pw_hash = match hash_password(&body.password) {
        Ok(h) => h,
        Err(e) => return e.into_response(),
    };
    let name = inv.invited_name.clone().unwrap_or_default();
    // The email was a NOT NULL unique COLUMN and a courier who signs in by
    // phone has no address, so a derived placeholder kept the constraint
    // honest. The constraint is gone; the placeholder stays, because the
    // storefront and the console both show an address field and an empty one
    // reads as a missing value rather than a deliberate absence.
    let email = format!("{}@courier.invalid", phone.replace(['+', ' '], ""));

    // ── THE ACCOUNT, THE ROSTER ROW AND THE USED INVITE, IN ONE WRITE ──
    //
    // They were three statements in sequence with a comment explaining the
    // order: "marked used only after the account exists, so a failure above
    // leaves the invite still claimable". That ordering was the mitigation for
    // a partial write. One transaction removes the partial write instead, and
    // the uniqueness check that used to be a SELECT before an INSERT happens
    // inside the same turn, so "this person already has an account" cannot be
    // answered by a stale read.
    let (c2, e2, p2, ph2, n2, pw2, loc2, inv2) = (
        cid.clone(),
        email.clone(),
        phone.clone(),
        phone_hash.clone(),
        name.clone(),
        pw_hash.clone(),
        inv.location_id.clone(),
        inv.id.clone(),
    );
    let taken = crate::identity_store::with_couriers(&ctx.env, move |t| {
        if crate::identity_store::courier_id_for_phone(t, &ph2).is_some() {
            return Ok(true);
        }
        let rec = serde_json::json!({
            "id": c2, "email_encrypted": e2, "email_hash": auth::sha256_hex(&e2),
            "phone_encrypted": p2, "phone_hash": ph2,
            "full_name_encrypted": n2, "password_hash": pw2,
            "status": "active", "created_at_ms": now,
        });
        let index = crate::identity_store::courier_index(&c2, &rec);
        t.put(crate::identity_store::K_COURIER, &c2, &rec.to_string(), &index, &[])
            .map_err(|e| Error::RustError(format!("courier: {e}")))?;
        let roster = serde_json::json!({
            "courier_id": c2, "location_id": loc2, "role": "courier", "added_at_ms": now,
        })
        .to_string();
        t.put(
            crate::identity_store::K_ROSTER,
            &crate::identity_store::roster_id(&loc2, &c2),
            &roster,
            &[
                (crate::identity_store::roster_by_venue(&loc2, &c2), c2.clone()),
                (crate::identity_store::roster_by_courier(&c2, &loc2), loc2.clone()),
            ],
            &[],
        )
        .map_err(|e| Error::RustError(format!("roster: {e}")))?;
        // ONE SHOT. A code that survived its own use would be a second key to
        // somebody else's account.
        if let Some(mut i) =
            crate::identity_store::rec(t, crate::identity_store::K_INVITE, &inv2)
        {
            i["used_at_ms"] = serde_json::json!(now);
            i["used_by_courier_id"] = serde_json::json!(c2);
            // The phone index goes: a used invite must not be findable by the
            // number it was sent to, or the next claim would match it again.
            let loc = crate::identity_store::s_of(&i, "location_id");
            let index = vec![(crate::identity_store::invite_at(&loc, &inv2), inv2.clone())];
            t.put(crate::identity_store::K_INVITE, &inv2, &i.to_string(), &index, &[])
                .map_err(|e| Error::RustError(format!("invite: {e}")))?;
        }
        Ok(false)
    })
    .await?;
    if taken {
        return Response::error("this person already has an account", 409);
    }

    let token_hash = auth::hash_opaque(&secret);
    let (sid, c3, f3, l3) = (
        session_id.clone(),
        cid.clone(),
        family_id.clone(),
        inv.location_id.clone(),
    );
    crate::identity_store::with_sessions(&ctx.env, move |t| {
        let rec = serde_json::json!({
            "courier_id": c3, "family_id": f3, "token_hash": token_hash,
            "active_location_id": l3,
            "issued_at_ms": now, "expires_at_ms": now + COURIER_REFRESH_TTL_MS,
            "revoked_at_ms": serde_json::Value::Null,
        })
        .to_string();
        t.put(crate::identity_store::K_CSESSION, &sid, &rec, &[], &[])
            .map_err(|e| Error::RustError(format!("courier session: {e}")))
    })
    .await?;

    let claims = Claims::Courier {
        sub: cid.clone(),
        active_location_id: inv.location_id.clone(),
        jti: session_id.clone(),
        iat: now,
        exp: now + COURIER_TTL_MS,
    };
    let jwt = match auth::sign(&ctx.env, &claims) {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    // Signed in on the spot: making them claim the code and then type the
    // password they set ten seconds ago is a step that exists only because the
    // two things were written separately.
    Response::from_json(&json!({
        "jwt": jwt,
        "refreshToken": format!("{session_id}.{secret}"),
        "courier": { "id": cid, "locationId": inv.location_id }
    }))
}

#[cfg(test)]
mod tests {
    use super::{venue_for_login as venue, VenueContradiction};

    /// THE BUG THIS ENCODES. A courier of `dubin-durres` signed in at
    /// `sushi-durres.dowiz.org` and was given a dubin session, because the app
    /// sends no `location_id` and the fallback was the courier's FIRST venue.
    /// The host is the one thing that request did say.
    #[test]
    fn the_host_names_the_venue_when_the_body_does_not() {
        assert_eq!(venue(Some("sushi-durres"), None).ok().flatten().as_deref(), Some("sushi-durres"));
    }

    /// A body that agrees with the host is not a conflict.
    #[test]
    fn agreement_is_allowed() {
        assert_eq!(
            venue(Some("sushi-durres"), Some("sushi-durres")).ok().flatten().as_deref(),
            Some("sushi-durres")
        );
    }

    /// FAIL CLOSED. Resolving a contradiction either way mints a session for a
    /// venue the caller did not ask for on a domain that is not it.
    #[test]
    fn a_body_that_contradicts_the_host_is_refused() {
        assert!(matches!(venue(Some("sushi-durres"), Some("dubin-durres")), Err(VenueContradiction)));
    }

    /// Where the host names no venue -- the apex, `*.workers.dev` -- the
    /// caller genuinely has not said, and the old single-membership guess is
    /// still the only answer available.
    #[test]
    fn a_host_that_names_no_venue_leaves_the_choice_open() {
        assert_eq!(venue(None, None).ok().flatten(), None);
        assert_eq!(venue(None, Some("dubin-durres")).ok().flatten().as_deref(), Some("dubin-durres"));
    }
}
