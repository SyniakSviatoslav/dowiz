//! `AuthConfig` — the S2 auth surface's config, ported from the JWT/dev-auth slice of
//! `packages/config/src/index.ts`'s `EnvSchema` + `assertDevAuthDisabledInProd` (boot-guard D).
//!
//! Kept SEPARATE from `crate::config::Config` (the S1 PORT/DATABASE_URL loader) deliberately:
//! S1's `Config` is the "surfaces that need them land their vars with the surface" pattern
//! (`config.rs` module doc). S2 is the surface that needs the JWT keys + dev-auth knobs, so they
//! land here. Loaded once at boot, fail-fast (collects every issue), never read piecemeal after.
//!
//! ## Dev-kid segregation (ADR-0003) is BOTH compile-time AND runtime here
//! - Compile-time: the dev keypair fields are only populated / consulted under
//!   `#[cfg(feature = "dev-routes")]`. A release binary built without that feature holds no dev
//!   public key and its verifier has no branch that could accept a dev kid (see `jwt.rs`).
//! - Runtime (belt): even in a `dev-routes` build, `accept_dev_kid()` additionally requires
//!   `NODE_ENV != production` AND a dev keypair present (`jwt.ts:91`), and `Self::from_map`
//!   reproduces boot-guard D — a `production` NODE_ENV with ANY dev-auth var set is a fatal boot
//!   error (`config/index.ts:223-237`).

use std::collections::HashMap;
use std::fmt;

/// The parsed auth environment. `dev` is `Some` only in a `dev-routes` build with the dev vars
/// present and `NODE_ENV != production`; `None` everywhere else (prod, or a release binary).
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub node_env: NodeEnv,
    /// `JWT_KID` — the prod key id written to every prod-minted token's header AND body.
    pub jwt_kid: String,
    /// `JWT_PRIVATE_KEY` PEM (RS256), `\n`-unescaped (`jwt.ts:16`). Used for signing.
    pub jwt_private_key_pem: String,
    /// `JWT_PUBLIC_KEY` PEM (RS256), `\n`-unescaped. Used for verification.
    pub jwt_public_key_pem: String,
    pub google_oauth_enabled: bool,
    pub app_base_url: String,
    /// The dev-auth slice — see the module doc. `#[cfg(feature = "dev-routes")]`-gated so the type
    /// itself can't carry dev key material into a release binary.
    #[cfg(feature = "dev-routes")]
    pub dev: Option<DevAuthConfig>,
    /// ADR-0003 layer 1: `ALLOW_DEV_LOGIN === 'true'`. Kept outside the `dev` sub-struct because
    /// the dev-guard's fail-closed decision needs it even when the keypair is absent.
    pub allow_dev_login: bool,
    /// ADR-0003 layer 1: `DEV_AUTH_SECRET` — the `x-dev-auth-secret` compare target.
    pub dev_auth_secret: Option<String>,
    pub telegram_bot_username: String,
}

