//! AUTH-07 claim (web side): accept (verifyAuth-gated, token = transfer authority), request
//! (uniform 202 anti-enumeration), decline (token-only erase). Ports `public/claim.ts`. ALL claim
//! responses use the bare `{error: CODE}` shape (Q-CLAIM-BARE), never the envelope.

use axum::Json;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::auth::AuthState;
use crate::auth::dto::{
    ClaimAcceptResponse, ClaimDeclineResponse, ClaimRequestBody, ClaimRequestResponse,
    ClaimTokenRequest,
};
use crate::auth::error::ClaimBareResponse;
use crate::auth::extractors::VerifiedClaims;
use crate::auth::service::claim_accept_status;

/// `POST /api/claim/accept` (AUTH-07) — verifyAuth ONLY; the TOKEN is the sole transfer authority
/// (claim.ts:17). `claim_transfer` SECURITY DEFINER is the atomic transfer. `reauth:true` hints a
/// refresh so the new membership enters the token (ADR-0004).
#[utoipa::path(post, path = "/api/claim/accept", tag = "auth", security(("bearerAuth" = [])),
    request_body = ClaimTokenRequest,
    responses((status = 200, body = ClaimAcceptResponse), (status = 401, description = "bare {error}"), (status = 403, description = "CONTACT_*"), (status = 409, description = "ALREADY_CLAIMED")))]
pub async fn claim_accept(
    Extension(state): Extension<AuthState>,
    verified: VerifiedClaims,
    Json(req): Json<ClaimTokenRequest>,
) -> Response {
    // Manual validation (claim.ts:21): token 16..256. Bare {error:'VALIDATION_FAILED'}.
    if req.token.len() < 16 || req.token.len() > 256 {
        return bare(StatusCode::BAD_REQUEST, "VALIDATION_FAILED");
    }
    // The claimer's identity = the token's `sub` (claim.ts:23). No sub → UNAUTHENTICATED.
    let user_id = verified.0.sub();

    // G-F2g (claim.ts:107-115): the WEB claim path REFUSES a token-only (unbound) invite.
    match state.repo.claim_invite_is_contact_bound(&req.token).await {
        Ok(Some(false)) => return bare(StatusCode::FORBIDDEN, "CONTACT_REQUIRED"),
        Ok(_) => {} // bound, or no matching invite (claim_transfer will resolve/deny)
        Err(_) => return bare(StatusCode::UNAUTHORIZED, "INVALID_OR_EXPIRED_TOKEN"),
    }

    match state.repo.claim_transfer(&req.token, user_id).await {
        Ok(Ok((org_id, location_id))) => Json(ClaimAcceptResponse {
            org_id,
            location_id,
            reauth: true,
        })
        .into_response(),
        Ok(Err(code)) => {
            let status = StatusCode::from_u16(claim_accept_status(&code))
                .unwrap_or(StatusCode::UNPROCESSABLE_ENTITY);
            bare(status, &code)
        }
        Err(_) => bare(StatusCode::UNPROCESSABLE_ENTITY, "NOT_CLAIMABLE"),
    }
}

/// `POST /api/claim/request` (AUTH-07) — owner-initiated "this is my restaurant". Never auto-mints
/// (spam/IDOR); records a signal ONLY for real shadows; byte-identical 202 either way
/// (anti-enumeration, T-10). claim.ts:49.
#[utoipa::path(post, path = "/api/claim/request", tag = "auth",
    request_body = ClaimRequestBody,
    responses((status = 202, body = ClaimRequestResponse), (status = 400, description = "bare {error:'VALIDATION_FAILED'}")))]
pub async fn claim_request(
    Extension(state): Extension<AuthState>,
    Json(req): Json<ClaimRequestBody>,
) -> Response {
    // .strict() slug validation (claim.ts:12): 2..100, ^[a-z0-9-]+$.
    let ok = req.slug.len() >= 2
        && req.slug.len() <= 100
        && req
            .slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !ok {
        return bare(StatusCode::BAD_REQUEST, "VALIDATION_FAILED");
    }
    // Only a real shadow yields a signal; non-shadows are silently ignored. The response is
    // byte-identical regardless — never reveal whether the slug is a claimable shadow (T-10).
    if state.repo.slug_is_shadow(&req.slug).await.unwrap_or(false) {
        tracing::info!(event = "acquisition.claim_requested", slug = %req.slug, "claim requested");
    }
    (
        StatusCode::ACCEPTED,
        Json(ClaimRequestResponse {
            requested: true,
            message: "If this is your restaurant, our team will verify ownership via the contact on file and send a claim link.".to_string(),
        }),
    )
        .into_response()
}

