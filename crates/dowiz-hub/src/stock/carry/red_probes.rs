//! W-QTYHUB RED probes, written against the API main already had (so they
//! compile on the tree before H3 and fail there by assertion).

use super::*;

/// H3: 2.5 pieces received leave a remainder of half a piece (3 booked).
#[test]
fn red_h3_a_fractional_delivery_moves_the_carry() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("nori", 3, &Meta { uq: Some(2_500_000), ..Meta::default() }).unwrap();
    let j = log.journal().unwrap();
    assert_eq!(j.ledger.level("nori").on_hand, 3);
    assert_eq!(j.carry.of("nori"), 500_000, "3 booked, 2.5 received: half a piece owed back");
}
