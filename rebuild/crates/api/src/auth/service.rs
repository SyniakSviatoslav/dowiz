//! Pure, DB/framework-free auth decision logic — the ported equivalents of the branchy
//! disposition code inside the Node auth route handlers, lifted out so the *decisions* are
//! unit-testable without a live Postgres (the same posture as `crate::service` for S1).
//!
//! The route handlers (in `crate::routes::auth_*`) do the IO (repo calls + mint), then hand the
//! query results to these functions to decide the outcome, then act on the returned disposition.

use uuid::Uuid;

use super::claims::CustomerClaims;

// ── Owner refresh rotation disposition (ADR-0004, auth.ts:235-318) ──────────────────────────

/// The five outcomes of an owner refresh, decided from the token-row lookup + the atomic claim
/// result + the recent-rotation probe + the live owner memberships. Mirrors `auth.ts:251-307`
/// branch-for-branch (proposal §6). The route performs the guarded UPDATE / DELETE / SELECTs and
/// feeds their results here; this function contains ZERO IO so every branch is a direct test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnerRefreshDisposition {
    /// Token unknown or past its `expires_at` → 401 UNAUTHORIZED "Invalid refresh token".
    Invalid,
    /// Lost the atomic claim, but a family token was created `< 5s` ago → benign concurrent
    /// refresh. Soft 409 `{error:'concurrent_refresh'}`; family NOT revoked (auth.ts:276-282).
    ConcurrentRefresh,
    /// Lost the claim, no recent rotation → genuine replay. DELETE the whole family, 401
    /// "Token reuse detected. Family revoked." (auth.ts:283-286).
    ReuseRevokeFamily,
    /// Claimed, but no active owner membership → 401 OWNER_REVOKED (auth.ts:293-301); the token
    /// is already consumed by the winning claim.
    OwnerRevoked,
    /// Claimed + active owner → mint a new 24h owner access + a new 7d refresh in the SAME family.
    /// Carries the deterministically-picked `active_location_id` (auth.ts:302-307).
    Rotate { active_location_id: Uuid },
}

/// The inputs the route gathers before deciding. `token_found`/`expired` come from the initial
/// lookup; `claimed` is `rowCount == 1` from the atomic `UPDATE ... WHERE used=false`;
/// `recent_family_rotation` is the `< interval '5 seconds'` probe (SQL is authority — 5s, not the
/// stale "10s" comment, Q-5S-COMMENT); `active_owner_locations` is the live P-c membership read.
pub struct OwnerRefreshInputs<'a> {
    pub token_found: bool,
    pub token_expired: bool,
    pub claimed: bool,
    pub recent_family_rotation: bool,
    /// Active owner-membership location ids, ordered `created_at, location_id` (auth.ts:296) —
    /// the FIRST is the stable deterministic pick, never a tiebreaker-less LIMIT 1.
    pub active_owner_locations: &'a [Uuid],
    /// The caller's requested working tenant (`active_location_id` body field), preserved iff it
    /// is still one of their active owner memberships (R2-2).
    pub requested_location: Option<Uuid>,
}

pub fn owner_refresh_disposition(input: &OwnerRefreshInputs<'_>) -> OwnerRefreshDisposition {
    if !input.token_found || input.token_expired {
        return OwnerRefreshDisposition::Invalid;
    }
    if !input.claimed {
        return if input.recent_family_rotation {
            OwnerRefreshDisposition::ConcurrentRefresh
        } else {
            OwnerRefreshDisposition::ReuseRevokeFamily
        };
    }
    // Claimed (won the atomic single-use). P-c: re-derive owner authority live.
    let Some(&first) = input.active_owner_locations.first() else {
        return OwnerRefreshDisposition::OwnerRevoked;
    };
    // Preserve the requested tenant iff still an active owner membership, else the stable first pick.
    let active_location_id = match input.requested_location {
        Some(req) if input.active_owner_locations.contains(&req) => req,
        _ => first,
    };
    OwnerRefreshDisposition::Rotate { active_location_id }
}

// ── Courier session live-bind (REV-1, plugins/auth.ts:24-30,63-92) ───────────────────────────

/// A `courier_sessions` row projected for the per-request live-bind check, plus the
/// `has_location` EXISTS flag (the courier still holds membership in the token's
/// `activeLocationId`). Mirrors `CourierSessionRow` (`plugins/auth.ts:10-15`).
#[derive(Debug, Clone)]
pub struct CourierSessionRow {
    pub revoked_at: bool,
    /// True iff `expires_at` is in the past.
    pub expired: bool,
    /// REV-1: the courier STILL holds membership in the token's `activeLocationId`.
    pub has_location: bool,
}

