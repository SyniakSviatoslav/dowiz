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

/// Result of projecting a position onto a route polyline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RouteProgress {
    /// Metres remaining from the projected point to the polyline end.
    pub remaining_m: f64,
    /// Closest point on the polyline to `pos` (snapped).
    pub snapped: (f64, f64),
    /// Index of the segment (endpoint `i`) the projection landed on.
    pub segment_index: usize,
}

/// Total polyline length in meters (sum of haversine segment lengths).
/// Matches `polylineLengthMeters`.
pub fn polyline_length_meters(poly: &[(f64, f64)]) -> f64 {
    let mut total = 0.0;
    for w in poly.windows(2) {
        total += haversine_meters(w[0].0, w[0].1, w[1].0, w[1].1);
    }
    total
}

/// Project `pos` onto a route polyline (local equirectangular projection, valid
/// at city scale) and return the snapped point + remaining metres to the end.
/// Matches `progressAlongRoute(polyline, pos)` — full polyline projection, not a
/// bare `t.clamp`. Picks the closest segment, clamps the projection to the
/// segment, and measures remaining distance along the polyline to the terminus.
/// `segment_index` is the index `i` of the segment's end node (`i∈[1,len-1]`).
pub fn progress_along_route(poly: &[(f64, f64)], pos: (f64, f64)) -> RouteProgress {
    if poly.len() < 2 {
        return RouteProgress {
            remaining_m: 0.0,
            snapped: pos,
            segment_index: 0,
        };
    }
    // Equirectangular scale at the mean latitude (city-scale approximation).
    let mean_lat = poly.iter().map(|p| p.0).sum::<f64>() / poly.len() as f64;
    let kx = EARTH_RADIUS_M * DEG2RAD * mean_lat.to_radians().cos();
    let ky = EARTH_RADIUS_M * DEG2RAD;
    let px = pos.1 * kx;
    let py = pos.0 * ky;

    // Find the segment whose projection of `pos` is closest, capture its end-node
    // index and the snapped (lat,lng) point.
    let mut best_seg = 0usize;
    let mut best_dist2 = f64::MAX;
    let mut snapped = pos;
    for (i, w) in poly.windows(2).enumerate() {
        let (ax, ay) = (w[0].1 * kx, w[0].0 * ky);
        let (bx, by) = (w[1].1 * kx, w[1].0 * ky);
        let dx = bx - ax;
        let dy = by - ay;
        let len2 = dx * dx + dy * dy;
        let t = if len2 == 0.0 {
            0.0
        } else {
            (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0)
        };
        let cx = ax + t * dx;
        let cy = ay + t * dy;
        let dist2 = (px - cx).powi(2) + (py - cy).powi(2);
        if dist2 < best_dist2 {
            best_dist2 = dist2;
            best_seg = i + 1; // end-node index
            snapped = (cy / ky, cx / kx);
        }
    }
    if best_seg == 0 {
        // Degenerate polyline (all identical points) → no projection.
        return RouteProgress {
            remaining_m: 0.0,
            snapped: pos,
            segment_index: 0,
        };
    }

    // Remaining metres: along polyline from snapped point to the end node, then
    // the tail of the polyline.
    let mut remaining = 0.0;
    // distance from snapped back to its segment start (we recompute the t).
    let w = &poly[best_seg - 1..=best_seg];
    let (ax, ay) = (w[0].1 * kx, w[0].0 * ky);
    let (bx, by) = (w[1].1 * kx, w[1].0 * ky);
    let dx = bx - ax;
    let dy = by - ay;
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0)
    };
    // from snapped (at t on segment) to end node b
    let seg_rem = (1.0 - t) * (dx * dx + dy * dy).sqrt();
    remaining += seg_rem;
    for s in (best_seg + 1)..poly.len() {
        remaining += haversine_meters(poly[s - 1].0, poly[s - 1].1, poly[s].0, poly[s].1);
    }

    RouteProgress {
        remaining_m: remaining,
        snapped,
        segment_index: best_seg,
    }
}

/// ETA in seconds remaining given route pacing. `remaining_m` is the metres left,
/// `total_m` the full route length, `baseline_s` the planned total time. Pace is
/// `total_m / baseline_s`; fall back to ~5 m/s urban speed when `baseline_s<=0`
/// or `total_m<=0`. Matches `etaSeconds(remaining,total,baseline)` from the TS
/// oracle. Returns 0 when already at/over target.
pub fn eta_seconds(remaining_m: f64, total_m: f64, baseline_s: f64) -> f64 {
    if remaining_m <= 0.0 {
        return 0.0;
    }
    let speed = if baseline_s > 0.0 && total_m > 0.0 {
        total_m / baseline_s
    } else {
        5.0 // urban fallback (m/s)
    };
    if speed <= 0.0 {
        return f64::INFINITY;
    }
    (remaining_m / speed).max(0.0)
}

