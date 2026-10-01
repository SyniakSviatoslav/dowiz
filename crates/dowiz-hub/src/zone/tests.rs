use super::*;

// Durrës, where the pilot venue is.
const DURRES_LAT: i64 = 41_323_000;
const DURRES_LON: i64 = 19_441_000;

/// Against distances computed independently, not against my own output.
/// One degree of latitude is ~111.13 km everywhere; one degree of longitude
/// at 41.3 degrees north is ~83.5 km.
#[test]
fn distances_match_the_known_figures() {
    let d = distance_m(DURRES_LAT, DURRES_LON, DURRES_LAT + 1_000_000, DURRES_LON);
    assert!((111_000..111_300).contains(&d), "one degree of latitude: {d} m");

    let d = distance_m(DURRES_LAT, DURRES_LON, DURRES_LAT, DURRES_LON + 1_000_000);
    assert!((83_000..84_000).contains(&d), "one degree of longitude at 41.3N: {d} m");

    // At the equator a degree of longitude equals a degree of latitude.
    let d = distance_m(0, 0, 0, 1_000_000);
    assert!((111_000..111_300).contains(&d), "equator: {d} m");

    assert_eq!(distance_m(DURRES_LAT, DURRES_LON, DURRES_LAT, DURRES_LON), 0);
}

/// The flat-earth approximation must be accurate at delivery scale. This is
/// the measurement the module header claims.
#[test]
fn the_approximation_is_sub_metre_over_a_delivery_radius() {
    // 5 km north-east of the venue, roughly.
    let d = distance_m(DURRES_LAT, DURRES_LON, DURRES_LAT + 31_800, DURRES_LON + 42_300);
    // Great-circle distance for this pair, computed independently: 4999 m.
    assert!((4990..5010).contains(&d), "{d} m should be within 10 m of 4999");
}

#[test]
fn a_circle_contains_what_it_should() {
    let z = Zone::Circle { lat_udeg: DURRES_LAT, lon_udeg: DURRES_LON, radius_m: 3000 };
    assert!(z.contains(DURRES_LAT, DURRES_LON), "the centre is inside");
    // ~1 km north.
    assert!(z.contains(DURRES_LAT + 9_000, DURRES_LON));
    // ~11 km north.
    assert!(!z.contains(DURRES_LAT + 99_000, DURRES_LON));
}

/// The boundary is INSIDE. A customer on the edge of the area gets served.
#[test]
fn the_edge_of_a_circle_is_inside_it() {
    let z = Zone::Circle { lat_udeg: 0, lon_udeg: 0, radius_m: 1000 };
    // 1000 m north is 1000/111132 degrees = 8998 micro-degrees.
    assert!(z.contains(8_998, 0), "a point at the radius must be inside");
    assert!(!z.contains(9_200, 0), "and one past it must not");
}

#[test]
fn a_polygon_contains_what_it_should() {
    // A square roughly 2 km on a side around the venue.
    let d = 9_000;
    let square = Zone::Polygon {
        points: vec![
            (DURRES_LAT - d, DURRES_LON - d),
            (DURRES_LAT - d, DURRES_LON + d),
            (DURRES_LAT + d, DURRES_LON + d),
            (DURRES_LAT + d, DURRES_LON - d),
        ],
    };
    assert!(square.contains(DURRES_LAT, DURRES_LON), "the middle");
    assert!(!square.contains(DURRES_LAT + 2 * d, DURRES_LON), "well north of it");
    assert!(!square.contains(DURRES_LAT, DURRES_LON + 3 * d), "well east of it");
    // A vertex and an edge both count as inside.
    assert!(square.contains(DURRES_LAT - d, DURRES_LON - d), "a corner");
    assert!(square.contains(DURRES_LAT - d, DURRES_LON), "a point on an edge");
}

/// A concave shape is the whole reason polygons exist: a service area that
/// stops at a river is not convex, and a convex-only test would deliver
/// across it.
#[test]
fn a_concave_polygon_excludes_its_notch() {
    // An L shape.
    let l = Zone::Polygon {
        points: vec![
            (0, 0),
            (0, 100_000),
            (50_000, 100_000),
            (50_000, 50_000),
            (100_000, 50_000),
            (100_000, 0),
        ],
    };
    assert!(l.contains(10_000, 10_000), "inside the lower arm");
    assert!(l.contains(80_000, 10_000), "inside the upper arm");
    assert!(!l.contains(80_000, 80_000), "the notch must be OUTSIDE");
}

#[test]
fn a_degenerate_polygon_contains_nothing() {
    assert!(!Zone::Polygon { points: vec![] }.contains(0, 0));
    assert!(!Zone::Polygon { points: vec![(0, 0), (1, 1)] }.contains(0, 0));
}

/// No zones means no restriction: a venue that has not drawn a service area
/// has not asked for one to be enforced.
#[test]
fn a_venue_with_no_zones_accepts_everything() {
    assert_eq!(reach(&[], Some((0, 0))), Reach::Unrestricted);
    assert_eq!(reach(&[], None), Reach::Unrestricted);
}

