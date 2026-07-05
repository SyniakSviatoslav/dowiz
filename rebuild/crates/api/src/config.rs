//! Boot config — the Rust analog of `packages/config/src/index.ts`'s `EnvSchema`/`loadEnv`:
//! parsed and validated ONCE at startup, fail-fast (collects every issue before returning, same
//! as Zod's `safeParse`), never read piecemeal via scattered `env::var()` calls afterward.
//!
//! Phase A scope only (per the build brief): `PORT`, `DATABASE_URL_OPERATIONAL`,
//! `DATABASE_URL_SESSION`. The other ~77 EnvSchema vars (REBUILD-MAP inventory/10 §5: "80
//! EnvSchema + 48 raw reads") land with the surfaces that need them — adding them here before
//! there is a consumer would just be dead validation.

use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub port: u16,
    pub database_url_operational: String,
    pub database_url_session: String,
    pub media: MediaConfig,
    pub notifications: NotificationsConfig,
}

/// Wraps a secret so `#[derive(Debug)]`/`{:?}` on any struct holding one never prints the raw
/// value — the S8 jobs/notifications council packet's 🔴 requirement for the VAPID private key
/// ("NEVER logged, NEVER in an error payload, NEVER in a DLQ row") generalizes to every secret
/// this surface carries (`RESEND_API_KEY`, `TELEGRAM_BOT_TOKEN`, `TELEGRAM_BOT_SECRET`) — a
/// `tracing::error!(?config, ...)` or a `Debug`-derived error payload must not leak any of them.
/// Deliberately stronger than `auth::config::AuthConfig` (whose JWT PEM fields are plain
/// `String`s under a bare `#[derive(Debug)]`) — that module's posture predates this council
/// packet's explicit red-line; this type is the new, correct default for anything S8 touches.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Secret(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

/// S8 jobs/notifications surface config (`docs/design/rebuild-jobs-s8-council/proposal.md` §4).
/// Every field is optional at the `Config` level — same "stays dark rather than FATAL-exiting
/// the whole process" posture as `MediaConfig`: an unconfigured channel soft-disables (the VAPID
/// push adapter registers only when BOTH keys are set; Telegram/email adapters degrade the same
/// way their Node originals do) rather than blocking boot. A value that IS present but malformed
/// (e.g. a non-base64url VAPID key) is still a fail-fast boot error, same as `MediaConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationsConfig {
    /// `VAPID_PUBLIC_KEY` — already read raw by `routes::vapid::get_vapid_public_key` (S1,
    /// un-migrated CARRY). Duplicated here (not moved) so the push-adapter wiring has a single,
    /// validated source without disturbing that already-shipped S1 route.
    pub vapid_public_key: Option<String>,
    /// `VAPID_PRIVATE_KEY` — raw base64url (URL-safe, no padding) EC private key scalar, the
    /// Node/PHP/`web-push`-generator convention (`VapidSignatureBuilder::from_base64`, NOT a PEM).
    /// 🔴 never logged/DLQ'd/committed — see `Secret`'s doc.
    pub vapid_private_key: Option<Secret>,
    /// `VAPID_SUBJECT` — a `mailto:`/https URI per RFC 8292. Source has TWO inconsistent
    /// fallbacks (`bootstrap/notifications.ts:20-23`: `mailto:admin@deliveryos.local`, the
    /// adapter-registration path that actually runs; `workers/index.ts:92`: `push@deliveryos.app`,
    /// missing the `mailto:` scheme entirely — a pre-existing inconsistency, NOT silently
    /// unified). This port CARRIES the registration-path default (`bootstrap`'s), since that is
    /// the value that is actually live in production today.
    pub vapid_subject: String,
    /// `RESEND_API_KEY` — email adapter, ops-alert-only (Q-EMAIL-DIRECT, §4.3). 🔴 secret.
    pub resend_api_key: Option<Secret>,
    /// `TELEGRAM_BOT_TOKEN` — send-path bot token. 🔴 secret.
    pub telegram_bot_token: Option<Secret>,
    /// `TELEGRAM_BOT_SECRET` — webhook header compare target (Q4 🔴, fail-closed). 🔴 secret.
    pub telegram_bot_secret: Option<Secret>,
    /// `TELEGRAM_BOT_USERNAME` — not a secret (used to build a `t.me/<username>` deep link),
    /// default matches `routes/auth.ts:194`.
    pub telegram_bot_username: String,
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        NotificationsConfig {
            vapid_public_key: None,
            vapid_private_key: None,
            vapid_subject: "mailto:admin@deliveryos.local".to_string(),
            resend_api_key: None,
            telegram_bot_token: None,
            telegram_bot_secret: None,
            telegram_bot_username: "dowiz_bot".to_string(),
        }
    }
}

