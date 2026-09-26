//! A member of staff signs in, and turns an invite into an account.
//!
//! THE COURIER'S TWO DOORS, for the room. `accounts::courier_login` and
//! `courier_claim` are the shape: a password checked with the same work on a
//! miss as on a hit, the host deciding which venue, a SESSION ROW bound to the
//! token by its `jti` so the owner can end it early, and a claim that writes the
//! account, the membership and the spent invite in one turn. What is different
//! is only where the person lives (`staff_rules` says why), and what the token
//! carries: the capabilities of the roster word, signed.

use serde::Deserialize;
use serde_json::json;
use worker::*;

use super::staff_rules as sr;
use crate::auth::{self, verify_password_constant_work, Claims};
use crate::identity_store as ids;
use dowiz_hub::caps::Preset;

/// The owner's catalogue and stock routes, opened to staff holding the word.
pub(crate) mod guard;

/// Which venue the HOST names, or `None` on the apex and `*.workers.dev`.
///
/// The courier's rule and the owner's (`accounts::venue_for_login`): a venue's
/// own subdomain names its venue, and a body that disagrees is refused.
async fn host_venue(req: &Request, ctx: &RouteContext<crate::Req>) -> Result<Option<String>> {
    let Some(slug) = crate::hubstore::Place::slug_of_host(req, ctx) else { return Ok(None) };
    let venue = crate::hubstore::Place::of_slug(ctx, &slug).await?.venue;
    Ok((venue != crate::hubstore::UNNAMED_VENUE).then_some(venue))
}

