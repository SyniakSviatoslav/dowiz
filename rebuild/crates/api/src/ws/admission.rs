//! WS admission — token extraction + the per-connection [`Principal`] build (proposal §4, Q1 🔴).
//!
//! **Reuses the S2 verifier, never mints a second one.** A browser cannot set an `Authorization`
//! header on a WS handshake, so the TRANSPORT differs from REST (`crate::auth::extractors`'s
//! `FromRequestParts` family reads a header), but the crypto is identical: this module calls the
//! exact same `AuthState.verifier.verify(&token)` the `VerifiedClaims` extractor uses
//! (`extractors.rs:87-90`).
//!
//! ## Q-WS-COURIER-SESSION / the admission half of REV-S6-2 (🔴)
//! A courier WS upgrade on the Node stack ran `verifyAuthToken` (crypto only) — never the REV-1
//! `courier_sessions` liveness check every REST courier request runs. [`build_principal`] closes
//! that gap by reusing the S2 `CourierSession` extractor's OWN logic (`AuthRepo::courier_session_bind`
//! together with `auth::service::courier_session_valid`, verbatim — no new session policy) so a
//! courier WS admission IS a live-session check, parity with REST. The mid-stream half of REV-S6-2
//! (a session revoked AFTER admission) is [`super::guard::CourierRelayGuard`]'s job, not this
//! module's — see that module's doc for the "does deactivation reset the binding?" determination.
//!
//! ## Token transport precedence (Q1 🔴)
//! 1. `Sec-WebSocket-Protocol: bearer.v1, <jwt>` — primary. The token never touches `req.url` (closes
//!    the JWT-in-URL residual, ledger #42 P1 continuation).
//! 2. `?token=` query — flagged (`WS_URL_TOKEN_ACCEPT`, default ON for the cutover dual-accept
//!    window, `main.rs` reads it), for cached PWA/SW clients that still set the URL param.
//! 3. In-band `ClientMsg::Auth{token}` — the 5s-deadline fallback (`ws::mod`), unconditionally kept.
//!
//! `sec-websocket-protocol` and `?token=` are BOTH kept out of logs by the scoped `TraceLayer` on
//! the `/ws` route (`ws::mod::ws_router` + `ws::mod::redact_ws_uri`, ledger #42's `redactUrlSecrets`
//! equivalent, addendum guardrail 1): its custom `make_span_with` records a token-REDACTED uri and
//! never records a request header, so neither transport reaches a span. That layer is wired in
//! `ws::mod`, not here — this module only ever sees an already-extracted token, and never logs it.

use axum::http::HeaderMap;
use uuid::Uuid;

use crate::auth::claims::Claims;
use crate::auth::mount::AuthState;
use crate::auth::service::{self, CourierBind, CourierSessionRow};

