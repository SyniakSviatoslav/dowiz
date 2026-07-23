struct ViewportUniforms {
  center_lat: f32,
  center_lng: f32,
  zoom: f32,
  bearing: f32,
  pitch: f32,
  screen_w: f32,
  screen_h: f32,
  tile_size: f32,
}

struct TileVertex {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec3<f32>,
  @location(1) uv: vec2<f32>,
}

struct TileFeature {
  type: u32,
  count: u32,
}

@group(0) @binding(0) var<uniform> viewport: ViewportUniforms;
@group(0) @binding(1) var<storage, read> tile_data: array<u32>;

const PI: f32 = 3.141592653589793;

fn mercator_x(lng: f32) -> f32 {
  return ((lng + 180.0) / 360.0) * pow(2.0, viewport.zoom);
}

fn mercator_y(lat: f32) -> f32 {
  let lat_rad: f32 = lat * PI / 180.0;
  let term: f32 = tan(lat_rad) + 1.0 / cos(lat_rad);
  return (1.0 - log(term) / PI) / 2.0 * pow(2.0, viewport.zoom);
}

fn screen_from_tile(tile_x: f32, tile_y: f32) -> vec2<f32> {
  let center_x: f32 = mercator_x(viewport.center_lng);
  let center_y: f32 = mercator_y(viewport.center_lat);

  let dx: f32 = tile_x - center_x;
  let dy: f32 = tile_y - center_y;

  let bearing_rad: f32 = viewport.bearing * PI / 180.0;
  let cos_b: f32 = cos(bearing_rad);
  let sin_b: f32 = sin(bearing_rad);

  let rx: f32 = dx * cos_b - dy * sin_b;
  let ry: f32 = dx * sin_b + dy * cos_b;

  let zoom_scale: f32 = pow(2.0, viewport.zoom) * viewport.tile_size / 256.0;
  let sx: f32 = rx * zoom_scale + viewport.screen_w / 2.0;
  let sy: f32 = ry * zoom_scale + viewport.screen_h / 2.0;

  return vec2<f32>(sx, sy);
}

fn pitch_scale(screen_y: f32) -> f32 {
  let normalized_y: f32 = (screen_y / viewport.screen_h) * 2.0 - 1.0;
  let pitch_rad: f32 = viewport.pitch * PI / 180.0;
  return 1.0 / (1.0 + normalized_y * tan(pitch_rad) * 0.5);
}

fn feature_color(feature_type: u32) -> vec3<f32> {
  switch (feature_type) {
    case 1u: { return vec3<f32>(0.25, 0.25, 0.25); }
    case 2u: { return vec3<f32>(0.85, 0.85, 0.90); }
    case 3u: { return vec3<f32>(0.35, 0.75, 0.35); }
    case 4u: { return vec3<f32>(0.55, 0.65, 0.75); }
    default: { return vec3<f32>(0.92, 0.92, 0.95); }
  }
}

@vertex
fn main(@builtin(vertex_index) vertex_id: u32) -> TileVertex {
  let tile_idx: u32 = vertex_id / 6u;
  let local_vert: u32 = vertex_id % 6u;

  let header_base: u32 = tile_idx * 8192u;
  let header_word: u32 = tile_data[header_base];
  let feat_type: u32 = header_word & 0xFFFFu;
  let feat_count: u32 = (header_word >> 16u) & 0xFFFFu;

  let vert_in_feature: u32 = local_vert / 2u;
  let coord_idx: u32 = min(vert_in_feature, feat_count - 1u);
  let coord_base: u32 = header_base + 1u + coord_idx * 2u;

  let lng_raw: u32 = tile_data[coord_base];
  let lat_raw: u32 = tile_data[coord_base + 1u];

  let lng: f32 = (f32(lng_raw) / 10000000.0) * 0.001;
  let lat: f32 = (f32(lat_raw) / 10000000.0) * 0.001;

  let tile_x: f32 = mercator_x(lng);
  let tile_y: f32 = mercator_y(lat);

  var screen: vec2<f32> = screen_from_tile(tile_x, tile_y);
  let p_scale: f32 = pitch_scale(screen.y);

  var offset: vec2<f32> = vec2<f32>(0.0, 0.0);
  if (local_vert == 0u) { offset = vec2<f32>(-2.0, -2.0); }
  else if (local_vert == 1u) { offset = vec2<f32>(2.0, -2.0); }
  else if (local_vert == 2u) { offset = vec2<f32>(2.0, 2.0); }
  else if (local_vert == 3u) { offset = vec2<f32>(-2.0, -2.0); }
  else if (local_vert == 4u) { offset = vec2<f32>(2.0, 2.0); }
  else { offset = vec2<f32>(-2.0, 2.0); }

  screen = screen + offset * p_scale;

  var output: TileVertex;
  output.position = vec4<f32>(
    (screen.x / viewport.screen_w) * 2.0 - 1.0,
    ((viewport.screen_h - screen.y) / viewport.screen_h) * 2.0 - 1.0,
    p_scale * 0.5,
    1.0,
  );
  output.color = feature_color(feat_type);
  output.uv = vec2<f32>(f32(local_vert % 2u), f32(local_vert / 2u));
  return output;
}

@fragment
fn main(input: TileVertex) -> @location(0) vec4<f32> {
  let base: vec3<f32> = input.color;
  let ambient: f32 = 0.5;
  let lit: vec3<f32> = base * (ambient + (1.0 - ambient) * input.position.z);
  return vec4<f32>(lit, 1.0);
}
