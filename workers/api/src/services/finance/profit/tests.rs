//! A delivery with no date sits in no month and is counted as such.

use super::receipts_of;
use dowiz_hub::stock::{StockEvent, StockLog};

#[test]
fn a_delivery_with_no_date_is_counted_and_placed_in_no_month_and_a_dated_one_is_placed() {
    let zone = dowiz_hub::tz::DEFAULT;
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    // A plain append carries no instant: the shape of every record older than the request clock.
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 1000 }).unwrap();
    log.append(&StockEvent::Received { item: "rice".into(), qty: 500 }).unwrap();
    let mut j = log.journal().unwrap();
    let (by_month, undated) = receipts_of(&j, zone, 0);
    assert_eq!((by_month.len(), undated), (0, 2));
    // The same two records, one given its instant: 2023-11-14 12:00 UTC.
    j.entries[0].meta.at = Some(1_699_963_200_000);
    j.entries[0].value = Some(1200);
    let (by_month, undated) = receipts_of(&j, zone, 0);
    assert_eq!(undated, 1);
    assert_eq!(by_month.get(&202311).copied(), Some((1, 1200, 0)));
    assert_eq!(by_month.len(), 1);
}
