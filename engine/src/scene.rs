//! W4-2 — Field-UI Phase 2: SDF Scene → field-buffer render pipeline.
//!
//! A [`Scene`] is a list of SDF primitives ([`SdfShape`]) composed via the
//! combinators in `sdf.rs` (default composition = boolean UNION). Rasterizing a
//! scene produces a flat, row-major `Vec<f32>` **field buffer** — the exact
//! zero-copy shape the rest of the engine consumes (see `zerocopy.rs` /
//! [`crate::VertexBridge`]): one signed-distance sample per pixel, ready to be
//! handed to the GPU upload path as a `&[f32]` with no copy and no JSON.
//!
//! CPU-side / offline: no `wgpu`, no new deps. The field buffer can be pushed
//! straight into the existing [`crate::VertexBridge`] (each cell becomes a
//! vertex at its grid coordinate, carrying its signed distance in the first
//! per-vertex channel), which is what `queue.writeBuffer(view)` would upload.
//!
//! Every gate in this module is a falsifiable RED→GREEN `#[test]`.

use crate::bridge::VertexBridge;
use crate::sdf::{op_union, sdf_box, sdf_circle, sdf_line_segment, sdf_rounded_box};

/// Signed distance from an empty scene: a uniform "background" field. Kept as a
/// single known constant so an empty render is bit-deterministic and trivially
/// distinguishable from any shape interior. Positive ⇒ outside (background).
pub const BACKGROUND: f32 = f32::INFINITY;

/// An SDF primitive placed in scene/world space. Mirrors the free functions in
/// `sdf.rs`; [`SdfShape::eval`] dispatches to the right primitive so a `Scene`
/// can hold a heterogeneous, composable list of shapes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SdfShape {
    /// Filled circle (negative inside).
    Circle { cx: f64, cy: f64, r: f64 },
    /// Axis-aligned box, centred at `(bx,by)` with half-extents `(hx,hy)`.
    Box { bx: f64, by: f64, hx: f64, hy: f64 },
    /// Box with rounded corners (corner radius `r`).
    RoundedBox {
        bx: f64,
        by: f64,
        hx: f64,
        hy: f64,
        r: f64,
    },
    /// Thin line primitive (distance to the nearest point on the segment).
    LineSegment { ax: f64, ay: f64, bx: f64, by: f64 },
}

impl SdfShape {
    /// Signed distance from world point `(wx,wy)` to this shape. Negative =
    /// inside, 0 = boundary, positive = outside (consistent with `sdf.rs`).
    #[inline]
    pub fn eval(&self, wx: f64, wy: f64) -> f64 {
        match *self {
            SdfShape::Circle { cx, cy, r } => sdf_circle(wx, wy, cx, cy, r),
            SdfShape::Box { bx, by, hx, hy } => sdf_box(wx, wy, bx, by, hx, hy),
            SdfShape::RoundedBox { bx, by, hx, hy, r } => {
                sdf_rounded_box(wx, wy, bx, by, hx, hy, r)
            }
            SdfShape::LineSegment { ax, ay, bx, by } => sdf_line_segment(wx, wy, ax, ay, bx, by),
        }
    }
}

/// A composable SDF scene: a list of [`SdfShape`]s rendered to a flat field
/// buffer. Shapes are combined with boolean UNION (`op_union`), the simplest
/// and most common composition; substitution/intersection combinators in
/// `sdf.rs` remain available for callers that build a custom closure.
///
/// The last-rasterized buffer is cached so it can be exposed as a **zero-copy**
/// `&[f32]` view (mirroring [`crate::ParticleBuffer::as_f32`]) — the slice the
/// GPU upload path consumes.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    shapes: Vec<SdfShape>,
    /// World-units per pixel (uniform square pixels). Default 1.0.
    scale: f64,
    /// Cached last field buffer (row-major, length = width*height).
    field: Vec<f32>,
    cached_w: usize,
    cached_h: usize,
}

