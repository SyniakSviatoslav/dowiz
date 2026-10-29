//! The key and value limits a writer checks BEFORE it stages anything (W-ES-P3; FORMAT §10,
//! R-EMBED-STORE §3.8, AC-ES-16). The format's own are `MAX_KEY_BYTES` / `MAX_VALUE_BYTES`; a
//! holder may only LOWER them (`Limits::lowered`). A length AT the limit is accepted, one past it
//! is refused, named, with both numbers (T-ES-16 pins both sides of every edge).

use crate::{MAX_KEY_BYTES, MAX_VALUE_BYTES};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The longest key, in bytes.
    pub key: usize,
    /// The longest value, in bytes.
    pub value: usize,
}

/// A length past a limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitError {
    KeyTooLong { len: usize, max: usize },
    ValueTooLong { len: usize, max: usize },
}

impl Limits {
    /// The format's limits: 65 535-byte keys, 1 GiB values.
    pub const FORMAT: Limits = Limits { key: MAX_KEY_BYTES, value: MAX_VALUE_BYTES };

    /// These limits, never above the format's.
    pub fn lowered(self) -> Limits {
        Limits { key: self.key.min(MAX_KEY_BYTES), value: self.value.min(MAX_VALUE_BYTES) }
    }

    pub fn check_key(&self, len: usize) -> Result<(), LimitError> {
        if len > self.key {
            return Err(LimitError::KeyTooLong { len, max: self.key });
        }
        Ok(())
    }

    pub fn check_value(&self, len: usize) -> Result<(), LimitError> {
        if len > self.value {
            return Err(LimitError::ValueTooLong { len, max: self.value });
        }
        Ok(())
    }
}

impl Default for Limits {
    fn default() -> Limits {
        Limits::FORMAT
    }
}

#[cfg(test)]
mod tests;
