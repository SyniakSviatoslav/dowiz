//! landmark.rs — geometric keypoint/landmark primitives (the substrate of
//! face analysis, item #10), zero-dep.
//!
//! Face detection/recognition itself needs trained neural networks and image
//! I/O (external deps, out of scope). But the *geometry* underneath — the
//! 68-point topology, interocular normalization, Procrustes alignment, and
//! landmark distance metrics — is pure vector math, exactly the kernel's
//! "geometry over algebra" law. This module ships that geometry, not a model.

/// A 2D landmark point.
pub type Point = (f64, f64);

/// Canonical 68-point face landmark topology (iBUG-68) — the indices of the
/// key regions. Stored as a const LUT so any consumer reads region bounds in
/// O(1) without scanning.
///
/// Region index ranges are inclusive.
#[derive(Debug, Clone, Copy)]
pub struct FaceTopology {
    /// Jaw outline [0, 16].
    pub jaw: (usize, usize),
    /// Right eyebrow [17, 21].
    pub right_eyebrow: (usize, usize),
    /// Left eyebrow [22, 26].
    pub left_eyebrow: (usize, usize),
    /// Nose bridge [27, 30].
    pub nose_bridge: (usize, usize),
    /// Nose [31, 35].
    pub nose: (usize, usize),
    /// Right eye [36, 41].
    pub right_eye: (usize, usize),
    /// Left eye [42, 47].
    pub left_eye: (usize, usize),
    /// Mouth [48, 67].
    pub mouth: (usize, usize),
}

/// The iBUG-68 topology, as a const.
pub const FACE_68: FaceTopology = FaceTopology {
    jaw: (0, 16),
    right_eyebrow: (17, 21),
    left_eyebrow: (22, 26),
    nose_bridge: (27, 30),
    nose: (31, 35),
    right_eye: (36, 41),
    left_eye: (42, 47),
    mouth: (48, 67),
};

/// Euclidean distance between two points.
pub fn dist(a: Point, b: Point) -> f64 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

/// Centroid of a set of points.
pub fn centroid(pts: &[Point]) -> Point {
    let n = pts.len().max(1) as f64;
    let sx: f64 = pts.iter().map(|p| p.0).sum();
    let sy: f64 = pts.iter().map(|p| p.1).sum();
    (sx / n, sy / n)
}

/// Interocular (inter-pupillary) distance — the standard normalization
/// reference. For the 68-point set, the pupil is the eye centroid.
pub fn interocular_distance(pts: &[Point], topology: &FaceTopology) -> f64 {
    let re = centroid(&pts[topology.right_eye.0..=topology.right_eye.1]);
    let le = centroid(&pts[topology.left_eye.0..=topology.left_eye.1]);
    dist(re, le)
}

/// Root-mean-square deviation between two equally-sized point sets.
/// `None` on length mismatch (fail-closed).
pub fn rmsd(a: &[Point], b: &[Point]) -> Option<f64> {
    if a.len() != b.len() {
        return None;
    }
    let n = a.len().max(1) as f64;
    let ssq: f64 = a.iter().zip(b).map(|(x, y)| {
        let dx = x.0 - y.0;
        let dy = x.1 - y.1;
        dx * dx + dy * dy
    }).sum();
    Some((ssq / n).sqrt())
}

/// 2D similarity transform: rotation θ, scale s, translation (tx, ty).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Similarity {
    pub scale: f64,
    pub theta: f64,
    pub tx: f64,
    pub ty: f64,
}

impl Similarity {
    pub fn identity() -> Self {
        Self { scale: 1.0, theta: 0.0, tx: 0.0, ty: 0.0 }
    }

    /// Apply the transform to a point.
    pub fn apply(&self, p: Point) -> Point {
        let (x, y) = p;
        let (c, s) = (self.theta.cos(), self.theta.sin());
        let rx = c * x - s * y;
        let ry = s * x + c * y;
        (self.scale * rx + self.tx, self.scale * ry + self.ty)
    }
}

