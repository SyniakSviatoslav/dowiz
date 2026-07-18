//! dowiz physics-render wasm bridge.
//!
//! Exposes the RUST KERNEL's field/vertex output to JS via wasm-bindgen so the
//! browser paints physics directly — ZERO TypeScript math. The kernel/engine is
//! the single source of truth; this crate is a thin, offline-clean FFI.

use dowiz_engine::field_frame::{compose, FieldEquilibrium, FieldFrame};
use dowiz_engine::VertexBridge;
use dowiz_engine::{Scene, SdfShape};
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
    let state: Vec<f64> = if x.len() == count {
        x.to_vec()
    } else {
        vec![1.0; count]
    };
    b.apply_field(&state);
    b.field().to_vec()
}

#[wasm_bindgen]
pub fn compose_field(circles: &[f64], w: usize, h: usize, steps: usize) -> Vec<u8> {
    compose_field_impl(circles, w, h, steps)
}

/// Stateful field-frame integrator exposed to JS for a live rAF loop. The
/// browser calls `step()` once per animation frame to advance the physics and
/// `frame()` to blit the returned RGBA — ALL math stays in the kernel/engine.
#[wasm_bindgen]
pub struct FieldSim {
    frame: FieldFrame,
    source: Vec<f32>,
    eq: FieldEquilibrium,
    w: usize,
    h: usize,
}

#[wasm_bindgen]
impl FieldSim {
    /// Build a sim from a flat circle list `[cx,cy,r, ...]`, rasterize the SDF
    /// source `S`, and allocate a zeroed field `U`. The browser only blits.
    #[wasm_bindgen(constructor)]
    pub fn new(circles: &[f64], w: usize, h: usize) -> FieldSim {
        let scene = scene_from_circles(circles);
        let source = scene.render_frame(w, h);
        FieldSim {
            frame: FieldFrame::new(w, h),
            source,
            eq: FieldEquilibrium::default(),
            w,
            h,
        }
    }

    /// Advance one physics timestep. The rAF loop calls this per frame.
    pub fn step(&mut self) {
        self.frame.step(&self.source, &self.eq);
    }

    /// RGBA8 frame the canvas paints (`len == w*h*4`, never NaN bytes).
    pub fn frame(&self) -> Vec<u8> {
        self.frame.frame_rgba()
    }

    /// Frame width (JS sizes the `ImageData`).
    pub fn width(&self) -> usize {
        self.w
    }

    /// Frame height (JS sizes the `ImageData`).
    pub fn height(&self) -> usize {
        self.h
    }
}

