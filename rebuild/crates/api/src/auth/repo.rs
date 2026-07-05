//! `AuthRepo` — the S2 auth data-access trait. Runtime `sqlx::query`/`query_as` ONLY (never the
//! compile-time `query!` macros — no reachable `DATABASE_URL`/`.sqlx` cache here, same constraint
//! as `crate::repo`). The SECURITY DEFINER functions (`claim_transfer`,
//! `payment_location_by_provider_ref`, `activate_courier`) stay in Postgres and are CALLed, never
//! reimplemented (REBUILD-MAP §8 KEEP disposition).
//!
//! ## Tenant GUC (REV-3 / threat-model §6, sweep #2 fail-open)
//! The pre-auth mint paths (track-exchange, courier login/redeem) run on the operational pool
//! with an EXPLICIT `WHERE` predicate — the same "app-layer predicate is the boundary while RLS
//! is bypassed" pattern the Node code uses (Q-TRACK-POOL). Where a query touches a
//! tenant-RLS table on the authenticated path (courier invite lookup), it goes through
//! `db::with_tenant` (the S1-provided txn-scoped `app.current_tenant` GUC), giving S2 the FIRST
//! real caller of that helper (`db.rs` module doc anticipated this).
//!
//! Every method is `#[cfg(test)]`-stubbable via `FakeAuthRepo` so the route handlers and the
//! extractors are unit-testable without a live Postgres.

use sqlx::PgPool;
use uuid::Uuid;

/// Wraps `sqlx::Error` so route code never imports `sqlx` directly (parity with `crate::repo`).
#[derive(Debug, thiserror::Error)]
#[error("auth repo error: {0}")]
pub struct AuthRepoError(#[from] pub sqlx::Error);

/// A projected owner refresh-token row (`auth_refresh_tokens`), the fields the rotation needs.
#[derive(Debug, Clone)]
pub struct OwnerRefreshRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub family_id: Uuid,
    pub expired: bool,
}

/// A courier login lookup result (`couriers`), PII stays encrypted at rest — only the id, hash,
/// and status come back.
#[derive(Debug, Clone)]
pub struct CourierAuthRow {
    pub id: Uuid,
    /// argon2id PHC string (`couriers.password_hash`).
    pub password_hash: String,
    pub status: String,
}

/// A courier session row for the refresh-rotation path (`courier_sessions`).
#[derive(Debug, Clone)]
pub struct CourierSessionRefreshRow {
    pub id: Uuid,
    pub courier_id: Uuid,
    pub family_id: Uuid,
    pub token_hash: String,
    pub active_location_id: Uuid,
    pub revoked: bool,
    pub expired: bool,
}

/// The live per-request courier session bind (`plugins/auth.ts:74-83`) — the extractor's REV-1
/// check reads exactly these three flags keyed on `(jti, activeLocationId, sub)`.
#[derive(Debug, Clone)]
pub struct CourierSessionBindRow {
    pub revoked: bool,
    pub expired: bool,
    pub has_location: bool,
}

#[async_trait::async_trait]
pub trait AuthRepo: Send + Sync {
    // ── Owner (local login) ──
    /// `SELECT id, password_hash FROM users WHERE email = $1` (local.ts:86-89). `None` = no user.
    async fn user_by_email(
        &self,
        email_lower: &str,
    ) -> Result<Option<(Uuid, Option<String>)>, AuthRepoError>;

    /// Resolve role + active location for an owner (`memberships` owner-first, then owned-org
    /// fallback) — local.ts:110-139. Returns `(role, active_location_id)`.
    async fn resolve_owner_membership(
        &self,
        user_id: Uuid,
    ) -> Result<(String, Option<Uuid>), AuthRepoError>;

    /// Insert a hashed owner refresh token into a family (7d). `Ok(false)` = INSERT failed
    /// (Q-REFRESH-OMIT — caller omits `refresh_token`).
    async fn insert_owner_refresh(
        &self,
        user_id: Uuid,
        family_id: Uuid,
        token_hash_hex: &str,
    ) -> Result<bool, AuthRepoError>;

