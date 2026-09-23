//! How an order reaches the person who ordered it — the kernel's half.
//!
//! The fulfilment kind is a closed set checked on input, and these constants
//! are the source of truth. Every surface (browser, API response, database
//! export) derives this list from the kernel rather than retyping it.

/// Every way an order can reach its customer. A CLOSED SET, checked on the way in.
pub const KINDS: &[&str; 3] = &["delivery", "pickup", "dine_in"];

/// All fulfilment kinds as a single array (for export to generated vocab).
pub const ALL: &[&str] = KINDS;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_are_three() {
        assert_eq!(KINDS.len(), 3);
    }

    #[test]
    fn all_equals_kinds() {
        assert_eq!(ALL, KINDS);
    }

    #[test]
    fn delivery_is_first() {
        assert_eq!(KINDS[0], "delivery");
    }

    #[test]
    fn pickup_is_second() {
        assert_eq!(KINDS[1], "pickup");
    }

    #[test]
    fn dine_in_is_third() {
        assert_eq!(KINDS[2], "dine_in");
    }
}
