//! `FakeAuthRepo` — the `cfg(test)` stub the build brief asks for, so the extractors and route
//! handlers are unit-testable without a live Postgres. Every field is a `Mutex`-guarded map/flag
//! configured per-test; each method reads its canned answer. Only the surface the auth
//! extractor/route tests actually exercise is filled with real behavior — the rest return safe
//! defaults (empty / None / Ok) that a test overrides when it needs them.

use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

use super::{
    AuthRepo, AuthRepoError, CourierAuthRow, CourierInviteDetailsRow, CourierInviteRedeemRow,
    CourierSessionBindRow, CourierSessionRefreshRow, OwnerRefreshRow, TelegramTokenState,
};

/// `(grant_id, order_id, location_id, customer_id)` — `track_grant_by_hash`'s tuple, aliased so
/// the `FakeAuthRepo` field doesn't trip clippy's `type_complexity` (workspace `deny`).
type TrackGrantEntry = (Uuid, Uuid, Uuid, Uuid);
/// `(user_id, password_hash?)` — `user_by_email`'s tuple.
type UserEntry = (Uuid, Option<String>);
/// `(jti, active_location_id, courier_id)` — the REV-1 courier-bind lookup key.
type CourierBindKey = (Uuid, Uuid, Uuid);
/// `Ok((org_id, location_id))` | `Err(claim_error_code)` — `claim_transfer`'s canned result.
type ClaimTransferResult = Result<(Uuid, Uuid), String>;

#[derive(Default)]
pub struct FakeAuthRepo {
    pub users_by_email: Mutex<HashMap<String, UserEntry>>,
    pub owner_membership: Mutex<HashMap<Uuid, (String, Option<Uuid>)>>,
    pub owner_refresh_by_hash: Mutex<HashMap<String, OwnerRefreshRow>>,
    pub claimed_ids: Mutex<HashMap<Uuid, bool>>,
    pub recent_family: Mutex<HashMap<Uuid, bool>>,
    pub active_owner_locations: Mutex<HashMap<Uuid, Vec<Uuid>>>,
    pub couriers_by_hash: Mutex<HashMap<String, CourierAuthRow>>,
    pub courier_location_roles: Mutex<HashMap<(Uuid, Uuid), String>>,
    pub courier_first_location: Mutex<HashMap<Uuid, (Uuid, String)>>,
    pub courier_sessions: Mutex<HashMap<Uuid, CourierSessionRefreshRow>>,
    pub courier_statuses: Mutex<HashMap<Uuid, String>>,
    /// REV-1: keyed on `(jti, active_location_id, courier_id)` → the live bind row.
    pub courier_binds: Mutex<HashMap<CourierBindKey, CourierSessionBindRow>>,
    pub track_grants: Mutex<HashMap<String, TrackGrantEntry>>,
    pub claim_contact_bound: Mutex<HashMap<String, bool>>,
    pub claim_transfers: Mutex<HashMap<String, ClaimTransferResult>>,
    /// S10 REV-S10-3: records the recipient `claim_transfer` was invoked with, so a test can prove
    /// the recipient is the AUTHENTICATED sub (never a body-supplied id) — the IDOR guard.
    pub claim_transfer_recipient: Mutex<Option<Uuid>>,
    pub shadows: Mutex<HashMap<String, bool>>,
    /// Track the last created courier session id so tests can assert a session was minted.
    pub last_created_session: Mutex<Option<Uuid>>,
    pub google_user: Mutex<Option<Uuid>>,
    pub last_telegram_token: Mutex<Option<Uuid>>,
    pub telegram_states: Mutex<HashMap<Uuid, TelegramTokenState>>,
    pub invite_details: Mutex<HashMap<Uuid, CourierInviteDetailsRow>>,
    pub invite_redeem: Mutex<HashMap<Uuid, CourierInviteRedeemRow>>,
    pub redeemed_courier: Mutex<Option<Uuid>>,
}

#[async_trait::async_trait]
impl AuthRepo for FakeAuthRepo {
    async fn user_by_email(
        &self,
        email_lower: &str,
    ) -> Result<Option<(Uuid, Option<String>)>, AuthRepoError> {
        Ok(self
            .users_by_email
            .lock()
            .unwrap()
            .get(email_lower)
            .cloned())
    }