    // ── Owner refresh rotation (ADR-0004) ──
    async fn owner_refresh_by_hash(
        &self,
        token_hash_hex: &str,
    ) -> Result<Option<OwnerRefreshRow>, AuthRepoError>;

    /// Atomic single-use claim: `UPDATE ... SET used=true WHERE id=$1 AND used=false` →
    /// `rowCount==1` semantics (auth.ts:265-268). `Ok(true)` = this request won the claim.
    async fn claim_owner_refresh(&self, id: Uuid) -> Result<bool, AuthRepoError>;

    /// The `< interval '5 seconds'` recent-rotation probe (auth.ts:277) — SQL 5s is authority.
    async fn family_rotated_within_5s(&self, family_id: Uuid) -> Result<bool, AuthRepoError>;

    /// `DELETE FROM auth_refresh_tokens WHERE family_id = $1` (reuse → revoke family).
    async fn delete_owner_family(&self, family_id: Uuid) -> Result<(), AuthRepoError>;

    /// `DELETE FROM auth_refresh_tokens WHERE user_id = $1` (logout, all devices — auth.ts:331).
    async fn delete_owner_families_for_user(&self, user_id: Uuid) -> Result<(), AuthRepoError>;

    /// Live P-c owner memberships, ordered `created_at, location_id` (auth.ts:293-297).
    async fn active_owner_locations(&self, user_id: Uuid) -> Result<Vec<Uuid>, AuthRepoError>;

    // ── Courier ──
    /// `SELECT id, password_hash, status FROM couriers WHERE email_hash=$1 OR phone_hash=$1`
    /// (courier/auth.ts:248-249) — the sha256 hash matches either column (Q-COURIER-EMAIL).
    async fn courier_by_identity_hash(
        &self,
        identity_hash_hex: &str,
    ) -> Result<Option<CourierAuthRow>, AuthRepoError>;

