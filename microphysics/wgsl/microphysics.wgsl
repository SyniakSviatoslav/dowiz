struct SpringParams {
  stiffness: f32,
  damping: f32,
  max_displacement: f32,
  rest_position: f32,
}

struct PhysicsState {
  displacement: f32,
  displacement_velocity: f32,
  position_x: f32,
  position_y: f32,
  velocity_x: f32,
  velocity_y: f32,
  pressure: f32,
  target_x: f32,
  target_y: f32,
  active: u32,
}

@group(0) @binding(0) var<storage, read> params: SpringParams;
@group(0) @binding(1) var<storage, read_write> states: array<PhysicsState>;
@group(0) @binding(2) var<uniform> dt: f32;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let i: u32 = id.x;
  let count: u32 = arrayLength(&states);
  if (i >= count) {
    return;
  }

  var s: PhysicsState = states[i];
  if (s.active == 0u) {
    return;
  }

  let target_displacement: f32 = s.pressure * params.max_displacement;

  let spring_force: f32 = -params.stiffness * (s.displacement - target_displacement);
  let damping_force: f32 = -params.damping * s.displacement_velocity;
  let press_accel: f32 = spring_force + damping_force;

  s.displacement_velocity = s.displacement_velocity + press_accel * dt;
  s.displacement = s.displacement + s.displacement_velocity * dt;
  s.displacement = clamp(s.displacement, 0.0, params.max_displacement);

  let g_stiffness: f32 = params.stiffness * 0.5;
  let g_damping: f32 = params.damping * 2.0;

  let dx: f32 = s.target_x - s.position_x;
  let dy: f32 = s.target_y - s.position_y;

  let g_accel_x: f32 = g_stiffness * dx - g_damping * s.velocity_x;
  let g_accel_y: f32 = g_stiffness * dy - g_damping * s.velocity_y;

  s.velocity_x = s.velocity_x + g_accel_x * dt;
  s.velocity_y = s.velocity_y + g_accel_y * dt;
  s.position_x = s.position_x + s.velocity_x * dt;
  s.position_y = s.position_y + s.velocity_y * dt;

  states[i] = s;
}
