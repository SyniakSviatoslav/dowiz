// glyph.wgsl — WebGPU glyph (text) SDF shader spine.
//
// The operator directive: ALL text renders through WebGPU + shaders — no HTML
// overlay, no DOM text. This shader paints letterforms whose signed-distance
// atlas is bound as a read-only storage texture/buffer, so glyph rasterises
// identically to the rest of the field UI. The atlas itself is produced by
// cosmic-text shaping (feature `text`); until that unlock, this shader is the
// spine + the Rust-side audit gate proving the SINGLE-WRITER discipline holds.
//
// SINGLE-WRITER discipline (per gpu_atomicity.rs):
//   * `glyph_atlas` is `var<storage, READ>` — NEVER written by this shader. The
//     shaper (CPU-side) is the single writer of the atlas; the GPU reads it.
//   * `out_color` is the framebuffer: per-fragment disjoint pixel writes →
//     SingleWriterProof (Class (c)): each invocation writes its own pixel.
//   * `glyph_index` is a per-fragment `var<function>` scratch (no storage) →
//     no shared write at all.
//
// Innovate: ceiling — v1 assumes the shaper emits a pre-distanced f32 atlas
// (one distance per atlas texel). A multi-channel (MSDF) atlas upgrades the
// `sample_atlas` function to three-channel median selection. Trigger: when
// `text_input::cosmic_text` is unlocked, replace the flat `f32` atlas with the
// shaper's MSDF output and bump `sample_atlas` to median(c.r,c.g,c.b).

struct GlyphUniforms {
    resolution: vec2<f32>,
    cell_size: f32, // world-units per atlas cell
    _pad: f32,
};

@group(1) @binding(0) var<uniform> gu: GlyphUniforms;
@group(1) @binding(1) var<storage, read> glyph_atlas: array<f32>; // SDF atlas (shaper-owned)
@group(1) @binding(2) var<storage, read> atlas_dims: vec2<u32>; // width×height of atlas in cells
// SINGLE-WRITER: out_glyph — framebuffer, per-fragment disjoint write (Class c).
@group(1) @binding(3) var<storage, read_write> out_glyph: array<vec4<f32>>;

@vertex fn gvs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let verts = array<vec2<f32>, 3>(vec2(-1,-1), vec2(3,-1), vec2(-1,3));
    return vec4<f32>(verts[i], 0.0, 1.0);
}

fn sample_atlas(at: vec2<u32>) -> f32 {
    let idx = at.y * atlas_dims.x + at.x;
    return glyph_atlas[idx];
}

@fragment fn gfs(@builtin(position) pix: vec4<f32>) -> @location(0) vec4<f32> {
    // Map pixel to an atlas cell by cell_size, then evaluate the letterform SDF.
    let px = vec2<u32>(u32(pix.x), u32(pix.y));
    let cell = px / vec2<u32>(u32(gu.cell_size));
    let at = cell % atlas_dims;
    let d = sample_atlas(at);

    // Letterform render: inside the glyph (d<0) ⇒ ink; outside ⇒ alpha-falloff
    // for anti-aliasing. AA width derived from the SDF gradient magnitude.
    let aa = clamp(0.5 - d, 0.0, 1.0);
    var ink = mix(vec3<f32>(0.05, 0.05, 0.10), vec3<f32>(0.945, 0.835, 0.541), aa);
    if (d > 0.0) { ink = vec3<f32>(0.078, 0.078, 0.122); } // background outside glyph
    let oi = px.y * u32(gu.resolution.x) + px.x;
    out_glyph[oi] = vec4<f32>(ink, 1.0);
    return vec4<f32>(ink, 1.0);
}