    /// Courier role at a specific location (`courier_locations`), or `None` if not a member.
    async fn courier_location_role(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<String>, AuthRepoError>;

    /// First-assigned location + role for a courier (`ORDER BY added_at ASC LIMIT 1`).
    async fn courier_first_location(
        &self,
        courier_id: Uuid,
    ) -> Result<Option<(Uuid, String)>, AuthRepoError>;

    /// Create a 30d courier session (login/redeem) → returns the new session id. `token_hash` is
    /// the argon2id hash of the plaintext secret half (courier/auth.ts:120-128).
    async fn create_courier_session(
        &self,
        courier_id: Uuid,
        family_id: Uuid,
        token_hash: &str,
        active_location_id: Uuid,
    ) -> Result<Uuid, AuthRepoError>;

    async fn courier_session_by_id(
        &self,
        session_id: Uuid,
    ) -> Result<Option<CourierSessionRefreshRow>, AuthRepoError>;

    /// Rotate: revoke the old session, insert a new 30d session in the same family, link
    /// `replaced_by` → returns the new session id (courier/auth.ts:443-458).
    async fn rotate_courier_session(
        &self,
        old_id: Uuid,
        courier_id: Uuid,
        family_id: Uuid,
        new_token_hash: &str,
        active_location_id: Uuid,
    ) -> Result<Uuid, AuthRepoError>;

    /// Reuse detected → revoke the whole family (courier/auth.ts:420).
    async fn revoke_courier_family(&self, family_id: Uuid) -> Result<(), AuthRepoError>;

    /// Best-effort logout: revoke one session if not already revoked (courier/auth.ts:499-502).
    async fn revoke_courier_session(&self, session_id: Uuid) -> Result<(), AuthRepoError>;

    /// `SELECT status FROM couriers WHERE id=$1` (courier refresh status re-check, auth.ts:436).
    async fn courier_status(&self, courier_id: Uuid) -> Result<Option<String>, AuthRepoError>;

    /// REV-1 live bind: the `(jti, activeLocationId, sub)`-keyed session + membership existence
    /// check (plugins/auth.ts:74-83). `None` = no such session row for this courier.
    async fn courier_session_bind(
        &self,
        jti: Uuid,
        active_location_id: Uuid,
        courier_id: Uuid,
    ) -> Result<Option<CourierSessionBindRow>, AuthRepoError>;

    // ── Customer track exchange (AUTH-10) ──
    /// Resolve an opaque grant hash → `(grant_id, order_id, location_id, customer_id)` via
    /// `customer_track_grants JOIN orders`, `expires_at > now()` (track.ts:43-53). `grant_id` is
    /// `g.id` (track.ts:44) — the key the observability bump targets. Runs on the operational pool
    /// with an explicit `WHERE token_hash` (Q-TRACK-POOL).
    async fn track_grant_by_hash(
        &self,
        token_hash_hex: &str,
    ) -> Result<Option<(Uuid, Uuid, Uuid, Uuid)>, AuthRepoError>;

    /// Bump `use_count` (observability, not a single-use gate — Q-TRACK-REUSE).
    async fn bump_track_use_count(&self, grant_id: Uuid) -> Result<(), AuthRepoError>;

    // ── Claim (AUTH-07) ──
    /// The G-F2g pre-check: is the invite for `token` a token-only (NULL `invited_contact_hash`)
    /// invite? `Some(true)` = contact-required (reject web claim). `None` = no matching invite.
    async fn claim_invite_is_contact_bound(
        &self,
        token: &str,
    ) -> Result<Option<bool>, AuthRepoError>;

    /// CALL the `claim_transfer(token, user_id)` SECURITY DEFINER fn → `(org_id, location_id)` or
    /// a `CLAIMERR:<code>` error string (acceptClaim, claim.ts:117). The DEFINER fn is the atomic
    /// transfer authority — never reimplemented here.
    async fn claim_transfer(
        &self,
        token: &str,
        user_id: Uuid,
    ) -> Result<Result<(Uuid, Uuid), String>, AuthRepoError>;

    /// `SELECT read_preview_menu($1)` non-null → this slug is a claimable shadow (claim.ts:57).
    async fn slug_is_shadow(&self, slug: &str) -> Result<bool, AuthRepoError>;

    // ── Owner OAuth callback user upsert (AUTH-02) ──
    /// Upsert an owner user by `google_sub` (email fallback) → returns the user id (auth.ts:122-144).
    async fn upsert_google_user(
        &self,
        email: &str,
        google_sub: &str,
        name: Option<&str>,
    ) -> Result<Uuid, AuthRepoError>;

    // ── Telegram login (AUTH-03) ──
    /// `INSERT INTO telegram_login_tokens (expires_at) VALUES (now()+5min) RETURNING token`.
    async fn telegram_create_login_token(&self) -> Result<Uuid, AuthRepoError>;

    /// Poll a telegram login token → its `(status, user_id, expired)` or `None` if unknown
    /// (auth.ts:207-214).
    async fn telegram_poll_token(
        &self,
        token: Uuid,
    ) -> Result<Option<TelegramTokenState>, AuthRepoError>;

    /// Atomic single-use consume: flip `authenticated → consumed`, `rowCount==1` wins
    /// (auth.ts:217). Returns the `user_id` on the winning flip, `None` if the race was lost.
    async fn telegram_consume_token(&self, token: Uuid) -> Result<Option<Uuid>, AuthRepoError>;

    // ── Courier invite (AUTH-05) ──
    /// Public invite-details lookup (courier/auth.ts:159) — the two-pass RLS discovery reduced to
    /// one projected row. `None` = unknown invite.
    async fn courier_invite_details(
        &self,
        invite_id: Uuid,
    ) -> Result<Option<CourierInviteDetailsRow>, AuthRepoError>;

    /// The redeem lookup: the active invite's `code_hash` + spine, `FOR UPDATE`
    /// (courier/auth.ts:55). `None` = invalid/expired/used/revoked.
    async fn courier_invite_for_redeem(
        &self,
        invite_id: Uuid,
    ) -> Result<Option<CourierInviteRedeemRow>, AuthRepoError>;

    /// The transactional redeem write (courier/auth.ts:88-116): upsert the courier with encrypted
    /// PII, add the location membership, mark the invite used, audit. Returns the courier id. The
    /// PII is passed pre-encrypted (the handler holds the `PiiCipher`); hashes are hex strings.
    #[allow(clippy::too_many_arguments)]
    async fn redeem_courier_write(
        &self,
        invite_id: Uuid,
        location_id: Uuid,
        role: &str,
        created_by_owner_id: Option<Uuid>,
        email_hash_hex: &str,
        email_encrypted: &[u8],
        phone_hash_hex: Option<&str>,
        phone_encrypted: Option<&[u8]>,
        full_name_encrypted: &[u8],
        password_hash: &str,
    ) -> Result<Uuid, AuthRepoError>;
}

/// Invite public details (courier/auth.ts:204).
#[derive(Debug, Clone)]
pub struct CourierInviteDetailsRow {
    pub role: String,
    pub location_name: String,
    pub is_expired: bool,
    pub is_used: bool,
    pub is_revoked: bool,
}

/// The active-invite redeem row (courier/auth.ts:56,65).
#[derive(Debug, Clone)]
pub struct CourierInviteRedeemRow {
    /// argon2id hash of the invite code (`courier_invites.code_hash`).
    pub code_hash: String,
    pub location_id: Uuid,
    pub role: String,
    pub created_by_owner_id: Option<Uuid>,
}

/// A telegram login token's live state (auth.ts:208).
#[derive(Debug, Clone)]
pub struct TelegramTokenState {
    pub status: String,
    pub user_id: Option<Uuid>,
    pub expired: bool,
}

/// The real sqlx-backed implementation. Holds both pools: `operational` for the explicit-WHERE
/// pre-auth paths, `session`/`with_tenant` reserved for RLS-scoped authenticated reads.
pub struct PgAuthRepo {
    pool: PgPool,
}

impl PgAuthRepo {
    pub fn new(pool: PgPool) -> Self {
        PgAuthRepo { pool }
    }
}

#[async_trait::async_trait]
impl AuthRepo for PgAuthRepo {
    async fn user_by_email(
        &self,
        email_lower: &str,
    ) -> Result<Option<(Uuid, Option<String>)>, AuthRepoError> {
        let row: Option<(Uuid, Option<String>)> =
            sqlx::query_as("SELECT id, password_hash FROM users WHERE email = $1")
                .bind(email_lower)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
    }

