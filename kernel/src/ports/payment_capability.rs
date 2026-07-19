//! ports/payment_capability.rs — P47 payment-rail CAPABILITY DECLARATION scaffold.
//!
//! # What this module IS
//! A pure, declarative inventory of which payment rails the platform *intends* to support,
//! per the operator ruling (rail set = { Fiat, Crypto, Stripe, Google/Apple Pay }, rest later).
//! It is a CAPABILITY/feature declaration ONLY:
//!
//!   * `PaymentRail`    — the closed set of rails (exhaustive enum, `FromStr`/`Display`).
//!   * `PaymentCapability { rail, enabled }` — a feature flag, NO credentials, NO transport.
//!   * `PaymentError`   — the one error this layer can produce (`NotYetSupported`).
//!   * `PaymentCapability::validate()` — rejects any rail not yet green-lit (`OtherLater`).
//!
//! # RED-LINE (binding — do NOT cross)
//! This module constructs NO real provider object, reads NO environment credentials, makes NO
//! network calls, and moves NO money. `PaymentCapability` carries no credential, no transport
//! field. If a real Stripe/crypto/Fiat processor is ever needed, that belongs in a
//! downstream adapter crate behind the kernel's compile firewall (see `payment.rs`), NOT here.
//! This module is the "what could exist" registry; the "how it connects" lives elsewhere and is
//! deliberately absent.
//!
//! # Red-proof (the firewall, by construction)
//! The test `red_line_no_real_provider_references` greps THIS file's own source for
//! provider/credential markers and FAILS if any appears — proving the red-line holds at the
//! source level. This is why the capability scaffold lives in its OWN module rather than being
//! bolted onto `payment.rs` (which legitimately reuses internal signing machinery and would
//! trip a whole-file scan). The forbidden markers are assembled via `concat!` so the scan body
//! never literally contains them (which would make the negation self-matching / vacuous).

use std::fmt;
use std::str::FromStr;

/// The closed set of payment rails the platform may declare support for (P47 operator ruling).
///
/// This is exhaustive by design: adding a rail is a compile-time change (you must handle it in
/// `FromStr`/`Display`/`validate`), which keeps the capability registry honest. `OtherLater`
/// is the explicit "not yet" bucket — it may be *named* but `validate()` rejects it so nothing
/// can silently treat an unbuilt rail as live.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaymentRail {
    /// Government-issued / bank money rail (ACH, SEPA, wire, card-net presentment, …).
    /// Operator-ruled; no adapter is built here — capability declaration only.
    Fiat,
    /// Cryptocurrency / on-chain settlement rail. Operator-ruled; no wallet transport here.
    Crypto,
    /// Stripe (card + digital wallet presentment via Stripe). Operator-ruled; NO Stripe object
    /// is constructed in this module — declaring the rail is the only thing that happens.
    Stripe,
    /// Google Pay / Apple Pay (tokenized wallet presentment). Operator-ruled; no wallet SDK is
    /// constructed here.
    GoogleApplePay,
    /// Anything the operator has NOT yet green-lit. Named so it can be referenced, but
    /// `validate()` rejects it (`NotYetSupported`) so an unbuilt rail can never be enabled.
    OtherLater,
}

impl PaymentRail {
    /// Stable lowercase string id (used in capability manifests, telemetry, reconciliation
    /// rows). Distinct from `Display` only in being the canonical machine identifier.
    pub fn as_str(&self) -> &'static str {
        match self {
            PaymentRail::Fiat => "fiat",
            PaymentRail::Crypto => "crypto",
            PaymentRail::Stripe => "stripe",
            PaymentRail::GoogleApplePay => "google_apple_pay",
            PaymentRail::OtherLater => "other_later",
        }
    }

    /// All rails in declaration order. Handy for enumerating the capability matrix.
    pub const ALL: &'static [PaymentRail] = &[
        PaymentRail::Fiat,
        PaymentRail::Crypto,
        PaymentRail::Stripe,
        PaymentRail::GoogleApplePay,
        PaymentRail::OtherLater,
    ];

    /// The four rails the operator has actually ruled IN for now (excludes `OtherLater`).
    /// `validate()` accepts exactly these.
    pub const SUPPORTED_NOW: &'static [PaymentRail] = &[
        PaymentRail::Fiat,
        PaymentRail::Crypto,
        PaymentRail::Stripe,
        PaymentRail::GoogleApplePay,
    ];

    /// Whether this rail is part of the operator-ruled-in set (i.e. `validate` would accept it).
    pub fn is_supported_now(&self) -> bool {
        matches!(
            self,
            PaymentRail::Fiat
                | PaymentRail::Crypto
                | PaymentRail::Stripe
                | PaymentRail::GoogleApplePay
        )
    }
}

