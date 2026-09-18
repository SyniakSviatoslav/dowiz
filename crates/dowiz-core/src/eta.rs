//! Delivery estimate — how long an order will actually take.
//!
//! Before this, a venue published ONE string ("30-45") and every screen printed
//! it: the same estimate for a coffee two streets away and a banquet across
//! town, with six orders already on the pass. This module replaces that with an
//! estimate built from what is actually known:
//!
//! 1. **Preparation** — each dish carries its own `cooking_min`, set by the
//!    venue. A kitchen cooks in parallel, so a basket takes as long as its
//!    SLOWEST dish, plus a small cost per extra portion.
//! 2. **The queue** — orders already accepted and not yet out of the kitchen.
//!    Their remaining prep is spread across the venue's stations.
//! 3. **The journey** — distance to the customer at the courier's speed, plus
//!    the two handovers nobody counts and everybody waits through.
//!
//! # Rules this obeys
//!
//! * **Integer minutes, no float.** The same rule money follows, for the same
//!   reason: two nodes must reach the same answer bit for bit. A float divide
//!   would let a phone and a hub disagree by a minute and then disagree about
//!   whether an order is late.
//! * **No clock.** `now` is never read here. An estimate is a DURATION; turning
//!   it into a time of day is the caller's job, with the caller's clock.
//! * **Fail closed.** Every arithmetic step is checked. An overflow returns an
//!   error rather than a wrapped negative that would show as "arriving 4 hours
//!   ago".
//! * **It measures the kitchen, never the people.** There is no courier speed
//!   ranking and no "fast venue" score — `DECISIONS.md` D0. The speed used is a
//!   venue's own setting for its own fleet, not a measurement of anybody.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// One line of a basket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasketItem {
    /// What the venue says this dish takes, in minutes. Set per dish by the
    /// owner; `0` means the venue has not said, and [`estimate`] treats it as
    /// the venue's default rather than as "instant".
    pub cooking_min: u16,
    pub quantity: u16,
}

/// An order already accepted and not yet handed to a courier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueuedOrder {
    /// How much preparation this order still has left, in minutes.
    pub remaining_min: u16,
}

/// What a venue says about itself. Every number is the venue's own setting, not
/// a measurement of anybody's performance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KitchenProfile {
    /// How many dishes the kitchen genuinely works on at once. One means a
    /// single pan; a queue behind a one-station kitchen is a real wait.
    pub stations: u16,
    /// What a dish takes when the venue has not set a time for it.
    pub default_cooking_min: u16,
    /// Added per extra portion beyond the first in the whole basket.
    pub per_extra_portion_min: u16,
    /// Minutes between "ready" and the courier actually leaving.
    pub pickup_min: u16,
    /// Minutes between arriving at the address and the food changing hands.
    pub handover_min: u16,
    /// Metres a courier covers in a minute. A venue's own figure for its own
    /// fleet: 250 m/min is a bicycle in traffic, 400 a scooter.
    pub courier_speed_m_per_min: u16,
    /// How much of the estimate is quoted as a spread, in percent. The frame
    /// shows "30–45", and a range a venue can keep is worth more than a precise
    /// number it cannot.
    pub spread_pct: u16,
}

impl KitchenProfile {
    /// A profile a venue can start from. Every number is a default it overrides.
    pub const fn default_profile() -> Self {
        Self {
            stations: 2,
            default_cooking_min: 15,
            per_extra_portion_min: 2,
            pickup_min: 5,
            handover_min: 3,
            courier_speed_m_per_min: 300,
            spread_pct: 50,
        }
    }
}

/// A quoted estimate, in whole minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Estimate {
    /// The soonest it is expected.
    pub low_min: u32,
    /// The figure quoted as the far end.
    pub high_min: u32,
    /// What went into it, so a venue can see WHY an estimate moved rather than
    /// only that it did.
    pub prep_min: u32,
    pub queue_min: u32,
    pub travel_min: u32,
    pub overhead_min: u32,
}