/// S4 media surface config (`docs/design/rebuild-media-s4-council/resolution.md` REV-S4-6:
/// "numeric global cap + kill-switch env in EnvSchema, red-line: no raw env reads" — so this,
/// not a scattered `std::env::var` at the route layer, is the one parsed/validated source).
/// Every field is OPTIONAL AT THE `Config` LEVEL (S4, like S2/S3, stays dark rather than
/// FATAL-exiting the whole process when its own env is absent — `main.rs` gates the media router
/// the same way it gates S2/S3) but each field that IS present is shape-validated here, once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaConfig {
    /// HMAC-SHA256 signing secret for the product-media upload token (REV-S4-2 token-proxy-PUT).
    /// Hex-encoded, ≥32 raw bytes (256 bits — matches the HMAC-SHA256 block/output size, no
    /// reason to accept a weaker key for a red-line signing secret). Absent → the product-media
    /// presign op degrades to 503 SERVICE_UNAVAILABLE (mirrors the old TS behavior when
    /// `R2_BUCKET`/`R2_ENDPOINT` are unset, `product-media.ts:138-140`), not a boot failure.
    pub upload_token_secret_hex: Option<String>,
    /// Kill-switch for the UNAUTHENTICATED entry-photo route (REV-S4-6 Q4b floor) — default ON
    /// (`true`) to carry parity with the current always-on Node route; ops can flip it off
    /// instantly without a deploy. `ENTRY_PHOTO_ENABLED=false` is the only way to disable it.
    pub entry_photo_enabled: bool,
    /// Global (cross-tenant, cross-IP) request cap for the entry-photo route, requests/minute.
    /// Breaker M3 named the exact gap this closes: the packet's own "global rate cap" proposal
    /// shipped with NO number, making it unverifiable as a control. Default **60/min**: the
    /// packet's own back-of-envelope (§2 — "low-hundreds of orders/day system-wide," entry-photo
    /// ≤1 per opted-in order) puts legitimate peak at a handful/minute; 60/min is generously above
    /// that (a lunch-rush burst of concurrent checkouts must not 429) while still bounding a
    /// botnet's fan-out to a fixed, auditable ceiling shared across every tenant — a real number,
    /// not "unlimited", and small enough that the per-IP 8/min carry (REV-S4-6) is still the
    /// FIRST line of defense for any single abusive IP. Override via
    /// `ENTRY_PHOTO_GLOBAL_CAP_PER_MINUTE` if this default proves wrong in practice.
    pub entry_photo_global_cap_per_minute: u32,
}