#[wasm_bindgen]
pub fn vertex_field(count: usize, edges: &[f64]) -> Vec<f32> {
    vertex_field_impl(count, edges, &[])
        .iter()
        .map(|v| *v as f32)
        .collect()
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

// ===== P57 M7: canvas text-input wasm exports (per P38 G7 ptr/len) =====
//
// These expose `dowiz_engine::text_input::TextField` to JS via wasm-bindgen.
// Gated behind `feature = "text"` (Lane B / O18a): the glyph-run `ptr/len`
// view rides the engine's cosmic-text shaping seam, which needs the same network
// grant as wgpu. Under the DEFAULT build these are NOT compiled — the engine
// `TextField` is still fully available to native Rust clients.
//
// P38 G7 convention: frame-scoped views are re-derived EVERY frame (the
// detached-buffer rule, P38 §3.7). `text_glyph_runs_ptr/len` return a
// `&[f32]`-equivalent raw view into a short-lived buffer that is rebuilt on
// each `text_apply_js`/`text_new_js` — never cached across frames.
//
// P57 M7 exports (§3):
//   text_new_js(field_id) -> u32
//   text_apply_js(handle, cmd_json) -> String  (EditEvent JSON)
//   text_value_js(handle) -> String
//   text_caret_js(handle) -> "[u,v,w,h]"
//   text_glyph_runs_ptr(handle) -> *const f32
//   text_glyph_runs_len(handle) -> usize

#[cfg(feature = "text")]
mod text_js {
    use dowiz_engine::text_input::{ClipboardPort, EditCmd, TextField, WAVE0_SCRIPTS};
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static FIELDS: RefCell<HashMap<u32, TextField>> = RefCell::new(HashMap::new());
        static CLIP: RefCell<Option<String>> = RefCell::new(None);
    }

    /// A `ClipboardPort` backed by a thread-local cell (wasm has no clipboard
    /// crate; the web path pre-reads `navigator.clipboard` and supplies the
    /// payload to `Paste`). This is the honest stub the shell fills natively
    /// on the native path (P63/P39-rev).
    struct WebClipboard;
    impl ClipboardPort for WebClipboard {
        fn read(&mut self) -> Option<String> {
            CLIP.with(|c| c.borrow().clone())
        }
        fn write(&mut self, s: &str) {
            CLIP.with(|c| *c.borrow_mut() = Some(s.to_string()));
        }
    }

    /// Deserialize an `EditCmd` from the JSON the web `KeyboardSource` emits.
    /// Only the committed grapheme-cluster + motion commands are serialized;
    /// there is NO preedit/composition variant (P57 §2.2).
    fn cmd_from_json(json: &str) -> Result<EditCmd, String> {
        // Minimal, dependency-light parse (no serde here to keep the wasm
        // bundle lean; mirrors the kernel's spine JSON approach).
        let json = json.trim();
        if let Some(rest) = json.strip_prefix("{\"Insert\":\"") {
            let s = rest.trim_end_matches("\"}").to_string();
            return Ok(EditCmd::Insert(s));
        }
        if let Some(rest) = json.strip_prefix("{\"Paste\":\"") {
            let s = rest.trim_end_matches("\"}").to_string();
            return Ok(EditCmd::Paste(s));
        }
        if let Some(rest) = json.strip_prefix("{\"MoveLeft\":") {
            let sel = rest.trim_end_matches('}').contains("true");
            return Ok(EditCmd::MoveLeft { select: sel });
        }
        if let Some(rest) = json.strip_prefix("{\"MoveRight\":") {
            let sel = rest.trim_end_matches('}').contains("true");
            return Ok(EditCmd::MoveRight { select: sel });
        }
        if let Some(rest) = json.strip_prefix("{\"MoveWordLeft\":") {
            let sel = rest.trim_end_matches('}').contains("true");
            return Ok(EditCmd::MoveWordLeft { select: sel });
        }
        if let Some(rest) = json.strip_prefix("{\"MoveWordRight\":") {
            let sel = rest.trim_end_matches('}').contains("true");
            return Ok(EditCmd::MoveWordRight { select: sel });
        }
        if let Some(rest) = json.strip_prefix("{\"MoveHome\":") {
            let sel = rest.trim_end_matches('}').contains("true");
            return Ok(EditCmd::MoveHome { select: sel });
        }
        if let Some(rest) = json.strip_prefix("{\"MoveEnd\":") {
            let sel = rest.trim_end_matches('}').contains("true");
            return Ok(EditCmd::MoveEnd { select: sel });
        }
        match json {
            "{\"Backspace\"}" => Ok(EditCmd::Backspace),
            "{\"Delete\"}" => Ok(EditCmd::Delete),
            "{\"SelectAll\"}" => Ok(EditCmd::SelectAll),
            "{\"Cut\"}" => Ok(EditCmd::Cut),
            "{\"Copy\"}" => Ok(EditCmd::Copy),
            "{\"Focus\"}" => Ok(EditCmd::Focus),
            "{\"Blur\"}" => Ok(EditCmd::Blur),
            "{\"Submit\"}" => Ok(EditCmd::Submit),
            "{\"PointerUp\"}" => Ok(EditCmd::PointerUp),
            _ => Err(format!("unknown EditCmd JSON: {json}")),
        }
    }

    /// Serialize the ordered `EditEvent` sequence to a JSON array string.
    fn events_to_json(events: &[dowiz_engine::text_input::EditEvent]) -> String {
        let mut out = String::from("[");
        for (i, e) in events.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&match e {
                dowiz_engine::text_input::EditEvent::TextChanged { value_bytes } => {
                    format!("{{\"TextChanged\":{{\"value_bytes\":{value_bytes}}}}}")
                }
                dowiz_engine::text_input::EditEvent::CaretMoved { caret } => {
                    format!("{{\"CaretMoved\":{{\"caret\":{}}}}}", caret.0)
                }
                dowiz_engine::text_input::EditEvent::SelectionChanged { sel } => {
                    format!(
                        "{{\"SelectionChanged\":{{\"anchor\":{},\"focus\":{}}}}}",
                        sel.anchor.0, sel.focus.0
                    )
                }
                dowiz_engine::text_input::EditEvent::Submitted => {
                    "{\"Submitted\":true}".to_string()
                }
                dowiz_engine::text_input::EditEvent::FocusChanged { focused } => {
                    format!("{{\"FocusChanged\":{{\"focused\":{focused}}}}}")
                }
                dowiz_engine::text_input::EditEvent::Rejected(r) => {
                    let name = match r {
                        dowiz_engine::text_input::EditReject::OutOfScope(c) => {
                            format!("{{\"OutOfScope\":\"{c}\"}}")
                        }
                        dowiz_engine::text_input::EditReject::ReadOnly => {
                            "\"ReadOnly\"".to_string()
                        }
                        dowiz_engine::text_input::EditReject::NothingToDelete => {
                            "\"NothingToDelete\"".to_string()
                        }
                        dowiz_engine::text_input::EditReject::NoSelection => {
                            "\"NoSelection\"".to_string()
                        }
                    };
                    format!("{{\"Rejected\":{name}}}")
                }
            });
        }
        out.push(']');
        out
    }
}