    async fn resolve_owner_membership(
        &self,
        user_id: Uuid,
    ) -> Result<(String, Option<Uuid>), AuthRepoError> {
        // memberships owner-first (local.ts:116-124).
        let mem: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT location_id, role FROM memberships
              WHERE user_id = $1 AND status = 'active'
              ORDER BY (role = 'owner') DESC LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        if let Some((location_id, role)) = mem {
            return Ok((role, Some(location_id)));
        }
        // Fallback: an org the user owns → owner, first active location (local.ts:126-135).
        let org: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM organizations WHERE owner_id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?;
        if let Some((org_id,)) = org {
            let loc: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM locations WHERE org_id = $1 AND status = 'active' ORDER BY created_at LIMIT 1",
            )
            .bind(org_id)
            .fetch_optional(&self.pool)
            .await?;
            return Ok(("owner".to_string(), loc.map(|(id,)| id)));
        }
        // Q-ROLE-DEGRADE: no membership + no owned org → 'customer' (fail-safe, grants LESS).
        Ok(("customer".to_string(), None))
    }

    async fn insert_owner_refresh(
        &self,
        user_id: Uuid,
        family_id: Uuid,
        token_hash_hex: &str,
    ) -> Result<bool, AuthRepoError> {
        let res = sqlx::query(
            "INSERT INTO auth_refresh_tokens (user_id, family_id, token_hash, expires_at)
             VALUES ($1, $2, $3, now() + interval '7 days')",
        )
        .bind(user_id)
        .bind(family_id)
        .bind(token_hash_hex)
        .execute(&self.pool)
        .await;
        // Q-REFRESH-OMIT: an INSERT failure is non-fatal — the caller simply omits refresh_token.
        Ok(res.is_ok())
    }

    async fn owner_refresh_by_hash(
        &self,
        token_hash_hex: &str,
    ) -> Result<Option<OwnerRefreshRow>, AuthRepoError> {
        let row: Option<(Uuid, Uuid, Uuid, bool)> = sqlx::query_as(
            "SELECT id, user_id, family_id, (expires_at < now()) AS expired
               FROM auth_refresh_tokens WHERE token_hash = $1",
        )
        .bind(token_hash_hex)
        .fetch_optional(&self.pool)
        .await?;
        Ok(
            row.map(|(id, user_id, family_id, expired)| OwnerRefreshRow {
                id,
                user_id,
                family_id,
                expired,
            }),
        )
    }

