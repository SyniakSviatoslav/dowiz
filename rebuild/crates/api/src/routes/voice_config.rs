//! S1 storefront-read: `getVoiceConfig`.
//! Source: `apps/api/src/routes/public/voice-config.ts:11-15` + `apps/api/src/lib/voice-flag.ts`.
//!
//! Raw `std::env::var` reads (not the `Config` struct) — this mirrors Node's OWN pattern:
//! `voice-flag.ts` reads `process.env.VOICE_CONTROL_ENABLED`/`VOICE_KILL` directly, un-migrated
//! into its Zod `EnvSchema` (REBUILD-MAP inventory/10 §5 "~20 shadow" raw reads). Carrying that
//! forward as a raw read here is CARRY-VERBATIM of the actual current behavior, not a shortcut —
//! extending `crates/api/src/config.rs`'s strict-validated surface for a deploy-time kill-switch
//! that Node itself never validates would be adding stricter behavior than the contract has.

use axum::http::header;
use axum::response::IntoResponse;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct VoiceConfigResponse {
    pub enabled: bool,
}

/// Ports `isVoiceEnabled()` (`voice-flag.ts:11-12`) verbatim:
/// `VOICE_CONTROL_ENABLED === 'true' && VOICE_KILL !== 'true'`. Pure over explicit `Option<&str>`
/// so the boot-time env values never need mocking through `std::env` in a test.
pub fn is_voice_enabled(control: Option<&str>, kill: Option<&str>) -> bool {
    control == Some("true") && kill != Some("true")
}

/// `GET /api/public/voice-config` — source: `voice-config.ts:11-15`. No DB, no auth, no PII.
#[utoipa::path(
    get,
    path = "/api/public/voice-config",
    responses((status = 200, description = "Voice kill-switch boolean", body = VoiceConfigResponse)),
    tag = "voice"
)]
pub async fn get_voice_config() -> impl IntoResponse {
    let enabled = is_voice_enabled(
        std::env::var("VOICE_CONTROL_ENABLED").ok().as_deref(),
        std::env::var("VOICE_KILL").ok().as_deref(),
    );
    (
        [(header::CACHE_CONTROL, "no-store")],
        axum::Json(VoiceConfigResponse { enabled }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_voice_enabled_requires_control_true_and_kill_not_true() {
        assert!(is_voice_enabled(Some("true"), None));
        assert!(!is_voice_enabled(None, None), "default OFF when unset");
        assert!(
            !is_voice_enabled(Some("true"), Some("true")),
            "VOICE_KILL wins"
        );
        assert!(!is_voice_enabled(Some("false"), None));
        assert!(
            !is_voice_enabled(Some("TRUE"), None),
            "must be exact lowercase 'true'"
        );
    }

    #[tokio::test]
    async fn get_voice_config_sets_no_store_header() {
        let response = get_voice_config().await.into_response();
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::CACHE_CONTROL)
                .unwrap(),
            "no-store"
        );
    }
}
