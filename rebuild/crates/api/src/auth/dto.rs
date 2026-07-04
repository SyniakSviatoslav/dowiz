//! Wire DTOs for the S2 auth surface — mirrors `openapi-s2-auth.yaml` request/response schemas
//! byte-for-byte (field names, the mixed `access_token`/`jwt`/`refreshToken` casings the live API
//! actually emits — reproduced verbatim, never re-cased). Read-only display/handoff data; the
//! token strings inside are minted by `auth::jwt`, never computed here.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ── AUTH-01 owner local login ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct OwnerLoginRequest {
    pub email: String,
    pub password: String,
}

/// `{ access_token, refresh_token?, userId, activeLocationId? }` (local.ts:70,163). `refresh_token`
/// conditionally omitted (Q-REFRESH-OMIT); `activeLocationId` `[string,null]`.
#[derive(Debug, Serialize, ToSchema)]
pub struct OwnerLoginResponse {
    pub access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(rename = "userId")]
    pub user_id: Uuid,
    #[serde(rename = "activeLocationId")]
    pub active_location_id: Option<Uuid>,
}

// ── AUTH-02 OAuth exchange ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct ExchangeRequest {
    pub code: Uuid,
}

/// `{ access_token, refresh_token }` — the stashed pair, verbatim (auth.ts:184).
#[derive(Debug, Serialize, ToSchema)]
pub struct TokenPairResponse {
    pub access_token: String,
    pub refresh_token: String,
}

// ── AUTH-03 telegram ─────────────────────────────────────────────────────────────────────────

/// `{ token, botUsername, deepLink }` (auth.ts:199).
#[derive(Debug, Serialize, ToSchema)]
pub struct TelegramStartResponse {
    pub token: Uuid,
    #[serde(rename = "botUsername")]
    pub bot_username: String,
    #[serde(rename = "deepLink")]
    pub deep_link: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TelegramPollQuery {
    pub token: Uuid,
}

/// The terminal authenticated telegram-poll payload (`{status:'authenticated', access_token,
/// refresh_token}`, auth.ts:232). The non-terminal `{status:...}` bodies are hand-built
/// non-envelope shapes (Q-TG-POLLSHAPE) so they stay verbatim.
#[derive(Debug, Serialize, ToSchema)]
pub struct TelegramAuthenticated {
    pub status: String,
    pub access_token: String,
    pub refresh_token: String,
}

// ── AUTH-09 owner refresh + logout ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct OwnerRefreshRequest {
    pub refresh_token: String,
    #[serde(default)]
    pub active_location_id: Option<Uuid>,
}

// ── AUTH-05 courier invite ───────────────────────────────────────────────────────────────────

