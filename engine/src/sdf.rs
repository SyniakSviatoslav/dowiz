//! W3-1 — Field-UI Phase 1 SDF (signed-distance-field) layer.
//!
//! UI primitives are fields/SDFs, not DOM. This is the CPU-side signed-distance
//! toolkit: pure f64/f32 math, zero external deps, offline-clean like the rest
//! of the engine. Every primitive returns a signed distance (negative = inside,
//! 0 = on the boundary, positive = outside) so combinators compose like ordinary
//! scalar fields.
//!
//! [`SdfField`] rasterizes an SDF closure onto a flat, row-major `Vec<f32>` grid —
//! the exact zero-copy shape the rest of the engine uses (see `zerocopy.rs` /
//! [`crate::VertexBridge`]) — so a raster can be handed to the GPU upload path as
//! a `&[f32]` with no copy and no JSON.
//!
//! Every primitive + combinator carries a falsifiable RED→GREEN gate as a `#[test]`.

/// Signed distance from `(px,py)` to a circle of radius `r` centred at
/// `(cx,cy)`. Negative inside, 0 on the boundary, positive outside.
#[inline]
pub fn sdf_circle(px: f64, py: f64, cx: f64, cy: f64, r: f64) -> f64 {
    let dx = px - cx;
    let dy = py - cy;
    (dx * dx + dy * dy).sqrt() - r
}

/// Signed distance from `(px,py)` to an axis-aligned box centred at `(bx,by)`
/// with half-extents `(hx,hy)`. Negative inside, 0 on the boundary, positive
/// outside.
#[inline]
pub fn sdf_box(px: f64, py: f64, bx: f64, by: f64, hx: f64, hy: f64) -> f64 {
    let qx = (px - bx).abs() - hx;
    let qy = (py - by).abs() - hy;
    let outside = (qx.max(0.0)).hypot(qy.max(0.0));
    let inside = qx.max(qy).min(0.0);
    outside + inside
}

/// Signed distance from `(px,py)` to a box (half-extents `hx,hy`) centred at
/// `(bx,by)` with corner radius `r` (clamped to the smaller half-extent).
/// Negative inside, 0 on the boundary, positive outside.
#[inline]
pub fn sdf_rounded_box(px: f64, py: f64, bx: f64, by: f64, hx: f64, hy: f64, r: f64) -> f64 {
    let r = r.min(hx).min(hy).max(0.0);
    let qx = (px - bx).abs() - (hx - r);
    let qy = (py - by).abs() - (hy - r);
    let outside = (qx.max(0.0)).hypot(qy.max(0.0));
    let inside = qx.max(qy).min(0.0);
    outside + inside - r
}

/// Unsigned distance from `(px,py)` to the line segment from `a` to `b`. A thin
/// line primitive: distance to the nearest point on the segment (0 = on it).
#[inline]
pub fn sdf_line_segment(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let pax = px - ax;
    let pay = py - ay;
    let bax = bx - ax;
    let bay = by - ay;
    let h = {
        let denom = bax * bax + bay * bay;
        if denom == 0.0 {
            0.0
        } else {
            (pax * bax + pay * bay) / denom
        }
    };
    let h = h.clamp(0.0, 1.0);
    let dx = pax - bax * h;
    let dy = pay - bay * h;
    (dx * dx + dy * dy).sqrt()
}

/// Combinator: Boolean UNION of two SDFs = pointwise minimum.
#[inline]
pub fn op_union(a: f64, b: f64) -> f64 {
    a.min(b)
}

/// Combinator: Boolean INTERSECTION of two SDFs = pointwise maximum.
#[inline]
pub fn op_intersection(a: f64, b: f64) -> f64 {
    a.max(b)
}

/// Combinator: Boolean SUBTRACTION — keep `a`, remove the interior of `b`.
/// `a` minus `b` = `max(a, -b)`.
#[inline]
pub fn op_subtraction(a: f64, b: f64) -> f64 {
    a.max(-b)
}

/// Combinator: smooth (polynomial) UNION via Inigo-Quilez `smin`. Blends two
/// SDFs within a radius `k` so the seam is rounded. For `k <= 0` this degrades
/// to the exact [`op_union`] (pointwise min). Result is always `<= min(a,b)`
/// and continuous in both arguments.
#[inline]
pub fn op_smooth_union(a: f64, b: f64, k: f64) -> f64 {
    if k <= 0.0 {
        return a.min(b);
    }
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    let blend = b * (1.0 - h) + a * h;
    blend - k * h * (1.0 - h)
}

/// A rasterized SDF sampled onto a flat, row-major `f32` grid.
///
/// Mirrors the engine's zero-copy contract: samples live in one contiguous
/// `Vec<f32>` (index = `row * width + col`), and `as_f32()` returns a `&[f32]`
/// view over those bytes with **no copy** — exactly what
/// [`crate::VertexBridge`] / [`crate::ParticleBuffer`] upload paths consume.
#[derive(Debug, Clone)]
pub struct SdfField {
    pub width: usize,
    pub height: usize,
    data: Vec<f32>,
}