#[cfg(feature = "text")]
#[wasm_bindgen]
pub fn text_new_js(field_id: u32) -> u32 {
    text_js::FIELDS.with(|f| {
        let mut m = f.borrow_mut();
        m.insert(field_id, TextField::new(field_id));
        field_id
    })
}

#[cfg(feature = "text")]
#[wasm_bindgen]
pub fn text_apply_js(handle: u32, cmd_json: &str) -> String {
    text_js::FIELDS.with(|f| {
        let mut m = f.borrow_mut();
        if let Some(tf) = m.get_mut(&handle) {
            match text_js::cmd_from_json(cmd_json) {
                Ok(cmd) => {
                    let mut clip = text_js::WebClipboard;
                    let ev = tf.apply(cmd, &mut clip);
                    text_js::events_to_json(&ev)
                }
                Err(e) => format!("{{\"error\":\"{e}\"}}"),
            }
        } else {
            format!("{{\"error\":\"unknown handle {handle}\"}}")
        }
    })
}

#[cfg(feature = "text")]
#[wasm_bindgen]
pub fn text_value_js(handle: u32) -> String {
    text_js::FIELDS.with(|f| {
        f.borrow()
            .get(&handle)
            .map(|tf| tf.value().to_string())
            .unwrap_or_default()
    })
}

#[cfg(feature = "text")]
#[wasm_bindgen]
pub fn text_caret_js(handle: u32) -> String {
    text_js::FIELDS.with(|f| {
        if let Some(tf) = f.borrow().get(&handle) {
            let r = tf.caret_rect();
            format!("[{},{},{},{}]", r.u, r.v, r.w, r.h_h * 2.0)
        } else {
            String::new()
        }
    })
}

#[cfg(feature = "text")]
#[wasm_bindgen]
pub fn text_glyph_runs_ptr(handle: u32) -> *const f32 {
    text_js::FIELDS.with(|f| {
        // Frame-scoped re-derivation (P38 §3.7): the glyph-run view is rebuilt
        // every call; a cached/detached view is rejected by construction.
        static mut VIEW: Vec<f32> = Vec::new();
        if let Some(tf) = f.borrow().get(&handle) {
            let runs = tf.glyph_runs();
            let mut v = unsafe { &mut *std::ptr::addr_of_mut!(VIEW) };
            v.clear();
            for g in runs {
                v.push(g.pos.u);
                v.push(g.pos.v);
                v.push(g.pos.w);
                v.push(g.adv);
                v.push(g.size);
            }
            v.as_ptr()
        } else {
            std::ptr::null()
        }
    })
}

#[cfg(feature = "text")]
#[wasm_bindgen]
pub fn text_glyph_runs_len(handle: u32) -> usize {
    text_js::FIELDS.with(|f| {
        f.borrow()
            .get(&handle)
            .map(|tf| tf.glyph_runs().len() * 5)
            .unwrap_or(0)
    })
}

// Default-build guard (DoD-10 mirror): under the DEFAULT wasm build the `text`
// feature is OFF, so the P57 text exports are NOT compiled. This pure test
// documents that the WAVE0_SCRIPTS constant is accessible from the engine
// (v2 boundary) even without the wasm text feature.
#[cfg(test)]
mod p57_tests {
    use dowiz_engine::text_input::{ByteCursor, EditCmd, EditEvent, TextField};
    use dowiz_engine::text_scope::in_wave0_scope;

