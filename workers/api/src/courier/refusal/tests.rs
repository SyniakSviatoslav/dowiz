//! W-FIX H3: a computed status outside the error range becomes 500, never a panic.

use super::status;

#[test]
fn a_status_outside_the_error_range_is_sent_as_500() {
    for s in [0, 100, 200, 204, 302, 399, 600, 999, u16::MAX] {
        assert_eq!(status(s), 500, "{s}");
    }
}

/// The twin: every error status is kept exactly.
#[test]
fn an_error_status_is_kept() {
    for s in [400, 401, 403, 404, 409, 429, 500, 503, 599] {
        assert_eq!(status(s), s);
    }
}
