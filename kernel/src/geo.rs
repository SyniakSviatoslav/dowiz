//! Geo / route kinematics — RW-06 (pure-logic port into kernel authority).
//!
//! 1:1 port of the pure math that lived in `geo-anim.ts` (haversine/lerp/bearing/
//! EMA/route-progress/ETA/snap/arriving) + `delivery-zone.ts` (ray-cast point-in-
//! polygon). Zero DOM, zero floats on money (ETA is seconds, not currency).
//!
//! RED→GREEN GATE: Rust output == TS on a fixture (parity within tolerance);
//! ray-cast parity on known polygons.

const EARTH_RADIUS_M: f64 = 6_371_000.0;
const DEG2RAD: f64 = std::f64::consts::PI / 180.0;
const RAD2DEG: f64 = 180.0 / std::f64::consts::PI;

/// Great-circle distance in meters (haversine). Matches `haversineMeters`.
pub fn haversine_meters(a_lat: f64, a_lng: f64, b_lat: f64, b_lng: f64) -> f64 {
    let (la1, la2) = (a_lat * DEG2RAD, b_lat * DEG2RAD);
    let dlat = (b_lat - a_lat) * DEG2RAD;
    let dlng = (b_lng - a_lng) * DEG2RAD;
    let h = (dlat / 2.0).sin().powi(2) + la1.cos() * la2.cos() * (dlng / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_M * h.sqrt().asin()
}

/// Linear interpolation between two lat/lng points (equirectangular approx for
/// short hops). Matches `lerpLatLng`.
pub fn lerp_lat_lng(a_lat: f64, a_lng: f64, b_lat: f64, b_lng: f64, t: f64) -> (f64, f64) {
    (a_lat + (b_lat - a_lat) * t, a_lng + (b_lng - a_lng) * t)
}

/// Initial bearing (compass degrees, 0=N, clockwise) from a→b. Matches `bearingDeg`.
pub fn bearing_deg(a_lat: f64, a_lng: f64, b_lat: f64, b_lng: f64) -> f64 {
    let (la1, la2) = (a_lat * DEG2RAD, b_lat * DEG2RAD);
    let dlng = (b_lng - a_lng) * DEG2RAD;
    let y = dlng.sin() * la2.cos();
    let x = la1.cos() * la2.sin() - la1.sin() * la2.cos() * dlng.cos();
    (y.atan2(x) * RAD2DEG + 360.0) % 360.0
}

/// Exponential moving average step. Matches `emaNext`.
pub fn ema_next(prev: f64, sample: f64, alpha: f64) -> f64 {
    prev + alpha * (sample - prev)
}

/// Fraction (0..1) of the route traversed, measured by equirectangular progress
/// along the a→b segment. Matches `progressAlongRoute`. `t` is the lerp param of
/// the current interpolated position between a and b.
pub fn progress_along_route(t: f64) -> f64 {
    t.clamp(0.0, 1.0)
}

/// ETA in seconds remaining given straight-line distance and average speed
/// (m/s). Matches `etaSeconds`. Returns 0 when at/over target.
pub fn eta_seconds(distance_m: f64, speed_mps: f64) -> f64 {
    if speed_mps <= 0.0 {
        return f64::INFINITY;
    }
    (distance_m / speed_mps).max(0.0)
}

/// Snap-to-target when within `threshold_m`. Matches `shouldSnap`.
pub fn should_snap(distance_m: f64, threshold_m: f64) -> bool {
    distance_m <= threshold_m
}

/// True when within arrival radius. Matches `isArriving`.
pub fn is_arriving(distance_m: f64, arrival_radius_m: f64) -> bool {
    distance_m <= arrival_radius_m
}

/// Ray-cast point-in-polygon (even-odd rule). Matches `delivery-zone.ts`.
/// `polygon` = flat `[(lat,lng), ...]` ring (open or closed — auto-wraps).
pub fn point_in_polygon(pt_lat: f64, pt_lng: f64, polygon: &[(f64, f64)]) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (yi, xi) = polygon[i];
        let (yj, xj) = polygon[j];
        let intersect = ((yi > pt_lat) != (yj > pt_lat))
            && (pt_lng < (xj - xi) * (pt_lat - yi) / (yj - yi) + xi);
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    // Parity: London→Paris great-circle ≈ 343_000 m.
    #[test]
    fn haversine_london_paris() {
        let d = haversine_meters(51.5074, -0.1278, 48.8566, 2.3522);
        assert!(
            (d - 343_000.0).abs() < 2_000.0,
            "haversine London→Paris ≈ 343km, got {d}"
        );
    }

    // Bearing London→Paris ≈ 148° (SE).
    #[test]
    fn bearing_london_paris() {
        let b = bearing_deg(51.5074, -0.1278, 48.8566, 2.3522);
        assert!(
            (b - 148.0).abs() < 3.0,
            "bearing London→Paris ≈ 148°, got {b}"
        );
    }

    // lerp midpoint is the average.
    #[test]
    fn lerp_midpoint() {
        let (lat, lng) = lerp_lat_lng(0.0, 0.0, 10.0, 20.0, 0.5);
        assert!((lat - 5.0).abs() < 1e-9);
        assert!((lng - 10.0).abs() < 1e-9);
    }

    // EMA converges toward the sample.
    #[test]
    fn ema_converges() {
        let mut v = 0.0;
        for _ in 0..50 {
            v = ema_next(v, 100.0, 0.1);
        }
        assert!((v - 100.0).abs() < 1.0, "EMA converges to 100, got {v}");
    }

    // ETA: 1000 m at 10 m/s = 100 s.
    #[test]
    fn eta_basic() {
        assert!((eta_seconds(1000.0, 10.0) - 100.0).abs() < 1e-9);
        assert!(
            eta_seconds(1000.0, 0.0).is_infinite(),
            "zero speed → infinite ETA"
        );
    }

    // snap / arriving thresholds.
    #[test]
    fn snap_and_arriving() {
        assert!(should_snap(5.0, 10.0));
        assert!(!should_snap(50.0, 10.0));
        assert!(is_arriving(3.0, 5.0));
        assert!(!is_arriving(9.0, 5.0));
    }

    // Ray-cast: a point inside a square is inside; outside is outside.
    #[test]
    fn point_in_polygon_square() {
        let sq = [(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)];
        assert!(point_in_polygon(5.0, 5.0, &sq));
        assert!(!point_in_polygon(15.0, 5.0, &sq));
        assert!(!point_in_polygon(-1.0, 5.0, &sq));
    }

    // progress clamps to [0,1].
    #[test]
    fn progress_clamps() {
        assert_eq!(progress_along_route(-1.0), 0.0);
        assert_eq!(progress_along_route(2.0), 1.0);
        assert_eq!(progress_along_route(0.4), 0.4);
    }
}
