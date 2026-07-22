@group(0) @binding(0) var<uniform> time: f32;
@group(0) @binding(1) var<uniform> resolution: vec2<f32>;
@group(0) @binding(2) var<uniform> mouse: vec2<f32>;
@group(0) @binding(3) var<storage, read> particles: array<vec2<f32>>;
@group(0) @binding(4) var<storage, read_write> field: array<f32>;

struct VSOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex fn vs(@builtin(vertex_index) i: u32) -> VSOutput {
    var out: VSOutput;
    let verts = array<vec2<f32>, 3>(
        vec2(-1, -1), vec2(3, -1), vec2(-1, 3)
    );
    out.pos = vec4(verts[i], 0, 1);
    out.uv = vec2(verts[i].x, -verts[i].y) * 0.5 + 0.5;
    return out;
}

fn hash21(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

fn smoothNoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash21(i + vec2(0,0)), hash21(i + vec2(1,0)), u.x),
               mix(hash21(i + vec2(0,1)), hash21(i + vec2(1,1)), u.x), u.y);
}

fn fbm(p: vec2<f32>) -> f32 {
    var v = 0.0; var a = 0.5; var off = p;
    for (var i = 0u; i < 5u; i++) {
        v += a * smoothNoise(off);
        off *= 2.0;
        a *= 0.5;
    }
    return v;
}

@fragment fn fs(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let aspect = resolution.x / resolution.y;
    var p = uv * 2.0 - 1.0;
    p.x *= aspect;

    // Animated wave field
    let wave1 = sin(p.x * 3.0 + time * 0.5) * cos(p.y * 3.0 + time * 0.3) * 0.3;
    let wave2 = sin((p.x * 2.0 + p.y * 2.0) * 2.0 + time * 0.7) * 0.2;
    let wave3 = fbm(p * 2.0 + time * 0.1) * 0.3;
    let field_val = wave1 + wave2 + wave3;

    // Mouse interaction
    let m = mouse * 2.0 - 1.0;
    m.x *= aspect;
    let dist = length(p - m);
    let ripple = sin(dist * 20.0 - time * 4.0) * 0.1 / (dist * 2.0 + 0.5);

    let val = field_val + ripple;

    // Color palette
    let c1 = vec3(0.91, 0.31, 0.09); // brand primary
    let c2 = vec3(1.0, 0.63, 0.18);  // brand hover
    let c3 = vec3(0.05, 0.05, 0.1);  // dark
    let t = val * 0.5 + 0.5;
    let color = mix(c3, mix(c1, c2, t), smoothstep(0.0, 1.0, t));

    // Vignette
    let vignette = 1.0 - length(p) * 0.4;
    let final_color = color * (0.8 + 0.4 * vignette);

    // Particle glow
    var glow = 0.0;
    for (var i = 0u; i < 64u; i++) {
        let pos = particles[i] * 2.0 - 1.0;
        let pd = length(p - pos);
        glow += exp(-pd * pd * 20.0) * 0.02;
    }

    return vec4(final_color + vec3(glow), 1.0);
}
