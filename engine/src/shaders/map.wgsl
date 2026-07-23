// map.wgsl — WebGPU map tile render shader spine.
//
// Renders geographic tile data (roads, buildings, parks, water) as a
// 2D overlay on the field-UI scene. The CPU prepares tile data as a flat
// storage buffer (see `geo_map::flatten_tile_for_gpu`); the shader projects
// each point from mercator space to screen space and emits coloured vertices.
//
// SINGLE-WRITER discipline (per gpu_atomicity.rs):
//   * `viewport_uniforms` is `var<uniform>` — single writer (CPU).
//   * `tile_data` is `var<storage, read>` — NEVER written by this shader.
//   * `output_verts` is `var<storage, read_write>` — each workgroup writes
//     disjoint tile-slot regions (tile_idx * 4096 .. tile_idx * 4096 + count),
//     producing per-vertex disjoint writes (Class (c) SingleWriterProof).
//
// innovate: ceiling — v1 renders points as coloured dots. A future upgrade
// adds line/polygon rasterisation for roads and building footprints.
// Upgrade trigger: when the courier tracking surface needs road-aware routing
// visualisation on the map.

struct ViewportUniforms {
    center_lat: f32,
    center_lng: f32,
    zoom: f32,
    bearing: f32,
    pitch: f32,
    screen_w: f32,
    screen_h: f32,
    tile_size: f32,
};

struct TileVertex {
    pos_x: f32,
    pos_y: f32,
    color_r: f32,
    color_g: f32,
    color_b: f32,
};

struct TileOutput {
    vertices: array<TileVertex>,
};

@group(0) @binding(0) var<uniform> u: ViewportUniforms;
@group(0) @binding(1) var<storage, read> tile_data: array<u32>;
@group(0) @binding(2) var<storage, read_write> output_verts: TileOutput;

const PI: f32 = 3.141592653589793;

fn mercator_x(lng: f32, zoom: f32) -> f32 {
    return ((lng + 180.0) / 360.0) * pow(2.0, zoom);
}

fn mercator_y(lat: f32, zoom: f32) -> f32 {
    let lat_rad: f32 = lat * PI / 180.0;
    let term: f32 = tan(lat_rad) + 1.0 / cos(lat_rad);
    return (1.0 - log(term) / PI) / 2.0 * pow(2.0, zoom);
}

fn tile_color(feature_type: u32) -> vec3<f32> {
    // Road, Building, Park, Water
    if (feature_type == 1u) {
        return vec3<f32>(0.55, 0.55, 0.55); // road — grey
    } else if (feature_type == 2u) {
        return vec3<f32>(0.7, 0.7, 0.75);    // building — light blue-grey
    } else if (feature_type == 3u) {
        return vec3<f32>(0.3, 0.7, 0.3);     // park — green
    } else if (feature_type == 4u) {
        return vec3<f32>(0.2, 0.5, 0.8);     // water — blue
    }
    return vec3<f32>(0.9, 0.9, 0.92);        // unknown — light
}

fn screen_x(tx: f32, cx: f32, scale: f32, bearing_cos: f32, bearing_sin: f32, sw: f32) -> f32 {
    let dx: f32 = tx - cx;
    let rotated: f32 = dx * bearing_cos; // - dy * bearing_sin — simplified for point
    return rotated * 256.0 * scale + sw / 2.0;
}

fn screen_y(ty: f32, cy: f32, scale: f32, bearing_cos: f32, bearing_sin: f32, sh: f32) -> f32 {
    let dy: f32 = ty - cy;
    let rotated: f32 = dy * bearing_cos; // + dx * bearing_sin — simplified
    return rotated * 256.0 * scale + sh / 2.0;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let tile_idx: u32 = id.x;
    let max_tiles: u32 = 64u;
    if (tile_idx >= max_tiles) {
        return;
    }

    // Each tile occupies 2048 u32 slots (8192 bytes).
    let tile_offset: u32 = tile_idx * 2048u;
    let header: u32 = tile_data[tile_offset];
    let feature_type: u32 = header & 0xFFFFu;
    let feature_count: u32 = (header >> 16u) & 0xFFFFu;

    if (feature_count == 0u) {
        return;
    }

    let clat: f32 = u.center_lat;
    let clng: f32 = u.center_lng;
    let zoom: f32 = u.zoom;
    let bearing: f32 = u.bearing;
    let sw: f32 = u.screen_w;
    let sh: f32 = u.screen_h;

    let bearing_rad: f32 = bearing * PI / 180.0;
    let bearing_cos: f32 = cos(bearing_rad);
    let bearing_sin: f32 = sin(bearing_rad);
    let zoom_scale: f32 = pow(2.0, zoom) * u.tile_size / 256.0;

    let cx: f32 = mercator_x(clng, zoom);
    let cy: f32 = mercator_y(clat, zoom);
    let color: vec3<f32> = tile_color(feature_type);

    let base_vert: u32 = tile_idx * 4096u; // max 4096 vertices per tile

    for (var i: u32 = 0u; i < feature_count && i < 2000u; i = i + 1u) {
        let coord_offset: u32 = tile_offset + 1u + i * 2u;
        let lng_raw: u32 = tile_data[coord_offset];
        let lat_raw: u32 = tile_data[coord_offset + 1u];

        let lng_f: f32 = f32(i32(lng_raw)) / 10000000.0;
        let lat_f: f32 = f32(i32(lat_raw)) / 10000000.0;

        let tx: f32 = mercator_x(lng_f, zoom);
        let ty: f32 = mercator_y(lat_f, zoom);

        let sx: f32 = screen_x(tx, cx, zoom_scale, bearing_cos, bearing_sin, sw);
        let sy: f32 = screen_y(ty, cy, zoom_scale, bearing_cos, bearing_sin, sh);

        let out_idx: u32 = base_vert + i;
        output_verts.vertices[out_idx].pos_x = (sx / sw) * 2.0 - 1.0;
        output_verts.vertices[out_idx].pos_y = (sy / sh) * 2.0 - 1.0;
        output_verts.vertices[out_idx].color_r = color.r;
        output_verts.vertices[out_idx].color_g = color.g;
        output_verts.vertices[out_idx].color_b = color.b;
    }
}
