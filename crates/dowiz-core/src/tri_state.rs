//! tri_state.rs — kernel-wide tri-state logic (no boolean is ever just
//! true/false). Extracted from `dowiz-kernel`'s crate root so `crate::TriState`
//! resolves unchanged in both the kernel (via `pub use dowiz_core::TriState`)
//! and any `no_std` module that imports it.
//!
//! Every observable state carries `True | False | Unknown`. Unknown means "we
//! don't know yet" — measurement pending, observation insufficient, or system
//! just booted. Code that acts on Unknown must treat it as "not safe to assume
//! either way" — fail-closed.

/// Kernel-wide tri-state: no boolean is ever just true/false.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriState {
    /// Confirmed positive / active / safe / stale / valid.
    True,
    /// Confirmed negative / inactive / unsafe / fresh / invalid.
    False,
    /// Unknown — observation pending or insufficient data.
    /// Fail-closed: treat as "cannot confirm".
    Unknown,
}

impl TriState {
    pub fn is_true(&self) -> bool {
        *self == TriState::True
    }
    pub fn is_false(&self) -> bool {
        *self == TriState::False
    }
    pub fn is_unknown(&self) -> bool {
        *self == TriState::Unknown
    }
    /// Resolve: True→true, False→false, Unknown→default.
    pub fn resolve(&self, default: bool) -> bool {
        match self {
            TriState::True => true,
            TriState::False => false,
            TriState::Unknown => default,
        }
    }
    /// Logical AND: True AND True = True, anything else = False.
    pub fn and(self, other: TriState) -> TriState {
        if self == TriState::True && other == TriState::True {
            TriState::True
        } else if self == TriState::False || other == TriState::False {
            TriState::False
        } else {
            TriState::Unknown
        }
    }
    /// Logical OR: False OR False = False, anything else = True.
    pub fn or(self, other: TriState) -> TriState {
        if self == TriState::True || other == TriState::True {
            TriState::True
        } else if self == TriState::False && other == TriState::False {
            TriState::False
        } else {
            TriState::Unknown
        }
    }
    /// Logical NOT: True→False, False→True, Unknown→Unknown.
    pub fn not(self) -> TriState {
        match self {
            TriState::True => TriState::False,
            TriState::False => TriState::True,
            TriState::Unknown => TriState::Unknown,
        }
    }
    /// From bool: true→True, false→False. Use when legacy code produces bool.
    pub fn from_bool(v: bool) -> TriState {
        if v {
            TriState::True
        } else {
            TriState::False
        }
    }
}

impl core::fmt::Display for TriState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TriState::True => write!(f, "TRUE"),
            TriState::False => write!(f, "FALSE"),
            TriState::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tri_state_logic_is_fail_closed() {
        assert!(TriState::True.is_true());
        assert!(TriState::Unknown.is_unknown());
        assert!(!TriState::Unknown.resolve(false), "Unknown fails closed");
        assert!(TriState::Unknown.resolve(true), "Unknown resolves to caller default");
        assert_eq!(TriState::True.and(TriState::Unknown), TriState::Unknown);
        assert_eq!(TriState::True.and(TriState::False), TriState::False);
        assert_eq!(TriState::False.or(TriState::Unknown), TriState::Unknown);
        assert_eq!(TriState::False.or(TriState::True), TriState::True);
        assert_eq!(TriState::Unknown.not(), TriState::Unknown);
        assert_eq!(TriState::from_bool(true), TriState::True);
    }

    #[test]
    fn tri_state_display_is_stable() {
        assert_eq!(TriState::True.to_string(), "TRUE");
        assert_eq!(TriState::False.to_string(), "FALSE");
        assert_eq!(TriState::Unknown.to_string(), "UNKNOWN");
    }
}
