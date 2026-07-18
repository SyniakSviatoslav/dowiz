// web/src/lib/fieldsim/shader.mjs
//
// G3 FieldSim — WebGPU WGSL shader (GPU program, not hand-written math).
//
// The field palette is driven entirely by REAL kernel scalars passed through
// the uniform buffer (rho, gap, fiedler, driftCode — from spectral_flat_js;
// intensity per vertex — from harmonic_centrality_js). The shader only maps
// those values onto color; it computes nothing domain-specific.
//
// Plain `.mjs` (not `.ts`). WGSL is a GPU shading language, not TypeScript, and
// carries no app logic. Kept here (not inline in the Svelte file) so it can be
// `node --check`-ed and reused.

export const WGSL = /* wgsl */ `
struct Uniforms {
  rho : f32,
  gap : f32,
  fiedler : f32,
  driftCode : f32,
  nodeCount : f32,
  maxIntensity : f32,
  courierRemaining : f32,
  courierSeg : f32,
  frame : f32,
  _p0 : f32,
  _p1 : f32,
  _p2 : f32,
  _p3 : f32,
  _p4 : f32,
  _p5 : f32,
  _p6 : f32,
};

@group(0) @binding(0) var<uniform> u : Uniforms;

struct VSIn {
  @location(0) pos : vec2<f32>,
  @location(1) intensity : f32,
  @location(2) radius : f32,
};

struct VSOut {
  @builtin(position) clip : vec4<f32>,
  @location(0) color : vec3<f32>,
};

@vertex
fn vs(in : VSIn) -> VSOut {
  var out : VSOut;
  // pos is already in clip-space unit disk (produced by buffer.mjs layout).
  out.clip = vec4<f32>(in.pos, 0.0, 1.0);

  // Field palette driven by REAL kernel scalars.
  // driftCode: 0 Damped (cool blue) / 1 Resonant (amber) / 2 Unstable (hot red).
  var base : vec3<f32>;
  if (u.driftCode < 0.5) {
    base = vec3<f32>(0.25, 0.65, 1.0);
  } else if (u.driftCode < 1.5) {
    base = vec3<f32>(1.0, 0.75, 0.25);
  } else {
    base = vec3<f32>(1.0, 0.30, 0.30);
  }
  let k = clamp(in.intensity / max(u.maxIntensity, 1.0), 0.0, 1.0);
  // Deterministic wave modulation from spectral radius + frame count.
  // Same kernel state + same frame ⇒ same color (no randomness).
  let wave = 0.85 + 0.15 * sin(u.frame * 0.05 + u.rho * in.intensity);
  out.color = base * (0.4 + 0.6 * k) * wave;
  return out;
}

@fragment
fn fs(in : VSOut) -> @location(0) vec4<f32> {
  return vec4<f32>(in.color, 1.0);
}
`;