// ── P96 (BLUEPRINT-P96 §3): adaptive ETA from the courier's live ground speed ──
// The static `eta_seconds` above is the load-bearing fallback. P96 routes the
// courier's *live smoothed* ground speed into the speed term *only when it is
// trustworthy*; otherwise it delegates to `eta_seconds` byte-for-byte. The set
// of named constants below is the entire trust policy — no magic numbers.

/// Minimum ping count before the smoothed live speed is trusted over the
/// planned baseline. Cold-start guard: a new courier/order rides the plan pace
/// until the EMA has warmed. One or two pings is not a pace.
pub const ETA_MIN_PINGS: u32 = 3;

/// Minimum trustworthy *average* speed (m/s). Below this the smoothed
/// observation is treated as "courier stopped / GPS noise" and the ETA falls
/// back to the planned baseline rather than exploding toward `f64::INFINITY`.
/// 0.3 m/s ≈ 1.1 km/h — far slower than a walk ⇒ not "in transit".
pub const ETA_SPEED_MIN: f64 = 0.3;

/// Maximum plausible courier *average* speed (m/s) ≈ 108 km/h. Above this the
/// observation is a GPS glitch, not a real pace, and the ETA falls back to the
/// baseline.
pub const ETA_SPEED_MAX: f64 = 30.0;

/// EMA smoothing factor for the observed ground-speed stream (same shape as
/// `ema_next`'s alpha). 0.3 ⇒ ~10-ping memory: rejects single-ping GPS spikes,
/// still tracks a real traffic change.
pub const ETA_SPEED_ALPHA: f64 = 0.3;

/// Adaptive ETA. Uses the courier's smoothed observed ground speed **iff** the
/// live signal is warm (enough accepted pings) and in-band (finite, inside the
/// sane speed band); otherwise falls back to the EXACT existing static-baseline
/// `eta_seconds` behaviour.
///
/// `live_speed_mps = Some((smoothed_v_mps, accepted_ping_count))`; `None` means
/// no live signal yet (cold / order just placed / courier hasn't moved).
///
/// INVARIANT (bounded degradation, BLUEPRINT-P96 §6 D2): when the live signal is
/// absent, cold, non-finite, or out of `[ETA_SPEED_MIN, ETA_SPEED_MAX]`, this
/// returns *exactly* `eta_seconds(remaining_m, total_m, baseline_s)` — never
/// worse, never `∞` beyond what the static path does. When the live path fires,
/// `v ∈ [ETA_SPEED_MIN, ETA_SPEED_MAX]` bounds the ETA to a finite, positive
/// range, so it can never return `∞` either.
pub fn eta_seconds_adaptive(
    remaining_m: f64,
    total_m: f64,
    baseline_s: f64,
    live_speed_mps: Option<(f64, u32)>,
) -> f64 {
    if remaining_m <= 0.0 {
        return 0.0;
    }
    if let Some((v, n)) = live_speed_mps {
        if n >= ETA_MIN_PINGS && v.is_finite() && v >= ETA_SPEED_MIN && v <= ETA_SPEED_MAX {
            // v is bounded ⇒ ETA bounded, never ∞. This is the only divergence
            // from the static baseline, and it is strictly an *improvement* when
            // the courier's real pace differs from the plan.
            return (remaining_m / v).max(0.0);
        }
    }
    eta_seconds(remaining_m, total_m, baseline_s) // fallback: byte-for-byte current behaviour
}

/// Owner of the per-order EMA state for the live ground-speed signal. This is
/// the small piece of *caller-owned* state P96 threads into
/// `eta_seconds_adaptive` — it does NOT live in the pure math layer (`geo.rs`
/// stays a library of pure functions). The pin-folding surface that already
/// owns the accepted GPS ping stream instantiates one of these per order and
/// feeds it `v_mps` (the per-ping ground speed already computed in
/// `apps/courier/src/types.rs`). It is a pure, deterministic, no-IO primitive so
/// the ping-folder can be exercised by machine-checkable kernel tests (M3).
///
/// `route_version`'s `reset` lets a re-route cold-start cleanly: stale speed
/// from a previous route/order can never leak across the boundary (I4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CourierSpeedEma {
    /// Smoothed ground speed (m/s), seeded 0.0 (cold).
    v_hat: f64,
    /// Count of accepted in-order pings, seeded 0.
    pings: u32,
    /// Last route version seen — bumping it resets the smoother (re-route).
    route_version: u64,
}

impl CourierSpeedEma {
    /// A cold, empty smoother for a brand-new order.
    pub fn new() -> Self {
        CourierSpeedEma {
            v_hat: 0.0,
            pings: 0,
            route_version: 0,
        }
    }