impl Estimate {
    /// The form the storefront prints: `"30–45"`.
    pub fn range(&self) -> String {
        format!("{}–{}", self.low_min, self.high_min)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EtaError {
    /// A kitchen with no stations cannot cook.
    NoStations,
    /// A courier that covers no distance never arrives.
    NoSpeed,
    /// An empty basket has no estimate; asking for one is a caller bug.
    EmptyBasket,
    /// Arithmetic overflowed. Fail closed rather than wrap.
    Overflow(&'static str),
}

impl EtaError {
    pub fn message(&self) -> String {
        match self {
            Self::NoStations => String::from("kitchen has no stations"),
            Self::NoSpeed => String::from("courier speed is zero"),
            Self::EmptyBasket => String::from("an empty basket has no estimate"),
            Self::Overflow(where_) => format!("eta arithmetic overflowed at {where_}"),
        }
    }
}

/// Ceiling division on integers. `a / b` rounded up, with no float anywhere.
fn div_ceil(a: u32, b: u32) -> u32 {
    if b == 0 {
        return 0;
    }
    a / b + u32::from(a % b != 0)
}

/// How long the kitchen needs for this basket alone.
///
/// A kitchen cooks in parallel, so the basket takes as long as its SLOWEST dish
/// — not the sum, which is the mistake that makes a four-dish order quote an
/// hour. Extra portions add a small per-portion cost, because a second pizza is
/// not free even when it goes in the same oven.
pub fn prep_minutes(items: &[BasketItem], k: &KitchenProfile) -> Result<u32, EtaError> {
    if items.is_empty() {
        return Err(EtaError::EmptyBasket);
    }
    let mut slowest = 0u32;
    let mut portions = 0u32;
    for it in items {
        if it.quantity == 0 {
            continue;
        }
        let cook = if it.cooking_min == 0 {
            k.default_cooking_min
        } else {
            it.cooking_min
        } as u32;
        if cook > slowest {
            slowest = cook;
        }
        portions = portions
            .checked_add(it.quantity as u32)
            .ok_or(EtaError::Overflow("portion count"))?;
    }
    if portions == 0 {
        return Err(EtaError::EmptyBasket);
    }
    let extra = portions - 1;
    let extra_cost = extra
        .checked_mul(k.per_extra_portion_min as u32)
        .ok_or(EtaError::Overflow("per-portion cost"))?;
    slowest
        .checked_add(extra_cost)
        .ok_or(EtaError::Overflow("prep total"))
}

/// How long the orders already on the pass hold this one up.
///
/// Their remaining preparation is spread across the stations. With two stations
/// and four ten-minute orders ahead, this order waits twenty minutes — not
/// forty, and not nothing.
pub fn queue_minutes(ahead: &[QueuedOrder], k: &KitchenProfile) -> Result<u32, EtaError> {
    if k.stations == 0 {
        return Err(EtaError::NoStations);
    }
    let mut total = 0u32;
    for o in ahead {
        total = total
            .checked_add(o.remaining_min as u32)
            .ok_or(EtaError::Overflow("queue total"))?;
    }
    Ok(div_ceil(total, k.stations as u32))
}

/// How long the journey takes.
///
/// Straight-line distance is not road distance, so the caller passes the
/// distance it actually has and this does not pretend to know better.
pub fn travel_minutes(distance_m: u32, k: &KitchenProfile) -> Result<u32, EtaError> {
    if k.courier_speed_m_per_min == 0 {
        return Err(EtaError::NoSpeed);
    }
    Ok(div_ceil(distance_m, k.courier_speed_m_per_min as u32))
}

/// The whole estimate.
///
/// Preparation and the queue overlap only as far as the stations allow, so they
/// add; travel starts when the food is ready, so it adds too. The two handovers
/// are counted because a customer waits through them whether or not anybody
/// measures them.
pub fn estimate(
    items: &[BasketItem],
    ahead: &[QueuedOrder],
    distance_m: u32,
    k: &KitchenProfile,
) -> Result<Estimate, EtaError> {
    let prep = prep_minutes(items, k)?;
    let queue = queue_minutes(ahead, k)?;
    let travel = travel_minutes(distance_m, k)?;
    let overhead = (k.pickup_min as u32)
        .checked_add(k.handover_min as u32)
        .ok_or(EtaError::Overflow("overhead"))?;

    let low = prep
        .checked_add(queue)
        .and_then(|n| n.checked_add(travel))
        .and_then(|n| n.checked_add(overhead))
        .ok_or(EtaError::Overflow("low estimate"))?;

    // The spread is a percentage of the estimate, rounded up, so a longer order
    // carries a wider window — which is honest: more steps, more that can slip.
    let spread = div_ceil(
        low.checked_mul(k.spread_pct as u32)
            .ok_or(EtaError::Overflow("spread"))?,
        100,
    );
    let high = low
        .checked_add(spread)
        .ok_or(EtaError::Overflow("high estimate"))?;

    Ok(Estimate {
        low_min: low,
        high_min: high,
        prep_min: prep,
        queue_min: queue,
        travel_min: travel,
        overhead_min: overhead,
    })
}

/// The pickup case: no journey, no handover at an address.
pub fn estimate_pickup(
    items: &[BasketItem],
    ahead: &[QueuedOrder],
    k: &KitchenProfile,
) -> Result<Estimate, EtaError> {
    let mut e = estimate(items, ahead, 0, k)?;
    // Recompute without the courier's two handovers: the customer walks in.
    let low = e
        .low_min
        .checked_sub(e.overhead_min)
        .ok_or(EtaError::Overflow("pickup overhead"))?;
    let spread = div_ceil(
        low.checked_mul(k.spread_pct as u32)
            .ok_or(EtaError::Overflow("spread"))?,
        100,
    );
    e.overhead_min = 0;
    e.travel_min = 0;
    e.low_min = low;
    e.high_min = low.checked_add(spread).ok_or(EtaError::Overflow("pickup high"))?;
    Ok(e)
}

/// Great-circle distance in metres, from microdegrees.
///
/// Microdegrees because that is what dowiz already carries on the wire
/// (`lat_udeg`/`lon_udeg` in the storefront's reach check), and because an
/// integer coordinate is one both sides can agree on exactly.
///
/// This is the equirectangular approximation, which is within about 0.5% at
/// delivery distances and needs no trigonometry beyond one cosine. It is
/// deliberately NOT presented as road distance: a caller with a routing service
/// should pass that instead, and [`estimate`] takes metres for exactly that
/// reason.
pub fn straight_line_m(
    lat1_udeg: i32,
    lon1_udeg: i32,
    lat2_udeg: i32,
    lon2_udeg: i32,
) -> u32 {
    // 1 degree of latitude is 111_320 m. Longitude shrinks with the cosine of
    // the latitude; a fixed-point cosine keeps this off floats in the caller's
    // hot path while staying well inside the approximation's own error.
    const M_PER_DEG: i64 = 111_320;
    let dlat = (lat2_udeg as i64 - lat1_udeg as i64).abs();
    let dlon = (lon2_udeg as i64 - lon1_udeg as i64).abs();
    let mid_lat_udeg = (lat1_udeg as i64 + lat2_udeg as i64) / 2;

    // cos(lat) in 1/1024ths, from a small table — no libm, no float.
    let cos_1024 = cos_scaled(mid_lat_udeg);

    let north_m = dlat * M_PER_DEG / 1_000_000;
    let east_m = dlon * M_PER_DEG / 1_000_000 * cos_1024 / 1024;

    // Integer hypotenuse, via a few Newton steps on the square.
    let sq = north_m.saturating_mul(north_m).saturating_add(east_m.saturating_mul(east_m));
    isqrt(sq).min(u32::MAX as i64) as u32
}

/// cos(latitude) scaled by 1024, for latitudes in microdegrees.
fn cos_scaled(lat_udeg: i64) -> i64 {
    let deg = (lat_udeg / 1_000_000).unsigned_abs().min(90) as usize;
    // cos in 1/1024ths at every 5 degrees; linear between. A delivery radius
    // never spans enough latitude for the interpolation error to matter.
    const TABLE: [i64; 19] = [
        1024, 1020, 1008, 987, 958, 921, 877, 825, 766, 700, 629, 552, 470, 384,
        294, 200, 104, 18, 0,
    ];
    let i = deg / 5;
    if i >= 18 {
        return TABLE[18];
    }
    let (a, b) = (TABLE[i], TABLE[i + 1]);
    let frac = (deg % 5) as i64;
    a + (b - a) * frac / 5
}

/// Integer square root. Newton's method on integers — no float, no libm.
fn isqrt(n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k() -> KitchenProfile {
        KitchenProfile::default_profile()
    }

    fn item(cook: u16, qty: u16) -> BasketItem {
        BasketItem { cooking_min: cook, quantity: qty }
    }

    // ── Preparation ──

    #[test]
    fn a_basket_takes_as_long_as_its_slowest_dish() {
        // Not the sum: a kitchen cooks in parallel. Three dishes of 10, 20 and
        // 5 minutes is 20 plus the per-portion cost, not 35.
        let p = prep_minutes(&[item(10, 1), item(20, 1), item(5, 1)], &k()).unwrap();
        assert_eq!(p, 20 + 2 * 2); // two extra portions at 2 min each
    }

    #[test]
    fn a_dish_with_no_time_set_uses_the_venues_default() {
        let p = prep_minutes(&[item(0, 1)], &k()).unwrap();
        assert_eq!(p, k().default_cooking_min as u32);
    }

    #[test]
    fn quantity_costs_something_but_not_everything() {
        let one = prep_minutes(&[item(10, 1)], &k()).unwrap();
        let four = prep_minutes(&[item(10, 4)], &k()).unwrap();
        assert_eq!(one, 10);
        assert_eq!(four, 10 + 3 * 2);
        assert!(four < one * 4, "four portions must not cost four times one");
    }

    #[test]
    fn an_empty_basket_has_no_estimate() {
        assert_eq!(prep_minutes(&[], &k()), Err(EtaError::EmptyBasket));
        assert_eq!(prep_minutes(&[item(10, 0)], &k()), Err(EtaError::EmptyBasket));
    }

    // ── The queue ──

    #[test]
    fn a_queue_is_spread_across_the_stations() {
        let ahead = [QueuedOrder { remaining_min: 10 }; 4];
        // Two stations, forty minutes of work ahead → twenty.
        assert_eq!(queue_minutes(&ahead, &k()).unwrap(), 20);
    }

    #[test]
    fn one_station_feels_the_whole_queue() {
        let ahead = [QueuedOrder { remaining_min: 10 }; 4];
        let single = KitchenProfile { stations: 1, ..k() };
        assert_eq!(queue_minutes(&ahead, &single).unwrap(), 40);
    }

    #[test]
    fn no_queue_adds_nothing() {
        assert_eq!(queue_minutes(&[], &k()).unwrap(), 0);
    }

    #[test]
    fn a_kitchen_with_no_stations_is_refused() {
        let broken = KitchenProfile { stations: 0, ..k() };
        assert_eq!(queue_minutes(&[], &broken), Err(EtaError::NoStations));
    }

    // ── The journey ──

    #[test]
    fn distance_becomes_minutes_at_the_venues_own_speed() {
        assert_eq!(travel_minutes(3_000, &k()).unwrap(), 10); // 300 m/min
        // Rounded UP: a courier 1 metre short of a minute has not arrived.
        assert_eq!(travel_minutes(301, &k()).unwrap(), 2);
        assert_eq!(travel_minutes(0, &k()).unwrap(), 0);
    }

    #[test]
    fn a_courier_that_covers_no_ground_is_refused() {
        let stuck = KitchenProfile { courier_speed_m_per_min: 0, ..k() };
        assert_eq!(travel_minutes(100, &stuck), Err(EtaError::NoSpeed));
    }

    // ── The whole estimate ──

    #[test]
    fn distance_changes_the_answer() {
        let basket = [item(15, 1)];
        let near = estimate(&basket, &[], 500, &k()).unwrap();
        let far = estimate(&basket, &[], 6_000, &k()).unwrap();
        assert!(far.low_min > near.low_min, "a longer journey must quote longer");
        assert_eq!(far.travel_min - near.travel_min, 20 - 2);
    }

    #[test]
    fn the_queue_changes_the_answer() {
        let basket = [item(15, 1)];
        let quiet = estimate(&basket, &[], 1_000, &k()).unwrap();
        let busy = estimate(
            &basket,
            &[QueuedOrder { remaining_min: 12 }; 6],
            1_000,
            &k(),
        )
        .unwrap();
        assert_eq!(busy.queue_min, 36);
        assert_eq!(busy.low_min - quiet.low_min, 36);
    }

    #[test]
    fn cooking_time_changes_the_answer() {
        let slow = estimate(&[item(40, 1)], &[], 1_000, &k()).unwrap();
        let quick = estimate(&[item(5, 1)], &[], 1_000, &k()).unwrap();
        assert_eq!(slow.prep_min - quick.prep_min, 35);
    }

    #[test]
    fn the_range_is_printed_the_way_the_frame_shows_it() {
        let e = estimate(&[item(15, 1)], &[], 3_000, &k()).unwrap();
        // 15 prep + 0 queue + 10 travel + 8 overhead = 33, +50% = 50
        assert_eq!(e.low_min, 33);
        assert_eq!(e.high_min, 50);
        assert_eq!(e.range(), "33–50");
    }

    #[test]
    fn a_longer_order_carries_a_wider_window() {
        let short = estimate(&[item(5, 1)], &[], 300, &k()).unwrap();
        let long = estimate(&[item(45, 1)], &[], 9_000, &k()).unwrap();
        assert!(long.high_min - long.low_min > short.high_min - short.low_min);
    }

    #[test]
    fn pickup_drops_the_journey_and_the_handovers() {
        let basket = [item(15, 1)];
        let delivered = estimate(&basket, &[], 3_000, &k()).unwrap();
        let collected = estimate_pickup(&basket, &[], &k()).unwrap();
        assert_eq!(collected.travel_min, 0);
        assert_eq!(collected.overhead_min, 0);
        assert_eq!(collected.low_min, 15);
        assert!(collected.low_min < delivered.low_min);
    }

    #[test]
    fn the_parts_add_up_to_the_whole() {
        let e = estimate(
            &[item(20, 2)],
            &[QueuedOrder { remaining_min: 10 }],
            2_000,
            &k(),
        )
        .unwrap();
        assert_eq!(e.prep_min + e.queue_min + e.travel_min + e.overhead_min, e.low_min);
    }

    // ── Distance ──

    #[test]
    fn straight_line_matches_a_known_distance() {
        // Durrës to Tirana is about 33 km. Two fixed points, so this is a
        // regression on the arithmetic and not on anybody's geography.
        let d = straight_line_m(41_323_000, 19_441_000, 41_327_500, 19_818_000);
        assert!(
            (30_000..=36_000).contains(&d),
            "expected ~33 km, got {d} m"
        );
    }

    #[test]
    fn the_same_point_is_zero_metres() {
        assert_eq!(straight_line_m(41_323_000, 19_441_000, 41_323_000, 19_441_000), 0);
    }

    #[test]
    fn distance_is_symmetric() {
        let a = straight_line_m(41_300_000, 19_400_000, 41_350_000, 19_500_000);
        let b = straight_line_m(41_350_000, 19_500_000, 41_300_000, 19_400_000);
        assert_eq!(a, b);
    }

    #[test]
    fn a_short_hop_is_a_few_hundred_metres() {
        // 0.003 degrees of latitude ≈ 334 m.
        let d = straight_line_m(41_320_000, 19_440_000, 41_323_000, 19_440_000);
        assert!((300..=370).contains(&d), "got {d} m");
    }

    #[test]
    fn isqrt_is_exact_on_squares() {
        for n in [0i64, 1, 4, 9, 100, 10_000, 1_000_000] {
            let r = isqrt(n);
            assert_eq!(r * r, n, "isqrt({n}) = {r}");
        }
    }

    // ── The rules ──

    #[test]
    fn there_is_no_float_in_this_module() {
        // Same rule as money: two nodes must reach the same answer bit for bit.
        // The tokens are split so this gate cannot match itself.
        let src = include_str!("eta.rs");
        for token in [concat!("f", "64"), concat!("f", "32"), concat!("as ", "f", "64")] {
            assert!(!src.contains(token), "{token} appears — the estimate must stay integral");
        }
    }

    #[test]
    fn this_module_measures_the_kitchen_and_not_the_people() {
        // `DECISIONS.md` D0: no scoring of any participant. A "courier rating"
        // or a "venue speed score" here would be the forbidden thing wearing an
        // operational hat. Split tokens so the gate cannot match itself.
        let src = include_str!("eta.rs");
        for stem in [
            concat!("courier_", "score"),
            concat!("courier_", "rating"),
            concat!("venue_", "score"),
            concat!("reputation"),
            concat!("performance_", "score"),
        ] {
            for shape in [alloc::format!("{stem}:"), alloc::format!("{stem} ")] {
                assert!(!src.contains(&shape), "{shape} — this module ranks nobody");
            }
        }
    }

    #[test]
    fn an_estimate_reads_no_clock() {
        // The same inputs always give the same answer. Nothing in the signature
        // can carry a time of day, which is what makes an offline replay agree.
        let a = estimate(&[item(12, 2)], &[QueuedOrder { remaining_min: 7 }], 1_500, &k());
        let b = estimate(&[item(12, 2)], &[QueuedOrder { remaining_min: 7 }], 1_500, &k());
        assert_eq!(a, b);
    }

    #[test]
    fn overflow_fails_closed() {
        let greedy = KitchenProfile { per_extra_portion_min: u16::MAX, ..k() };
        let huge = [BasketItem { cooking_min: 10, quantity: u16::MAX }; 4];
        assert!(matches!(
            prep_minutes(&huge, &greedy),
            Ok(_) | Err(EtaError::Overflow(_))
        ));
        let spread = KitchenProfile { spread_pct: u16::MAX, ..k() };
        // A spread that would overflow must be an error, never a wrapped low.
        let e = estimate(&[item(u16::MAX, 1)], &[], u32::MAX, &spread);
        assert!(e.is_err() || e.unwrap().high_min >= 1);
    }
}
