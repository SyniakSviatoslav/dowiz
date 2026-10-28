//! Deterministic sales rows for the fixtures (`examples/generate_fixtures.rs`) and the tests.

use super::{schema, Sale};

/// A deterministic span of DAY ROWS for fixtures and tests: `days` days from `first` (`yyyymmdd`),
/// `per_day` (<= 12) distinct dishes a day out of twelve; a third of the rows have no stamped
/// line, a third are all stamped, a third mix the two.
pub fn sample(first: i32, days: i32, per_day: i32) -> Vec<(Sale, String)> {
    let start = crate::stock::meta::day_number(i64::from(first));
    let mut out = Vec::new();
    for d in 0..days {
        let day = crate::stock::meta::day_of_number(start + i64::from(d)) as i32;
        for i in 0..per_day {
            let id = format!("dish-{:03}", (d * 7 + i * 5) % 12);
            let qty = 1 + (d + i) % 3;
            let gross = i64::from(qty) * (400 + 100 * i64::from((d + 2 * i) % 9));
            let (cqty, cgross) = match (d + i) % 3 {
                0 => (0, 0),
                1 => (qty, gross),
                _ => (1, gross / i64::from(qty)),
            };
            let dish = schema::k64(id.as_bytes()) as i64;
            out.push((Sale { day, dish, qty, gross, cqty, cgross, cost: cgross * 3 / 10 }, id));
        }
    }
    out
}
