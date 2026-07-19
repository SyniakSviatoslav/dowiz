//! P81 — FIRST bench harness for the `dowiz-engine` crate.
//!
//! The engine crate had ZERO benchmarks before P81 and runs hot code every
//! frame (field evolution, scene raster, motion springs, the vertex bridge's
//! graph-Laplacian field, and the money-surface guard). This file establishes
//! the crate's native telemetry baseline per AGENTS.md ("Mandatory native
//! telemetry & benchmarks after every change/wave") and the engine row of the
//! performance audit (`SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §3.3-C2).
//!
//! Bench-id convention: `<group>/<n>` (owned by P75; P80/P81/P82 cite it).
//! Each sweep variable is encoded as the `<n>` suffix so `bench_track.py`
//! (the autotrack gate) keys each run into the committed `baseline.json`.
//!
//! IMPORTANT (constraint): this file is BENCHMARKS ONLY. No production logic
//! is changed. Where a bench surfaces a real contract gap (see
//! `vertex_bridge_apply_field` below) the gap is *documented*, not patched —
//! P81 is a coverage/observability blueprint, not a rewrite.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use dowiz_engine::field_frame::{compose, laplacian_into, FieldEquilibrium, FieldFrame};
use dowiz_engine::{Scene, SdfShape, Spring, TweenGuard};
use dowiz_kernel::csr::{Csr, LaplacianKind};

/// Grid sizes shared by the field-frame / laplacian sweeps.
const GRIDS: [(usize, &str); 3] = [(64, "64"), (128, "128"), (256, "256")];

/// Build a zeroed source buffer (SDF of a single centered disk) for `w×h`.
fn disk_source(w: usize, h: usize) -> Vec<f32> {
    let mut scene = Scene::new().with_scale(1.0);
    scene.add(SdfShape::Circle {
        cx: 0.0,
        cy: 0.0,
        r: (w.min(h) as f64) * 0.25,
    });
    scene.render_frame(w, h)
}

// ── field_frame_step: grid-swept integration step ────────────────────────────
// `FieldFrame::step` is THE every-frame hot path. It must stay allocation-free
// (pre-allocated scratch + swap-rotation, see field_frame.rs). The sweep pins
// its linear cost in grid cells so a future regression is caught by bench_track.
fn bench_field_frame_step(c: &mut Criterion) {
    let eq = FieldEquilibrium::default();
    let mut group = c.benchmark_group("field_frame_step");
    for &(n, label) in &GRIDS {
        let source = disk_source(n, n);
        let mut frame = FieldFrame::new(n, n);
        // Warm: advance once so the scratch buffers are really live.
        frame.step(&source, &eq);
        group.bench_with_input(BenchmarkId::from_parameter(label), &n, |b, &_n| {
            b.iter(|| {
                frame.step(black_box(&source), black_box(&eq));
            })
        });
    }
    group.finish();
}

// ── laplacian_into: grid-swept stencil (the inner hot loop of step) ──────────
// `laplacian_into` is the 5-point ∇²U kernel `step` calls every frame. Pinning
// it separately isolates the per-cell stencil cost from the integrator math.
fn bench_laplacian_into(c: &mut Criterion) {
    let mut group = c.benchmark_group("laplacian_into");
    for &(n, label) in &GRIDS {
        let u = vec![0.0f32; n * n];
        let mut out = vec![0.0f32; n * n];
        group.bench_with_input(BenchmarkId::from_parameter(label), &n, |b, &_n| {
            b.iter(|| {
                laplacian_into(black_box(&u), n, n, black_box(&mut out));
            })
        });
    }
    group.finish();
}

/// Shape counts for the scene/compose sweep.
const SHAPES: [(usize, &str); 4] = [(1, "1"), (4, "4"), (16, "16"), (64, "64")];

