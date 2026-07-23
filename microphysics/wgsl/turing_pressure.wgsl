struct TuringParams {
  diffusion_rate_u: f32,
  diffusion_rate_v: f32,
  feed_rate: f32,
  kill_rate: f32,
  dt: f32,
  grid_width: f32,
  grid_height: f32,
  injection_strength: f32,
  injection_radius: f32,
}

struct InjectionPoint {
  x: f32,
  y: f32,
  pressure: f32,
  active: u32,
}

@group(0) @binding(0) var<storage, read> params: TuringParams;
@group(0) @binding(1) var<storage, read> grid_u_in: array<f32>;
@group(0) @binding(2) var<storage, read> grid_v_in: array<f32>;
@group(0) @binding(3) var<storage, read_write> grid_u_out: array<f32>;
@group(0) @binding(4) var<storage, read_write> grid_v_out: array<f32>;
@group(0) @binding(5) var<storage, read> injections: array<InjectionPoint>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let x: u32 = id.x;
  let y: u32 = id.y;
  let w: u32 = u32(params.grid_width);
  let h: u32 = u32(params.grid_height);

  if (x >= w || y >= h) {
    return;
  }

  let xm: u32 = select(x - 1u, 0u, x == 0u);
  let xp: u32 = select(x + 1u, w - 1u, x >= w - 1u);
  let ym: u32 = select(y - 1u, 0u, y == 0u);
  let yp: u32 = select(y + 1u, h - 1u, y >= h - 1u);

  let i_c: u32 = y * w + x;
  let i_l: u32 = y * w + xm;
  let i_r: u32 = y * w + xp;
  let i_t: u32 = ym * w + x;
  let i_b: u32 = yp * w + x;

  let u_c: f32 = grid_u_in[i_c];
  let v_c: f32 = grid_v_in[i_c];

  let lap_u: f32 = grid_u_in[i_l] + grid_u_in[i_r] + grid_u_in[i_t] + grid_u_in[i_b] - 4.0 * u_c;
  let lap_v: f32 = grid_v_in[i_l] + grid_v_in[i_r] + grid_v_in[i_t] + grid_v_in[i_b] - 4.0 * v_c;

  let uv2: f32 = u_c * v_c * v_c;

  let du: f32 = params.diffusion_rate_u * lap_u - uv2 + params.feed_rate * (1.0 - u_c);
  let dv: f32 = params.diffusion_rate_v * lap_v + uv2 - (params.feed_rate + params.kill_rate) * v_c;

  var new_u: f32 = u_c + du * params.dt;
  var new_v: f32 = v_c + dv * params.dt;

  let fx: f32 = f32(x);
  let fy: f32 = f32(y);
  let num_injections: u32 = arrayLength(&injections);

  for (var j: u32 = 0u; j < num_injections; j = j + 1u) {
    let ip: InjectionPoint = injections[j];
    if (ip.active == 0u) {
      continue;
    }
    let dx_val: f32 = fx - ip.x;
    let dy_val: f32 = fy - ip.y;
    let dist_sq: f32 = dx_val * dx_val + dy_val * dy_val;
    let radius_sq: f32 = params.injection_radius * params.injection_radius;
    if (dist_sq < radius_sq) {
      let gaussian: f32 = exp(-dist_sq / (2.0 * radius_sq));
      new_v = new_v + params.injection_strength * ip.pressure * gaussian * params.dt;
    }
  }

  new_u = clamp(new_u, 0.0, 1.0);
  new_v = clamp(new_v, 0.0, 1.0);

  grid_u_out[i_c] = new_u;
  grid_v_out[i_c] = new_v;
}