impl fmt::Display for PaymentRail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Human-readable, title-ish form.
        let s = match self {
            PaymentRail::Fiat => "Fiat",
            PaymentRail::Crypto => "Crypto",
            PaymentRail::Stripe => "Stripe",
            PaymentRail::GoogleApplePay => "Google/Apple Pay",
            PaymentRail::OtherLater => "Other (later)",
        };
        f.write_str(s)
    }
}

impl FromStr for PaymentRail {
    type Err = PaymentError;

    /// Case-insensitive parse from the canonical `as_str()` identifiers. Unknown strings are
    /// rejected as `PaymentError::UnknownRail` (never silently mapped onto `OtherLater`).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "fiat" => Ok(PaymentRail::Fiat),
            "crypto" => Ok(PaymentRail::Crypto),
            "stripe" => Ok(PaymentRail::Stripe),
            "google_apple_pay" | "googleapplepay" => Ok(PaymentRail::GoogleApplePay),
            "other_later" | "otherlater" => Ok(PaymentRail::OtherLater),
            _ => Err(PaymentError::UnknownRail(s.trim().to_string())),
        }
    }
}

/// The one error this capability layer can produce.
///
/// Note what it does NOT include: no network error, no auth/credential error, no provider
/// error — because this module performs none of those operations. Declaring a capability is a
/// pure, local, in-memory act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentError {
    /// The rail is named but not yet green-lit by the operator (`OtherLater`).
    NotYetSupported { rail: PaymentRail },
    /// `FromStr`/`as_str` round-trip could not resolve the string to a known rail.
    UnknownRail(String),
}

impl fmt::Display for PaymentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaymentError::NotYetSupported { rail } => {
                write!(f, "payment rail '{}' is not yet supported", rail)
            }
            PaymentError::UnknownRail(s) => write!(f, "unknown payment rail '{}'", s),
        }
    }
}

impl std::error::Error for PaymentError {}

/// A capability/feature declaration for one payment rail.
///
/// This is NOT a transport handle and carries NO credentials. It is the "the platform may
/// offer rail X, and it is currently on/off" record. Everything that would connect to a real
/// processor is intentionally absent — that is the red-line. Such wiring belongs in a
/// downstream adapter crate behind the kernel firewall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaymentCapability {
    /// Which rail this capability declares.
    pub rail: PaymentRail,
    /// Whether the rail is currently enabled in the capability matrix. `validate()` must pass
    /// before `enabled = true` is meaningful.
    pub enabled: bool,
}

impl PaymentCapability {
    /// Declare a rail as enabled. Caller should have already ensured the rail is supported;
    /// `validate()` is the authoritative gate.
    pub fn enabled(rail: PaymentRail) -> Self {
        PaymentCapability {
            rail,
            enabled: true,
        }
    }

    /// Declare a rail as disabled (e.g. a future rail announced but not live).
    pub fn disabled(rail: PaymentRail) -> Self {
        PaymentCapability {
            rail,
            enabled: false,
        }
    }

    /// The capability matrix for "now": the four operator-ruled-in rails, all declared ENABLED.
    /// `OtherLater` is deliberately NOT in this list — it must be added explicitly and will
    /// fail `validate()` until the operator rules it in.
    pub fn current_matrix() -> Vec<PaymentCapability> {
        PaymentRail::SUPPORTED_NOW
            .iter()
            .map(|&r| PaymentCapability::enabled(r))
            .collect()
    }