/// Build a scene with `k` heterogeneous shapes (deterministic, no RNG).
fn shaped_scene(k: usize) -> Scene {
    let mut scene = Scene::new().with_scale(1.0);
    for i in 0..k {
        let f = i as f64;
        match i % 4 {
            0 => scene.add(SdfShape::Circle {
                cx: (f - 16.0).cos() * 8.0,
                cy: (f - 16.0).sin() * 8.0,
                r: 3.0,
            }),
            1 => scene.add(SdfShape::Box {
                bx: (f - 16.0) * 0.5,
                by: 0.0,
                hx: 1.0,
                hy: 1.0,
            }),
            2 => scene.add(SdfShape::RoundedBox {
                bx: 0.0,
                by: (f - 16.0) * 0.5,
                hx: 1.0,
                hy: 1.0,
                r: 0.3,
            }),
            _ => scene.add(SdfShape::LineSegment {
                ax: -8.0,
                ay: f - 16.0,
                bx: 8.0,
                by: f - 16.0,
            }),
        };
    }
    scene
}

// ── scene_render_frame: shape-swept SDF rasterization ────────────────────────
// Fixed grid; sweep the number of composed shapes. `render_frame` allocates one
// `Vec<f32>` per call and walks every pixel through every shape's SDF.
fn bench_scene_render_frame(c: &mut Criterion) {
    let (w, h) = (128usize, 96);
    let mut group = c.benchmark_group("scene_render_frame");
    for &(k, label) in &SHAPES {
        let scene = shaped_scene(k);
        group.bench_with_input(BenchmarkId::from_parameter(label), &k, |b, &_k| {
            b.iter(|| {
                black_box(scene.render_frame(w, h));
            })
        });
    }
    group.finish();
}

// ── frame_rgba: shape-swept field→RGBA8 mapping ──────────────────────────────
// `FieldFrame::frame_rgba` maps the current field U to an RGBA8 display frame
// every render. Fixed grid; sweep shapes because more shapes ⇒ a more "active"
// (less flat) field, exercising the full hue/brightness branch spread.
fn bench_frame_rgba(c: &mut Criterion) {
    let (w, h) = (128usize, 96);
    let eq = FieldEquilibrium::default();
    let steps = 30usize;
    let mut group = c.benchmark_group("frame_rgba");
    for &(k, label) in &SHAPES {
        let scene = shaped_scene(k);
        let source = scene.render_frame(w, h);
        let mut frame = FieldFrame::new(w, h);
        for _ in 0..steps {
            frame.step(&source, &eq);
        }
        group.bench_with_input(BenchmarkId::from_parameter(label), &k, |b, &_k| {
            b.iter(|| {
                black_box(frame.frame_rgba());
            })
        });
    }
    group.finish();
}

// ── compose: shape-swept full render (raster → evolve → frame) ───────────────
// `compose` is the end-to-end single call a future `wgpu` blit would consume:
// rasterize the scene, evolve `steps` field steps, map to RGBA8. Shape-swept.
fn bench_compose(c: &mut Criterion) {
    let (w, h) = (96usize, 72);
    let eq = FieldEquilibrium::default();
    let steps = 20usize;
    let mut group = c.benchmark_group("compose");
    for &(k, label) in &SHAPES {
        let scene = shaped_scene(k);
        group.bench_with_input(BenchmarkId::from_parameter(label), &k, |b, &_k| {
            b.iter(|| {
                black_box(compose(black_box(&scene), black_box(&eq), w, h, steps));
            })
        });
    }
    group.finish();
}

/// Angular-frequency sweep points (ω). Tension k = ω², friction = 2·ζ·ω with
/// ζ = 1 (critically damped), so each `Spring` carries a distinct ω into
/// `step`, which substeps so ω·dt_sub ≤ 0.1 — sweeping ω sweeps the per-call
/// substep count and thus the hot arithmetic cost.
const OMEGAS: [(f32, &str); 4] = [(3.0, "3"), (10.0, "10"), (30.0, "30"), (100.0, "100")];

