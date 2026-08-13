//! NodeState and NodeId: the only tolerant/4-state primitive in KTG-2.
//!
//! This module deliberately does NOT expose an "InvalidEncoding" at the
//! application level. Physical 2-bit cells use the codes below; the
//! runtime treats 0b11 as a recoverable infrastructure-level ERROR_STATE,
//! NOT a fourth truth value and NOT a poison sentinel. Applications keep
//! running; downstream consumers may observe Error unless they resolve it.

/// Three truth states plus one recoverable Error state within the same 2-bit encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct NodeState(u8);

impl NodeState {
    /// 2-bit physical codes.
    pub const FALSE: Self = Self(0b00);
    pub const UNKNOWN: Self = Self(0b01);
    pub const TRUE: Self = Self(0b10);
    pub const ERROR_STATE: Self = Self(0b11);

    #[inline]
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits & 0b11)
    }

    #[inline]
    pub const fn bits(self) -> u8 {
        self.0
    }

    #[inline]
    pub const fn is_error(self) -> bool {
        self.0 == 0b11
    }

    #[inline]
    pub const fn is_known(self) -> bool {
        self.0 != 0b01 && self.0 != 0b11
    }

    #[inline]
    pub const fn is_false(self) -> bool {
        self.0 == 0b00
    }

    #[inline]
    pub const fn is_true(self) -> bool {
        self.0 == 0b10
    }

    #[inline]
    pub const fn is_unknown(self) -> bool {
        self.0 == 0b01
    }

    /// AND: True && True = True, False anywhere => False, Error sticky,
    /// Unknown otherwise => Unknown. Apps keep running; Error propagates.
    #[inline]
    pub const fn and(self, other: Self) -> Self {
        match (self.0, other.0) {
            (0b11, _) | (_, 0b11) => Self::ERROR_STATE,
            (0b00, _) | (_, 0b00) => Self::FALSE,
            (0b01, _) | (_, 0b01) => Self::UNKNOWN,
            _ => Self::TRUE,
        }
    }

    /// OR: False || False = False, True anywhere => True, Error sticky,
    /// Unknown otherwise => Unknown. Apps keep running; Error propagates.
    #[inline]
    pub const fn or(self, other: Self) -> Self {
        match (self.0, other.0) {
            (0b11, _) | (_, 0b11) => Self::ERROR_STATE,
            (0b10, _) | (_, 0b10) => Self::TRUE,
            (0b01, _) | (_, 0b01) => Self::UNKNOWN,
            _ => Self::FALSE,
        }
    }

    /// NOT: True <-> False, Unknown stays Unknown, Error stays Error.
    #[inline]
    pub const fn not(self) -> Self {
        match self.0 {
            0b00 => Self::TRUE,
            0b01 => Self::UNKNOWN,
            0b10 => Self::FALSE,
            _ => Self::ERROR_STATE,
        }
    }

    /// Resolve to a safe boolean with an explicit default.
    /// ERROR_STATE resolves to `default`; callers decide the policy.
    #[inline]
    pub const fn resolve(self, default: bool) -> bool {
        match self.0 {
            0b00 => false,
            0b10 => true,
            _ => default,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_physical_codes_and_only_four_physical_codes() {
        assert_eq!(NodeState::FALSE.bits(), 0b00);
        assert_eq!(NodeState::UNKNOWN.bits(), 0b01);
        assert_eq!(NodeState::TRUE.bits(), 0b10);
        assert_eq!(NodeState::ERROR_STATE.bits(), 0b11);
        assert_eq!(NodeState::from_bits(0b1100), NodeState::ERROR_STATE);
    }

    #[test]
    fn sticky_error_propagates_through_logic() {
        assert_eq!(NodeState::ERROR_STATE.and(NodeState::FALSE), NodeState::ERROR_STATE);
        assert_eq!(NodeState::ERROR_STATE.or(NodeState::TRUE), NodeState::ERROR_STATE);
        assert_eq!(NodeState::FALSE.and(NodeState::ERROR_STATE), NodeState::ERROR_STATE);
        assert_eq!(NodeState::ERROR_STATE.not(), NodeState::ERROR_STATE);
        assert_eq!(NodeState::UNKNOWN.and(NodeState::ERROR_STATE), NodeState::ERROR_STATE);
    }

    #[test]
    fn ordinary_kleene_truth_tables_still_hold() {
        assert_eq!(NodeState::FALSE.and(NodeState::UNKNOWN), NodeState::FALSE);
        assert_eq!(NodeState::UNKNOWN.and(NodeState::TRUE), NodeState::UNKNOWN);
        assert_eq!(NodeState::TRUE.and(NodeState::TRUE), NodeState::TRUE);
        assert_eq!(NodeState::FALSE.or(NodeState::UNKNOWN), NodeState::UNKNOWN);
        assert_eq!(NodeState::UNKNOWN.or(NodeState::TRUE), NodeState::TRUE);
        assert_eq!(NodeState::FALSE.or(NodeState::FALSE), NodeState::FALSE);
        assert_eq!(NodeState::UNKNOWN.not(), NodeState::UNKNOWN);
    }

    #[test]
    fn resolve_is_explicit_default_policy_not_magic() {
        assert_eq!(NodeState::ERROR_STATE.resolve(true), true);
        assert_eq!(NodeState::ERROR_STATE.resolve(false), false);
        assert_eq!(NodeState::UNKNOWN.resolve(true), true);
        assert_eq!(NodeState::UNKNOWN.resolve(false), false);
    }
}