impl Default for MediaConfig {
    fn default() -> Self {
        MediaConfig {
            upload_token_secret_hex: None,
            entry_photo_enabled: true,
            entry_photo_global_cap_per_minute: 60,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    pub issues: Vec<String>,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Invalid environment variables:")?;
        for issue in &self.issues {
            writeln!(f, "- {issue}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ConfigError {}

const DEFAULT_PORT: u16 = 8080;

impl Config {
    /// Load from the real process environment. Thin wrapper around `from_map` so the actual
    /// validation logic is unit-testable without mutating `std::env` (which is process-global
    /// and racy under parallel test execution).
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_map(&std::env::vars().collect())
    }

    fn from_map(vars: &HashMap<String, String>) -> Result<Self, ConfigError> {
        let mut issues = Vec::new();

        let port = match vars.get("PORT") {
            None => DEFAULT_PORT,
            Some(raw) => match raw.parse::<u16>() {
                Ok(0) => {
                    issues.push("PORT: must be a positive integer, got 0".to_string());
                    DEFAULT_PORT
                }
                Ok(port) => port,
                Err(_) => {
                    issues.push(format!("PORT: invalid u16: {raw:?}"));
                    DEFAULT_PORT
                }
            },
        };

        let database_url_operational =
            require_postgres_url(vars, "DATABASE_URL_OPERATIONAL", &mut issues);
        let database_url_session = require_postgres_url(vars, "DATABASE_URL_SESSION", &mut issues);
        let media = MediaConfig::from_map(vars, &mut issues);
        let notifications = NotificationsConfig::from_map(vars, &mut issues);

        if !issues.is_empty() {
            return Err(ConfigError { issues });
        }

        Ok(Config {
            port,
            // Both `require_postgres_url` calls return `Some` whenever `issues` stays empty, so
            // these `unwrap_or_default`s are unreachable on the `Ok` path — kept over
            // `.unwrap()`/`.expect()` so a future refactor that reorders the early-return can't
            // turn a logic bug into a boot-time panic (the whole point of a config loader that
            // is supposed to fail with a message, not a panic).
            database_url_operational: database_url_operational.unwrap_or_default(),
            database_url_session: database_url_session.unwrap_or_default(),
            media,
            notifications,
        })
    }
}

/// A deliberately light check — enough to catch "unset" and "obviously not a Postgres URL"
/// (the two failure modes that matter at boot) without pulling in a full URL-parsing dependency
/// for a single scheme check.
fn require_postgres_url(
    vars: &HashMap<String, String>,
    key: &str,
    issues: &mut Vec<String>,
) -> Option<String> {
    match vars.get(key) {
        None => {
            issues.push(format!("{key}: required, not set"));
            None
        }
        Some(value) if value.is_empty() => {
            issues.push(format!("{key}: required, not set"));
            None
        }
        Some(value)
            if !(value.starts_with("postgres://") || value.starts_with("postgresql://")) =>
        {
            issues.push(format!("{key}: must be a postgres:// or postgresql:// URL"));
            None
        }
        Some(value) => Some(value.clone()),
    }
}

impl MediaConfig {
    /// Parses the S4 media env, pushing any shape issues into the shared `issues` vec (same
    /// fail-fast-and-collect-everything posture as `Config::from_map`) rather than returning its
    /// own `Result` — a malformed `MEDIA_UPLOAD_TOKEN_SECRET`/`ENTRY_PHOTO_GLOBAL_CAP_PER_MINUTE`
    /// is a genuine boot-config error (it is NOT one of the "S4 stays dark" absence cases below),
    /// so it must fail the SAME boot-time validation the DB URLs do, not silently fall back.
    fn from_map(vars: &HashMap<String, String>, issues: &mut Vec<String>) -> Self {
        let upload_token_secret_hex = match vars.get("MEDIA_UPLOAD_TOKEN_SECRET") {
            None => None,
            Some(value) if value.is_empty() => None,
            Some(value) => match hex::decode(value) {
                Ok(bytes) if bytes.len() >= 32 => Some(value.clone()),
                Ok(bytes) => {
                    issues.push(format!(
                        "MEDIA_UPLOAD_TOKEN_SECRET: must decode to >=32 bytes, got {}",
                        bytes.len()
                    ));
                    None
                }
                Err(_) => {
                    issues.push("MEDIA_UPLOAD_TOKEN_SECRET: must be a hex string".to_string());
                    None
                }
            },
        };

        let entry_photo_enabled = match vars.get("ENTRY_PHOTO_ENABLED").map(String::as_str) {
            None => true,
            Some("true") => true,
            Some("false") => false,
            Some(other) => {
                issues.push(format!(
                    "ENTRY_PHOTO_ENABLED: must be \"true\" or \"false\", got {other:?}"
                ));
                true
            }
        };

        let entry_photo_global_cap_per_minute = match vars.get("ENTRY_PHOTO_GLOBAL_CAP_PER_MINUTE")
        {
            None => 60,
            Some(raw) => match raw.parse::<u32>() {
                Ok(0) => {
                    issues.push(
                        "ENTRY_PHOTO_GLOBAL_CAP_PER_MINUTE: must be a positive integer, got 0"
                            .to_string(),
                    );
                    60
                }
                Ok(n) => n,
                Err(_) => {
                    issues.push(format!(
                        "ENTRY_PHOTO_GLOBAL_CAP_PER_MINUTE: invalid u32: {raw:?}"
                    ));
                    60
                }
            },
        };

        MediaConfig {
            upload_token_secret_hex,
            entry_photo_enabled,
            entry_photo_global_cap_per_minute,
        }
    }
}

impl NotificationsConfig {
    /// Same "collect every issue, never panic" posture as `MediaConfig::from_map` — an absent
    /// value soft-disables its channel (pushed into the `Default` below by the caller NOT calling
    /// this at all in that branch — see `from_map`'s per-field `match`), a PRESENT-but-malformed
    /// one is a genuine boot-config error.
    fn from_map(vars: &HashMap<String, String>, issues: &mut Vec<String>) -> Self {
        use base64::Engine;
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;

        let vapid_public_key = non_empty(vars, "VAPID_PUBLIC_KEY");

        // A P-256 private key scalar is exactly 32 bytes — `VapidSignatureBuilder::from_base64`
        // (the `web-push` crate) decodes the SAME raw-base64url convention Node's `web-push`
        // generator + PHP/other VAPID libraries use (NOT a PEM). Validating the decoded LENGTH
        // here (not just "is it valid base64") catches a truncated/wrong-key paste at boot,
        // before the adapter ever tries to sign with it.
        let vapid_private_key = match vars.get("VAPID_PRIVATE_KEY") {
            None => None,
            Some(value) if value.is_empty() => None,
            Some(value) => match URL_SAFE_NO_PAD.decode(value) {
                Ok(bytes) if bytes.len() == 32 => Some(Secret::new(value.clone())),
                Ok(bytes) => {
                    issues.push(format!(
                        "VAPID_PRIVATE_KEY: must decode (base64url, no padding) to exactly 32 bytes, got {}",
                        bytes.len()
                    ));
                    None
                }
                Err(_) => {
                    issues.push(
                        "VAPID_PRIVATE_KEY: must be base64url (no padding) encoded".to_string(),
                    );
                    None
                }
            },
        };

        let vapid_subject = match vars.get("VAPID_SUBJECT") {
            None => "mailto:admin@deliveryos.local".to_string(),
            Some(value) if value.is_empty() => "mailto:admin@deliveryos.local".to_string(),
            Some(value) => value.clone(),
        };

        let resend_api_key = non_empty(vars, "RESEND_API_KEY").map(Secret::new);
        let telegram_bot_token = non_empty(vars, "TELEGRAM_BOT_TOKEN").map(Secret::new);
        let telegram_bot_secret = non_empty(vars, "TELEGRAM_BOT_SECRET").map(Secret::new);
        let telegram_bot_username =
            non_empty(vars, "TELEGRAM_BOT_USERNAME").unwrap_or_else(|| "dowiz_bot".to_string());

        NotificationsConfig {
            vapid_public_key,
            vapid_private_key,
            vapid_subject,
            resend_api_key,
            telegram_bot_token,
            telegram_bot_secret,
            telegram_bot_username,
        }
    }

    /// Both VAPID keys must be present together (`bootstrap/notifications.ts:52` CARRY, §4.1) —
    /// a public key with no private key (or vice versa) cannot sign, so the push adapter must
    /// stay dark rather than register half-configured.
    pub fn vapid_ready(&self) -> bool {
        self.vapid_public_key.is_some() && self.vapid_private_key.is_some()
    }
}

fn non_empty(vars: &HashMap<String, String>, key: &str) -> Option<String> {
    vars.get(key).filter(|v| !v.is_empty()).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn loads_valid_config() {
        let vars = map(&[
            ("PORT", "9090"),
            (
                "DATABASE_URL_OPERATIONAL",
                "postgres://user:pass@host:6543/db",
            ),
            ("DATABASE_URL_SESSION", "postgres://user:pass@host:5432/db"),
        ]);
        let config = Config::from_map(&vars).unwrap();
        assert_eq!(config.port, 9090);
        assert_eq!(
            config.database_url_operational,
            "postgres://user:pass@host:6543/db"
        );
        assert_eq!(
            config.database_url_session,
            "postgres://user:pass@host:5432/db"
        );
    }

    #[test]
    fn port_defaults_when_unset() {
        let vars = map(&[
            ("DATABASE_URL_OPERATIONAL", "postgres://host:6543/db"),
            ("DATABASE_URL_SESSION", "postgres://host:5432/db"),
        ]);
        let config = Config::from_map(&vars).unwrap();
        assert_eq!(config.port, DEFAULT_PORT);
    }

    #[test]
    fn rejects_missing_database_urls_collecting_all_issues() {
        let vars = map(&[("PORT", "8080")]);
        let err = Config::from_map(&vars).unwrap_err();
        assert_eq!(
            err.issues.len(),
            2,
            "both missing vars must be reported, not just the first"
        );
        assert!(
            err.issues
                .iter()
                .any(|i| i.contains("DATABASE_URL_OPERATIONAL"))
        );
        assert!(
            err.issues
                .iter()
                .any(|i| i.contains("DATABASE_URL_SESSION"))
        );
    }

    #[test]
    fn rejects_non_postgres_url() {
        let vars = map(&[
            ("DATABASE_URL_OPERATIONAL", "mysql://host/db"),
            ("DATABASE_URL_SESSION", "postgres://host:5432/db"),
        ]);
        let err = Config::from_map(&vars).unwrap_err();
        assert_eq!(
            err.issues,
            vec!["DATABASE_URL_OPERATIONAL: must be a postgres:// or postgresql:// URL"]
        );
    }

    #[test]
    fn rejects_invalid_and_zero_port() {
        let base = [
            ("DATABASE_URL_OPERATIONAL", "postgres://host:6543/db"),
            ("DATABASE_URL_SESSION", "postgres://host:5432/db"),
        ];

        let mut vars = map(&base);
        vars.insert("PORT".to_string(), "not-a-number".to_string());
        assert!(
            Config::from_map(&vars)
                .unwrap_err()
                .issues
                .iter()
                .any(|i| i.starts_with("PORT"))
        );

        let mut vars = map(&base);
        vars.insert("PORT".to_string(), "0".to_string());
        assert!(
            Config::from_map(&vars)
                .unwrap_err()
                .issues
                .iter()
                .any(|i| i.contains("PORT"))
        );
    }

    #[test]
    fn error_display_lists_every_issue() {
        let err = ConfigError {
            issues: vec!["A: bad".to_string(), "B: bad".to_string()],
        };
        let rendered = err.to_string();
        assert!(rendered.contains("- A: bad"));
        assert!(rendered.contains("- B: bad"));
    }

    // ── S4 MediaConfig (REV-S4-6: numeric cap + kill-switch, red-line: no raw env reads) ──

    fn base_db_vars() -> Vec<(&'static str, &'static str)> {
        vec![
            ("DATABASE_URL_OPERATIONAL", "postgres://host:6543/db"),
            ("DATABASE_URL_SESSION", "postgres://host:5432/db"),
        ]
    }

    #[test]
    fn media_config_defaults_when_unset() {
        let vars = map(&base_db_vars());
        let config = Config::from_map(&vars).unwrap();
        assert_eq!(config.media.upload_token_secret_hex, None);
        assert!(
            config.media.entry_photo_enabled,
            "default ON — carries parity"
        );
        assert_eq!(config.media.entry_photo_global_cap_per_minute, 60);
    }

    #[test]
    fn media_config_kill_switch_can_disable_entry_photo() {
        let mut base = base_db_vars();
        base.push(("ENTRY_PHOTO_ENABLED", "false"));
        let config = Config::from_map(&map(&base)).unwrap();
        assert!(!config.media.entry_photo_enabled);
    }

    #[test]
    fn media_config_rejects_malformed_entry_photo_enabled() {
        let mut base = base_db_vars();
        base.push(("ENTRY_PHOTO_ENABLED", "yes"));
        let err = Config::from_map(&map(&base)).unwrap_err();
        assert!(
            err.issues
                .iter()
                .any(|i| i.starts_with("ENTRY_PHOTO_ENABLED"))
        );
    }

    #[test]
    fn media_config_rejects_zero_global_cap() {
        let mut base = base_db_vars();
        base.push(("ENTRY_PHOTO_GLOBAL_CAP_PER_MINUTE", "0"));
        let err = Config::from_map(&map(&base)).unwrap_err();
        assert!(
            err.issues
                .iter()
                .any(|i| i.contains("ENTRY_PHOTO_GLOBAL_CAP_PER_MINUTE"))
        );
    }

    #[test]
    fn media_config_accepts_a_valid_hex_secret_of_32_bytes() {
        let mut base = base_db_vars();
        let secret = "ab".repeat(32); // 32 bytes hex-encoded = 64 hex chars.
        base.push(("MEDIA_UPLOAD_TOKEN_SECRET", secret.as_str()));
        let config = Config::from_map(&map(&base)).unwrap();
        assert_eq!(config.media.upload_token_secret_hex, Some(secret));
    }

    #[test]
    fn media_config_rejects_a_too_short_secret() {
        let mut base = base_db_vars();
        base.push(("MEDIA_UPLOAD_TOKEN_SECRET", "abcd"));
        let err = Config::from_map(&map(&base)).unwrap_err();
        assert!(
            err.issues
                .iter()
                .any(|i| i.starts_with("MEDIA_UPLOAD_TOKEN_SECRET"))
        );
    }

    #[test]
    fn media_config_rejects_non_hex_secret() {
        let mut base = base_db_vars();
        base.push(("MEDIA_UPLOAD_TOKEN_SECRET", "not-hex-at-all!!"));
        let err = Config::from_map(&map(&base)).unwrap_err();
        assert!(
            err.issues
                .iter()
                .any(|i| i.starts_with("MEDIA_UPLOAD_TOKEN_SECRET"))
        );
    }

    // ── S8 NotificationsConfig (docs/design/rebuild-jobs-s8-council/) ──

    /// The exact 32-byte-decoded base64url test key `web-push`'s own doctest uses
    /// (`VapidSignatureBuilder::from_base64_no_sub` example) — a real, valid VAPID private key
    /// shape, not an arbitrary string.
    const TEST_VAPID_PRIVATE_KEY: &str = "IQ9Ur0ykXoHS9gzfYX0aBjy9lvdrjx_PFUXmie9YRcY";

    #[test]
    fn notifications_config_defaults_to_fully_dark_when_unset() {
        let vars = map(&base_db_vars());
        let config = Config::from_map(&vars).unwrap();
        assert_eq!(config.notifications.vapid_public_key, None);
        assert!(config.notifications.vapid_private_key.is_none());
        assert!(!config.notifications.vapid_ready());
        assert_eq!(
            config.notifications.vapid_subject, "mailto:admin@deliveryos.local",
            "carries the bootstrap.ts registration-path default, not the workers/index.ts one \
             (see NotificationsConfig::vapid_subject doc — the two disagree upstream)"
        );
        assert!(config.notifications.resend_api_key.is_none());
        assert_eq!(config.notifications.telegram_bot_username, "dowiz_bot");
    }

    #[test]
    fn notifications_config_vapid_ready_requires_both_keys() {
        let mut base = base_db_vars();
        base.push(("VAPID_PUBLIC_KEY", "some-public-key"));
        let config = Config::from_map(&map(&base)).unwrap();
        assert!(
            !config.notifications.vapid_ready(),
            "public key alone must not flip the adapter on"
        );

        let mut base = base_db_vars();
        base.push(("VAPID_PUBLIC_KEY", "some-public-key"));
        base.push(("VAPID_PRIVATE_KEY", TEST_VAPID_PRIVATE_KEY));
        let config = Config::from_map(&map(&base)).unwrap();
        assert!(config.notifications.vapid_ready());
    }

    #[test]
    fn notifications_config_rejects_malformed_vapid_private_key() {
        let mut base = base_db_vars();
        base.push(("VAPID_PRIVATE_KEY", "not-valid-base64url-!!!"));
        let err = Config::from_map(&map(&base)).unwrap_err();
        assert!(
            err.issues
                .iter()
                .any(|i| i.starts_with("VAPID_PRIVATE_KEY"))
        );
    }

    #[test]
    fn notifications_config_rejects_wrong_length_vapid_private_key() {
        let mut base = base_db_vars();
        // Valid base64url, but decodes to far fewer than the required 32 bytes.
        base.push(("VAPID_PRIVATE_KEY", "YWJj"));
        let err = Config::from_map(&map(&base)).unwrap_err();
        assert!(
            err.issues
                .iter()
                .any(|i| i.starts_with("VAPID_PRIVATE_KEY") && i.contains("32 bytes"))
        );
    }

    /// REV-S8 VAPID guardrail: the private key must be structurally absent from every `Debug`
    /// format of anything that carries it — `Secret`'s `fmt::Debug` never touches the wrapped
    /// `String`, so this holds independent of what value is inside. Proven both on the bare
    /// `Secret` and on the whole `Config` (the shape a `tracing::error!(?config, ...)` or a
    /// `Debug`-derived error payload would actually print).
    #[test]
    fn vapid_private_key_is_absent_from_every_debug_format() {
        let secret_value = "super-secret-vapid-scalar-value";
        let secret = Secret::new(secret_value);
        let rendered = format!("{secret:?}");
        assert!(!rendered.contains(secret_value));
        assert_eq!(rendered, "Secret(<redacted>)");

        let mut base = base_db_vars();
        base.push(("VAPID_PRIVATE_KEY", TEST_VAPID_PRIVATE_KEY));
        base.push(("RESEND_API_KEY", "re_super_secret_key"));
        base.push(("TELEGRAM_BOT_TOKEN", "123:AA_super_secret_bot_token"));
        base.push(("TELEGRAM_BOT_SECRET", "webhook-secret-value"));
        let config = Config::from_map(&map(&base)).unwrap();
        let rendered_config = format!("{config:?}");
        assert!(!rendered_config.contains(TEST_VAPID_PRIVATE_KEY));
        assert!(!rendered_config.contains("re_super_secret_key"));
        assert!(!rendered_config.contains("123:AA_super_secret_bot_token"));
        assert!(!rendered_config.contains("webhook-secret-value"));
    }

    #[test]
    fn notifications_config_secrets_are_absent_when_unset() {
        let vars = map(&base_db_vars());
        let config = Config::from_map(&vars).unwrap();
        assert!(config.notifications.telegram_bot_token.is_none());
        assert!(config.notifications.telegram_bot_secret.is_none());
    }
}