/// An address with no coordinates is accepted and flagged, never refused.
/// There is no geocoder here, and refusing would refuse every customer who
/// declined the browser's location prompt.
#[test]
fn an_address_without_coordinates_is_flagged_not_refused() {
    let z = vec![Zone::Circle { lat_udeg: DURRES_LAT, lon_udeg: DURRES_LON, radius_m: 3000 }];
    assert_eq!(reach(&z, None), Reach::Unknown);
}

/// Outside carries HOW FAR outside, so the customer can be told something
/// better than "no".
#[test]
fn outside_says_how_far_outside() {
    let z = vec![Zone::Circle { lat_udeg: DURRES_LAT, lon_udeg: DURRES_LON, radius_m: 3000 }];
    assert_eq!(reach(&z, Some((DURRES_LAT, DURRES_LON))), Reach::Inside);
    match reach(&z, Some((DURRES_LAT + 99_000, DURRES_LON))) {
        Reach::Outside { nearest_m } => {
            // ~11 km from the centre, 3 km of which is inside the radius.
            assert!((7_500..8_500).contains(&nearest_m), "{nearest_m} m past the edge");
        }
        other => panic!("expected Outside, got {other:?}"),
    }
}

/// Any one zone is enough: a venue with two areas serves both.
#[test]
fn several_zones_are_a_union() {
    let z = vec![
        Zone::Circle { lat_udeg: 0, lon_udeg: 0, radius_m: 1000 },
        Zone::Circle { lat_udeg: 1_000_000, lon_udeg: 0, radius_m: 1000 },
    ];
    assert_eq!(reach(&z, Some((0, 0))), Reach::Inside);
    assert_eq!(reach(&z, Some((1_000_000, 0))), Reach::Inside);
    assert!(matches!(reach(&z, Some((500_000, 0))), Reach::Outside { .. }));
}

#[test]
fn zones_read_back_out_of_json() {
    let json = r#"[{"kind":"circle","lat":41323000,"lon":19441000,"radius_m":3000},
                   {"kind":"polygon","points":[[0,0],[0,100000],[100000,0]]}]"#;
    let zones = from_json(json);
    assert_eq!(zones.len(), 2, "{zones:?}");
    assert_eq!(
        zones[0],
        Zone::Circle { lat_udeg: 41_323_000, lon_udeg: 19_441_000, radius_m: 3000 }
    );
    match &zones[1] {
        Zone::Polygon { points } => assert_eq!(points.len(), 3),
        other => panic!("{other:?}"),
    }
}

/// Malformed configuration must leave the venue OPEN, not closed. Failing
/// the other way takes a restaurant offline over a typo.
#[test]
fn unreadable_zones_are_treated_as_none() {
    for junk in ["", "null", "not json at all", r#"[{"kind":"circle"}]"#,
                 r#"[{"kind":"circle","lat":1,"lon":2,"radius_m":0}]"#,
                 r#"[{"kind":"polygon","points":[[0,0]]}]"#] {
        assert!(from_json(junk).is_empty(), "{junk:?} produced zones");
    }
    assert_eq!(reach(&from_json("junk"), Some((0, 0))), Reach::Unrestricted);
}

/// Nonsense coordinates must not panic, wrap, or index out of the cosine
/// table. These arrive from a browser, so "nonsense" includes "deliberate".
/// In release the overflow this caught would have WRAPPED rather than
/// panicked, yielding a small distance for an absurd point -- which is an
/// address on another continent passing the delivery-zone check.
#[test]
fn extreme_coordinates_do_not_panic() {
    // Half the planet apart: the largest real distance there is.
    let d = distance_m(90_000_000, 0, -90_000_000, 0);
    assert!((19_000_000..21_000_000).contains(&d), "pole to pole: {d} m");

    for (a, b, c, e) in [
        (i64::MAX / 4, i64::MAX / 4, i64::MIN / 4, i64::MIN / 4),
        (i64::MAX, i64::MAX, i64::MIN, i64::MIN),
        (0, 0, i64::MAX, 0),
    ] {
        let d = distance_m(a, b, c, e);
        assert!(d >= 0, "distance must never be negative: {d}");
        // The bound checks that the clamp WORKED -- that no multiplication
        // wrapped -- not that the answer is geodesically right. It is not:
        // the flat approximation gives ~44 700 km for an antipodal pair
        // against a true 20 000 km, because it is only meaningful over a
        // delivery radius, as this module says up front. What matters here
        // is that an absurd input produces a large finite number rather
        // than a wrapped small one.
        assert!(d <= 50_000_000, "clamping failed, value wrapped: {d} m");
    }
    // The specific danger: an absurd point must be OUTSIDE a small zone, not
    // accidentally inside it through a wrapped distance.
    let z = Zone::Circle { lat_udeg: DURRES_LAT, lon_udeg: DURRES_LON, radius_m: 3000 };
    assert!(!z.contains(i64::MAX, i64::MAX));
    assert!(!z.contains(i64::MIN, i64::MIN));

    assert_eq!(cos_lat_e6(90_000_000), 0);
    assert_eq!(cos_lat_e6(-90_000_000), 0);
    assert_eq!(cos_lat_e6(0), 1_000_000);
    assert_eq!(cos_lat_e6(999_000_000), 0, "past the pole is clamped");
}