/// Procrustes alignment: find the similarity transform mapping `src` onto
/// `dst` (least-squares, no reflection). Uses the classic Umeyama/Schönemann
/// closed form on centered coordinates. `None` on length mismatch.
pub fn procrustes(src: &[Point], dst: &[Point]) -> Option<Similarity> {
    if src.len() != dst.len() || src.is_empty() {
        return None;
    }
    let n = src.len() as f64;
    let cs = centroid(src);
    let cd = centroid(dst);
    let sx: Vec<Point> = src.iter().map(|p| (p.0 - cs.0, p.1 - cs.1)).collect();
    let dx: Vec<Point> = dst.iter().map(|p| (p.0 - cd.0, p.1 - cd.1)).collect();

    // Covariance sums.
    let mut a = 0.0f64; // Σ x_s x_d + y_s y_d
    let mut b = 0.0f64; // Σ x_s y_d − y_s x_d
    let mut norm = 0.0f64; // Σ x_s² + y_s²
    for i in 0..src.len() {
        let (xs, ys) = sx[i];
        let (xd, yd) = dx[i];
        a += xs * xd + ys * yd;
        b += xs * yd - ys * xd;
        norm += xs * xs + ys * ys;
    }
    if norm < 1e-12 {
        return Some(Similarity::identity());
    }
    let scale = (a * a + b * b).sqrt() / norm;
    let theta = b.atan2(a);
    // Translation maps the source centroid (scaled+rotated) onto dst centroid.
    let (c, s) = (theta.cos(), theta.sin());
    let (rx, ry) = (c * cs.0 - s * cs.1, s * cs.0 + c * cs.1);
    let tx = cd.0 - scale * rx;
    let ty = cd.1 - scale * ry;
    Some(Similarity { scale, theta, tx, ty })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_and_centroid() {
        assert!((dist((0.0, 0.0), (3.0, 4.0)) - 5.0).abs() < 1e-12);
        let c = centroid(&[(0.0, 0.0), (2.0, 2.0)]);
        assert_eq!(c, (1.0, 1.0));
    }

    #[test]
    fn rmsd_identical_is_zero() {
        let a = [(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)];
        assert_eq!(rmsd(&a, &a), Some(0.0));
    }

    #[test]
    fn rmsd_rejects_length_mismatch() {
        assert_eq!(rmsd(&[(0.0, 0.0)], &[(0.0, 0.0), (1.0, 1.0)]), None);
    }

    #[test]
    fn procrustes_recovers_known_transform() {
        // Build dst by rotating src 90° and scaling 2×.
        let t = Similarity { scale: 2.0, theta: core::f64::consts::FRAC_PI_2, tx: 5.0, ty: -3.0 };
        let src = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        let dst: Vec<Point> = src.iter().map(|&p| t.apply(p)).collect();
        let rec = procrustes(&src, &dst).unwrap();
        assert!((rec.scale - 2.0).abs() < 1e-9, "scale {}", rec.scale);
        assert!((rec.theta - core::f64::consts::FRAC_PI_2).abs() < 1e-9, "theta {}", rec.theta);
        assert!((rec.tx - 5.0).abs() < 1e-9, "tx {}", rec.tx);
        assert!((rec.ty + 3.0).abs() < 1e-9, "ty {}", rec.ty);
    }

    #[test]
    fn procrustes_rejects_empty_or_mismatch() {
        assert_eq!(procrustes(&[], &[]), None);
        assert_eq!(procrustes(&[(0.0, 0.0)], &[]), None);
    }

    #[test]
    fn interocular_uses_eye_regions() {
        // Synthesize a 68-point set with the right-eye region at x=0 and the
        // left-eye region at x=10, so interocular distance = 10.
        let mut pts = vec![(0.0, 0.0); 68];
        for i in 36..=41 {
            pts[i] = (0.0, 0.0);
        }
        for i in 42..=47 {
            pts[i] = (10.0, 0.0);
        }
        let d = interocular_distance(&pts, &FACE_68);
        assert!((d - 10.0).abs() < 1e-9, "interocular {d}");
    }
}
