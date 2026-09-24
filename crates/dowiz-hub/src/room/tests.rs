//! The refusal's status is part of it: each kind keeps the status the Worker
//! answered with before `Refused` moved here.

use super::Refused;

#[test]
fn every_refusal_keeps_its_status_and_its_words() {
    let cases = [
        (Refused::Stock("rice: 2 wanted, 0 available".into()), 409),
        (Refused::Promo("expired".into()), 400),
        (Refused::Append("full".into()), 500),
        (Refused::NotFound, 404),
        (Refused::Conflict("stale".into()), 409),
        (Refused::Invalid("no signer".into()), 400),
        (Refused::Untaxed("no rate".into()), 500),
    ];
    for (r, status) in cases {
        assert_eq!(r.status(), status, "{r:?}");
        assert!(!r.message().is_empty(), "{r:?} says why");
    }
    assert_eq!(Refused::NotFound.message(), "order not found", "a 404 does not say whose order it was");
    assert_eq!(Refused::Stock("rice: 2 wanted".into()).message(), "rice: 2 wanted");
}
