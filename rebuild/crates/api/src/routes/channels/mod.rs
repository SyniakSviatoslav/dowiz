//! S3 channel registry CRUD — Phase 1.1 MVP. Owner control of distribution channels.
//! Per A5 (module placement), this lands as a hub-module with manifest.
//!
//! Routes:
//! - GET `/api/owner/locations/:locationId/channels` — list owned channels (owner+loc pattern)
//! - POST `/api/owner/locations/:locationId/channels` — create channel
//! - PATCH `/api/owner/locations/:locationId/channels/:channelId` — update
//! - DELETE `/api/owner/locations/:locationId/channels/:channelId` — delete

use async_trait::async_trait;
use axum::extract::{Extension, Path, Json};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

use crate::auth::AuthState;
use crate::auth::extractors::OwnerClaimsExt;
use crate::error::ApiError;
use crate::repo::RepoError;
use crate::routes::correlation_id_string;
use tower_http::request_id::RequestId;

use super::{assert_active_owner_membership, require_location_access};

#[derive(Clone)]
pub struct ChannelsState {
    pub auth: AuthState,
    pub repo: std::sync::Arc<dyn ChannelsRepo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChannelRow {
    pub id: Uuid,
    pub location_id: Uuid,
    pub kind: String,
    pub name: String,
    pub token: String,
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateChannelRequest {
    pub kind: String,
    pub name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateChannelRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
}

#[async_trait]
pub trait ChannelsRepo: Send + Sync {
    async fn list(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Vec<ChannelRow>, RepoError>;

    async fn create(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        kind: String,
        name: String,
    ) -> Result<ChannelRow, RepoError>;

    async fn update(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        channel_id: Uuid,
        name: Option<String>,
        active: Option<bool>,
    ) -> Result<Option<ChannelRow>, RepoError>;

    async fn delete(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        channel_id: Uuid,
    ) -> Result<bool, RepoError>;
}

#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/channels",
    params(("locationId" = Uuid, Path, description = "Location ID")),
    responses((status = 200, description = "List of channels", body = Vec<ChannelRow>)),
    tag = "channels"
)]
pub async fn list_channels(
    OwnerClaimsExt(claims): OwnerClaimsExt,
    Extension(auth): Extension<AuthState>,
    Extension(state): Extension<ChannelsState>,
    Path(location_id): Path<Uuid>,
) -> Result<Json<Vec<ChannelRow>>, ApiError> {
    require_location_access(&auth, &claims, location_id).await?;
    state.repo.list(claims.user_id, location_id).await
        .map(Json)
        .map_err(Into::into)
}

#[utoipa::path(
    post,
    path = "/api/owner/locations/{locationId}/channels",
    request_body = CreateChannelRequest,
    responses((status = 201, description = "Channel created", body = ChannelRow)),
    tag = "channels"
)]
pub async fn create_channel(
    OwnerClaimsExt(claims): OwnerClaimsExt,
    Extension(auth): Extension<AuthState>,
    Extension(state): Extension<ChannelsState>,
    Path(location_id): Path<Uuid>,
    Json(req): Json<CreateChannelRequest>,
) -> Result<(StatusCode, Json<ChannelRow>), ApiError> {
    require_location_access(&auth, &claims, location_id).await?;
    let channel = state.repo.create(claims.user_id, location_id, req.kind, req.name).await?;
    Ok((StatusCode::CREATED, Json(channel)))
}

#[utoipa::path(
    patch,
    path = "/api/owner/locations/{locationId}/channels/{channelId}",
    request_body = UpdateChannelRequest,
    responses((status = 200, description = "Channel updated", body = ChannelRow)),
    tag = "channels"
)]
pub async fn update_channel(
    OwnerClaimsExt(claims): OwnerClaimsExt,
    Extension(auth): Extension<AuthState>,
    Extension(state): Extension<ChannelsState>,
    Path((location_id, channel_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateChannelRequest>,
) -> Result<Json<ChannelRow>, ApiError> {
    require_location_access(&auth, &claims, location_id).await?;
    state.repo.update(claims.user_id, location_id, channel_id, req.name, req.active).await?
        .ok_or_else(|| ApiError::not_found("Channel not found"))
        .map(Json)
}

#[utoipa::path(
    delete,
    path = "/api/owner/locations/{locationId}/channels/{channelId}",
    responses((status = 204, description = "Channel deleted")),
    tag = "channels"
)]
pub async fn delete_channel(
    OwnerClaimsExt(claims): OwnerClaimsExt,
    Extension(auth): Extension<AuthState>,
    Extension(state): Extension<ChannelsState>,
    Path((location_id, channel_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    require_location_access(&auth, &claims, location_id).await?;
    state.repo.delete(claims.user_id, location_id, channel_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_allowlist_parity() {
        use crate::modules::channel_attribution::CHANNEL_ALLOWLIST;
        // Each valid kind in the schema must match a value in the allowlist
        let kinds = vec![
            "web-direct", "qr", "nfc", "gbp", "apple-maps",
            "instagram", "facebook", "whatsapp", "telegram-tma",
            "kiosk", "widget", "agent", "other"
        ];
        for kind in kinds {
            assert!(CHANNEL_ALLOWLIST.contains(&kind), "kind {} not in CHANNEL_ALLOWLIST", kind);
        }
    }
}