impl Scene {
    /// An empty scene (renders to the uniform [`BACKGROUND`] field).
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a shape; returns `&mut Self` for builder-style chaining.
    pub fn add(&mut self, shape: SdfShape) -> &mut Self {
        self.shapes.push(shape);
        self
    }

    /// Set the world-units-per-pixel scale (builder-style). Default 1.0.
    pub fn with_scale(mut self, scale: f64) -> Self {
        self.scale = if scale <= 0.0 { 1.0 } else { scale };
        self
    }

    /// Signed distance of the composed scene at world point `(wx,wy)`. An empty
    /// scene yields [`BACKGROUND`] (uniformly outside). Otherwise the shapes are
    /// folded with boolean UNION (`op_union`) — negative inside the union,
    /// positive outside.
    pub fn sample(&self, wx: f64, wy: f64) -> f64 {
        if self.shapes.is_empty() {
            return f64::INFINITY;
        }
        let mut d = self.shapes[0].eval(wx, wy);
        for s in &self.shapes[1..] {
            d = op_union(d, s.eval(wx, wy));
        }
        d
    }

    /// Rasterize the composed SDF scene into a flat, row-major `Vec<f32>` field
    /// buffer of size `width × height`. Pixel `(col,row)` samples world
    /// coordinate `(x0 + col*scale, y0 + row*scale)` with a **centered** origin
    /// (`x0 = -width/2*scale`, `y0 = -height/2*scale`), so the world origin sits
    /// at the image centre. This is the buffer the GPU upload path consumes.
    ///
    /// Bit-deterministic: two calls with identical inputs yield identical
    /// `Vec<f32>` bytes (pure `f64 → f32` downcast, no allocation mid-loop).
    pub fn render_frame(&self, width: usize, height: usize) -> Vec<f32> {
        let scale = self.scale;
        let x0 = -(width as f64) * 0.5 * scale;
        let y0 = -(height as f64) * 0.5 * scale;
        if self.shapes.is_empty() {
            return vec![BACKGROUND; width * height];
        }
        let mut data = vec![0.0f32; width * height];
        for row in 0..height {
            let wy = y0 + (row as f64) * scale;
            for col in 0..width {
                let wx = x0 + (col as f64) * scale;
                data[row * width + col] = self.sample(wx, wy) as f32;
            }
        }
        data
    }

    /// Rasterize into the cached internal buffer and return a **zero-copy**
    /// `&[f32]` view over it (mirrors [`crate::ParticleBuffer::as_f32`]). The
    /// view borrows the scene's owned memory — no allocation, no copy.
    pub fn render_into(&mut self, width: usize, height: usize) -> &[f32] {
        self.field = self.render_frame(width, height);
        self.cached_w = width;
        self.cached_h = height;
        &self.field
    }

    /// Zero-copy `&[f32]` view over the last rasterized field buffer. Panics if
    /// no render has occurred yet; call [`Scene::render_into`] first.
    #[inline]
    pub fn field_view(&self) -> &[f32] {
        &self.field
    }

    /// Dimensions of the cached field buffer (0,0 before any render).
    #[inline]
    pub fn cached_dims(&self) -> (usize, usize) {
        (self.cached_w, self.cached_h)
    }