#[cfg(feature = "dev-routes")]
#[derive(Debug, Clone)]
pub struct DevAuthConfig {
    pub jwt_dev_kid: String,
    pub jwt_dev_private_key_pem: String,
    pub jwt_dev_public_key_pem: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeEnv {
    Development,
    Test,
    Production,
}

impl NodeEnv {
    pub const fn is_production(self) -> bool {
        matches!(self, NodeEnv::Production)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthConfigError {
    pub issues: Vec<String>,
}

impl fmt::Display for AuthConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Invalid auth environment variables:")?;
        for issue in &self.issues {
            writeln!(f, "- {issue}")?;
        }
        Ok(())
    }
}

impl std::error::Error for AuthConfigError {}

/// PEM `\n`-unescaping parity (`jwt.ts:16` `raw.replace(/\\n/g, '\n')`): env carries keys with
/// literal backslash-n, which must become real newlines before `jsonwebtoken` parses the PEM.
pub fn unescape_pem(raw: &str) -> String {
    raw.replace("\\n", "\n")
}

impl AuthConfig {
    pub fn from_env() -> Result<Self, AuthConfigError> {
        Self::from_map(&std::env::vars().collect())
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one fail-fast validator that collects every issue before returning — splitting \
                  it would scatter the boot-guard-D invariant across helpers"
    )]
    fn from_map(vars: &HashMap<String, String>) -> Result<Self, AuthConfigError> {
        let mut issues = Vec::new();

        let node_env = match vars.get("NODE_ENV").map(String::as_str) {
            Some("development") => NodeEnv::Development,
            Some("test") => NodeEnv::Test,
            Some("production") => NodeEnv::Production,
            Some(other) => {
                issues.push(format!(
                    "NODE_ENV: must be development|test|production, got {other:?}"
                ));
                NodeEnv::Development
            }
            None => {
                issues.push("NODE_ENV: required, not set".to_string());
                NodeEnv::Development
            }
        };

        let jwt_kid = require_nonempty(vars, "JWT_KID", &mut issues);
        let jwt_private_key_pem =
            require_nonempty(vars, "JWT_PRIVATE_KEY", &mut issues).map(|s| unescape_pem(&s));
        let jwt_public_key_pem =
            require_nonempty(vars, "JWT_PUBLIC_KEY", &mut issues).map(|s| unescape_pem(&s));

        let google_oauth_enabled =
            vars.get("GOOGLE_OAUTH_ENABLED").map(String::as_str) == Some("true");
        let app_base_url = vars
            .get("APP_BASE_URL")
            .cloned()
            .unwrap_or_else(|| "https://dowiz.fly.dev".to_string());

        let allow_dev_login = vars.get("ALLOW_DEV_LOGIN").map(String::as_str) == Some("true");
        let dev_auth_secret = vars
            .get("DEV_AUTH_SECRET")
            .filter(|s| !s.is_empty())
            .cloned();
        let telegram_bot_username = vars
            .get("TELEGRAM_BOT_USERNAME")
            .filter(|s| !s.is_empty())
            .cloned()
            .unwrap_or_else(|| "dowiz_bot".to_string());

        // ── Boot-guard D (ADR-0003, `config/index.ts:223-237`) ──
        // A production box must NEVER carry a dev-auth surface. This fires on the DANGEROUS
        // direction only (NODE_ENV=production with any dev-auth knob set).
        if node_env.is_production() {
            let mut offenders = Vec::new();
            if allow_dev_login {
                offenders.push("ALLOW_DEV_LOGIN");
            }
            if dev_auth_secret.is_some() {
                offenders.push("DEV_AUTH_SECRET");
            }
            if vars.get("JWT_DEV_KID").is_some_and(|s| !s.is_empty()) {
                offenders.push("JWT_DEV_KID");
            }
            if vars
                .get("JWT_DEV_PRIVATE_KEY")
                .is_some_and(|s| !s.is_empty())
            {
                offenders.push("JWT_DEV_PRIVATE_KEY");
            }
            if vars
                .get("JWT_DEV_PUBLIC_KEY")
                .is_some_and(|s| !s.is_empty())
            {
                offenders.push("JWT_DEV_PUBLIC_KEY");
            }
            if !offenders.is_empty() {
                issues.push(format!(
                    "FATAL: dev-auth surface present on a production box (NODE_ENV=production): \
                     {} must be unset in production. Refusing to boot.",
                    offenders.join(", ")
                ));
            }
        }

        #[cfg(feature = "dev-routes")]
        let dev = Self::load_dev(vars, node_env, &mut issues);

        if !issues.is_empty() {
            return Err(AuthConfigError { issues });
        }

        Ok(AuthConfig {
            node_env,
            // Unreachable unwraps on the Ok path (issues empty ⇒ every require_nonempty was Some);
            // `.unwrap_or_default()` over `.expect()` so a future early-return reorder can't turn a
            // logic slip into a boot panic — same posture as `crate::config::Config`.
            jwt_kid: jwt_kid.unwrap_or_default(),
            jwt_private_key_pem: jwt_private_key_pem.unwrap_or_default(),
            jwt_public_key_pem: jwt_public_key_pem.unwrap_or_default(),
            google_oauth_enabled,
            app_base_url,
            #[cfg(feature = "dev-routes")]
            dev,
            allow_dev_login,
            dev_auth_secret,
            telegram_bot_username,
        })
    }

    /// Loads the dev keypair — only compiled in a `dev-routes` build. Returns `None` (not an
    /// error) when the dev keypair is absent or `NODE_ENV == production`: a dev-routes binary
    /// running on a non-dev box simply has no dev-kid acceptance branch, exactly like Node where
    /// `getDevPublicKey()` returns null and `acceptDevKid` short-circuits false (`jwt.ts:37-41,91`).
    #[cfg(feature = "dev-routes")]
    fn load_dev(
        vars: &HashMap<String, String>,
        node_env: NodeEnv,
        _issues: &mut [String],
    ) -> Option<DevAuthConfig> {
        if node_env.is_production() {
            return None;
        }
        let kid = vars.get("JWT_DEV_KID").filter(|s| !s.is_empty())?;
        let priv_raw = vars.get("JWT_DEV_PRIVATE_KEY").filter(|s| !s.is_empty())?;
        let pub_raw = vars.get("JWT_DEV_PUBLIC_KEY").filter(|s| !s.is_empty())?;
        Some(DevAuthConfig {
            jwt_dev_kid: kid.clone(),
            jwt_dev_private_key_pem: unescape_pem(priv_raw),
            jwt_dev_public_key_pem: unescape_pem(pub_raw),
        })
    }