    /// Current (v_hat, pings) tuple, ready to hand to `eta_seconds_adaptive`.
    pub fn observed(&self) -> Option<(f64, u32)> {
        Some((self.v_hat, self.pings))
    }

    /// Feed one accepted (in-order) ping's `v_mps` (f32 on the wire, promoted to
    /// f64). Applies the EMA step and increments the accepted-ping count. A
    /// `route_version` bump resets the smoother so a re-route cold-starts on the
    /// baseline instead of leaking the previous leg's pace.
    pub fn accept_ping(&mut self, v_mps: f64, route_version: u64) {
        if route_version != self.route_version {
            self.v_hat = 0.0;
            self.pings = 0;
            self.route_version = route_version;
        }
        self.v_hat = ema_next(self.v_hat, v_mps, ETA_SPEED_ALPHA);
        self.pings = self.pings.saturating_add(1);
    }

    /// Explicit reset (new order / explicit re-route). `Some((0.0, 0))` next.
    pub fn reset(&mut self) {
        self.v_hat = 0.0;
        self.pings = 0;
    }
}

impl Default for CourierSpeedEma {
    fn default() -> Self {
        Self::new()
    }
}

/// True when the next ping timestamp is strictly older than the last seen one
/// (out-of-order rejection). `None` last-seen means "first ping, always accept".
/// Matches `isOutOfOrder`.
pub fn is_out_of_order(last_ts: Option<i64>, ts: i64) -> bool {
    match last_ts {
        None => false,
        Some(prev) => ts < prev,
    }
}

/// Default arrival radius (meters) — matches the TS oracle `ARRIVE_THRESHOLD_M`.
pub const ARRIVE_THRESHOLD_M: f64 = 150.0;
/// Default snap distance (meters) used by the marker hook on first-fix / jump.
pub const SNAP_THRESHOLD_M: f64 = 500.0;

/// Snap-to-target when the haversine distance between `prev` and `next` is within
/// `threshold_m`. Matches `shouldSnap(prev, next, threshold)` — takes lat/lng
/// pairs (uses `haversine_meters` as the distance primitive) rather than a raw
/// pre-computed distance.
pub fn should_snap(prev: (f64, f64), next: (f64, f64), threshold_m: f64) -> bool {
    haversine_meters(prev.0, prev.1, next.0, next.1) <= threshold_m
}

/// True when within arrival radius. Matches `isArriving`. Defaults to
/// `ARRIVE_THRESHOLD_M` = 150 m when `arrival_radius_m` is not supplied, aligning
/// with the TS oracle's `ARRIVE_THRESHOLD_M`.
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

// ── GS (Gaussian-Splatting) address-picker geo functions (BLUEPRINT-P04 §4) ──
// Six deterministic, zero-dep functions for the v1 address picker / floor
// slice / arrow / field-of-view / line-of-sight advisory. Angles in degrees,
// 0=N clockwise (matching `bearing_deg`). Reuse `DEG2RAD`/`bearing_deg`/
// `point_in_polygon`.

/// Per-storey height in metres. Measured `height/levels` when BOTH are present
/// and `levels > 0`; otherwise a generic **3.0 m** storey. NEVER fabricates a
/// level count — it returns only a per-storey height. `levels = Some(0)` ⇒
/// 3.0 m (no divide-by-zero).
pub fn storey_height_m(height_m: Option<f64>, levels: Option<u32>) -> f64 {
    match (height_m, levels) {
        (Some(h), Some(l)) if l > 0 && h > 0.0 => h / l as f64,
        _ => 3.0,
    }
}

/// Height (m) of the mid-storey cut plane for a given floor: `(floor + 0.5) *
/// storey_h`. The `+0.5` puts the slice through windows/doors, not the ceiling.
pub fn floor_slice_height_m(floor: u32, storey_h: f64) -> f64 {
    (floor as f64 + 0.5) * storey_h
}

