//! Keys a venue makes for its own tools.
//!
//! A KEY IS SHOWN ONCE AND STORED HASHED, exactly like a password, because
//! that is what it is. A copied image must not hand over working credentials.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::owner::{now_ms, owner_and_venue};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyIn {
    label: String,
}

/// `POST /api/owner/apikeys` — mint one, shown ONCE.
///
/// A YEAR, because the thing holding it is a script on somebody's machine and a
/// credential that expires in an hour is one that gets replaced by a password
/// in a config file. Revocable individually: an owner who suspects one key
/// should not have to invalidate the rest.
pub async fn create_api_key(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    const YEAR_MS: i64 = 365 * 24 * 60 * 60 * 1000;

    let body: KeyIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let (owner, loc) = match owner_and_venue(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let label = body.label.trim().to_string();
    if label.is_empty() || label.chars().count() > 80 {
        // A key with no label is a key nobody can decide about later. The list
        // is read months after the keys were made.
        return Response::error("say what this key is for", 400);
    }
    let (Some(id), Some(secret)) = (crate::edge_id(), crate::edge_id()) else {
        return Response::error("no platform CSPRNG", 500);
    };
    let secret = secret.replace('-', "");
    // Ours, not a person's: 122 bits from the platform CSPRNG. See
    // `auth::hash_opaque` for why argon2 would be the wrong primitive here and
    // what it cost when it was used for the session secrets.
    let hash = crate::auth::hash_opaque(&secret);
    let now = now_ms();
    let (kid, l2, own, lab, h2) = (id.clone(), loc.clone(), owner.clone(), label.clone(), hash);
    crate::identity_store::with_sessions(&ctx.env, move |t| {
        let rec = serde_json::json!({
            "id": kid, "location_id": l2, "owner_id": own, "label": lab, "key_hash": h2,
            "created_at_ms": now, "expires_at_ms": now + YEAR_MS,
            "last_used_ms": serde_json::Value::Null,
            "revoked_at_ms": serde_json::Value::Null,
        })
        .to_string();
        t.put(
            crate::identity_store::K_APIKEY,
            &kid,
            &rec,
            &[(crate::identity_store::apikey_at(&l2, &kid), kid.clone())],
            &[],
        )
        .map_err(|e| Error::RustError(format!("api key: {e}")))
    })
    .await?;
    // Shown once. The hub stores a hash and genuinely cannot show it again,
    // which the console says rather than letting the owner assume otherwise.
    Response::from_json(&json!({
        "key": format!("dowiz_{id}.{secret}"),
        "id": id, "label": label, "expiresMs": now + YEAR_MS,
    }))
}

/// `GET /api/owner/apikeys` — which keys exist, and whether anything uses them.
pub async fn list_api_keys(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    struct K {
        id: String,
        label: String,
        created_at_ms: i64,
        expires_at_ms: i64,
        last_used_ms: Option<i64>,
    }
    // THIS VENUE'S KEYS, from the prefix that is the venue's list. Sorted
    // newest first here rather than by a second index: a venue has a handful
    // of keys, and an index maintained on every use to save one sort of five
    // records is a key that exists to be forgotten.
    let sess = crate::identity_store::sessions(&ctx.env).await?;
    let mut rows: Vec<K> = sess
        .scan(&format!("apikey.loc/{loc}/"))
        .into_iter()
        .filter_map(|(_, id)| {
            let r = crate::identity_store::rec(&sess, crate::identity_store::K_APIKEY, &id)?;
            if r.get("revoked_at_ms").map_or(false, |v| !v.is_null()) {
                return None;
            }
            Some(K {
                label: crate::identity_store::s_of(&r, "label"),
                created_at_ms: crate::identity_store::i_of(&r, "created_at_ms"),
                expires_at_ms: crate::identity_store::i_of(&r, "expires_at_ms"),
                last_used_ms: r.get("last_used_ms").and_then(Value::as_i64),
                id,
            })
        })
        .collect();
    rows.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
    Response::from_json(&json!({
        "keys": rows.iter().map(|k| json!({
            "id": k.id, "label": k.label, "createdMs": k.created_at_ms,
            "expiresMs": k.expires_at_ms, "lastUsedMs": k.last_used_ms,
        })).collect::<Vec<_>>(),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevokeIn {
    id: String,
}

/// `POST /api/owner/apikeys/revoke`
pub async fn revoke_api_key(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: RevokeIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // Revoked, not deleted: the row is the record that this key existed and
    // when it stopped, which is the question asked after an incident.
    let (kid, l2, now) = (body.id.clone(), loc.clone(), now_ms());
    let revoked = crate::identity_store::with_sessions(&ctx.env, move |t| {
        let Some(mut r) = crate::identity_store::rec(t, crate::identity_store::K_APIKEY, &kid)
        else {
            return Ok(false);
        };
        // The venue is checked against the RECORD, and the answer to a key of
        // another restaurant is the same 404 as to one that does not exist: a
        // 403 would confirm it does.
        if crate::identity_store::s_of(&r, "location_id") != l2
            || r.get("revoked_at_ms").map_or(false, |v| !v.is_null())
        {
            return Ok(false);
        }
        r["revoked_at_ms"] = serde_json::json!(now);
        let index = vec![(crate::identity_store::apikey_at(&l2, &kid), kid.clone())];
        t.put(crate::identity_store::K_APIKEY, &kid, &r.to_string(), &index, &[])
            .map_err(|e| Error::RustError(format!("api key: {e}")))?;
        Ok(true)
    })
    .await?;
    if !revoked {
        return Response::error("not found", 404);
    }
    Response::from_json(&json!({ "ok": true }))
}