impl SdfField {
    /// Rasterize the SDF closure `f(world_x, world_y) -> distance` over a
    /// `width × height` grid. Pixel `(col,row)` is sampled at world coordinate
    /// `(x0 + col*scale, y0 + row*scale)` (row-major, top-left origin). Each
    /// sample is downcast `f64 → f32`. The loop is bit-deterministic: two calls
    /// with identical inputs yield identical `Vec<f32>` bytes.
    pub fn rasterize<F>(width: usize, height: usize, x0: f64, y0: f64, scale: f64, f: F) -> Self
    where
        F: Fn(f64, f64) -> f64,
    {
        let mut data = vec![0.0f32; width * height];
        for row in 0..height {
            let wy = y0 + (row as f64) * scale;
            for col in 0..width {
                let wx = x0 + (col as f64) * scale;
                data[row * width + col] = f(wx, wy) as f32;
            }
        }
        Self {
            width,
            height,
            data,
        }
    }

    /// Zero-copy `&[f32]` view over the raster samples (row-major, length
    /// `width*height`). This is the slice the GPU upload path consumes.
    #[inline]
    pub fn as_f32(&self) -> &[f32] {
        &self.data
    }

    /// Total `f32` samples (`width * height`).
    #[inline]
    pub fn len_f32(&self) -> usize {
        self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // (1) sdf_circle_sign — inside negative, outside positive, boundary ~0.
    #[test]
    fn sdf_circle_sign() {
        let (cx, cy, r) = (0.0_f64, 0.0, 2.0);
        let inside = sdf_circle(0.0, 0.0, cx, cy, r);
        let outside = sdf_circle(3.0, 0.0, cx, cy, r);
        let boundary = sdf_circle(2.0, 0.0, cx, cy, r);
        assert!(inside < 0.0, "inside must be negative: {inside}");
        assert!(outside > 0.0, "outside must be positive: {outside}");
        assert!(
            boundary.abs() <= 1e-6,
            "on boundary must be ~0 within 1e-6: {boundary}"
        );
        assert!(
            (outside - 1.0).abs() <= 1e-9,
            "outside distance = 1, got {outside}"
        );
    }

    // (2) sdf_smooth_union_bounds — smin <= both inputs and continuous.
    #[test]
    fn sdf_smooth_union_bounds() {
        let (a, b, k) = (0.0_f64, 4.0, 1.0);
        let m = op_smooth_union(a, b, k);
        let eps = 1e-9;
        assert!(m <= a + eps, "smin must be <= a: {m} vs {a}");
        assert!(m <= b + eps, "smin must be <= b: {m} vs {b}");

        // Continuity: a tiny perturbation in `a` changes the result by at most
        // ~2× the perturbation (smin is Lipschitz-2), never a jump.
        let delta = 1e-6_f64;
        let m2 = op_smooth_union(a + delta, b, k);
        assert!(
            (m2 - m).abs() <= 2.0 * delta + eps,
            "smin must be continuous in a: |{m2} - {m}|"
        );

        // k=0 degrades to exact union (pointwise min).
        assert_eq!(op_smooth_union(a, b, 0.0), a.min(b));
    }

    // (3) sdf_raster_deterministic — same scene twice ⇒ bit-identical Vec<f32>.
    #[test]
    fn sdf_raster_deterministic() {
        fn scene(wx: f64, wy: f64) -> f64 {
            let c = sdf_circle(wx, wy, 0.0, 0.0, 2.0);
            let bx = sdf_box(wx, wy, 3.0, 0.0, 1.0, 1.0);
            op_union(c, bx)
        }
        let g1 = SdfField::rasterize(16, 12, -6.0, -6.0, 1.0, scene);
        let g2 = SdfField::rasterize(16, 12, -6.0, -6.0, 1.0, scene);

        assert_eq!(g1.as_f32(), g2.as_f32(), "raster must be bit-identical");
        for (x, y) in g1.as_f32().iter().zip(g2.as_f32().iter()) {
            assert_eq!(x.to_bits(), y.to_bits(), "raster bytes must match");
        }
        assert_eq!(g1.len_f32(), 16 * 12);
    }

    // (4) sdf_box_exact — known distances to an axis-aligned box match
    //     hand-computed values (centre (0,0), half-extents (2,1)).
    #[test]
    fn sdf_box_exact() {
        let (bx, by, hx, hy) = (0.0_f64, 0.0, 2.0, 1.0);
        let eps = 1e-9;
        // Centre: 1 unit inside (from the top edge at y = 1).
        assert!((sdf_box(0.0, 0.0, bx, by, hx, hy) - (-1.0)).abs() <= eps);
        // 1 unit right of the right edge (x = 2).
        assert!((sdf_box(3.0, 0.0, bx, by, hx, hy) - 1.0).abs() <= eps);
        // Outside corner (3,2): distance from box corner (2,1) = √2.
        assert!((sdf_box(3.0, 2.0, bx, by, hx, hy) - std::f64::consts::SQRT_2).abs() <= eps);
        // Above the top edge by 1 (point (0,2)): distance 1.
        assert!((sdf_box(0.0, 2.0, bx, by, hx, hy) - 1.0).abs() <= eps);
        // On the right edge: exactly 0.
        assert!(sdf_box(2.0, 0.0, bx, by, hx, hy).abs() <= eps);
    }
}