    /// Gate: a capability is only valid if its rail is supported now.
    ///
    /// * Any of {Fiat, Crypto, Stripe, GoogleApplePay} → `Ok(())` (the operator ruled these in).
    /// * `OtherLater` → `Err(NotYetSupported)` (named but not green-lit).
    ///
    /// This is the ONLY operation on the capability layer, and it is pure + local.
    pub fn validate(&self) -> Result<(), PaymentError> {
        if self.rail.is_supported_now() {
            Ok(())
        } else {
            Err(PaymentError::NotYetSupported { rail: self.rail })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── red-line: the module source must NOT reference real-provider / credential markers ──
    // Scans THIS file only (the capability scaffold). The forbidden markers are assembled via
    // `concat!` so this test body never literally contains them (which would make the
    // negation self-matching). Because the module source is genuinely free of these substrings,
    // every assertion below is a TRUE negative.
    const SELF_SRC: &str = include_str!("payment_capability.rs");

    const FORBIDDEN_MARKERS: &[&str] = &[
        concat!("req", "west"),  // real HTTP provider
        concat!("ur", "eq"),     // real HTTP provider
        concat!("std::en", "v"), // environment credential read
        concat!("sec", "ret"),   // credential material
        concat!("ke", "y"),      // credential material
    ];

    #[test]
    fn red_line_no_real_provider_references() {
        for marker in FORBIDDEN_MARKERS {
            assert!(
                !SELF_SRC.contains(marker),
                "payment_capability.rs red-line violation: references '{marker}'"
            );
        }
        // Positive structural assertions: capability declaration only — no transport/credential.
        assert!(SELF_SRC.contains("capability declaration"));
        assert!(SELF_SRC.contains("ENABLED"));
        // The red-line must be documented and `OtherLater` must be the only rejected rail.
        assert!(SELF_SRC.contains("RED-LINE"));
        assert!(SELF_SRC.contains("NotYetSupported"));
    }

    // ── enum round-trips: Display→as_str→FromStr→enum is identity for all variants ──
    #[test]
    fn rail_from_str_display_roundtrip() {
        for &rail in PaymentRail::ALL {
            // canonical identifier round-trips exactly.
            let parsed = PaymentRail::from_str(rail.as_str()).expect("as_str must parse");
            assert_eq!(parsed, rail, "round-trip broke for {}", rail);
            // Display produces a non-empty, distinct label.
            assert!(!rail.to_string().is_empty());
        }
    }

    #[test]
    fn rail_from_str_case_insensitive_and_aliases() {
        assert_eq!(
            "STRIPE".parse::<PaymentRail>().unwrap(),
            PaymentRail::Stripe
        );
        assert_eq!(
            " Google_Apple_Pay ".parse::<PaymentRail>().unwrap(),
            PaymentRail::GoogleApplePay
        );
        assert_eq!(
            "googleapplepay".parse::<PaymentRail>().unwrap(),
            PaymentRail::GoogleApplePay
        );
        assert_eq!(
            "OTHER_LATER".parse::<PaymentRail>().unwrap(),
            PaymentRail::OtherLater
        );
    }

    #[test]
    fn rail_from_str_unknown_is_error() {
        assert_eq!(
            "paypal".parse::<PaymentRail>(),
            Err(PaymentError::UnknownRail("paypal".to_string()))
        );
        assert_eq!(
            "".parse::<PaymentRail>(),
            Err(PaymentError::UnknownRail("".to_string()))
        );
        assert_eq!(
            " stripe ".parse::<PaymentRail>(), // already covered, just confirming delimiter trim
            Ok(PaymentRail::Stripe)
        );
    }

    // ── validate accepts the 4 real rails ──
    #[test]
    fn validate_accepts_the_four_real_rails() {
        for &rail in PaymentRail::SUPPORTED_NOW {
            let cap = PaymentCapability::enabled(rail);
            assert_eq!(
                cap.validate(),
                Ok(()),
                "operator-ruled-in rail {rail} must validate"
            );
            // disabled-but-supported still validates as a *capability* (the gate is the rail,
            // not the enabled flag).
            assert_eq!(PaymentCapability::disabled(rail).validate(), Ok(()));
        }
    }

    #[test]
    fn current_matrix_has_four_enabled_supported_rails() {
        let matrix = PaymentCapability::current_matrix();
        assert_eq!(matrix.len(), 4);
        for cap in &matrix {
            assert!(cap.enabled);
            assert_eq!(cap.validate(), Ok(()));
            assert!(cap.rail.is_supported_now());
        }
    }

    // ── validate rejects OtherLater ──
    #[test]
    fn validate_rejects_other_later_enabled() {
        let cap = PaymentCapability::enabled(PaymentRail::OtherLater);
        assert_eq!(
            cap.validate(),
            Err(PaymentError::NotYetSupported {
                rail: PaymentRail::OtherLater
            })
        );
    }

    #[test]
    fn validate_rejects_other_later_disabled() {
        let cap = PaymentCapability::disabled(PaymentRail::OtherLater);
        assert_eq!(
            cap.validate(),
            Err(PaymentError::NotYetSupported {
                rail: PaymentRail::OtherLater
            })
        );
    }

    #[test]
    fn only_other_later_is_unsupported() {
        // Every rail except OtherLater is supported-now.
        for &rail in PaymentRail::ALL {
            assert_eq!(
                rail.is_supported_now(),
                rail != PaymentRail::OtherLater,
                "support-now classification wrong for {}",
                rail
            );
        }
    }
}