    async fn resolve_owner_membership(
        &self,
        user_id: Uuid,
    ) -> Result<(String, Option<Uuid>), AuthRepoError> {
        Ok(self
            .owner_membership
            .lock()
            .unwrap()
            .get(&user_id)
            .cloned()
            .unwrap_or_else(|| ("customer".to_string(), None)))
    }

    async fn insert_owner_refresh(
        &self,
        _user_id: Uuid,
        _family_id: Uuid,
        _token_hash_hex: &str,
    ) -> Result<bool, AuthRepoError> {
        Ok(true)
    }

    async fn owner_refresh_by_hash(
        &self,
        token_hash_hex: &str,
    ) -> Result<Option<OwnerRefreshRow>, AuthRepoError> {
        Ok(self
            .owner_refresh_by_hash
            .lock()
            .unwrap()
            .get(token_hash_hex)
            .cloned())
    }

    async fn claim_owner_refresh(&self, id: Uuid) -> Result<bool, AuthRepoError> {
        // Default: claim succeeds unless a test pre-set it to false (already-used).
        Ok(*self.claimed_ids.lock().unwrap().get(&id).unwrap_or(&true))
    }

    async fn family_rotated_within_5s(&self, family_id: Uuid) -> Result<bool, AuthRepoError> {
        Ok(*self
            .recent_family
            .lock()
            .unwrap()
            .get(&family_id)
            .unwrap_or(&false))
    }

    async fn delete_owner_family(&self, _family_id: Uuid) -> Result<(), AuthRepoError> {
        Ok(())
    }

    async fn delete_owner_families_for_user(&self, _user_id: Uuid) -> Result<(), AuthRepoError> {
        Ok(())
    }