/// `{ id, role, locationName, isValid, isExpired, isUsed, isRevoked }` (courier/auth.ts:204).
#[derive(Debug, Serialize, ToSchema)]
pub struct CourierInviteResponse {
    pub id: Uuid,
    pub role: String,
    #[serde(rename = "locationName")]
    pub location_name: String,
    #[serde(rename = "isValid")]
    pub is_valid: bool,
    #[serde(rename = "isExpired")]
    pub is_expired: bool,
    #[serde(rename = "isUsed")]
    pub is_used: bool,
    #[serde(rename = "isRevoked")]
    pub is_revoked: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CourierRedeemRequest {
    pub email: String,
    pub code: String,
    pub password: String,
    pub full_name: String,
    #[serde(default)]
    pub phone: Option<String>,
}

/// `{ jwt, refreshToken, courier:{id, masked_email, full_name, locations[]} }` (courier/auth.ts:140).
#[derive(Debug, Serialize, ToSchema)]
pub struct CourierRedeemResponse {
    pub jwt: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    pub courier: CourierBrief,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CourierBrief {
    pub id: Uuid,
    pub masked_email: String,
    pub full_name: String,
    pub locations: Vec<CourierLocationBrief>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CourierLocationBrief {
    pub id: Uuid,
    pub role: String,
}

// ── AUTH-06 courier login / refresh / logout ─────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CourierLoginRequest {
    /// Email OR phone (not format-checked — Q-COURIER-EMAIL). Lowercased+trimmed server-side.
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub location_id: Option<Uuid>,
}

/// `{ jwt, refreshToken, activeLocationId, role }` (courier/auth.ts:339).
#[derive(Debug, Serialize, ToSchema)]
pub struct CourierLoginResponse {
    pub jwt: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(rename = "activeLocationId")]
    pub active_location_id: Uuid,
    pub role: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CourierRefreshRequest {
    pub refresh_token: String,
}

/// `{ jwt, refreshToken }` (courier/auth.ts:469).
#[derive(Debug, Serialize, ToSchema)]
pub struct CourierRefreshResponse {
    pub jwt: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CourierLogoutRequest {
    pub refresh_token: String,
}

/// `{ success: true }` — always (Q-COURIER-LOGOUT, courier/auth.ts:503).
#[derive(Debug, Serialize, ToSchema)]
pub struct CourierLogoutResponse {
    pub success: bool,
}

// ── AUTH-07 claim ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct ClaimTokenRequest {
    pub token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ClaimRequestBody {
    pub slug: String,
}

/// `{ org_id, location_id, reauth:true }` (claim.ts:28).
#[derive(Debug, Serialize, ToSchema)]
pub struct ClaimAcceptResponse {
    pub org_id: Uuid,
    pub location_id: Uuid,
    pub reauth: bool,
}

/// `{ requested:true, message }` — uniform ack (claim.ts:65).
#[derive(Debug, Serialize, ToSchema)]
pub struct ClaimRequestResponse {
    pub requested: bool,
    pub message: String,
}

/// `{ erased:true }` (claim.ts:77).
#[derive(Debug, Serialize, ToSchema)]
pub struct ClaimDeclineResponse {
    pub erased: bool,
}

// ── AUTH-10 customer track exchange ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct TrackExchangeRequest {
    pub code: String,
}

/// `{ token }` — the 7d customer JWT (track.ts:85).
#[derive(Debug, Serialize, ToSchema)]
pub struct TrackExchangeResponse {
    pub token: String,
}

// ── AUTH-08 dev mock-auth ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct MockAuthRequest {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    #[serde(rename = "locationSlug")]
    pub location_slug: Option<String>,
    #[serde(default)]
    #[serde(rename = "locationId")]
    pub location_id: Option<Uuid>,
    #[serde(default)]
    pub synthetic: Option<bool>,
    /// server.ts:595 `fresh` mode — the `/api/dev/mock-auth` handler ONLY (Q11/REV-5 divergence).
    #[serde(default)]
    pub fresh: Option<bool>,
}

/// `{ access_token, userId, activeLocationId?, synthetic? }` (mock-auth.ts:50,67,118).
#[derive(Debug, Serialize, ToSchema)]
pub struct MockAuthResponse {
    pub access_token: String,
    #[serde(rename = "userId")]
    pub user_id: Uuid,
    #[serde(rename = "activeLocationId")]
    pub active_location_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthetic: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_login_response_omits_refresh_when_none() {
        // Q-REFRESH-OMIT: refresh_token absent (not null) when the INSERT failed / dev path.
        let r = OwnerLoginResponse {
            access_token: "jwt".to_string(),
            refresh_token: None,
            user_id: Uuid::nil(),
            active_location_id: None,
        };
        let json = serde_json::to_value(&r).unwrap();
        assert!(json.get("refresh_token").is_none());
        // activeLocationId is [string,null] — present as null, not omitted.
        assert!(json.get("activeLocationId").is_some());
        assert!(json["activeLocationId"].is_null());
    }

    #[test]
    fn courier_response_uses_camelcase_refresh_token_and_jwt_key() {
        let r = CourierLoginResponse {
            jwt: "j".to_string(),
            refresh_token: "sid.plain".to_string(),
            active_location_id: Uuid::nil(),
            role: "courier".to_string(),
        };
        let json = serde_json::to_value(&r).unwrap();
        assert!(json.get("jwt").is_some());
        assert!(
            json.get("refreshToken").is_some(),
            "camelCase per courier/auth.ts"
        );
        assert!(json.get("refresh_token").is_none());
        assert!(json.get("activeLocationId").is_some());
    }

    #[test]
    fn mock_auth_response_omits_synthetic_when_none() {
        let r = MockAuthResponse {
            access_token: "t".to_string(),
            user_id: Uuid::nil(),
            active_location_id: None,
            synthetic: None,
        };
        let json = serde_json::to_value(&r).unwrap();
        assert!(json.get("synthetic").is_none());
    }
}
