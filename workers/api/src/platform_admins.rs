//! `POST /api/platform/admins` -- THE FIRST PLATFORM ADMINISTRATOR (W-ATOMIC row 4, operator
//! 2026-10-07).
//!
//! Production had no way to create one: `platform_admins` (identity `K_ADMIN`) was written only
//! by the test seeding in `edge/site.rs`, so `/api/platform/*` -- and `tools/deploy/rollback.sh`'s
//! compaction step with it -- had no caller who could pass `platform::admin_only`.
//!
//! GATED EXACTLY AS `bootstrap::seed` IS, and failing closed the same way: no `BOOTSTRAP_SECRET`
//! on the Worker answers 404 (there is no route), a secret under 32 bytes answers 503, and a
//! missing or wrong `x-dowiz-bootstrap` header answers 404 -- never 401, which would confirm
//! there is something here to guess at. The compare is `bootstrap::secret_ok`, the same one.
//!
//! IT CREATES, IT NEVER ADOPTS. An address already held by any user is refused (409) rather than
//! promoted: making an existing owner a platform administrator through a shared secret is
//! exactly the grant this route must not be able to make. The password is hashed with
//! `auth::hash_password`, the same argon2 every owner and staff login verifies against, and it
//! is never logged, echoed or stored in any other form.

use crate::edge::Ctx;
use crate::wire::{Call, Reply};
use serde::Deserialize;
use serde_json::json;
use worker::{Error, Result};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AdminIn {
    email: String,
    password: String,
}

pub(crate) async fn create(mut req: Call, ctx: Ctx<crate::Req>) -> Result<Reply> {
    // No secret configured => the route does not exist (bootstrap.rs `seed`, copied exactly).
    let Ok(want) = ctx.env.secret("BOOTSTRAP_SECRET") else {
        return Reply::error("not found", 404);
    };
    let want = want.to_string();
    if want.len() < 32 {
        return Reply::error("bootstrap secret is too short to be one", 503);
    }
    let given = req.headers().get("x-dowiz-bootstrap").ok().flatten().unwrap_or_default();
    if !crate::bootstrap::secret_ok(&given, &want) {
        return Reply::error("not found", 404);
    }

    let body: AdminIn = match crate::body::strict(&mut req).await {
        Ok(b) => b,
        Err(r) => return Ok(r),
    };
    let email = body.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Reply::error("email is not an address", 400);
    }
    if body.password.chars().count() < crate::services::identity::staff_rules::MIN_PASSWORD_CHARS {
        return Reply::error("password is too short", 400);
    }
    let hash = match crate::auth::hash_password(&body.password) {
        Ok(h) => h,
        Err(e) => return e.into_response(),
    };
    let Some(uid) = crate::edge_id() else {
        return Reply::error("no platform CSPRNG", 500);
    };
    let now = ctx.data.now_ms;
    let (u2, e2) = (uid.clone(), email.clone());
    // ONE TURN: the address is checked, the user written and the mark set together, so two
    // concurrent calls cannot both see the address free.
    let made = crate::identity_store::with_identity(&ctx.env, move |t| {
        if crate::identity_store::user_id_for_email(t, &e2).is_some() {
            return Ok(false);
        }
        let rec = json!({ "id": u2, "email": e2, "display_name": "", "password_hash": hash, "created_at_ms": now }).to_string();
        let by_email = crate::identity_store::user_by_email(&e2);
        t.put(crate::identity_store::K_USER, &u2, &rec, &[(by_email.clone(), u2.clone())], &[&by_email])
            .map_err(|e| Error::RustError(format!("user: {e}")))?;
        let mark = json!({ "user_id": u2, "created_at_ms": now }).to_string();
        t.put(crate::identity_store::K_ADMIN, &u2, &mark, &[], &[]).map_err(|e| Error::RustError(format!("admin: {e}")))?;
        Ok(true)
    })
    .await?;
    if !made {
        return Reply::error("that address already has an account", 409);
    }
    Reply::from_json(&json!({ "ok": true, "userId": uid, "email": email }))
}

#[cfg(test)]
mod tests;