    async fn active_owner_locations(&self, user_id: Uuid) -> Result<Vec<Uuid>, AuthRepoError> {
        Ok(self
            .active_owner_locations
            .lock()
            .unwrap()
            .get(&user_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn courier_by_identity_hash(
        &self,
        identity_hash_hex: &str,
    ) -> Result<Option<CourierAuthRow>, AuthRepoError> {
        Ok(self
            .couriers_by_hash
            .lock()
            .unwrap()
            .get(identity_hash_hex)
            .cloned())
    }

    async fn courier_location_role(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<String>, AuthRepoError> {
        Ok(self
            .courier_location_roles
            .lock()
            .unwrap()
            .get(&(courier_id, location_id))
            .cloned())
    }

    async fn courier_first_location(
        &self,
        courier_id: Uuid,
    ) -> Result<Option<(Uuid, String)>, AuthRepoError> {
        Ok(self
            .courier_first_location
            .lock()
            .unwrap()
            .get(&courier_id)
            .cloned())
    }

    async fn create_courier_session(
        &self,
        _courier_id: Uuid,
        _family_id: Uuid,
        _token_hash: &str,
        _active_location_id: Uuid,
    ) -> Result<Uuid, AuthRepoError> {
        let id = Uuid::new_v4();
        *self.last_created_session.lock().unwrap() = Some(id);
        Ok(id)
    }

    async fn courier_session_by_id(
        &self,
        session_id: Uuid,
    ) -> Result<Option<CourierSessionRefreshRow>, AuthRepoError> {
        Ok(self
            .courier_sessions
            .lock()
            .unwrap()
            .get(&session_id)
            .cloned())
    }

    async fn rotate_courier_session(
        &self,
        _old_id: Uuid,
        _courier_id: Uuid,
        _family_id: Uuid,
        _new_token_hash: &str,
        _active_location_id: Uuid,
    ) -> Result<Uuid, AuthRepoError> {
        Ok(Uuid::new_v4())
    }

    async fn revoke_courier_family(&self, _family_id: Uuid) -> Result<(), AuthRepoError> {
        Ok(())
    }

    async fn revoke_courier_session(&self, _session_id: Uuid) -> Result<(), AuthRepoError> {
        Ok(())
    }

    async fn courier_status(&self, courier_id: Uuid) -> Result<Option<String>, AuthRepoError> {
        Ok(self
            .courier_statuses
            .lock()
            .unwrap()
            .get(&courier_id)
            .cloned())
    }

    async fn courier_session_bind(
        &self,
        jti: Uuid,
        active_location_id: Uuid,
        courier_id: Uuid,
    ) -> Result<Option<CourierSessionBindRow>, AuthRepoError> {
        Ok(self
            .courier_binds
            .lock()
            .unwrap()
            .get(&(jti, active_location_id, courier_id))
            .cloned())
    }

    async fn track_grant_by_hash(
        &self,
        token_hash_hex: &str,
    ) -> Result<Option<(Uuid, Uuid, Uuid, Uuid)>, AuthRepoError> {
        Ok(self
            .track_grants
            .lock()
            .unwrap()
            .get(token_hash_hex)
            .copied())
    }

    async fn bump_track_use_count(&self, _grant_id: Uuid) -> Result<(), AuthRepoError> {
        Ok(())
    }

    async fn claim_invite_is_contact_bound(
        &self,
        token: &str,
    ) -> Result<Option<bool>, AuthRepoError> {
        Ok(self.claim_contact_bound.lock().unwrap().get(token).copied())
    }

    async fn claim_transfer(
        &self,
        token: &str,
        user_id: Uuid,
    ) -> Result<Result<(Uuid, Uuid), String>, AuthRepoError> {
        // S10 REV-S10-3: record the recipient so a test can prove it is the authenticated sub.
        *self.claim_transfer_recipient.lock().unwrap() = Some(user_id);
        Ok(self
            .claim_transfers
            .lock()
            .unwrap()
            .get(token)
            .cloned()
            .unwrap_or_else(|| Err("INVALID_OR_EXPIRED_TOKEN".to_string())))
    }

    async fn slug_is_shadow(&self, slug: &str) -> Result<bool, AuthRepoError> {
        Ok(*self.shadows.lock().unwrap().get(slug).unwrap_or(&false))
    }

    async fn upsert_google_user(
        &self,
        _email: &str,
        _google_sub: &str,
        _name: Option<&str>,
    ) -> Result<Uuid, AuthRepoError> {
        Ok(*self
            .google_user
            .lock()
            .unwrap()
            .get_or_insert_with(Uuid::new_v4))
    }

    async fn telegram_create_login_token(&self) -> Result<Uuid, AuthRepoError> {
        let token = Uuid::new_v4();
        *self.last_telegram_token.lock().unwrap() = Some(token);
        Ok(token)
    }

    async fn telegram_poll_token(
        &self,
        token: Uuid,
    ) -> Result<Option<TelegramTokenState>, AuthRepoError> {
        Ok(self.telegram_states.lock().unwrap().get(&token).cloned())
    }

    async fn telegram_consume_token(&self, token: Uuid) -> Result<Option<Uuid>, AuthRepoError> {
        let mut states = self.telegram_states.lock().unwrap();
        match states.get_mut(&token) {
            Some(s) if s.status == "authenticated" => {
                s.status = "consumed".to_string();
                Ok(s.user_id)
            }
            _ => Ok(None),
        }
    }

    async fn courier_invite_details(
        &self,
        invite_id: Uuid,
    ) -> Result<Option<CourierInviteDetailsRow>, AuthRepoError> {
        Ok(self.invite_details.lock().unwrap().get(&invite_id).cloned())
    }

    async fn courier_invite_for_redeem(
        &self,
        invite_id: Uuid,
    ) -> Result<Option<CourierInviteRedeemRow>, AuthRepoError> {
        Ok(self.invite_redeem.lock().unwrap().get(&invite_id).cloned())
    }

    #[allow(clippy::too_many_arguments)]
    async fn redeem_courier_write(
        &self,
        _invite_id: Uuid,
        _location_id: Uuid,
        _role: &str,
        _created_by_owner_id: Option<Uuid>,
        _email_hash_hex: &str,
        _email_encrypted: &[u8],
        _phone_hash_hex: Option<&str>,
        _phone_encrypted: Option<&[u8]>,
        _full_name_encrypted: &[u8],
        _password_hash: &str,
    ) -> Result<Uuid, AuthRepoError> {
        let id = Uuid::new_v4();
        *self.redeemed_courier.lock().unwrap() = Some(id);
        Ok(id)
    }
}

impl FakeAuthRepo {
    /// Impls a manual `Clone`-ish helper: not needed. Convenience setters keep tests terse.
    pub fn with_courier_bind(
        self,
        jti: Uuid,
        loc: Uuid,
        courier_id: Uuid,
        row: CourierSessionBindRow,
    ) -> Self {
        self.courier_binds
            .lock()
            .unwrap()
            .insert((jti, loc, courier_id), row);
        self
    }
}