/// Decide whether a courier access token is still live (`courierSessionValid`, plugins/auth.ts:24-30).
/// A signed JWT alone is not enough: the session row must be present, not revoked, not expired,
/// AND `has_location` must still be true. REV-1's whole point — dropping `has_location` silently
/// regresses per-location revocation from ~1 request to the full 14d/24h TTL.
pub fn courier_session_valid(row: Option<&CourierSessionRow>, has_jti: bool) -> CourierBind {
    // A courier token with NO jti can only come from the dev-mock minter (no real session row).
    // The extractor decides whether to allow that (dev-gated); this function reports it distinctly.
    if !has_jti {
        return CourierBind::NoJti;
    }
    match row {
        None => CourierBind::Rejected,
        Some(r) if r.revoked_at || r.expired || !r.has_location => CourierBind::Rejected,
        Some(_) => CourierBind::Valid,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourierBind {
    /// Session present, live, not revoked, and courier still holds the location.
    Valid,
    /// Missing / revoked / expired / location-removed → 401 (REV-1).
    Rejected,
    /// Token carries no jti — only a dev-mock token does; the extractor gates this on the dev flag.
    NoJti,
}

// ── Customer order-scope (REV-3 / T-12, unify to the minted tuple) ──────────────────────────

/// REV-3/T-12: a customer token's authority is the minted `(orderId, locationId, sub)` tuple,
/// NOT customer-wide. A token minted for order A must NOT authorize an action on order B. The
/// live Node bug (`customer/orders.ts:50` binds `customer_id = sub`, ignoring the claim) is
/// FIXED-IN-PORT here: every customer-scoped action checks the claim's `order_id` against the
/// target order id. Returns `true` iff authorized.
pub fn customer_authorized_for_order(claims: &CustomerClaims, target_order_id: Uuid) -> bool {
    claims.order_id == target_order_id
}

/// REV-3 companion: a customer token is also location-scoped to its minted `locationId`.
pub fn customer_authorized_for_location(claims: &CustomerClaims, target_location_id: Uuid) -> bool {
    claims.location_id == target_location_id
}

// ── Claim surface error mapping (public/claim.ts:30-38) ──────────────────────────────────────

/// The `ClaimError.code` → HTTP-status mapping for `/api/claim/accept` (claim.ts:32-37). Bare
/// `{error: CODE}` shape (ClaimBareError, Q-CLAIM-BARE) — the status is decided here, the body
/// is always `{error: code}`.
pub fn claim_accept_status(code: &str) -> u16 {
    match code {
        "ALREADY_CLAIMED" => 409,
        "INVALID_OR_EXPIRED_TOKEN" => 401,
        // authenticated, but not the invited identity / token-only invite not web-claimable
        "CONTACT_MISMATCH" | "CONTACT_REQUIRED" => 403,
        _ => 422,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc(n: u8) -> Uuid {
        Uuid::from_u128(u128::from(n))
    }

    #[test]
    fn refresh_unknown_or_expired_token_is_invalid() {
        let base = OwnerRefreshInputs {
            token_found: false,
            token_expired: false,
            claimed: false,
            recent_family_rotation: false,
            active_owner_locations: &[],
            requested_location: None,
        };
        assert_eq!(
            owner_refresh_disposition(&base),
            OwnerRefreshDisposition::Invalid
        );
        let expired = OwnerRefreshInputs {
            token_found: true,
            token_expired: true,
            ..base
        };
        assert_eq!(
            owner_refresh_disposition(&expired),
            OwnerRefreshDisposition::Invalid
        );
    }

    #[test]
    fn refresh_lost_claim_with_recent_rotation_is_concurrent_not_revoke() {
        // T-2: the benign two-tab race must stay 409, family intact — never mis-revoke.
        let input = OwnerRefreshInputs {
            token_found: true,
            token_expired: false,
            claimed: false,
            recent_family_rotation: true,
            active_owner_locations: &[loc(1)],
            requested_location: None,
        };
        assert_eq!(
            owner_refresh_disposition(&input),
            OwnerRefreshDisposition::ConcurrentRefresh
        );
    }

    #[test]
    fn refresh_lost_claim_no_recent_rotation_revokes_family() {
        // T-1: genuine replay of a stale token → revoke the whole family.
        let input = OwnerRefreshInputs {
            token_found: true,
            token_expired: false,
            claimed: false,
            recent_family_rotation: false,
            active_owner_locations: &[loc(1)],
            requested_location: None,
        };
        assert_eq!(
            owner_refresh_disposition(&input),
            OwnerRefreshDisposition::ReuseRevokeFamily
        );
    }

    #[test]
    fn refresh_claimed_but_no_owner_membership_is_owner_revoked() {
        // T-4: a demoted owner must not roll forward.
        let input = OwnerRefreshInputs {
            token_found: true,
            token_expired: false,
            claimed: true,
            recent_family_rotation: false,
            active_owner_locations: &[],
            requested_location: Some(loc(9)),
        };
        assert_eq!(
            owner_refresh_disposition(&input),
            OwnerRefreshDisposition::OwnerRevoked
        );
    }

    #[test]
    fn refresh_preserves_requested_location_when_still_a_membership() {
        let input = OwnerRefreshInputs {
            token_found: true,
            token_expired: false,
            claimed: true,
            recent_family_rotation: false,
            active_owner_locations: &[loc(1), loc(2), loc(3)],
            requested_location: Some(loc(2)),
        };
        assert_eq!(
            owner_refresh_disposition(&input),
            OwnerRefreshDisposition::Rotate {
                active_location_id: loc(2)
            }
        );
    }

    #[test]
    fn refresh_falls_back_to_stable_first_pick_when_requested_not_a_membership() {
        // Never the tiebreaker-less LIMIT 1 — the FIRST of the ordered list (auth.ts:296,306).
        let input = OwnerRefreshInputs {
            token_found: true,
            token_expired: false,
            claimed: true,
            recent_family_rotation: false,
            active_owner_locations: &[loc(1), loc(2)],
            requested_location: Some(loc(99)),
        };
        assert_eq!(
            owner_refresh_disposition(&input),
            OwnerRefreshDisposition::Rotate {
                active_location_id: loc(1)
            }
        );
    }

    #[test]
    fn courier_bind_rejects_when_location_removed() {
        // REV-1: the whole point — has_location=false → Rejected (per-location revoke is immediate).
        let removed = CourierSessionRow {
            revoked_at: false,
            expired: false,
            has_location: false,
        };
        assert_eq!(
            courier_session_valid(Some(&removed), true),
            CourierBind::Rejected
        );
    }

    #[test]
    fn courier_bind_valid_when_present_live_and_holds_location() {
        let ok = CourierSessionRow {
            revoked_at: false,
            expired: false,
            has_location: true,
        };
        assert_eq!(courier_session_valid(Some(&ok), true), CourierBind::Valid);
    }

    #[test]
    fn courier_bind_rejects_revoked_or_expired_or_missing() {
        assert_eq!(courier_session_valid(None, true), CourierBind::Rejected);
        let revoked = CourierSessionRow {
            revoked_at: true,
            expired: false,
            has_location: true,
        };
        assert_eq!(
            courier_session_valid(Some(&revoked), true),
            CourierBind::Rejected
        );
        let expired = CourierSessionRow {
            revoked_at: false,
            expired: true,
            has_location: true,
        };
        assert_eq!(
            courier_session_valid(Some(&expired), true),
            CourierBind::Rejected
        );
    }

    #[test]
    fn courier_bind_no_jti_is_distinct() {
        assert_eq!(courier_session_valid(None, false), CourierBind::NoJti);
    }

    #[test]
    fn customer_scope_denies_cross_order() {
        // REV-3/T-12: a token minted for order A must NOT authorize order B.
        let order_a = Uuid::from_u128(0xA);
        let order_b = Uuid::from_u128(0xB);
        let claims = CustomerClaims::new(Uuid::new_v4(), order_a, Uuid::new_v4());
        assert!(customer_authorized_for_order(&claims, order_a));
        assert!(
            !customer_authorized_for_order(&claims, order_b),
            "token(orderA) must be denied on orderB (403)"
        );
    }

    #[test]
    fn customer_scope_denies_cross_location() {
        let loc_a = Uuid::from_u128(0xA);
        let loc_b = Uuid::from_u128(0xB);
        let claims = CustomerClaims::new(Uuid::new_v4(), Uuid::new_v4(), loc_a);
        assert!(customer_authorized_for_location(&claims, loc_a));
        assert!(!customer_authorized_for_location(&claims, loc_b));
    }

    #[test]
    fn claim_accept_status_mapping() {
        assert_eq!(claim_accept_status("ALREADY_CLAIMED"), 409);
        assert_eq!(claim_accept_status("INVALID_OR_EXPIRED_TOKEN"), 401);
        assert_eq!(claim_accept_status("CONTACT_MISMATCH"), 403);
        assert_eq!(claim_accept_status("CONTACT_REQUIRED"), 403);
        assert_eq!(claim_accept_status("NOT_CLAIMABLE"), 422);
        assert_eq!(claim_accept_status("RACED"), 422);
    }
}
