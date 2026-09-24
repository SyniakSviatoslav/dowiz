//! D13 (G6): a waiter cannot spend a wallet by typing its id.

use super::*;

/// THE HOLE: a waiter (not the owner) names somebody's wallet and no code.
/// Before G6 the only check was the balance; now it is refused.
#[test]
fn a_waiter_cannot_spend_a_typed_wallet_id() {
    assert_eq!(payer(false, Some("someone-else"), None).map_err(|e| e.0), Err(403));
    // A code was shown, and the typed id is somebody else's: a contradiction.
    assert_eq!(payer(false, Some("someone-else"), Some("guest-key")).map_err(|e| e.0), Err(403));
}

/// The twins: the wallet of the customer whose code was shown, with or
/// without the id typed as well; and the owner naming a customer.
#[test]
fn the_presented_customers_wallet_and_the_owners_named_one_are_spent() {
    assert_eq!(payer(false, None, Some("guest-key")), Ok("guest-key".into()));
    assert_eq!(payer(false, Some("guest-key"), Some("guest-key")), Ok("guest-key".into()));
    assert_eq!(payer(true, Some("a-customer"), None), Ok("a-customer".into()));
}
