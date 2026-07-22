// ui.wgsl — WebGPU UI field shader spine (P-screen / shader lane).
//
// Renders the composed SDF scene (`Scene::render_frame` output →
// `VertexBridge::vertex_view()`) on the GPU. ONE read-only input binding
// (the per-cell signed-distance field, `field`) drives the fragment colour via
// the brand-tint palette. This shader is the WebGPU paint the backend
// pixel-verification harness rasterizes CPU-side; both produce the same field.
//
// SINGLE-WRITER discipline (per `gpu_atomicity.rs` `SINGLE_WRITER_MARKER`):
//   * `field` is `var<storage, READ>` — this shader NEVER writes it. The CPU
//     (the engine author) is the single writer (Scene::render_into). The GPU is
//     read-only by construction; no atomicity concern can arise here.
//   * The only storage output is `out_color` (the framebuffer texture), which is
//     a per-fragment write at disjoint pixel coords → SingleWriterProof applies:
//     each fragment invocation writes exactly one pixel; no cross-invocation
//     shared write exists.
//
// Innovate: ceiling — this spine paints the SDF field only. Glyph/text raster
// (a multi-channel atlas lookup) lands in `glyph.wgsl`, which extends this
// pipeline with a second read-only atlas binding. Upgrade trigger: when
// `cosmic_text` shaping (feature `text`) is unlocked, the glyph atlas is bound
// here and the `letterform_sdf` function is added.

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    role: u32, // 0 = customer, 1 = owner, 2 = courier — tints the palette
    cart_total_minor: i32, // integer Lek, never tweened (see money_guard.rs)
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var<storage, read> field: array<f32>; // per-cell SDF (signed distance)
// SINGLE-WRITER: out_color — framebuffer texture, each fragment write is at a
// disjoint @builtin(position).xy mapping 1:1 to a unique pixel. No two
// invocations share an output cell; no atomicity concern arises by construction.
// (Class (c) SingleWriterProof — see gpu_atomicity.rs SharedWriteClass.)
@group(0) @binding(2) var<storage, read_write> out_color: array<vec4<f32>>;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) cell: vec2<u32>, // grid cell index (col,row) into `field`
};

@vertex fn vs(@builtin(vertex_index) i: u32) -> VSOut {
    // Fullscreen triangle; cell computed in fs from fragment coords.
    var out: VSOut;
    let verts = array<vec2<f32>, 3>(vec2(-1,-1), vec2(3,-1), vec2(-1,3));
    out.pos = vec4(verts[i], 0, 1);
    out.cell = vec2<u32>(0u, 0u);
    return out;
}

fn role_palette(role: u32) -> (vec3<f32>, vec3<f32>, vec3<f32>) {
    // Brand palette (Dubin & Sushi gold-on-dark) tinted per role.
    // Customer: full gold; owner: cooler gold (analytics tint);
    // courier: amber (motion/high-contrast for in-motion reading §16.53).
    var base = vec3(0.027, 0.078, 0.110); // #07141c surface
    var gold = vec3(0.831, 0.686, 0.216); // #d4af37 brand
    var hi   = vec3(0.945, 0.835, 0.541); // #f1d58a gold-light
    if (role == 1u) { gold = vec3(0.760, 0.700, 0.300); } // owner: cooler
    if (role == 2u) { hi   = vec3(1.000, 0.600, 0.180); } // courier: amber
    return (base, gold, hi);
}

@fragment fn fs(@builtin(position) pix: vec4<f32>) -> @location(0) vec4<f32> {
    let px = vec2<u32>(u32(pix.x), u32(pix.y));
    let cols = u32(u.resolution.x);
    let idx = px.y * cols + px.x;
    let d = field[idx]; // signed distance (negative = inside)

    let (base, gold, hi) = role_palette(u.role);

    // Inside the union ⇒ fill with brand-tinted surface; outside ⇒ background
    // dimmed by distance (depth cue). The SDF is the SOLE geometry authority —
    // no DOM/hit-test feeds this path.
    var color = base;
    if (d < 0.0) {
        // Interior: lerp gold→hi by depth (-d normalized).
        let depth = clamp(-d * 2.0, 0.0, 1.0);
        color = mix(gold, hi, depth);
    } else {
        // Outside: subtle radial fade by signed distance.
        let fade = 1.0 / (1.0 + d * 0.5);
        color = mix(base, vec3(0.063, 0.110, 0.145), fade);
    }

    // Cart-total micro-stripe (owner/courier readout) — encoded as a thin
    // band whose width is a function of `cart_total_minor`. Money remains an
    // integer in the shader; no tween (money_guard.rs RED-LINE).
    if (u.role == 1u || u.role == 2u) {
        let band = clamp(f32(u.cart_total_minor) / 5250.0, 0.0, 1.0);
        if (pix.y < 4.0 && (f32(u.cart_total_minor) > 0.0)) {
            color = mix(color, hi, band * 0.4);
        }
    }

    // SINGLE-WRITER: write the single output pixel for this fragment.
    let oi = px.y * cols + px.x;
    out_color[oi] = vec4<f32>(color, 1.0);
    return vec4<f32>(color, 1.0);
}