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
}