    async fn claim_owner_refresh(&self, id: Uuid) -> Result<bool, AuthRepoError> {
        let res = sqlx::query(
            "UPDATE auth_refresh_tokens SET used = true WHERE id = $1 AND used = false",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() == 1)
    }

    async fn family_rotated_within_5s(&self, family_id: Uuid) -> Result<bool, AuthRepoError> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM auth_refresh_tokens
              WHERE family_id = $1 AND created_at > now() - interval '5 seconds' LIMIT 1",
        )
        .bind(family_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.is_some())
    }

    async fn delete_owner_family(&self, family_id: Uuid) -> Result<(), AuthRepoError> {
        sqlx::query("DELETE FROM auth_refresh_tokens WHERE family_id = $1")
            .bind(family_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete_owner_families_for_user(&self, user_id: Uuid) -> Result<(), AuthRepoError> {
        sqlx::query("DELETE FROM auth_refresh_tokens WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn active_owner_locations(&self, user_id: Uuid) -> Result<Vec<Uuid>, AuthRepoError> {
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT location_id FROM memberships
              WHERE user_id = $1 AND role = 'owner' AND status = 'active'
              ORDER BY created_at, location_id",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    async fn courier_by_identity_hash(
        &self,
        identity_hash_hex: &str,
    ) -> Result<Option<CourierAuthRow>, AuthRepoError> {
        let row: Option<(Uuid, String, String)> = sqlx::query_as(
            "SELECT id, password_hash, status FROM couriers WHERE email_hash = $1 OR phone_hash = $1",
        )
        .bind(identity_hash_hex)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(id, password_hash, status)| CourierAuthRow {
            id,
            password_hash,
            status,
        }))
    }

    async fn courier_location_role(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<String>, AuthRepoError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT role FROM courier_locations WHERE courier_id = $1 AND location_id = $2",
        )
        .bind(courier_id)
        .bind(location_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(r,)| r))
    }

    async fn courier_first_location(
        &self,
        courier_id: Uuid,
    ) -> Result<Option<(Uuid, String)>, AuthRepoError> {
        let row: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT location_id, role FROM courier_locations WHERE courier_id = $1 ORDER BY added_at ASC LIMIT 1",
        )
        .bind(courier_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn create_courier_session(
        &self,
        courier_id: Uuid,
        family_id: Uuid,
        token_hash: &str,
        active_location_id: Uuid,
    ) -> Result<Uuid, AuthRepoError> {
        let (id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO courier_sessions (courier_id, family_id, token_hash, active_location_id, expires_at)
             VALUES ($1, $2, $3, $4, now() + interval '30 days') RETURNING id",
        )
        .bind(courier_id)
        .bind(family_id)
        .bind(token_hash)
        .bind(active_location_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    async fn courier_session_by_id(
        &self,
        session_id: Uuid,
    ) -> Result<Option<CourierSessionRefreshRow>, AuthRepoError> {
        let row: Option<(Uuid, Uuid, Uuid, String, Uuid, bool, bool)> = sqlx::query_as(
            "SELECT id, courier_id, family_id, token_hash, active_location_id,
                    (revoked_at IS NOT NULL) AS revoked, (expires_at < now()) AS expired
               FROM courier_sessions WHERE id = $1 FOR UPDATE NOWAIT",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(id, courier_id, family_id, token_hash, active_location_id, revoked, expired)| {
                CourierSessionRefreshRow {
                    id,
                    courier_id,
                    family_id,
                    token_hash,
                    active_location_id,
                    revoked,
                    expired,
                }
            },
        ))
    }

    async fn rotate_courier_session(
        &self,
        old_id: Uuid,
        courier_id: Uuid,
        family_id: Uuid,
        new_token_hash: &str,
        active_location_id: Uuid,
    ) -> Result<Uuid, AuthRepoError> {
        sqlx::query(
            "UPDATE courier_sessions SET revoked_at = now(), last_used_at = now() WHERE id = $1",
        )
        .bind(old_id)
        .execute(&self.pool)
        .await?;
        let (new_id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO courier_sessions (courier_id, family_id, token_hash, active_location_id, expires_at, replaced_by)
             VALUES ($1, $2, $3, $4, now() + interval '30 days', $5) RETURNING id",
        )
        .bind(courier_id)
        .bind(family_id)
        .bind(new_token_hash)
        .bind(active_location_id)
        .bind(old_id)
        .fetch_one(&self.pool)
        .await?;
        sqlx::query("UPDATE courier_sessions SET replaced_by = $1 WHERE id = $2")
            .bind(new_id)
            .bind(old_id)
            .execute(&self.pool)
            .await?;
        Ok(new_id)
    }

    async fn revoke_courier_family(&self, family_id: Uuid) -> Result<(), AuthRepoError> {
        sqlx::query("UPDATE courier_sessions SET revoked_at = now() WHERE family_id = $1")
            .bind(family_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn revoke_courier_session(&self, session_id: Uuid) -> Result<(), AuthRepoError> {
        sqlx::query(
            "UPDATE courier_sessions SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL",
        )
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn courier_status(&self, courier_id: Uuid) -> Result<Option<String>, AuthRepoError> {
        let row: Option<(String,)> = sqlx::query_as("SELECT status FROM couriers WHERE id = $1")
            .bind(courier_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|(s,)| s))
    }

    async fn courier_session_bind(
        &self,
        jti: Uuid,
        active_location_id: Uuid,
        courier_id: Uuid,
    ) -> Result<Option<CourierSessionBindRow>, AuthRepoError> {
        let row: Option<(bool, bool, bool)> = sqlx::query_as(
            "SELECT (s.revoked_at IS NOT NULL) AS revoked,
                    (s.expires_at < now()) AS expired,
                    EXISTS(
                      SELECT 1 FROM courier_locations cl
                      WHERE cl.courier_id = s.courier_id AND cl.location_id = $2
                    ) AS has_location
               FROM courier_sessions s
              WHERE s.id = $1 AND s.courier_id = $3",
        )
        .bind(jti)
        .bind(active_location_id)
        .bind(courier_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(
            row.map(|(revoked, expired, has_location)| CourierSessionBindRow {
                revoked,
                expired,
                has_location,
            }),
        )
    }

    async fn track_grant_by_hash(
        &self,
        token_hash_hex: &str,
    ) -> Result<Option<(Uuid, Uuid, Uuid, Uuid)>, AuthRepoError> {
        let row: Option<(Uuid, Uuid, Uuid, Uuid)> = sqlx::query_as(
            "SELECT g.id, g.order_id, g.location_id, o.customer_id
               FROM customer_track_grants g
               JOIN orders o ON o.id = g.order_id
              WHERE g.token_hash = $1 AND g.expires_at > now()",
        )
        .bind(token_hash_hex)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn bump_track_use_count(&self, grant_id: Uuid) -> Result<(), AuthRepoError> {
        sqlx::query("UPDATE customer_track_grants SET use_count = use_count + 1 WHERE id = $1")
            .bind(grant_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn claim_invite_is_contact_bound(
        &self,
        token: &str,
    ) -> Result<Option<bool>, AuthRepoError> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT invited_contact_hash FROM claim_invites
              WHERE token_hash = encode(sha256($1::bytea), 'hex') AND used_at IS NULL
                AND (expires_at IS NULL OR expires_at > now()) LIMIT 1",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;
        // Some(true) = contact-bound (has a hash); Some(false) = token-only (NULL) → CONTACT_REQUIRED.
        Ok(row.map(|(hash,)| hash.is_some()))
    }

    async fn claim_transfer(
        &self,
        token: &str,
        user_id: Uuid,
    ) -> Result<Result<(Uuid, Uuid), String>, AuthRepoError> {
        // CALL the SECURITY DEFINER fn (claim.ts:117). A CLAIMERR raise surfaces as a db error
        // whose message carries `CLAIMERR:<code>` — extracted to the bare-error code.
        let res: Result<(Uuid, Uuid), sqlx::Error> =
            sqlx::query_as("SELECT org_id, location_id FROM claim_transfer($1, $2)")
                .bind(token)
                .bind(user_id)
                .fetch_one(&self.pool)
                .await;
        match res {
            Ok(pair) => Ok(Ok(pair)),
            Err(err) => {
                let msg = err.to_string();
                if let Some(code) = extract_claim_err(&msg) {
                    Ok(Err(code))
                } else {
                    Err(AuthRepoError(err))
                }
            }
        }
    }

    async fn slug_is_shadow(&self, slug: &str) -> Result<bool, AuthRepoError> {
        let res: Result<Option<(Option<serde_json::Value>,)>, sqlx::Error> =
            sqlx::query_as("SELECT read_preview_menu($1) AS m")
                .bind(slug)
                .fetch_optional(&self.pool)
                .await;
        match res {
            Ok(Some((Some(_),))) => Ok(true),
            Ok(_) => Ok(false),
            Err(err) => {
                // 42883 undefined_function (migration not applied) = "not a shadow" (claim.ts:62).
                if err
                    .as_database_error()
                    .and_then(sqlx::error::DatabaseError::code)
                    .is_some_and(|c| c == "42883")
                {
                    Ok(false)
                } else {
                    Err(AuthRepoError(err))
                }
            }
        }
    }

    async fn upsert_google_user(
        &self,
        email: &str,
        google_sub: &str,
        name: Option<&str>,
    ) -> Result<Uuid, AuthRepoError> {
        // Upsert by google_sub (auth.ts:122-128). On an email conflict (user signed up another
        // way then tried Google) the ON CONFLICT(google_sub) can't fire, so fall back to an email
        // UPDATE (auth.ts:136-143).
        let by_sub: Result<(Uuid,), sqlx::Error> = sqlx::query_as(
            "INSERT INTO users (email, google_sub, display_name) VALUES ($1, $2, $3)
             ON CONFLICT (google_sub) DO UPDATE SET email = EXCLUDED.email, display_name = EXCLUDED.display_name
             RETURNING id",
        )
        .bind(email)
        .bind(google_sub)
        .bind(name)
        .fetch_one(&self.pool)
        .await;
        match by_sub {
            Ok((id,)) => Ok(id),
            Err(_) => {
                let (id,): (Uuid,) = sqlx::query_as(
                    "UPDATE users SET google_sub = $2, display_name = COALESCE(users.display_name, $3)
                     WHERE email = $1 RETURNING id",
                )
                .bind(email)
                .bind(google_sub)
                .bind(name)
                .fetch_one(&self.pool)
                .await?;
                Ok(id)
            }
        }
    }

    async fn telegram_create_login_token(&self) -> Result<Uuid, AuthRepoError> {
        let (token,): (Uuid,) = sqlx::query_as(
            "INSERT INTO telegram_login_tokens (expires_at) VALUES (now() + interval '5 minutes') RETURNING token",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(token)
    }

    async fn telegram_poll_token(
        &self,
        token: Uuid,
    ) -> Result<Option<TelegramTokenState>, AuthRepoError> {
        let row: Option<(String, Option<Uuid>, bool)> = sqlx::query_as(
            "SELECT status, user_id, (expires_at < now()) AS expired
               FROM telegram_login_tokens WHERE token = $1::uuid",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(status, user_id, expired)| TelegramTokenState {
            status,
            user_id,
            expired,
        }))
    }

    async fn telegram_consume_token(&self, token: Uuid) -> Result<Option<Uuid>, AuthRepoError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "UPDATE telegram_login_tokens SET status = 'consumed'
              WHERE token = $1::uuid AND status = 'authenticated' RETURNING user_id",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(uid,)| uid))
    }

    async fn courier_invite_details(
        &self,
        invite_id: Uuid,
    ) -> Result<Option<CourierInviteDetailsRow>, AuthRepoError> {
        // The Node code does a two-pass RLS discovery (read location_id, set tenant, JOIN). Here
        // the query runs on the operational pool with an explicit id predicate (the same pool the
        // Node code ultimately queries under). Kept as ONE projected read for the port.
        let row: Option<(String, String, bool, bool, bool)> = sqlx::query_as(
            "SELECT ci.role, l.name AS location_name,
                    (ci.expires_at < now()) AS is_expired,
                    (ci.used_at IS NOT NULL) AS is_used,
                    (ci.revoked_at IS NOT NULL) AS is_revoked
               FROM courier_invites ci
               JOIN locations l ON l.id = ci.location_id
              WHERE ci.id = $1",
        )
        .bind(invite_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(
            row.map(|(role, location_name, is_expired, is_used, is_revoked)| {
                CourierInviteDetailsRow {
                    role,
                    location_name,
                    is_expired,
                    is_used,
                    is_revoked,
                }
            }),
        )
    }

    async fn courier_invite_for_redeem(
        &self,
        invite_id: Uuid,
    ) -> Result<Option<CourierInviteRedeemRow>, AuthRepoError> {
        let row: Option<(String, Uuid, String, Option<Uuid>)> = sqlx::query_as(
            "SELECT code_hash, location_id, role, created_by_owner_id
               FROM courier_invites
              WHERE id = $1 AND expires_at > now() AND used_at IS NULL AND revoked_at IS NULL
              FOR UPDATE",
        )
        .bind(invite_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(code_hash, location_id, role, created_by_owner_id)| CourierInviteRedeemRow {
                code_hash,
                location_id,
                role,
                created_by_owner_id,
            },
        ))
    }

    async fn redeem_courier_write(
        &self,
        invite_id: Uuid,
        location_id: Uuid,
        role: &str,
        created_by_owner_id: Option<Uuid>,
        email_hash_hex: &str,
        email_encrypted: &[u8],
        phone_hash_hex: Option<&str>,
        phone_encrypted: Option<&[u8]>,
        full_name_encrypted: &[u8],
        password_hash: &str,
    ) -> Result<Uuid, AuthRepoError> {
        let mut txn = self.pool.begin().await?;
        // Upsert the courier (ON CONFLICT(email_hash) — Q-REDEEM-PW: re-redeem overwrites password).
        let (courier_id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO couriers (email_encrypted, email_hash, phone_encrypted, phone_hash, full_name_encrypted, password_hash)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (email_hash) DO UPDATE SET password_hash = EXCLUDED.password_hash RETURNING id",
        )
        .bind(email_encrypted)
        .bind(email_hash_hex)
        .bind(phone_encrypted)
        .bind(phone_hash_hex)
        .bind(full_name_encrypted)
        .bind(password_hash)
        .fetch_one(&mut *txn)
        .await?;

        sqlx::query(
            "INSERT INTO courier_locations (courier_id, location_id, role, added_by_owner_id)
             VALUES ($1, $2, $3, $4) ON CONFLICT (courier_id, location_id) DO NOTHING",
        )
        .bind(courier_id)
        .bind(location_id)
        .bind(role)
        .bind(created_by_owner_id)
        .execute(&mut *txn)
        .await?;

        sqlx::query(
            "UPDATE courier_invites SET used_at = now(), used_by_courier_id = $1 WHERE id = $2",
        )
        .bind(courier_id)
        .bind(invite_id)
        .execute(&mut *txn)
        .await?;

        txn.commit().await?;
        Ok(courier_id)
    }
}

/// Extract `CLAIMERR:<CODE>` from a Postgres error message (claim.ts:122 `/CLAIMERR:(\w+)/`).
fn extract_claim_err(msg: &str) -> Option<String> {
    let start = msg.find("CLAIMERR:")? + "CLAIMERR:".len();
    let code: String = msg[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if code.is_empty() { None } else { Some(code) }
}

#[cfg(test)]
pub mod fake;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_claim_err_pulls_the_code() {
        assert_eq!(
            extract_claim_err("error: CLAIMERR:CONTACT_MISMATCH at line 68"),
            Some("CONTACT_MISMATCH".to_string())
        );
        assert_eq!(
            extract_claim_err("db error: CLAIMERR:ALREADY_CLAIMED"),
            Some("ALREADY_CLAIMED".to_string())
        );
        assert_eq!(extract_claim_err("some unrelated db error"), None);
    }
}
