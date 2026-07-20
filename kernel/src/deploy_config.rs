//! C2 — DEPLOY CONFIG: roster + provider selection + default currency are load-bearing
//! deployment inputs, NOT compiled-in constants. They are read from a deploy config at
//! boot (operator ruling D14/C2, 2026-07-20).
//!
//! Std-only, serde-free: this keeps the kernel's compile-firewall discipline (no external
//! crate sneaks in through a config parser) and makes the module trivially testable offline.
//! The format is a small line-based dialect:
//!
//! ```text
//! default_currency = EUR
//! active_providers = stripe:eu, cash:cod
//! [roster]
//! node = 0000000000000000000000000000000000000000000000000000000000000001 : courier
//! node = 0000000000000000000000000000000000000000000000000000000000000002 : vendor
//! ```
//!
//! `currency` is intentionally NOT hardcoded (see `payment_provider::currency_not_hardcoded`):
//! the config selects it, proving it is deployment-driven.

use crate::money::Currency;
use std::str::FromStr;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterKind {
    Courier,
    Vendor,
    Hub,
}

impl FromStr for RosterKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "courier" => Ok(RosterKind::Courier),
            "vendor" => Ok(RosterKind::Vendor),
            "hub" => Ok(RosterKind::Hub),
            other => Err(format!("unknown roster kind: {other}")),
        }
    }
}

/// One enrolled node in the deployment roster.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterEntry {
    pub id: [u8; 32],
    pub kind: RosterKind,
}

/// A deploy-time configuration: the enrolled roster, the active payment providers, and the
/// default currency. None of these are compiled in — they are supplied at boot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployConfig {
    pub default_currency: Currency,
    pub active_providers: Vec<String>,
    pub roster: Vec<RosterEntry>,
}

impl DeployConfig {
    /// Parse the line-based deploy config dialect. Fail-closed: an unparseable line or an
    /// unknown currency/provider-kind returns `Err` rather than silently defaulting.
    pub fn parse(src: &str) -> Result<Self, String> {
        let mut default_currency = None;
        let mut active_providers = Vec::new();
        let mut roster = Vec::new();
        let mut in_roster = false;

        for (lineno, raw) in src.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line == "[roster]" {
                in_roster = true;
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim();
                let val = v.trim();
                match key {
                    "default_currency" => {
                        default_currency = Some(
                            Currency::from_code(val)
                                .ok_or_else(|| format!("line {lineno}: unknown currency `{val}`"))?,
                        );
                    }
                    "active_providers" => {
                        active_providers = val
                            .split(',')
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(str::to_string)
                            .collect();
                    }
                    "node" if in_roster => {
                        // roster entry: `node = <64-hex> : <kind>`
                        let (id_hex, kind_str) = val
                            .split_once(':')
                            .ok_or_else(|| {
                                format!("line {lineno}: roster entry needs `<hex> : <kind>`")
                            })?;
                        let id = parse_hex32(id_hex.trim())
                            .ok_or_else(|| format!("line {lineno}: bad 32-byte hex id"))?;
                        let kind = RosterKind::from_str(kind_str.trim())?;
                        roster.push(RosterEntry { id, kind });
                    }
                    _ if !in_roster => {
                        return Err(format!("line {lineno}: unknown key `{key}`"));
                    }
                    _ => {}
                }
            } else if in_roster {
                // Defensive: a bare roster entry without `node =` prefix is malformed.
                return Err(format!("line {lineno}: roster entry must be `node = <hex> : <kind>`"));
            } else {
                return Err(format!("line {lineno}: expected `key = value`"));
            }
        }

        let default_currency =
            default_currency.ok_or_else(|| "missing required `default_currency`".to_string())?;
        Ok(DeployConfig {
            default_currency,
            active_providers,
            roster,
        })
    }

    /// Is `provider` active in this deployment?
    pub fn provider_active(&self, provider: &str) -> bool {
        self.active_providers.iter().any(|p| p == provider)
    }

    /// Is `id` enrolled as `kind` in the roster?
    pub fn roster_contains(&self, id: &[u8; 32], kind: RosterKind) -> bool {
        self.roster
            .iter()
            .any(|e| &e.id == id && e.kind == kind)
    }
}

fn parse_hex32(s: &str) -> Option<[u8; 32]> {
    let s = s.trim();
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let byte = u8::from_str_radix(std::str::from_utf8(chunk).ok()?, 16).ok()?;
        out[i] = byte;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
default_currency = EUR
active_providers = stripe:eu, cash:cod
[roster]
node = 00000000000000000000000000000000000000000000000000000000000000aa : courier
node = 00000000000000000000000000000000000000000000000000000000000000bb : vendor
";

    #[test]
    fn parse_loads_all_three_deploy_inputs() {
        let cfg = DeployConfig::parse(SAMPLE).expect("parse");
        assert_eq!(cfg.default_currency, Currency::Eur);
        assert!(cfg.provider_active("stripe:eu"));
        assert!(cfg.provider_active("cash:cod"));
        assert_eq!(cfg.roster.len(), 2);
        assert!(cfg.roster_contains(
            &cfg.roster[0].id,
            RosterKind::Courier
        ));
        assert!(cfg.roster_contains(&cfg.roster[1].id, RosterKind::Vendor));
    }

    #[test]
    fn currency_is_not_hardcoded_config_drives_it() {
        // Changing the config changes the currency — proves it is deployment-driven.
        let us = DeployConfig::parse("default_currency = USD\nactive_providers = cash:cod\n").unwrap();
        assert_eq!(us.default_currency, Currency::Usd);
        let eu = DeployConfig::parse("default_currency = EUR\nactive_providers = cash:cod\n").unwrap();
        assert_eq!(eu.default_currency, Currency::Eur);
    }

    #[test]
    fn missing_currency_is_fail_closed() {
        assert!(DeployConfig::parse("active_providers = cash:cod\n").is_err());
    }

    #[test]
    fn unknown_provider_kind_is_rejected() {
        let bad = "default_currency = EUR\n[roster]\nnode = 00000000000000000000000000000000000000000000000000000000000000aa : gremlin\n";
        assert!(DeployConfig::parse(bad).is_err());
    }
}
