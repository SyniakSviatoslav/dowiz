//! dowiz physics-render wasm bridge.
//!
//! Exposes the RUST KERNEL's field/vertex output to JS via wasm-bindgen so the
//! browser paints physics directly — ZERO TypeScript math. The kernel/engine is
//! the single source of truth; this crate is a thin, offline-clean FFI.

use dowiz_engine::field_frame::{compose, FieldEquilibrium};
use dowiz_engine::{Scene, SdfShape};
use dowiz_engine::VertexBridge;
use dowiz_kernel::csr::Csr;
use dowiz_kernel::retrieval::spine::{build_map, SpineIndex};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

/// Build a `Scene` from a flat circle list `[cx,cy,r, cx,cy,r, ...]`.
fn scene_from_circles(circles: &[f64]) -> Scene {
    let mut scene = Scene::new();
    let mut i = 0;
    while i + 2 < circles.len() {
        scene.add(SdfShape::Circle {
            cx: circles[i],
            cy: circles[i + 1],
            r: circles[i + 2],
        });
        i += 3;
    }
    scene
}

/// Physics render: the kernel-computed field → RGBA8 the GPU blits. Pure,
/// deterministic. Private so host `cargo test` can verify it without a browser.
fn compose_field_impl(circles: &[f64], w: usize, h: usize, steps: usize) -> Vec<u8> {
    let scene = scene_from_circles(circles);
    let eq = FieldEquilibrium::default();
    compose(&scene, &eq, w, h, steps)
}

/// Graph-Laplacian field `y = L·x` (engine VertexBridge over kernel CSR),
/// returned full-precision (f64) so the physics matches the engine exactly.
/// `x` is per-vertex scalar state; empty ⇒ uniform excitation (`1.0`).
fn vertex_field_impl(count: usize, edges: &[f64], x: &[f64]) -> Vec<f64> {
    let mut triples: Vec<(usize, usize, f64)> = Vec::with_capacity(edges.len() / 3);
    let mut i = 0;
    while i + 2 < edges.len() {
        triples.push((edges[i] as usize, edges[i + 1] as usize, edges[i + 2]));
        i += 3;
    }
    let csr = Csr::from_edges(count, &triples);
    let mut b = VertexBridge::new(count, 4);
    b.set_field_graph(csr);
    let state: Vec<f64> = if x.len() == count { x.to_vec() } else { vec![1.0; count] };
    b.apply_field(&state);
    b.field().to_vec()
}

#[wasm_bindgen]
pub fn compose_field(circles: &[f64], w: usize, h: usize, steps: usize) -> Vec<u8> {
    compose_field_impl(circles, w, h, steps)
}

#[wasm_bindgen]
pub fn vertex_field(count: usize, edges: &[f64]) -> Vec<f32> {
    vertex_field_impl(count, edges, &[]).iter().map(|v| *v as f32).collect()
}

// ===== W7: knowledge-spine field (browser-rendered, NOT DOM/TS) =====
//
// The kernel's knowledge-spine (`retrieval::spine`) is the single source of
// truth. These FFI entry points return *fields* (a `String` MAP + `Vec<String>`
// lookups) for the browser to render directly — zero TS string logic, no
// re-implementation of `parse_tags`/grouping in JS.

/// JS-supplied doc record: `[{id, title, tags:[..], path}]`. Mirrors the
/// kernel `SpineIndex` `(id, title, tags, path)` tuple shape exactly.
#[derive(Debug, Deserialize)]
struct SpineDoc {
    id: String,
    title: String,
    tags: Vec<String>,
    path: String,
}

/// Parse the JS JSON doc array into kernel `(id, title, tags, path)` tuples.
/// Private host-testable helper so the doc-format stays in one place.
fn spine_records_from_json(docs_json: &str) -> Vec<(String, String, Vec<String>, String)> {
    let docs: Vec<SpineDoc> = match serde_json::from_str(docs_json) {
        Ok(d) => d,
        Err(_) => return Vec::new(), // fail-soft: bad JSON yields empty corpus
    };
    docs.into_iter()
        .map(|d| (d.id, d.title, d.tags, d.path))
        .collect()
}

/// Build a `SpineIndex` from the JS JSON doc array (single source of truth).
fn spine_index_from_json(docs_json: &str) -> SpineIndex {
    SpineIndex::build(spine_records_from_json(docs_json))
}

/// Knowledge Map: grouped `## <tag>` sections over the corpus. Returns the
/// kernel's deterministic MAP markdown; the browser renders it as a field.
#[wasm_bindgen]
pub fn knowledge_map(docs_json: &str) -> String {
    let records = spine_records_from_json(docs_json);
    build_map(&records)
}