// ── spring_step: ω-swept motion integrator ───────────────────────────────────
fn bench_spring_step(c: &mut Criterion) {
    let dt = 1.0f32 / 60.0;
    let mut group = c.benchmark_group("spring_step");
    for &(omega, label) in &OMEGAS {
        let tension = omega * omega;
        let friction = 2.0 * 1.0 * omega; // ζ = 1
        let mut spring = Spring::new(tension, friction, 0.0);
        spring.target = 1.0;
        group.bench_with_input(BenchmarkId::from_parameter(label), &omega, |b, &_omega| {
            b.iter(|| {
                spring.step(black_box(dt));
            })
        });
    }
    group.finish();
}

/// Build an `n`-node ring graph whose nnz (= number of valid directed edges)
/// is `n * radius`. Radius < n/2 guarantees unique (src,dst) pairs, so nnz is
/// exactly controllable. Used to sweep the vertex bridge's field cost.
fn ring_graph(n: usize, radius: usize) -> Csr {
    let mut edges = Vec::with_capacity(n * radius);
    for i in 0..n {
        for j in 1..=radius {
            let d = (i + j) % n;
            edges.push((i, d, 1.0));
        }
    }
    Csr::from_edges(n, &edges)
}

/// nnz sweep points: n = 256 nodes, radius ∈ {1,2,4,8} ⇒ nnz ∈ {256,512,1024,2048}.
const NNZ: [(usize, usize, &str); 4] = [
    (256, 1, "256"),
    (256, 2, "512"),
    (256, 4, "1024"),
    (256, 8, "2048"),
];

// ── vertex_bridge_apply_field: nnz-swept graph-Laplacian field ───────────────
// `VertexBridge::apply_field` drives the kernel's `laplacian_spmv` (normalized
// Laplacian) over the particle state every frame. DOCUMENTED-GAP NOTE: the
// production method allocates a fresh `Vec<f64>` per call (`let mut y =
// vec![0.0; n]; self.field = y;` in bridge.rs) — a per-call heap allocation
// living inside what the module docstring calls the "no allocation in the hot
// loop" path. P81 does NOT change this; it only PINS the cost so a future
// allocation-free rewrite has a before/after bench gate. nnz-swept so the
// linear-in-nnz SpMV cost is on the record.
fn bench_vertex_bridge_apply_field(c: &mut Criterion) {
    let mut group = c.benchmark_group("vertex_bridge_apply_field");
    for &(n, radius, label) in &NNZ {
        let graph = ring_graph(n, radius);
        let nnz = graph.nnz();
        let mut bridge = dowiz_engine::VertexBridge::new(n, 4);
        bridge.set_field_graph(graph);
        let x: Vec<f64> = (0..n).map(|i| (i as f64) * 0.01).collect();
        group.bench_with_input(BenchmarkId::from_parameter(label), &nnz, |b, &_nnz| {
            b.iter(|| {
                bridge.apply_field(black_box(&x));
            })
        });
    }
    group.finish();
}

// ── money_present: RED-LINE baseline PIN ─────────────────────────────────────
// `TweenGuard::present_money` is the money-surface value-formatting guard on the
// FE-09 RED-LINE (money never tweens). P81 pins its cost as a STABLE baseline.
// Logic is NOT changed — this bench only locks in the current per-call cost so a
// future edit that makes money presentation hot is caught by bench_track. Single
// id `pin`: the cost must stay flat, regressions are a real defect to kill.
fn bench_money_present(c: &mut Criterion) {
    let mut group = c.benchmark_group("money_present");
    group.bench_function("pin", |b| {
        b.iter(|| {
            // A clean decided integer (the GREEN path) — the common case.
            black_box(TweenGuard::present_money(black_box(155.0))).unwrap();
        })
    });
    group.finish();
    // Compile-time anchor: the absolute nnz/LaplacianKind wiring this bench
    // relies on. Keeps the kernel↔engine seam explicit (mirrors bridge.rs).
    let _ = LaplacianKind::Normalized;
}

criterion_group!(
    benches,
    bench_field_frame_step,
    bench_laplacian_into,
    bench_scene_render_frame,
    bench_frame_rgba,
    bench_compose,
    bench_spring_step,
    bench_vertex_bridge_apply_field,
    bench_money_present,
);
criterion_main!(benches);