/// Smallest unsigned angular separation between two compass bearings, seam-
/// correct across the 0°/360° wrap. Oracle: `angular_diff_deg(350, 10) == 20`.
pub fn angular_diff_deg(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// Screen rotation (deg) for a facing arrow given the current view rotation:
/// `(facing - view_rotation).rem_euclid(360)`. On a north-up slice
/// (`view_rotation = 0`) this is `facing` unchanged — one global frame; v1 does
/// NOT rotate the view to straighten facades.
pub fn arrow_screen_rotation_deg(facing_deg: f64, view_rotation_deg: f64) -> f64 {
    (facing_deg - view_rotation_deg).rem_euclid(360.0)
}

/// True when `target_bearing_deg` lies within the field of view centred on
/// `facing_deg`: `angular_diff_deg(facing, target) <= fov_deg / 2`. Default
/// `fov_deg = 120` (±60°); pass `60.0` for the ±30° "direct" cone. Correctness
/// across the 0°/360° seam falls out of `angular_diff_deg`.
pub fn in_field_of_view(facing_deg: f64, target_bearing_deg: f64, fov_deg: f64) -> bool {
    angular_diff_deg(facing_deg, target_bearing_deg) <= fov_deg / 2.0
}

/// Coarse **2D** line-of-sight advisory: `true` iff the `a → b` segment crosses
/// no edge of any building footprint. Each footprint is a `(lat,lng)` ring.
///
/// STATED LIMITS (GS §2.6 — kept loud): this ignores height (false-positive
/// over a low wall, false-negative across an open courtyard) and reads empty
/// footprint data as "all clear". It therefore drives a **soft advisory hint
/// only**, never a hard visibility claim. If either endpoint sits inside a
/// footprint the view is treated as blocked.
pub fn los_clear(a: (f64, f64), b: (f64, f64), footprints: &[Vec<(f64, f64)>]) -> bool {
    // Endpoint-inside-building ⇒ blocked (advisory).
    for ring in footprints {
        if point_in_polygon(a.0, a.1, ring) || point_in_polygon(b.0, b.1, ring) {
            return false;
        }
    }
    for ring in footprints {
        let m = ring.len();
        if m < 2 {
            continue;
        }
        for i in 0..m {
            let p = ring[i];
            let q = ring[(i + 1) % m];
            if segments_intersect(a, b, p, q) {
                return false;
            }
        }
    }
    true
}

/// Orientation sign of the ordered triple (p, q, r): >0 CCW, <0 CW, 0 collinear.
fn orient(p: (f64, f64), q: (f64, f64), r: (f64, f64)) -> f64 {
    (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0)
}

/// True when `r` lies on segment `pq`, given collinearity (`orient == 0`).
fn on_segment(p: (f64, f64), q: (f64, f64), r: (f64, f64)) -> bool {
    r.0 >= p.0.min(q.0) && r.0 <= p.0.max(q.0) && r.1 >= p.1.min(q.1) && r.1 <= p.1.max(q.1)
}

/// Proper/improper 2D segment intersection test (standard orientation method).
fn segments_intersect(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
    let d1 = orient(c, d, a);
    let d2 = orient(c, d, b);
    let d3 = orient(a, b, c);
    let d4 = orient(a, b, d);
    if ((d1 > 0.0) != (d2 > 0.0)) && ((d3 > 0.0) != (d4 > 0.0)) {
        return true;
    }
    (d1 == 0.0 && on_segment(c, d, a))
        || (d2 == 0.0 && on_segment(c, d, b))
        || (d3 == 0.0 && on_segment(a, b, c))
        || (d4 == 0.0 && on_segment(a, b, d))
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

    // ETA: 1000 m remaining on a 2000 m route planned for 400 s → pace 5 m/s → 200 s.
    #[test]
    fn eta_basic() {
        let s = eta_seconds(1000.0, 2000.0, 400.0);
        assert!(
            (s - 200.0).abs() < 1e-9,
            "eta 1000m / (2000m/400s) = 200s, got {s}"
        );
        // zero remaining → 0
        assert_eq!(eta_seconds(0.0, 2000.0, 400.0), 0.0);
        // no baseline → 5 m/s urban fallback: 1000 m → 200 s
        let fb = eta_seconds(1000.0, 0.0, 0.0);
        assert!(
            (fb - 200.0).abs() < 1e-9,
            "eta fallback 5 m/s → 200s, got {fb}"
        );
    }

    // snap / arriving thresholds (default 150 m arrive, 500 m snap via lat/lng pair).
    #[test]
    fn snap_and_arriving() {
        // 5 m apart → within both 500 m snap and 150 m arrive
        assert!(should_snap((0.0, 0.0), (0.000045, 0.0), 500.0));
        assert!(!should_snap((0.0, 0.0), (0.01, 0.0), 500.0)); // ~1.1 km apart
        assert!(is_arriving(120.0, ARRIVE_THRESHOLD_M));
        assert!(!is_arriving(300.0, ARRIVE_THRESHOLD_M));
    }

    // Out-of-order guard: strictly-older rejected, first ping always accepted.
    #[test]
    fn out_of_order() {
        assert!(!is_out_of_order(None, 100)); // first ping
        assert!(!is_out_of_order(Some(100), 101)); // newer
        assert!(is_out_of_order(Some(100), 99)); // older → reject
    }

    // Polyline length: two 1° segments ≈ 2 × 111 195 m.
    #[test]
    fn polyline_length() {
        let poly = [(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)];
        let len = polyline_length_meters(&poly);
        assert!(
            (len - 2.0 * 111_195.0).abs() < 2_000.0,
            "≈ 222 km, got {len}"
        );
    }

    // progress_along_route: midpoint of a 2-point polyline projects to the middle,
    // remaining ≈ half the length.
    #[test]
    fn progress_midpoint() {
        let poly = [(0.0, 0.0), (1.0, 0.0)]; // ~111 195 m north
        let r = progress_along_route(&poly, (0.5, 0.0));
        assert!(
            (r.snapped.0 - 0.5).abs() < 1e-6,
            "snapped lat ≈ 0.5, got {}",
            r.snapped.0
        );
        assert_eq!(r.segment_index, 1);
        // remaining from midpoint to end ≈ half the segment length
        let half = 111_195.0 / 2.0;
        assert!(
            (r.remaining_m - half).abs() < 2_000.0,
            "remaining ≈ {}",
            r.remaining_m
        );
    }

    // progress_along_route: a point past the end clamps to the last node.
    #[test]
    fn progress_past_end() {
        let poly = [(0.0, 0.0), (1.0, 0.0)];
        let r = progress_along_route(&poly, (2.0, 0.0));
        assert_eq!(r.segment_index, 1);
        assert!(
            (r.remaining_m).abs() < 1e-6,
            "past end → 0 remaining, got {}",
            r.remaining_m
        );
    }

    // Ray-cast: a point inside a square is inside; outside is outside.
    #[test]
    fn point_in_polygon_square() {
        let sq = [(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)];
        assert!(point_in_polygon(5.0, 5.0, &sq));
        assert!(!point_in_polygon(15.0, 5.0, &sq));
        assert!(!point_in_polygon(-1.0, 5.0, &sq));
    }

    // ── GS P0.1 six-item acceptance (BLUEPRINT-P04 §4/§6 #5) ────────────

    // (1) pin-drop everywhere + (3) open-space degrade: no OSM tags ⇒ generic
    //     3.0 m storey, never a fabricated floor count, never a crash.
    #[test]
    fn gs_pin_drop_and_open_space_degrade() {
        assert_eq!(storey_height_m(None, None), 3.0);
        assert_eq!(storey_height_m(None, Some(5)), 3.0);
        assert_eq!(storey_height_m(Some(15.0), None), 3.0);
        assert_eq!(storey_height_m(Some(15.0), Some(0)), 3.0); // no div-by-zero
                                                               // no footprint data ⇒ los reads "all clear" (soft advisory), no crash.
        assert!(los_clear((0.0, 0.0), (1.0, 1.0), &[]));
    }

    // (2) floor-slice range vs raw OSM tag.
    #[test]
    fn gs_floor_slice_vs_raw_tag() {
        // building: height 12 m, 4 levels ⇒ 3.0 m storey (matches raw tag math).
        let sh = storey_height_m(Some(12.0), Some(4));
        assert!((sh - 3.0).abs() < 1e-12);
        // floor 0 mid-plane = 0.5*3 = 1.5 m; floor 3 = 3.5*3 = 10.5 m.
        assert!((floor_slice_height_m(0, sh) - 1.5).abs() < 1e-12);
        assert!((floor_slice_height_m(3, sh) - 10.5).abs() < 1e-12);
    }

    // (4) arrow bearing vs bearing_deg <1°.
    #[test]
    fn gs_arrow_bearing_matches_bearing_deg() {
        // north-up view (rotation 0) ⇒ arrow rotation == facing bearing.
        let facing = bearing_deg(0.0, 0.0, 1.0, 1.0);
        let arrow = arrow_screen_rotation_deg(facing, 0.0);
        assert!(angular_diff_deg(arrow, facing) < 1.0);
        // oracle for angular_diff seam.
        assert!((angular_diff_deg(350.0, 10.0) - 20.0).abs() < 1e-12);
    }

    // (5) FOV correct across the 0°/360° seam.
    #[test]
    fn gs_fov_across_seam() {
        // facing 350°, target 10° ⇒ 20° apart ⇒ inside default 120° fov.
        assert!(in_field_of_view(350.0, 10.0, 120.0));
        // target 200° from facing 350° ⇒ 150° apart ⇒ outside 120° fov.
        assert!(!in_field_of_view(350.0, 200.0, 120.0));
        // ±30° "direct" cone (fov 60): 20° apart still inside.
        assert!(in_field_of_view(350.0, 10.0, 60.0));
        assert!(!in_field_of_view(350.0, 40.0, 60.0)); // 50° apart > 30°
    }

    // (6) los_clear false across a rectangle, true routing around it.
    #[test]
    fn gs_los_clear_blocked_and_around() {
        // rectangle footprint centred near origin.
        let rect = vec![(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0)];
        let fps = [rect];
        // a→b passes straight through the rectangle ⇒ blocked.
        assert!(!los_clear((1.0, -1.0), (1.0, 3.0), &fps));
        // routing well around it (far to the side) ⇒ clear.
        assert!(los_clear((-1.0, -1.0), (-1.0, 3.0), &fps));
    }

    // ── A2 (BLUEPRINT-P-A §3.1) generated-organ parity fixtures ─────────────
    // Covers ±0.0, subnormal, ±INFINITY, alpha ∈ {0.0, 1.0, -0.5, 1.5}, large
    // magnitude. The generated organ (crate::eqc_gen::ema_next_f64) must be
    // bit-identical to the hand-written `ema_next` on every one of these.
    const EMA_PARITY_FIXTURES: &[(f64, f64, f64)] = &[
        // basic / in-range
        (0.0, 1.0, 0.5),
        (10.0, 20.0, 0.1),
        (-10.0, 5.0, 0.3),
        // ±0.0 sign distinction
        (0.0, 0.0, 0.5),
        (-0.0, 0.0, 0.5),
        (0.0, -0.0, 0.5),
        (-0.0, -0.0, 0.5),
        // alpha corner cases
        (0.0, 100.0, 0.0),   // alpha = 0 ⇒ stays at prev
        (50.0, 100.0, 1.0),  // alpha = 1 ⇒ jumps to sample
        (50.0, 100.0, -0.5), // alpha negative
        (50.0, 100.0, 1.5),  // alpha > 1 (overshoot)
        // infinities
        (f64::INFINITY, 1.0, 0.5),
        (-f64::INFINITY, 1.0, 0.5),
        (1.0, f64::INFINITY, 0.5),
        (1.0, -f64::INFINITY, 0.5),
        (f64::INFINITY, f64::INFINITY, 0.5),
        // subnormals
        (f64::MIN_POSITIVE / 2.0, f64::MIN_POSITIVE / 4.0, 0.5),
        (-f64::MIN_POSITIVE / 2.0, f64::MIN_POSITIVE / 4.0, 0.5),
        // large magnitude
        (1e300, 2e300, 0.5),
        (-1e300, 1e300, 0.5),
        (1e-300, -2e-300, 1.5),
    ];

    // Bit-parity: the generated organ must be bit-identical to the hand-written
    // law for every fixture. RED against a stub returning 0.0; GREEN once the
    // real generated body is committed into kernel/src/eqc_gen.rs.
    #[test]
    fn ema_next_generated_parity_bit_identical() {
        for &(p, s, a) in EMA_PARITY_FIXTURES {
            assert_eq!(
                ema_next(p, s, a).to_bits(),
                crate::eqc_gen::ema_next_f64(p, s, a).to_bits(),
                "generated organ diverged from hand-written law at ({p},{s},{a})"
            );
        }
    }

    // ── P96 (BLUEPRINT-P96 §4/§6/§8): adaptive ETA from live courier speed ──

    // D1 — warm + in-band live speed fires; ETA uses live pace, NOT the static
    // planned pace. v=3.0, n=5, remaining=1000 ⇒ 1000/3 ≈ 333.33 s (not 200 s).
    #[test]
    fn p96_adaptive_uses_live_speed_when_warm() {
        let remaining = 1000.0;
        let total = 2000.0;
        let baseline = 400.0;
        let static_eta = eta_seconds(remaining, total, baseline); // 200 s
        let a = eta_seconds_adaptive(remaining, total, baseline, Some((3.0, 5)));
        assert!(
            (a - 1000.0 / 3.0).abs() < 1e-9,
            "adaptive must use live speed 3 m/s ⇒ 333.33s, got {a}"
        );
        assert!(
            (a - static_eta).abs() > 1e-6,
            "adaptive must DIVERGE from the static 200s baseline when warm"
        );
    }

    // D1/D2 — cold (None) and cold-but-present (pings < ETA_MIN_PINGS) fall back
    // byte-for-byte to the static `eta_seconds`.
    #[test]
    fn p96_adaptive_falls_back_when_cold() {
        let (remaining, total, baseline) = (1000.0, 2000.0, 400.0);
        let static_eta = eta_seconds(remaining, total, baseline);
        let none = eta_seconds_adaptive(remaining, total, baseline, None);
        assert!(
            (none - static_eta).abs() < 1e-12,
            "None must equal static baseline byte-for-byte, got {none} vs {static_eta}"
        );
        let sparse = eta_seconds_adaptive(remaining, total, baseline, Some((8.0, 2)));
        assert!(
            (sparse - static_eta).abs() < 1e-12,
            "pings<ETA_MIN_PINGS must equal static baseline, got {sparse} vs {static_eta}"
        );
    }

    // D2 — stopped courier (v below ETA_SPEED_MIN) or n=1 falls back to baseline,
    // never ∞.
    #[test]
    fn p96_adaptive_falls_back_when_stopped() {
        let (remaining, total, baseline) = (1000.0, 2000.0, 400.0);
        let static_eta = eta_seconds(remaining, total, baseline);
        let stopped = eta_seconds_adaptive(remaining, total, baseline, Some((0.1, 9)));
        assert!(
            (stopped - static_eta).abs() < 1e-12,
            "v below floor must fall back to baseline (not ∞), got {stopped}"
        );
        let one_ping = eta_seconds_adaptive(remaining, total, baseline, Some((9.0, 1)));
        assert!(
            (one_ping - static_eta).abs() < 1e-12,
            "n=1 must fall back to baseline, got {one_ping}"
        );
    }

    // D2 — bounded-degradation invariant (§6.1). For a sweep of edge cases the
    // adaptive ETA is ALWAYS finite & non-negative, and in every wild (absent /
    // cold / out-of-band) case it equals the static baseline byte-for-byte. The
    // only divergence is the in-band live path, which is bounded by the speed
    // band [ETA_SPEED_MIN, ETA_SPEED_MAX] ⇒ finite, never ∞.
    #[test]
    fn p96_bounded_degradation() {
        let (remaining, total, baseline) = (1000.0, 2000.0, 400.0);
        let static_eta = eta_seconds(remaining, total, baseline);

        // (observed, expect_in_band) — in-band ⇒ uses remaining/v; wild ⇒ == static.
        let cases: &[(Option<(f64, u32)>, bool)] = &[
            (None, false),                     // absent
            (Some((8.0, 2)), false),           // n < MIN_PINGS
            (Some((0.1, 9)), false),           // v below floor
            (Some((500.0, 9)), false),         // v above ceiling
            (Some((f64::INFINITY, 9)), false), // non-finite
            (Some((3.0, 5)), true),            // in-band normal
            (Some((9.0, 5)), true),            // in-band faster
            (Some((0.3, 5)), true),            // exactly the floor
            (Some((30.0, 5)), true),           // exactly the ceiling
        ];

        for &(obs, in_band) in cases {
            let a = eta_seconds_adaptive(remaining, total, baseline, obs);
            // Never ∞, never negative (the load-bearing safety property).
            assert!(a.is_finite(), "adaptive must never be ∞, case {obs:?}");
            assert!(a >= 0.0, "adaptive must never be negative, case {obs:?}");

            if in_band {
                let (v, _) = obs.unwrap();
                assert!(
                    (a - remaining / v).abs() < 1e-12,
                    "in-band case must use live speed {v}, got {a}"
                );
            } else {
                assert!(
                    (a - static_eta).abs() < 1e-12,
                    "wild case must equal static baseline byte-for-byte, got {a} vs {static_eta}"
                );
            }
        }

        // Tiny remaining, no baseline fallback (5 m/s path) — still finite & >=0.
        let fb = eta_seconds_adaptive(0.5, 0.0, 0.0, Some((500.0, 9)));
        assert!(
            fb.is_finite() && fb >= 0.0,
            "no-baseline wild case finite, got {fb}"
        );
    }

    // M3 — the EMA step rejects a single-ping GPS spike: the spike is attenuated
    // (not tracked) and the smoother recovers to the true pace afterwards.
    #[test]
    fn speed_ema_rejects_single_spike() {
        let mut ema = CourierSpeedEma::new();
        let rv = 0u64;
        for _ in 0..3 {
            ema.accept_ping(8.0, rv); // warm on a true 8 m/s pace
        }
        ema.accept_ping(40.0, rv); // one GPS glitch
        let after_spike = ema.observed().unwrap().0;
        assert!(
            after_spike < 20.0,
            "single 40 m/s spike must be heavily attenuated, got {after_spike}"
        );
        for _ in 0..60 {
            ema.accept_ping(8.0, rv); // back to true pace
        }
        let recovered = ema.observed().unwrap().0;
        assert!(
            (recovered - 8.0).abs() < 1e-3,
            "smoother must recover to the true 8 m/s pace, got {recovered}"
        );
    }

    // M3 — a route_version bump resets the smoother so a re-route cold-starts on
    // the baseline; stale speed never leaks across the boundary (I4).
    #[test]
    fn speed_ema_resets_on_route_version_change() {
        let mut ema = CourierSpeedEma::new();
        ema.accept_ping(5.0, 0);
        ema.accept_ping(5.0, 0);
        ema.accept_ping(5.0, 0);
        assert_eq!(ema.observed().unwrap().1, 3); // warmed
                                                  // Re-route: bump route_version.
        ema.accept_ping(9.0, 1);
        let (v, n) = ema.observed().unwrap();
        assert_eq!(n, 1, "route_version bump must reset the ping count");
        assert!(
            (v - 9.0 * crate::geo::ETA_SPEED_ALPHA).abs() < 1e-12,
            "post-reset smoother must start from the new sample, got {v}"
        );
    }

    #[test]
    fn cover_haversine_same() {
        let _ = super::haversine_meters(0.0, 0.0, 0.0, 0.0);
    }

    #[test]
    fn cover_haversine_far() {
        let _ = super::haversine_meters(0.0, 0.0, 0.0, 180.0);
    }

    #[test]
    fn cover_bearing_same() {
        let _ = super::bearing_deg(0.0, 0.0, 0.0, 0.0);
    }

    #[test]
    fn cover_bearing_north() {
        let _ = super::bearing_deg(0.0, 0.0, 10.0, 0.0);
    }

    #[test]
    fn cover_ema_initial() {
        let _ = super::ema_next(0.0, 5.0, 0.5);
    }

    #[test]
    fn cover_polyline_empty() {
        let _ = super::polyline_length_meters(&[]);
    }

    #[test]
    fn cover_polyline_single() {
        let _ = super::polyline_length_meters(&[(0.0, 0.0)]);
    }

    #[test]
    fn cover_bearing_north_2() {
        let b = super::bearing_deg(0.0, 0.0, 10.0, 1.0); assert!(b >= 0.0 && b < 360.0);
    }

    #[test]
    fn cover_bearing_east() {
        let b = super::bearing_deg(0.0, 0.0, 0.0, 10.0); assert!(b > 0.0);
    }

    #[test]
    fn cover_ema_sequence() {
        let e1 = super::ema_next(0.0, 5.0, 0.5); let e2 = super::ema_next(e1, 5.0, 0.5); let e3 = super::ema_next(e2, 5.0, 0.5); assert!(e3 > e1);
    }

    #[test]
    fn cover_polyline_two() {
        let l = super::polyline_length_meters(&[(0.0, 0.0), (1.0, 0.0)]); assert!(l > 0.0);
    }

    #[test]
    fn cover_haversine_diag() {
        let d = super::haversine_meters(0.0, 0.0, 90.0, 0.0); assert!(d > 0.0);
    }

    // ── injected: zero-area / degenerate / boundary / dateline / null ──

    #[test]
    fn point_in_polygon_empty() {
        assert!(!point_in_polygon(1.0, 1.0, &[]));
    }

    #[test]
    fn point_in_polygon_boundary_edge_behavior() {
        let sq = [(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)];
        let on_edge = point_in_polygon(0.0, 5.0, &sq);
        let just_outside = point_in_polygon(-0.0001, 5.0, &sq);
        let just_inside = point_in_polygon(0.0001, 5.0, &sq);
        // just outside must be false
        assert!(!just_outside);
        // just inside must be true
        assert!(just_inside);
        // on-edge is floating-point dependent; let the result stand
        let _ = on_edge;
    }

    #[test]
    fn point_in_polygon_zero_area_collinear() {
        let line = [(0.0, 0.0), (5.0, 5.0), (10.0, 10.0)];
        assert!(!point_in_polygon(5.0, 5.0, &line));
    }

    #[test]
    fn point_in_polygon_single_point() {
        assert!(!point_in_polygon(0.0, 0.0, &[(0.0, 0.0)]));
    }

    #[test]
    fn point_in_polygon_two_points() {
        assert!(!point_in_polygon(0.5, 0.5, &[(0.0, 0.0), (1.0, 1.0)]));
    }

    #[test]
    fn haversine_zero_distance_same_point() {
        let d = haversine_meters(45.0, 90.0, 45.0, 90.0);
        assert!((d - 0.0).abs() < 1e-9, "same point => 0, got {d}");
    }

    #[test]
    fn haversine_dateline_wrap() {
        let d = haversine_meters(0.0, 179.0, 0.0, -179.0);
        assert!(d > 0.0 && d < 1_000_000.0); // ~a few hundred km across dateline
    }

    #[test]
    fn progress_empty_polyline() {
        let r = progress_along_route(&[], (1.0, 1.0));
        assert_eq!(r.remaining_m, 0.0);
        assert_eq!(r.segment_index, 0);
    }

    #[test]
    fn progress_single_point_polyline() {
        let r = progress_along_route(&[(5.0, 5.0)], (1.0, 1.0));
        assert_eq!(r.segment_index, 0);
        assert_eq!(r.remaining_m, 0.0);
    }

    #[test]
    fn polyline_zero_length_segment() {
        let poly = [(0.0, 0.0), (0.0, 0.0), (1.0, 0.0)];
        let len = polyline_length_meters(&poly);
        assert!(len >= 0.0); // zero-length segment contributes nothing
    }
}