    /// Feed the rendered field through the existing [`crate::VertexBridge`]:
    /// each pixel becomes one vertex at its grid coordinate `(col,row)`, with
    /// its signed distance packed into the first per-vertex channel (`vx`). The
    /// bridge's `vertex_view()` is exactly the linear `&[f32]` buffer the GPU
    /// upload (`queue.writeBuffer`) would consume — one upload, zero JSON.
    pub fn render_to_bridge(&self, width: usize, height: usize) -> VertexBridge {
        let field = self.render_frame(width, height);
        let mut bridge = VertexBridge::new(width * height, 4);
        for row in 0..height {
            for col in 0..width {
                let i = row * width + col;
                let d = field[i];
                bridge.write_particle(i, col as f32, row as f32, d, 0.0);
            }
        }
        bridge
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // (1) scene_render_deterministic — same scene rasterized twice ⇒ bit-identical
    //     buffer (and the cached zero-copy view matches, proving no drift).
    #[test]
    fn scene_render_deterministic() {
        let mut scene = Scene::new().with_scale(0.5);
        scene
            .add(SdfShape::Circle {
                cx: 0.0,
                cy: 0.0,
                r: 2.0,
            })
            .add(SdfShape::Box {
                bx: 3.0,
                by: 0.0,
                hx: 1.0,
                hy: 1.0,
            });

        let g1 = scene.render_frame(24, 18);
        let g2 = scene.render_frame(24, 18);

        assert_eq!(g1.len(), g2.len());
        assert_eq!(g1, g2, "raster must be value-identical");
        for (x, y) in g1.iter().zip(g2.iter()) {
            assert_eq!(x.to_bits(), y.to_bits(), "raster bytes must match");
        }

        // The cached zero-copy view of a third render is also bit-identical.
        let mut s2 = scene.clone();
        let view = s2.render_into(24, 18);
        assert_eq!(view, &g1[..]);
        assert_eq!(view.to_vec(), g1);
    }

    // (2) scene_empty_is_background — empty scene ⇒ uniform known-constant field.
    #[test]
    fn scene_empty_is_background() {
        let scene = Scene::new();
        let buf = scene.render_frame(10, 7);
        assert_eq!(buf.len(), 10 * 7);
        for &v in &buf {
            assert_eq!(v, BACKGROUND, "empty scene must be uniform background");
            assert!(v.is_sign_positive(), "background is outside (positive)");
        }
        // Cached view exposes the same constant zero-copy.
        let mut scene2 = Scene::new();
        let view = scene2.render_into(10, 7);
        assert_eq!(view, &buf[..]);
    }

    // (3) scene_shape_visible — a circle at the centre ⇒ the centre cell is
    //     inside (negative sign) while a corner cell is outside (positive sign).
    #[test]
    fn scene_shape_visible() {
        let mut scene = Scene::new().with_scale(1.0);
        scene.add(SdfShape::Circle {
            cx: 0.0,
            cy: 0.0,
            r: 3.0,
        });

        let (w, h) = (16usize, 16usize);
        let buf = scene.render_frame(w, h);

        // Centre pixel (w/2, h/2) maps to world origin (0,0) ⇒ inside (negative).
        let cx = w / 2;
        let cy = h / 2;
        let center = buf[cy * w + cx];
        assert!(
            center < 0.0,
            "centre cell must be inside (negative): {center}"
        );

        // Corner pixel (0,0) is far from the circle ⇒ outside (positive).
        let corner = buf[0];
        assert!(
            corner > 0.0,
            "corner cell must be outside (positive): {corner}"
        );
    }

    // (4) scene_dims — the output field buffer length equals width*height.
    #[test]
    fn scene_dims() {
        let mut scene = Scene::new().with_scale(0.25);
        scene
            .add(SdfShape::RoundedBox {
                bx: 0.0,
                by: 0.0,
                hx: 2.0,
                hy: 1.0,
                r: 0.5,
            })
            .add(SdfShape::LineSegment {
                ax: -3.0,
                ay: -3.0,
                bx: 3.0,
                by: 3.0,
            });

        let (w, h) = (31usize, 17usize);
        let buf = scene.render_frame(w, h);
        assert_eq!(buf.len(), w * h, "field buffer length must be w*h");

        // And the bridge it feeds carries the same number of cells (as vertices).
        let bridge = scene.render_to_bridge(w, h);
        assert_eq!(bridge.count(), w * h, "bridge drives one vertex per cell");
        assert_eq!(bridge.vertex_view().len(), w * h * 4);
    }
}