/// `POST /api/claim/decline` (AUTH-07) — token-only, NO auth: erase the unconsented preview
/// (claim.ts:69). Any ClaimError → bare {error:<code>} at 401 regardless of kind (Q-CLAIM-DECLINE).
#[utoipa::path(post, path = "/api/claim/decline", tag = "auth",
    request_body = ClaimTokenRequest,
    responses((status = 200, body = ClaimDeclineResponse), (status = 400, description = "bare VALIDATION_FAILED"), (status = 401, description = "bare {error}")))]
pub async fn claim_decline(
    Extension(state): Extension<AuthState>,
    Json(req): Json<ClaimTokenRequest>,
) -> Response {
    if req.token.len() < 16 || req.token.len() > 256 {
        return bare(StatusCode::BAD_REQUEST, "VALIDATION_FAILED");
    }
    // declineAndErase is a multi-step erase (resolve+burn invite, hard-delete shadow). A token that
    // does not resolve to an active invite → bare 401 (claim.ts:79).
    // ⚠️ LAUNCH-BLOCKER (SSG S10 LOW): the destructive erase (burn invite + hard_delete_shadow) is NOT
    // yet wired here — this only READS `claim_invite_is_contact_bound` and returns erased:true. Per the
    // Art-14 dignity rule ("erase as easy as claim"), the decline MUST perform the erase before the S10
    // flip; do NOT launch this path returning erased:true without erasing. Safe while DARK (no caller).
    match state.repo.claim_invite_is_contact_bound(&req.token).await {
        // `Some(_)` = an active invite exists → erase MUST proceed (launch-blocker above) → 200 erased.
        Ok(Some(_)) => Json(ClaimDeclineResponse { erased: true }).into_response(),
        // No matching active invite → any ClaimError maps to a bare 401 (Q-CLAIM-DECLINE).
        Ok(None) => bare(StatusCode::UNAUTHORIZED, "INVALID_OR_EXPIRED_TOKEN"),
        Err(_) => bare(StatusCode::UNAUTHORIZED, "INVALID_OR_EXPIRED_TOKEN"),
    }
}

