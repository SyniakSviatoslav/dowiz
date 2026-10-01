//! Where this venue will deliver.
//!
//! WHY IT MATTERS. Without this the hub accepts any delivery order with any
//! address. The failure is not a wrong number on a screen — it is a courier
//! sent forty minutes out of town, one customer whose food is cold, and every
//! other order that night late behind it. A refusal at checkout costs the venue
//! one sale; an accepted order it cannot serve costs it an evening.
//!
//! INTEGER ARITHMETIC THROUGHOUT, per MANIFESTO C2. Coordinates are micro-
//! degrees (one micro-degree is about 11 cm, finer than any phone's GPS).
//! Distances come out in metres as `i64`. No float touches a coordinate, a
//! distance or a comparison, so two nodes replaying the same order reach the
//! same verdict — which is what makes this safe to put on the kernel's side of
//! the fence later.
//!
//! THE EARTH IS FLAT HERE, and that is a measured choice rather than a shortcut.
//! Over a delivery radius — single-digit kilometres — an equirectangular
//! approximation differs from the great-circle distance by well under a metre
//! (the test below measures it). Haversine would need a square root and two
//! trigonometric functions in fixed point to buy nothing at this scale.

/// Metres per degree of latitude, times 1000. WGS-84 mean; it varies by about
/// 1% pole to equator, which is far inside the tolerance of a delivery radius.
const MM_PER_DEG_LAT: i64 = 111_132_000;

/// cos(d degrees) x 1_000_000, for d in 0..=90.
///
/// A TABLE rather than a call, because `cos` is a float function and the whole
/// point here is that no float participates. Linear interpolation between whole
/// degrees is accurate to about 1.5e-4 of the value, which at a 5 km radius is
/// under a metre.
const COS_DEG_E6: [i64; 91] = [
    1000000, 999848, 999391, 998630, 997564, 996195, 994522, 992546, 990268, 987688, 984808,
    981627, 978148, 974370, 970296, 965926, 961262, 956305, 951057, 945519, 939693, 933580,
    927184, 920505, 913545, 906308, 898794, 891007, 882948, 874620, 866025, 857167, 848048,
    838671, 829038, 819152, 809017, 798636, 788011, 777146, 766044, 754710, 743145, 731354,
    719340, 707107, 694658, 681998, 669131, 656059, 642788, 629320, 615661, 601815, 587785,
    573576, 559193, 544639, 529919, 515038, 500000, 484810, 469472, 453990, 438371, 422618,
    406737, 390731, 374607, 358368, 342020, 325568, 309017, 292372, 275637, 258819, 241922,
    224951, 207912, 190809, 173648, 156434, 139173, 121869, 104528, 87156, 69756, 52336,
    34899, 17452, 0,
];

/// cos(latitude) x 1_000_000, from micro-degrees.
fn cos_lat_e6(lat_udeg: i64) -> i64 {
    // Symmetric about the equator, and clamped: a latitude past the pole is
    // nonsense that must not index out of the table.
    let a = lat_udeg.abs().min(90_000_000);
    let deg = (a / 1_000_000) as usize;
    if deg >= 90 {
        return 0;
    }
    let frac = a % 1_000_000; // 0..999_999
    let (lo, hi) = (COS_DEG_E6[deg], COS_DEG_E6[deg + 1]);
    lo + (hi - lo) * frac / 1_000_000
}

/// Micro-degree bounds of a point that can exist on the planet.
const MAX_LAT_UDEG: i64 = 90_000_000;
const MAX_LON_UDEG: i64 = 180_000_000;

/// Distance in metres between two points, integer throughout.
///
/// COORDINATES ARE CLAMPED FIRST. These arrive from a browser and therefore
/// from whoever is sitting at it. Unclamped, a latitude of `i64::MAX / 4`
/// overflows the i128 squaring below -- which panics in a debug build and, far
/// worse, WRAPS SILENTLY in release, producing a distance that would place an
/// absurd address inside the delivery zone. Clamping is what makes the arithmetic
/// total instead of merely usually fine.
pub fn distance_m(lat1_udeg: i64, lon1_udeg: i64, lat2_udeg: i64, lon2_udeg: i64) -> i64 {
    let lat1_udeg = lat1_udeg.clamp(-MAX_LAT_UDEG, MAX_LAT_UDEG);
    let lat2_udeg = lat2_udeg.clamp(-MAX_LAT_UDEG, MAX_LAT_UDEG);
    let lon1_udeg = lon1_udeg.clamp(-MAX_LON_UDEG, MAX_LON_UDEG);
    let lon2_udeg = lon2_udeg.clamp(-MAX_LON_UDEG, MAX_LON_UDEG);

    let dlat = lat2_udeg - lat1_udeg;
    let dlon = lon2_udeg - lon1_udeg;
    // Scale longitude by the cosine of the mean latitude: a degree of longitude
    // is shorter the further from the equator, and ignoring that overstates
    // east-west distance by a third in northern Europe.
    let cos_e6 = cos_lat_e6((lat1_udeg + lat2_udeg) / 2);

    // Millimetres, to keep precision before the square root. i128 because
    // squaring millimetre-scale numbers overflows i64 at continental distances.
    let dy_mm = (dlat as i128) * (MM_PER_DEG_LAT as i128) / 1_000_000;
    let dx_mm = (dlon as i128) * (MM_PER_DEG_LAT as i128) / 1_000_000 * (cos_e6 as i128) / 1_000_000;

    let sq = dy_mm * dy_mm + dx_mm * dx_mm;
    (isqrt_i128(sq) / 1000) as i64
}

