//! KTG-2 canonical cell: the single three-valued truth type `State`.
//!
//! Physical encoding is 2 bits (four codes); only three are valid at the
//! application boundary. The fourth code (`0b11`) is the structural
//! invalid-encoding sentinel — it is NOT a fourth truth value and NOT a
//! poison sentinel. It is rejected at the API boundary via [`State::from_bits`].

/// Three-valued truth (strong Kleene) — the only canonical state type in KTG-2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum State {
    False = 0b00,
    Unknown = 0b01,
    True = 0b10,
}

/// Error for an invalid 2-bit physical code (`0b11`) at the API boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidEncoding;

impl State {
    pub const FALSE: State = State::False;
    pub const UNKNOWN: State = State::Unknown;
    pub const TRUE: State = State::True;

    /// Decode a physical 2-bit code. `0b11` is the invalid-encoding sentinel.
    #[inline]
    pub const fn from_bits(bits: u8) -> Result<Self, InvalidEncoding> {
        match bits & 0b11 {
            0b00 => Ok(State::False),
            0b01 => Ok(State::Unknown),
            0b10 => Ok(State::True),
            _ => Err(InvalidEncoding),
        }
    }

    /// Physical 2-bit code.
    #[inline]
    pub const fn bits(self) -> u8 {
        self as u8
    }

    /// Strong Kleene AND: False dominates, True ∧ True = True, else Unknown.
    #[inline]
    pub const fn and(self, other: Self) -> Self {
        match (self, other) {
            (State::False, _) | (_, State::False) => State::False,
            (State::True, State::True) => State::True,
            _ => State::Unknown,
        }
    }

    /// Strong Kleene OR: True dominates, False ∨ False = False, else Unknown.
    #[inline]
    pub const fn or(self, other: Self) -> Self {
        match (self, other) {
            (State::True, _) | (_, State::True) => State::True,
            (State::False, State::False) => State::False,
            _ => State::Unknown,
        }
    }

    /// Strong Kleene NOT: True ↔ False, Unknown stays Unknown.
    #[inline]
    pub const fn not(self) -> Self {
        match self {
            State::True => State::False,
            State::False => State::True,
            State::Unknown => State::Unknown,
        }
    }

    /// Resolve to a safe boolean with an explicit default (Unknown → default).
    #[inline]
    pub const fn resolve(self, default: bool) -> bool {
        match self {
            State::True => true,
            State::False => false,
            State::Unknown => default,
        }
    }
}

impl core::fmt::Display for State {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            State::False => f.write_str("FALSE"),
            State::Unknown => f.write_str("UNKNOWN"),
            State::True => f.write_str("TRUE"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_physical_codes_and_invalid_rejected() {
        assert_eq!(State::FALSE.bits(), 0b00);
        assert_eq!(State::UNKNOWN.bits(), 0b01);
        assert_eq!(State::TRUE.bits(), 0b10);
        assert!(State::from_bits(0b11).is_err());
        assert!(State::from_bits(0b11_11).is_err());
    }

    #[test]
    fn kleene_tables_hold() {
        assert_eq!(State::FALSE.and(State::UNKNOWN), State::FALSE);
        assert_eq!(State::UNKNOWN.and(State::TRUE), State::UNKNOWN);
        assert_eq!(State::TRUE.and(State::TRUE), State::TRUE);
        assert_eq!(State::FALSE.or(State::UNKNOWN), State::UNKNOWN);
        assert_eq!(State::UNKNOWN.or(State::TRUE), State::TRUE);
        assert_eq!(State::FALSE.or(State::FALSE), State::FALSE);
        assert_eq!(State::UNKNOWN.not(), State::UNKNOWN);
        assert_eq!(State::TRUE.not(), State::FALSE);
        assert_eq!(State::FALSE.not(), State::TRUE);
    }

    #[test]
    fn resolve_is_explicit_default() {
        assert_eq!(State::UNKNOWN.resolve(true), true);
        assert_eq!(State::UNKNOWN.resolve(false), false);
        assert_eq!(State::TRUE.resolve(false), true);
        assert_eq!(State::FALSE.resolve(true), false);
    }
}