/// Tag lookup (case-insensitive). Returns the sorted bucket of doc ids tagged
/// with `tag` — a field the browser lists, not a DOM tree.
#[wasm_bindgen]
pub fn lookup_tag(tag: &str, docs_json: &str) -> Vec<String> {
    spine_index_from_json(docs_json).lookup_by_tag(tag)
}

/// Related docs: every doc sharing ≥1 tag with `id` (sorted, excludes self).
/// Browser renders the returned id list as a field.
#[wasm_bindgen]
pub fn related_docs(id: &str, docs_json: &str) -> Vec<String> {
    spine_index_from_json(docs_json).related(id)
}

// innovate: ceiling — no real GPU/wasm runtime in headless CI. Verified by the
// wasm32 BUILD gate (links as wasm) + host unit tests; upgrade: add
// wasm-bindgen-cli + a browser smoke (canvas.putImageData) once CI has a display.

#[cfg(test)]
mod tests {
    use super::*;

    // (1) compose returns an RGBA8 buffer of exactly w*h*4 bytes.
    #[test]
    fn wasm_compose_returns_rgba_of_wxh_x4() {
        let circles = [0.0_f64, 0.0, 3.0];
        let out = compose_field_impl(&circles, 32, 32, 10);
        assert_eq!(out.len(), 32 * 32 * 4, "RGBA8 = w*h*4");
    }

    // (2) wrapper's Laplacian field matches the engine's known value (-0.55 for
    //     the triangle graph, x=[0.1,0.4,0.9]) — kernel is the source of truth.
    #[test]
    fn wasm_vertex_field_matches_engine_laplacian() {
        let edges = [
            0.0_f64, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 2.0, 1.0, 2.0, 1.0, 1.0, 0.0, 2.0, 1.0,
            2.0, 0.0, 1.0,
        ];
        let field = vertex_field_impl(3, &edges, &[0.1, 0.4, 0.9]);
        assert_eq!(field.len(), 3);
        assert!((field[0] + 0.55).abs() <= 1e-9, "field[0] = {}", field[0]);
        let sum: f64 = field.iter().sum(); // mass conserved: Σ y ≈ 0
        assert!(sum.abs() <= 1e-9, "Σ field = {sum}");
    }

    // (3) compose is bit-deterministic for identical input.
    #[test]
    fn wasm_compose_deterministic() {
        let circles = [0.0_f64, 0.0, 3.0, 4.0, 2.0, 1.0];
        let a = compose_field_impl(&circles, 32, 24, 20);
        let b = compose_field_impl(&circles, 32, 24, 20);
        assert_eq!(a.len(), 32 * 24 * 4);
        assert_eq!(a, b, "compose must be bit-identical across calls");
    }

    // ===== W7: knowledge-spine field RED→GREEN tests =====

    // (4) knowledge_map groups 3 docs under 2 tags and lists both titles.
    #[test]
    fn wasm_knowledge_map_groups_by_tag() {
        let docs = r#"[
            {"id":"a","title":"Alpha","tags":["rust"],"path":"docs/a.md"},
            {"id":"b","title":"Beta","tags":["rust"],"path":"docs/b.md"},
            {"id":"c","title":"Gamma","tags":["ml"],"path":"docs/c.md"}
        ]"#;
        let map = knowledge_map(docs);
        assert!(map.contains("## rust"), "rust section header present");
        assert!(map.contains("## ml"), "ml section header present");
        assert!(map.contains("Alpha"), "doc title rendered");
        assert!(map.contains("Beta"), "doc title rendered");
        assert!(map.contains("Gamma"), "doc title rendered");
    }

    // (5) lookup_tag is case-insensitive: "FOO" finds doc tagged "foo".
    #[test]
    fn wasm_lookup_tag_case_insensitive() {
        let docs = r#"[
            {"id":"x","title":"X","tags":["foo"],"path":"docs/x.md"},
            {"id":"y","title":"Y","tags":["bar"],"path":"docs/y.md"}
        ]"#;
        let hit = lookup_tag("FOO", docs);
        assert_eq!(hit, vec!["x".to_string()], "case-insensitive tag lookup");
        assert!(lookup_tag("baz", docs).is_empty(), "missing tag ⇒ empty");
    }

    // (6) related_docs returns docs sharing ≥1 tag with the id (excludes self).
    #[test]
    fn wasm_related_returns_shared() {
        let docs = r#"[
            {"id":"a","title":"A","tags":["rust"],"path":"docs/a.md"},
            {"id":"b","title":"B","tags":["rust","ml"],"path":"docs/b.md"},
            {"id":"c","title":"C","tags":["ml"],"path":"docs/c.md"}
        ]"#;
        let rel = related_docs("a", docs);
        assert_eq!(rel, vec!["b".to_string()], "a shares rust with b only");
        let rel_b = related_docs("b", docs);
        assert_eq!(rel_b, vec!["a".to_string(), "c".to_string()], "b↔a rust, b↔c ml");
    }
}