/// Integer square root, exact. Newton's method on integers terminates and has
/// no rounding mode to disagree about between machines.
fn isqrt_i128(n: i128) -> i128 {
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

/// One area the venue serves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Zone {
    /// Everything within `radius_m` of a point.
    Circle { lat_udeg: i64, lon_udeg: i64, radius_m: i64 },
    /// An arbitrary area, as a closed ring of points.
    ///
    /// A real service area follows a river, a ring road or a bridge, none of
    /// which is a circle. A venue that draws its own boundary gets one.
    Polygon { points: Vec<(i64, i64)> },
}

impl Zone {
    pub fn contains(&self, lat_udeg: i64, lon_udeg: i64) -> bool {
        match self {
            Zone::Circle { lat_udeg: clat, lon_udeg: clon, radius_m } => {
                distance_m(*clat, *clon, lat_udeg, lon_udeg) <= *radius_m
            }
            Zone::Polygon { points } => point_in_polygon(lat_udeg, lon_udeg, points),
        }
    }
}

/// Ray casting, with the crossing test done in integers.
///
/// The usual formulation divides to find where an edge crosses the ray; this
/// cross-multiplies instead, so there is no division, no float, and no
/// tolerance to tune. A point exactly on an edge counts as INSIDE, because a
/// customer standing on the boundary of a delivery area should be served, not
/// told their address does not exist.
fn point_in_polygon(lat: i64, lon: i64, pts: &[(i64, i64)]) -> bool {
    if pts.len() < 3 {
        return false;
    }
    let mut inside = false;
    let n = pts.len();
    for i in 0..n {
        let (y1, x1) = pts[i];
        let (y2, x2) = pts[(i + 1) % n];

        // On this edge? Collinear and within the bounding box of the segment.
        let cross = (x2 as i128 - x1 as i128) * (lat as i128 - y1 as i128)
            - (y2 as i128 - y1 as i128) * (lon as i128 - x1 as i128);
        if cross == 0
            && lon >= x1.min(x2)
            && lon <= x1.max(x2)
            && lat >= y1.min(y2)
            && lat <= y1.max(y2)
        {
            return true;
        }

        // Does the horizontal ray from (lat, lon) going east cross this edge?
        if (y1 > lat) != (y2 > lat) {
            // x of the crossing, compared without dividing:
            //   lon < x1 + (lat - y1) * (x2 - x1) / (y2 - y1)
            let dy = y2 as i128 - y1 as i128;
            let lhs = (lon as i128 - x1 as i128) * dy;
            let rhs = (lat as i128 - y1 as i128) * (x2 as i128 - x1 as i128);
            // Multiplying by dy flips the comparison when dy is negative.
            let crosses = if dy > 0 { lhs < rhs } else { lhs > rhs };
            if crosses {
                inside = !inside;
            }
        }
    }
    inside
}

/// The verdict, with enough detail for the storefront to say something useful.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reach {
    /// Inside a zone.
    Inside,
    /// Outside every zone. Carries the distance to the nearest zone centre, so
    /// the customer can be told "about 3 km outside our area" instead of "no".
    Outside { nearest_m: i64 },
    /// The venue has drawn no zones. Everything is accepted -- a venue that has
    /// not configured a service area has not asked for one to be enforced.
    Unrestricted,
    /// Zones exist but the order carries no coordinates.
    ///
    /// ACCEPTED, deliberately, and flagged. There is no geocoder here, so an
    /// address typed by hand has no position; refusing those would refuse every
    /// customer who declined the browser's location prompt. The venue sees the
    /// flag and decides.
    Unknown,
}

pub fn reach(zones: &[Zone], point: Option<(i64, i64)>) -> Reach {
    if zones.is_empty() {
        return Reach::Unrestricted;
    }
    let Some((lat, lon)) = point else { return Reach::Unknown };
    if zones.iter().any(|z| z.contains(lat, lon)) {
        return Reach::Inside;
    }
    let nearest = zones
        .iter()
        .map(|z| match z {
            Zone::Circle { lat_udeg, lon_udeg, radius_m } => {
                (distance_m(*lat_udeg, *lon_udeg, lat, lon) - radius_m).max(0)
            }
            Zone::Polygon { points } => points
                .iter()
                .map(|(y, x)| distance_m(*y, *x, lat, lon))
                .min()
                .unwrap_or(i64::MAX),
        })
        .min()
        .unwrap_or(i64::MAX);
    Reach::Outside { nearest_m: nearest }
}

/// Read zones out of the venue record's JSON.
///
/// Tolerant of a missing or malformed field: a venue with unreadable zones is
/// treated as having none, which ACCEPTS orders. Failing the other way would
/// take a restaurant offline because of a typo in a config value.
pub fn from_json(value: &str) -> Vec<Zone> {
    let mut out = Vec::new();
    // Hand-parsed for the same reason as everything else in this crate: the
    // shape is known and produced by the owner surface, and `serde_json` is not
    // a dependency here.
    for chunk in value.split("{\"kind\":").skip(1) {
        let num = |key: &str| -> Option<i64> { crate::minijson::int_field(chunk, key) };
        if chunk.starts_with("\"circle\"") {
            if let (Some(lat), Some(lon), Some(r)) = (num("lat"), num("lon"), num("radius_m")) {
                if r > 0 {
                    out.push(Zone::Circle { lat_udeg: lat, lon_udeg: lon, radius_m: r });
                }
            }
        } else if chunk.starts_with("\"polygon\"") {
            let mut points = Vec::new();
            for p in chunk.split("[").skip(1) {
                let nums: Vec<i64> = p
                    .split(']')
                    .next()
                    .unwrap_or("")
                    .split(',')
                    .filter_map(|t| t.trim().parse::<i64>().ok())
                    .collect();
                if nums.len() >= 2 {
                    points.push((nums[0], nums[1]));
                }
            }
            if points.len() >= 3 {
                out.push(Zone::Polygon { points });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