/// Mint the session row and the token for a person already proved to be
/// `preset` at `venue`. Shared by login and claim, which is the point: the two
/// must bind a token to a session in the same way or one of them is the hole.
async fn open_session(
    env: &Env,
    user_id: &str,
    venue: &str,
    preset: Preset,
    now: i64,
) -> Result<Response> {
    let Some(session_id) = crate::edge_id() else {
        return Response::error("no platform CSPRNG", 500);
    };
    let rec = sr::session_record(user_id, venue, preset, now).to_string();
    let (sid, uid, loc) = (session_id.clone(), user_id.to_string(), venue.to_string());
    ids::with_sessions(env, move |t| {
        let index = vec![(sr::ssession_of(&loc, &uid, &sid), sid.clone())];
        t.put(sr::K_SSESSION, &sid, &rec, &index, &[])
            .map_err(|e| Error::RustError(format!("staff session: {e}")))
    })
    .await?;
    let claims = Claims::Staff {
        sub: user_id.to_string(),
        active_location_id: venue.to_string(),
        jti: session_id,
        caps: preset.caps().to_string(),
        iat: now,
        exp: now + sr::STAFF_TTL_MS,
    };
    let jwt = match auth::sign(env, &claims) {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    Response::from_json(&json!({
        "jwt": jwt,
        "staff": { "id": user_id, "locationId": venue, "role": preset.as_str(),
                   "caps": preset.caps().to_string(), "expiresMs": now + sr::STAFF_TTL_MS }
    }))
}

#[derive(Deserialize)]
struct LoginIn {
    email: String,
    password: String,
    #[serde(default)]
    location_id: Option<String>,
}

/// `POST /api/staff/login` — email + password, at the venue the host names.
///
/// THE ROLE IS READ FROM THE ROSTER, never from the request: the membership row
/// at this venue names the preset, and its capabilities are what is signed.
/// A person with no staff word here — a courier, a stranger, the unnamed fourth
/// word — is refused exactly as a wrong password is not: the password was
/// right, and they need the owner, not another try.
pub async fn staff_login(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: LoginIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let ident = ids::identity(&ctx.env).await?;
    let email = body.email.trim().to_lowercase();
    let user = ids::user_id_for_email(&ident, &email)
        .and_then(|id| ids::rec(&ident, ids::K_USER, &id).map(|u| (id, u)));
    let stored = user.as_ref().map(|(_, u)| ids::s_of(u, "password_hash"));
    // Same work on a miss as on a hit.
    if !verify_password_constant_work(&body.password, stored.as_deref()) {
        return Response::error("invalid credentials", 401);
    }
    let (user_id, _) = user.expect("verified above");
    let host = host_venue(&req, &ctx).await?;
    let venue = match crate::accounts::venue_for_login(host.as_deref(), body.location_id.as_deref()) {
        Ok(Some(v)) => v,
        Ok(None) => return Response::error("sign in at your venue's own address", 400),
        Err(_) => return Response::error("that location is not this venue", 403),
    };
    let Some(m) = ids::membership(&ident, &venue, &user_id) else {
        return Response::error("not staff at this venue", 403);
    };
    let Some(preset) = Preset::from_str(&ids::s_of(&m, "role")) else {
        return Response::error("your role here has no staff capabilities", 403);
    };
    open_session(&ctx.env, &user_id, &venue, preset, ctx.data.now_ms).await
}

#[derive(Deserialize)]
struct ClaimIn {
    email: String,
    code: String,
    password: String,
}

/// `POST /api/staff/claim` — turn an invite into a membership, and sign in.
///
/// PUBLIC BY NECESSITY, like the courier's: the code stands in for credentials
/// and is compared as a digest. AN ADDRESS THAT ALREADY HAS AN ACCOUNT — an
/// owner of another venue, say — keeps its password: the claim must present it,
/// or the code would be a way to take over somebody else's account by being
/// invited under their address.
pub async fn staff_claim(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: ClaimIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    if body.password.chars().count() < sr::MIN_PASSWORD_CHARS {
        return Response::error("choose a password of at least 8 characters", 400);
    }
    let now = ctx.data.now_ms;
    let email = body.email.trim().to_ascii_lowercase();
    let code_hash = auth::sha256_hex(body.code.trim());
    let ident = ids::identity(&ctx.env).await?;
    let inv = ident
        .lookup(&sr::sinvite_by_email(&email))
        .and_then(|id| ids::rec(&ident, sr::K_SINVITE, &id).map(|r| (id, r)));
    if let Err(why) = sr::claimable(inv.as_ref().map(|(_, r)| r), &code_hash, now) {
        return Response::error(why, 400);
    }
    let (inv_id, inv) = inv.expect("claimable above");
    let Some(preset) = sr::invitable(&ids::s_of(&inv, "role")).ok() else {
        return Response::error("that invite names no staff role", 400);
    };
    let venue = ids::s_of(&inv, "location_id");
    let existing = ids::user_id_for_email(&ident, &email)
        .and_then(|id| ids::rec(&ident, ids::K_USER, &id).map(|u| (id, u)));
    // AN EXISTING ACCOUNT KEEPS ITS PASSWORD, checked with the same work as a
    // login; a new one gets the hash of the password chosen here.
    let fresh_hash = match &existing {
        Some((_, u)) => {
            if !verify_password_constant_work(&body.password, Some(&ids::s_of(u, "password_hash"))) {
                return Response::error("this address already has an account: use its password", 401);
            }
            None
        }
        None => match auth::hash_password(&body.password) {
            Ok(h) => Some(h),
            Err(e) => return e.into_response(),
        },
    };
    let Some(new_id) = crate::edge_id() else {
        return Response::error("no platform CSPRNG", 500);
    };
    let user_id = existing.as_ref().map(|(id, _)| id.clone()).unwrap_or(new_id);
    let name = ids::s_of(&inv, "invited_name");
    let (uid, em, loc, iid, ch) = (user_id.clone(), email.clone(), venue.clone(), inv_id.clone(), code_hash.clone());
    // ── THE ACCOUNT, THE MEMBERSHIP AND THE SPENT INVITE, IN ONE WRITE ──
    // The invite is re-checked INSIDE the turn: two claims of one code racing
    // must not both succeed.
    let claimed = ids::with_identity(&ctx.env, move |t| {
        let live = ids::rec(t, sr::K_SINVITE, &iid);
        if sr::claimable(live.as_ref(), &ch, now).is_err() {
            return Ok(false);
        }
        if let Some(h) = &fresh_hash {
            let rec = json!({ "id": uid, "email": em, "display_name": name,
                              "password_hash": h, "created_at_ms": now }).to_string();
            t.put(ids::K_USER, &uid, &rec, &[(ids::user_by_email(&em), uid.clone())], &[&ids::user_by_email(&em)])
                .map_err(|e| Error::RustError(format!("user: {e}")))?;
        }
        // AN OWNER'S ROW IS NEVER OVERWRITTEN by a staff word: an owner invited
        // as kitchen at their own venue stays the owner.
        let own = ids::membership(t, &loc, &uid).is_some_and(|m| ids::s_of(&m, "role") == "owner");
        if !own {
            let m = sr::member_record(&uid, &loc, preset, now).to_string();
            t.put(ids::K_MEMBER, &ids::member_id(&loc, &uid), &m,
                  &[(ids::member_by_venue(&loc, &uid), uid.clone()), (ids::member_by_user(&uid, &loc), loc.clone())], &[])
                .map_err(|e| Error::RustError(format!("member: {e}")))?;
        }
        if let Some(mut i) = live {
            i["used_at_ms"] = json!(now);
            i["used_by_user_id"] = json!(uid);
            // The address index goes: a spent invite is not findable again.
            t.put(sr::K_SINVITE, &iid, &i.to_string(), &[(sr::sinvite_at(&loc, &iid), iid.clone())], &[])
                .map_err(|e| Error::RustError(format!("invite: {e}")))?;
        }
        Ok(true)
    })
    .await?;
    if !claimed {
        return Response::error("that code does not match", 400);
    }
    open_session(&ctx.env, &user_id, &venue, preset, now).await
}