/// The per-connection principal, pinned ONCE at admission (proposal §4's type-state). No
/// `Principal::Channel` variant: heads have no WS runtime in v1 (Q11) — the seam is `Room`'s own
/// exhaustiveness (a channel scope has no room predicate at all), not a fourth principal case here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Principal {
    Owner {
        user_id: Uuid,
    },
    Courier {
        sub: Uuid,
        active_location_id: Uuid,
        jti: Option<Uuid>,
    },
    Customer {
        order_id: Uuid,
        location_id: Uuid,
        sub: Uuid,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionError {
    /// Crypto verify failed (missing/expired/tampered/unknown-kid) — `plugins/auth.ts:54` parity.
    InvalidToken,
    /// REV-1/REV-S6-2: the courier's session is absent/revoked/expired/location-removed. A live
    /// crypto-valid JWT is not enough — parity with every REST courier request.
    SessionRevoked,
    /// The live-session repo call itself failed (DB blip) — distinct from a real revocation so the
    /// caller can choose a retryable close code rather than a hard 401-equivalent.
    RepoUnavailable,
}

/// Verify `token` and build the [`Principal`]. For a courier claim, ALSO runs the REV-1 live-bind
/// check (reusing `AuthState.repo`/`auth::service` verbatim) — see module doc.
pub async fn build_principal(auth: &AuthState, token: &str) -> Result<Principal, AdmissionError> {
    let claims = auth
        .verifier
        .verify(token)
        .map_err(|_e| AdmissionError::InvalidToken)?;
    match claims {
        Claims::Owner(o) => Ok(Principal::Owner { user_id: o.user_id }),
        Claims::Customer(c) => Ok(Principal::Customer {
            order_id: c.order_id,
            location_id: c.location_id,
            sub: c.sub,
        }),
        Claims::Courier(c) => {
            match c.jti {
                None => {
                    // No jti: only a dev-mock-minted token has none. Same gate the S2
                    // `CourierSession` extractor applies (`extractors.rs:178-183`).
                    if auth.config.dev_login_allowed() {
                        Ok(Principal::Courier {
                            sub: c.sub,
                            active_location_id: c.active_location_id,
                            jti: None,
                        })
                    } else {
                        Err(AdmissionError::InvalidToken)
                    }
                }
                Some(jti) => {
                    let bind = auth
                        .repo
                        .courier_session_bind(jti, c.active_location_id, c.sub)
                        .await
                        .map_err(|_e| AdmissionError::RepoUnavailable)?;
                    let row = bind.map(|b| CourierSessionRow {
                        revoked_at: b.revoked,
                        expired: b.expired,
                        has_location: b.has_location,
                    });
                    match service::courier_session_valid(row.as_ref(), true) {
                        CourierBind::Valid => Ok(Principal::Courier {
                            sub: c.sub,
                            active_location_id: c.active_location_id,
                            jti: Some(jti),
                        }),
                        CourierBind::Rejected | CourierBind::NoJti => {
                            Err(AdmissionError::SessionRevoked)
                        }
                    }
                }
            }
        }
    }
}

/// Parses `Sec-WebSocket-Protocol: bearer.v1, <jwt>` (RFC 6455 §4.2.2 — the header may carry a
/// comma-separated offer list; the server picks one and echoes ONLY `bearer.v1`, never the token,
/// in the upgrade response — that echo happens in `ws::mod`, this function only extracts the
/// offered token half).
pub fn subprotocol_token(headers: &HeaderMap) -> Option<String> {
    let raw = headers
        .get(axum::http::header::SEC_WEBSOCKET_PROTOCOL)?
        .to_str()
        .ok()?;
    let mut parts = raw.split(',').map(str::trim);
    if parts.next()? != "bearer.v1" {
        return None;
    }
    let token = parts.next()?;
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

// `?token=` extraction (Q1's flagged dual-accept transport) is NOT hand-rolled here: `ws::mod`'s
// upgrade handler uses axum's own `Query<T>` extractor (`serde_urlencoded`), which already
// percent-decodes correctly — reinventing that parsing here would be exactly the kind of
// unnecessary duplication the build brief's lazy-senior-dev discipline warns against.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::claims::{CourierClaims, CustomerClaims, OwnerClaims};
    use crate::auth::repo::CourierSessionBindRow;
    use crate::auth::repo::fake::FakeAuthRepo;
    use std::sync::Arc;

    fn headers_with_protocol(value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            axum::http::header::SEC_WEBSOCKET_PROTOCOL,
            value.parse().unwrap(),
        );
        h
    }

    #[test]
    fn subprotocol_token_extracts_the_jwt_after_the_bearer_v1_marker() {
        let headers = headers_with_protocol("bearer.v1, eyJhbGciOiJSUzI1NiJ9.abc.def");
        assert_eq!(
            subprotocol_token(&headers).as_deref(),
            Some("eyJhbGciOiJSUzI1NiJ9.abc.def")
        );
    }

    #[test]
    fn subprotocol_token_rejects_a_missing_bearer_v1_marker() {
        let headers = headers_with_protocol("some-other-protocol");
        assert_eq!(subprotocol_token(&headers), None);
    }

    #[test]
    fn subprotocol_token_absent_header_is_none() {
        assert_eq!(subprotocol_token(&HeaderMap::new()), None);
    }

    // ── build_principal: admission auth (the named DoD test) ──

    #[tokio::test]
    async fn build_principal_owner_token() {
        let auth = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let user_id = Uuid::new_v4();
        let token = auth
            .verifier
            .mint(Claims::Owner(OwnerClaims::new(user_id, None)), 3600)
            .unwrap();
        let principal = build_principal(&auth, &token).await.unwrap();
        assert_eq!(principal, Principal::Owner { user_id });
    }

    #[tokio::test]
    async fn build_principal_customer_token() {
        let auth = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let (sub, order, loc) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let token = auth
            .verifier
            .mint(Claims::Customer(CustomerClaims::new(sub, order, loc)), 3600)
            .unwrap();
        let principal = build_principal(&auth, &token).await.unwrap();
        assert_eq!(
            principal,
            Principal::Customer {
                order_id: order,
                location_id: loc,
                sub
            }
        );
    }

    #[tokio::test]
    async fn build_principal_rejects_a_tampered_token() {
        let auth = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let token = auth
            .verifier
            .mint(Claims::Owner(OwnerClaims::new(Uuid::new_v4(), None)), 3600)
            .unwrap();
        let mut tampered = token.into_bytes();
        let last = tampered.len() - 1;
        tampered[last] ^= 1;
        let tampered = String::from_utf8(tampered).unwrap();
        assert_eq!(
            build_principal(&auth, &tampered).await,
            Err(AdmissionError::InvalidToken)
        );
    }

    /// REV-S6-2 admission half: a courier whose session is LIVE admits.
    #[tokio::test]
    async fn build_principal_courier_with_live_session_admits() {
        let jti = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let sub = Uuid::new_v4();
        let repo = Arc::new(FakeAuthRepo::default().with_courier_bind(
            jti,
            loc,
            sub,
            CourierSessionBindRow {
                revoked: false,
                expired: false,
                has_location: true,
            },
        ));
        let auth = AuthState::test_state(repo);
        let token = auth
            .verifier
            .mint(
                Claims::Courier(CourierClaims::new(sub, loc, Some(jti))),
                24 * 3600,
            )
            .unwrap();
        let principal = build_principal(&auth, &token).await.unwrap();
        assert_eq!(
            principal,
            Principal::Courier {
                sub,
                active_location_id: loc,
                jti: Some(jti)
            }
        );
    }

    /// Q-WS-COURIER-SESSION (the finding this port closes): a crypto-valid but LOGGED-OUT courier
    /// (session revoked) must be denied admission even though the JWT itself hasn't expired.
    #[tokio::test]
    async fn build_principal_denies_a_courier_with_a_revoked_session() {
        let jti = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let sub = Uuid::new_v4();
        let repo = Arc::new(FakeAuthRepo::default().with_courier_bind(
            jti,
            loc,
            sub,
            CourierSessionBindRow {
                revoked: true,
                expired: false,
                has_location: true,
            },
        ));
        let auth = AuthState::test_state(repo);
        let token = auth
            .verifier
            .mint(
                Claims::Courier(CourierClaims::new(sub, loc, Some(jti))),
                24 * 3600,
            )
            .unwrap();
        assert_eq!(
            build_principal(&auth, &token).await,
            Err(AdmissionError::SessionRevoked)
        );
    }

    #[tokio::test]
    async fn build_principal_denies_a_courier_removed_from_the_location() {
        let jti = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let sub = Uuid::new_v4();
        let repo = Arc::new(FakeAuthRepo::default().with_courier_bind(
            jti,
            loc,
            sub,
            CourierSessionBindRow {
                revoked: false,
                expired: false,
                has_location: false,
            },
        ));
        let auth = AuthState::test_state(repo);
        let token = auth
            .verifier
            .mint(
                Claims::Courier(CourierClaims::new(sub, loc, Some(jti))),
                24 * 3600,
            )
            .unwrap();
        assert_eq!(
            build_principal(&auth, &token).await,
            Err(AdmissionError::SessionRevoked)
        );
    }

    #[tokio::test]
    async fn build_principal_denies_a_courier_with_no_session_row_at_all() {
        let auth = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let token = auth
            .verifier
            .mint(
                Claims::Courier(CourierClaims::new(
                    Uuid::new_v4(),
                    Uuid::new_v4(),
                    Some(Uuid::new_v4()),
                )),
                24 * 3600,
            )
            .unwrap();
        assert_eq!(
            build_principal(&auth, &token).await,
            Err(AdmissionError::SessionRevoked)
        );
    }
}
