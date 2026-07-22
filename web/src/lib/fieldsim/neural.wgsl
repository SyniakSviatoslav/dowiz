// ─── Neural Field Compute Shader ───
// Izhikevich spiking neuron model on WebGPU compute
// 0.04v² + 5v + 140 - u + I → spike at 30mV → reset

struct Neuron { v: f32, u: f32, spike: u32, }
struct Params { a: f32, b: f32, c: f32, d: f32, }

@group(0) @binding(0) var<storage, read_write> neurons: array<Neuron>;
@group(0) @binding(1) var<storage, read_write> field: array<f32>;
@group(0) @binding(2) var<uniform> time: f32;
@group(0) @binding(3) var<uniform> mouse: vec2<f32>;
@group(0) @binding(4) var<uniform> dims: vec2<u32>;

// Regular Spiking (excitatory)
fn neuron_params(id: u32) -> Params {
  let h = fract(f32(id) * 0.6180339);
  if h < 0.5 {
    return Params(0.02, 0.2, -65.0, 8.0);   // RS
  } else if h < 0.75 {
    return Params(0.1, 0.2, -65.0, 2.0);    // FS
  } else if h < 0.9 {
    return Params(0.02, 0.2, -55.0, 4.0);   // IB
  } else {
    return Params(0.02, 0.25, -65.0, 2.0);  // LTS
  }
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let idx = id.x;
  let total = dims.x * dims.y;
  if (idx >= total) { return; }

  var n = neurons[idx];
  let p = neuron_params(idx);

  // Input current: noise + field + mouse interaction
  let x = f32(idx % dims.x) / f32(dims.x);
  let y = f32(idx / dims.x) / f32(dims.y);
  let m = mouse;
  let dist = distance(vec2(x, y), m);
  let mouse_in = exp(-dist * dist * 20.0) * 10.0;
  let noise = fract(sin(f32(idx) * 12.9898 + time * 1.3) * 43758.5453) * 2.0;
  let field_val = field[idx] * 0.5;
  let I = 5.0 + noise * 3.0 + mouse_in + field_val;

  // Izhikevich step (split into 2 half-steps for stability)
  for (var step = 0u; step < 2u; step++) {
    n.v = n.v + 0.5 * (0.04 * n.v * n.v + 5.0 * n.v + 140.0 - n.u + I);
    n.u = n.u + 0.5 * p.a * (p.b * n.v - n.u);
  }

  // Spike detection
  n.spike = 0u;
  if (n.v >= 30.0) {
    n.v = p.c;
    n.u = n.u + p.d;
    n.spike = 1u;
  }

  neurons[idx] = n;

  // Update field (wave equation contribution from spikes)
  if (n.spike == 1u) {
    let fi = idx;
    let radius = 2u;
    for (var dy = 0u; dy <= radius; dy++) {
      for (var dx = 0u; dx <= radius; dx++) {
        let nx = (idx % dims.x + dx) % dims.x;
        let ny = (idx / dims.x + dy) % dims.y;
        let ni = ny * dims.x + nx;
        if (ni < total) {
          field[ni] = field[ni] + exp(-f32(dx*dx + dy*dy) * 0.5) * 0.3;
        }
      }
    }
  }
}