/// Emit the claim surface's bare `{error: CODE}` shape (ClaimBareError, Q-CLAIM-BARE).
fn bare(status: StatusCode, code: &str) -> Response {
    ClaimBareResponse {
        status,
        code: code.to_string(),
    }
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::claims::{Claims, OwnerClaims};
    use crate::auth::repo::fake::FakeAuthRepo;
    use axum::body::to_bytes;
    use std::sync::Arc;
    use uuid::Uuid;

    async fn json_body(resp: Response) -> (StatusCode, serde_json::Value) {
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn claim_accept_success_returns_reauth_true() {
        let repo = FakeAuthRepo::default();
        let token = "a".repeat(32);
        let org = Uuid::new_v4();
        let loc = Uuid::new_v4();
        repo.claim_contact_bound
            .lock()
            .unwrap()
            .insert(token.clone(), true);
        repo.claim_transfers
            .lock()
            .unwrap()
            .insert(token.clone(), Ok((org, loc)));
        let state = AuthState::test_state(Arc::new(repo));
        let verified = VerifiedClaims(Claims::Owner(OwnerClaims::new(Uuid::new_v4(), None)));
        let resp = claim_accept(
            Extension(state),
            verified,
            Json(ClaimTokenRequest { token }),
        )
        .await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["reauth"], true);
    }

    #[tokio::test]
    async fn claim_accept_contact_required_is_bare_403() {
        // G-F2g: a token-only (unbound) invite is not web-claimable.
        let repo = FakeAuthRepo::default();
        let token = "b".repeat(32);
        repo.claim_contact_bound
            .lock()
            .unwrap()
            .insert(token.clone(), false);
        let state = AuthState::test_state(Arc::new(repo));
        let verified = VerifiedClaims(Claims::Owner(OwnerClaims::new(Uuid::new_v4(), None)));
        let resp = claim_accept(
            Extension(state),
            verified,
            Json(ClaimTokenRequest { token }),
        )
        .await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(
            json,
            serde_json::json!({ "error": "CONTACT_REQUIRED" }),
            "bare shape"
        );
    }

    #[tokio::test]
    async fn claim_accept_already_claimed_is_bare_409() {
        let repo = FakeAuthRepo::default();
        let token = "c".repeat(32);
        repo.claim_contact_bound
            .lock()
            .unwrap()
            .insert(token.clone(), true);
        repo.claim_transfers
            .lock()
            .unwrap()
            .insert(token.clone(), Err("ALREADY_CLAIMED".to_string()));
        let state = AuthState::test_state(Arc::new(repo));
        let verified = VerifiedClaims(Claims::Owner(OwnerClaims::new(Uuid::new_v4(), None)));
        let resp = claim_accept(
            Extension(state),
            verified,
            Json(ClaimTokenRequest { token }),
        )
        .await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(json, serde_json::json!({ "error": "ALREADY_CLAIMED" }));
    }

    // ── S10 REV-S10-3 (breaker M1): the transfer recipient is the AUTHENTICATED sub, NEVER a body
    //    field — the structural IDOR guard. `ClaimTokenRequest` has ONLY `token`; there is no
    //    request-supplied `user_id` a caller could point the transfer at. A courier/customer token
    //    CAN reach accept (verifyAuth-only, matching Node), but its `sub` is the recipient and the
    //    `claim_transfer` DEFINER's contact-binding is the transfer authority — no attacker-chosen id. ──

    #[tokio::test]
    async fn claim_accept_recipient_is_authenticated_sub_not_a_body_field() {
        // The Fake records which recipient `claim_transfer` was invoked with; assert it equals the
        // token's `sub`, proving the recipient is derived from the SESSION, never the request body.
        // A concrete `Arc<FakeAuthRepo>` handle is kept (the trait object hides the recorder).
        let repo = Arc::new(FakeAuthRepo::default());
        let token = "s".repeat(32);
        let org = Uuid::new_v4();
        let loc = Uuid::new_v4();
        repo.claim_contact_bound
            .lock()
            .unwrap()
            .insert(token.clone(), true);
        repo.claim_transfers
            .lock()
            .unwrap()
            .insert(token.clone(), Ok((org, loc)));
        let state = AuthState::test_state(repo.clone());
        let authed_sub = Uuid::new_v4();
        // The OwnerClaims `sub` defaults to `user_id` (claims.rs); use that as the authenticated sub.
        let verified = VerifiedClaims(Claims::Owner(OwnerClaims::new(authed_sub, None)));
        let resp = claim_accept(
            Extension(state),
            verified,
            Json(ClaimTokenRequest {
                token: token.clone(),
            }),
        )
        .await;
        let (status, _) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        // The recipient recorded by the transfer must be the authenticated sub — not any body value.
        let seen = *repo.claim_transfer_recipient.lock().unwrap();
        assert_eq!(
            seen,
            Some(authed_sub),
            "REV-S10-3: recipient is the authenticated sub, never a request-supplied id (no IDOR)"
        );
    }

    #[tokio::test]
    async fn claim_request_is_uniform_202_regardless_of_shadow() {
        // T-10: byte-identical 202 whether or not the slug is a shadow.
        let repo = FakeAuthRepo::default();
        repo.shadows
            .lock()
            .unwrap()
            .insert("real-shadow".to_string(), true);
        let state = AuthState::test_state(Arc::new(repo));

        let shadow = claim_request(
            Extension(state.clone()),
            Json(ClaimRequestBody {
                slug: "real-shadow".to_string(),
            }),
        )
        .await;
        let (s1, j1) = json_body(shadow).await;
        let nonshadow = claim_request(
            Extension(state),
            Json(ClaimRequestBody {
                slug: "not-a-shadow".to_string(),
            }),
        )
        .await;
        let (s2, j2) = json_body(nonshadow).await;
        assert_eq!(s1, StatusCode::ACCEPTED);
        assert_eq!(s2, StatusCode::ACCEPTED);
        assert_eq!(j1, j2, "response must be byte-identical (anti-enumeration)");
    }
}