    /// ADR-0003 layers 1+3 collapsed: dev/test auth bypasses are permitted iff BOTH
    /// `ALLOW_DEV_LOGIN` AND `DEV_AUTH_SECRET` are set (`dev-guard.ts:30-32`). The secret alone
    /// is not enough. This is the single source for every dev mint/gate decision.
    pub fn dev_login_allowed(&self) -> bool {
        self.allow_dev_login && self.dev_auth_secret.is_some()
    }
}

fn require_nonempty(
    vars: &HashMap<String, String>,
    key: &str,
    issues: &mut Vec<String>,
) -> Option<String> {
    match vars.get(key) {
        Some(v) if !v.is_empty() => Some(v.clone()),
        _ => {
            issues.push(format!("{key}: required, not set"));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Vec<(&'static str, &'static str)> {
        vec![
            ("NODE_ENV", "development"),
            ("JWT_KID", "prod-kid-1"),
            (
                "JWT_PRIVATE_KEY",
                "-----BEGIN PRIVATE KEY-----\\nAAA\\n-----END PRIVATE KEY-----",
            ),
            (
                "JWT_PUBLIC_KEY",
                "-----BEGIN PUBLIC KEY-----\\nBBB\\n-----END PUBLIC KEY-----",
            ),
            ("APP_BASE_URL", "https://dowiz.fly.dev"),
        ]
    }

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn unescape_pem_turns_literal_backslash_n_into_newlines() {
        assert_eq!(unescape_pem("a\\nb\\nc"), "a\nb\nc");
    }

    #[test]
    fn loads_valid_config_and_unescapes_pem() {
        let cfg = AuthConfig::from_map(&map(&base())).unwrap();
        assert_eq!(cfg.jwt_kid, "prod-kid-1");
        assert!(cfg.jwt_private_key_pem.contains('\n'));
        assert!(!cfg.jwt_private_key_pem.contains("\\n"));
        assert_eq!(cfg.node_env, NodeEnv::Development);
    }

    #[test]
    fn rejects_missing_keys_collecting_all_issues() {
        let vars = map(&[("NODE_ENV", "development"), ("APP_BASE_URL", "https://x")]);
        let err = AuthConfig::from_map(&vars).unwrap_err();
        assert!(err.issues.iter().any(|i| i.contains("JWT_KID")));
        assert!(err.issues.iter().any(|i| i.contains("JWT_PRIVATE_KEY")));
        assert!(err.issues.iter().any(|i| i.contains("JWT_PUBLIC_KEY")));
    }

    #[test]
    fn boot_guard_d_fatals_when_prod_has_dev_auth_secret() {
        let mut pairs = base();
        pairs[0] = ("NODE_ENV", "production");
        pairs.push(("DEV_AUTH_SECRET", "leaked-secret"));
        let err = AuthConfig::from_map(&map(&pairs)).unwrap_err();
        assert!(
            err.issues
                .iter()
                .any(|i| i.contains("FATAL") && i.contains("DEV_AUTH_SECRET")),
            "prod + DEV_AUTH_SECRET must refuse to boot (ADR-0003 boot-guard D)"
        );
    }

    #[test]
    fn boot_guard_d_fatals_when_prod_has_allow_dev_login() {
        let mut pairs = base();
        pairs[0] = ("NODE_ENV", "production");
        pairs.push(("ALLOW_DEV_LOGIN", "true"));
        let err = AuthConfig::from_map(&map(&pairs)).unwrap_err();
        assert!(
            err.issues
                .iter()
                .any(|i| i.contains("FATAL") && i.contains("ALLOW_DEV_LOGIN"))
        );
    }

    #[test]
    fn dev_login_allowed_requires_both_flag_and_secret() {
        let mut pairs = base();
        pairs.push(("ALLOW_DEV_LOGIN", "true"));
        let only_flag = AuthConfig::from_map(&map(&pairs)).unwrap();
        assert!(!only_flag.dev_login_allowed(), "flag alone is not enough");

        pairs.push(("DEV_AUTH_SECRET", "s3cr3t"));
        let both = AuthConfig::from_map(&map(&pairs)).unwrap();
        assert!(both.dev_login_allowed());
    }

    #[test]
    fn telegram_bot_username_defaults_to_dowiz_bot() {
        let cfg = AuthConfig::from_map(&map(&base())).unwrap();
        assert_eq!(cfg.telegram_bot_username, "dowiz_bot");
    }
}