    struct MemClip {
        s: Option<String>,
    }
    impl dowiz_engine::text_input::ClipboardPort for MemClip {
        fn read(&mut self) -> Option<String> {
            self.s.clone()
        }
        fn write(&mut self, s: &str) {
            self.s = Some(s.to_string());
        }
    }

    // RED→GREEN (M2, host-testable): type Latin+Cyrillic, assert value + events.
    #[test]
    fn wasm_text_field_roundtrip() {
        let mut tf = TextField::new(1);
        let mut clip = MemClip { s: None };
        for ch in ["h", "i", " ", "п", "р"] {
            let ev = tf.apply(EditCmd::Insert(ch.to_string()), &mut clip);
            assert!(ev
                .iter()
                .any(|e| matches!(e, EditEvent::TextChanged { .. })));
        }
        assert_eq!(tf.value(), "hi пр");
        assert_eq!(tf.caret(), ByteCursor(7)); // 2 Latin (2b) + 1 sp + 2 Cyr (4b)
    }

    // ADVERSARIAL (M2): a CJK Insert is refused; scope gate holds at apply time.
    #[test]
    fn wasm_text_rejects_out_of_scope() {
        let mut tf = TextField::new(2);
        let mut clip = MemClip { s: None };
        let ev = tf.apply(EditCmd::Insert("中".to_string()), &mut clip);
        assert!(ev.iter().any(|e| matches!(e, EditEvent::Rejected(_))));
        assert_eq!(tf.value(), "");
    }

    // v2 classifier reachable from the wasm crate (the boundary is a shared type).
    #[test]
    fn wasm_text_scope_constant() {
        assert!(in_wave0_scope('a'));
        assert!(in_wave0_scope('Я'));
        assert!(!in_wave0_scope('中'));
    }
}
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
            0.0_f64, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 2.0, 1.0, 2.0, 1.0, 1.0, 0.0, 2.0, 1.0, 2.0,
            0.0, 1.0,
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
        assert_eq!(
            rel_b,
            vec!["a".to_string(), "c".to_string()],
            "b↔a rust, b↔c ml"
        );
    }

    // ===== W8: stateful FieldSim (live rAF physics) RED→GREEN tests =====

    // (7) FieldSim advances with finite bytes + actually evolves. 20 steps with
    //     3 circles must yield a w*h*4 frame, all bytes finite (no NaN), and a
    //     frame that differs from step 0 (proving the physics moved, not frozen).
    #[test]
    fn wasm_fieldsim_advances_finite() {
        let circles = [0.0_f64, 0.0, 3.0, 4.0, 2.0, 1.0, -4.0, -2.0, 2.0];
        let mut sim = FieldSim::new(&circles, 32, 32);
        assert_eq!(sim.width(), 32);
        assert_eq!(sim.height(), 32);

        let f0 = sim.frame();
        assert_eq!(f0.len(), 32 * 32 * 4, "RGBA8 = w*h*4");
        // u8 bytes are finite by construction (frame_rgba maps non-finite→0).
        // A non-all-zero step-0 frame proves real SDF-driven content.
        assert!(f0.iter().any(|&b| b != 0), "step-0 frame must have content");

        for _ in 0..20 {
            sim.step();
        }
        let f20 = sim.frame();
        assert_eq!(f20.len(), 32 * 32 * 4, "RGBA8 = w*h*4");
        // u8 bytes are finite by construction; frame must be non-collapsed.
        assert!(
            f20.iter().any(|&b| b != 0),
            "step-20 frame must have content"
        );
        // Field must have evolved: step-20 frame differs from step-0 frame.
        assert_ne!(f20, f0, "FieldSim must evolve over 20 steps (not frozen)");
    }

    // (8) FieldSim is deterministic: two sims with identical circles produce
    //     bit-identical frame bytes after the same number of steps.
    #[test]
    fn wasm_fieldsim_deterministic() {
        let circles = [0.0_f64, 0.0, 3.0, 4.0, 2.0, 1.0, -4.0, -2.0, 2.0];
        let mut a = FieldSim::new(&circles, 32, 32);
        let mut b = FieldSim::new(&circles, 32, 32);
        for _ in 0..20 {
            a.step();
            b.step();
        }
        let fa = a.frame();
        let fb = b.frame();
        assert_eq!(fa.len(), 32 * 32 * 4);
        assert_eq!(fa, fb, "FieldSim must be bit-deterministic across sims");
    }
}
