// web/src/lib/fieldsim/buffer.mjs
//
// G3 FieldSim — kernel-state → WebGPU typed-buffer marshaller.
//
// This module contains ZERO domain math. It ONLY:
//   1. calls the already-bound dowiz-kernel wasm exports
//      (harmonic_centrality_js / spectral_flat_js / geo_progress_flat_js /
//       geo_lerp_js — all 24/24 wired in kernel_client.mjs), and
//   2. copies their real numeric outputs, unchanged except for a trivial
//      display-scale min/max pass, into flat Float32Arrays that the WebGPU
//      vertex + uniform stages consume.
//
// Determinism: same kernel state in ⇒ byte-identical buffers out (the kernel
// owns every number). The kernel must be bound first: call bindKernel(bytes)
// from kernel_client.mjs before buildFieldBuffer(). We import the already-shipped
// client rather than re-implement the wasm ABI.
//
// Node/SSR note: this file is plain `.mjs`, not `.ts` — it is a kernel-call
// wrapper (same role as kernel_client.mjs), not hand-written math logic.

import {
  harmonic_centrality_js,
  spectral_flat_js,
  geo_progress_flat_js,
  geo_lerp_js,
} from '../kernel/kernel_client.mjs';

export const FLOATS_PER_VERTEX = 4; // layout: [x, y, intensity, radius]
export const UNIFORM_FLOATS = 16; // reserved uniform block (16 f32)

// Decode a kernel flat-array string ("[a,b,c,...]") into a JS number[].
function parseFlatArray(s) {
  const v = JSON.parse(s);
  if (!Array.isArray(v)) throw new Error(`expected flat array, got ${typeof v}`);
  return v;
}

// Decode a kernel {lat,lng} object string into [lat, lng].
function parseLatLng(s) {
  const v = JSON.parse(s);
  if (typeof v.lat !== 'number' || typeof v.lng !== 'number') {
    throw new Error(`expected {lat,lng}, got ${JSON.stringify(v)}`);
  }
  return [v.lat, v.lng];
}

function asArray(res, label) {
  if (!res.ok) throw new Error(`field buffer: kernel ${label} rejected: ${res.err}`);
  return parseFlatArray(res.value);
}

function asLatLng(res, label) {
  if (!res.ok) throw new Error(`field buffer: kernel ${label} rejected: ${res.err}`);
  return parseLatLng(res.value);
}

// Map a lat/lng into the unit-disk render space using the route bounds.
// This is a *display projection* (render scaffold), not domain math — it only
// places real kernel coordinates onto the canvas.
function makeProjector(routeArr) {
  let minLat = Infinity, maxLat = -Infinity, minLng = Infinity, maxLng = -Infinity;
  for (const [lat, lng] of routeArr) {
    if (lat < minLat) minLat = lat;
    if (lat > maxLat) maxLat = lat;
    if (lng < minLng) minLng = lng;
    if (lng > maxLng) maxLng = lng;
  }
  const spanLat = Math.max(maxLat - minLat, 1e-9);
  const spanLng = Math.max(maxLng - minLng, 1e-9);
  const S = 0.8; // keep inside the clip-space disk
  return (lat, lng) => [
    ((lng - minLng) / spanLng) * 2 * S - S,
    ((lat - minLat) / spanLat) * 2 * S - S,
  ];
}

/**
 * Build the FieldSim vertex + uniform buffers from a kernel state snapshot.
 *
 * @param {object} state
 * @param {number} state.n            node count
 * @param {string} state.edges        JSON "[[u,v],...]" (harmonic centrality graph)
 * @param {string} state.matrix       JSON adjacency "[[...]]" (spectral summary)
 * @param {string} state.route        JSON polyline "[[lat,lng],...]" (courier path)
 * @param {number} [state.courierT]   0..1 progress along the route
 * @returns {{vertices:Float32Array, uniforms:Float32Array, vertexCount:number,
 *            floatsPerVertex:number, layout:{fields:string[]}}}
 */
export function buildFieldBuffer(state) {
  const { n, edges, matrix, route, courierT = 0.5 } = state;

  // ── REAL kernel math (the only source of field values) ──────────────
  const harm = asArray(harmonic_centrality_js(n, edges), 'harmonic_centrality_js');
  const spec = asArray(spectral_flat_js(matrix), 'spectral_flat_js');
  // spectral_flat layout: [rho, gap, fiedler, drift_code, nEig, e0re,e0im, ...]
  const rho = spec[0];
  const gap = spec[1];
  const fiedler = spec[2];
  const driftCode = spec[3];

  // Courier position along the route via real geo math.
  const routeArr = JSON.parse(route);
  const segCount = Math.max(routeArr.length - 1, 1);
  const tt = Math.min(Math.max(courierT, 0), 1) * segCount;
  const seg = Math.min(Math.floor(tt), segCount - 1);
  const frac = tt - seg;
  const a = routeArr[seg];
  const b = routeArr[seg + 1];
  const [courierLat, courierLng] = asLatLng(geo_lerp_js(a[0], a[1], b[0], b[1], frac), 'geo_lerp_js');
  const prog = asArray(geo_progress_flat_js(route, courierLat, courierLng), 'geo_progress_flat_js');
  const courierRemaining = prog[0]; // meters remaining (REAL geo math)
  const courierSeg = prog[3]; // segment index (REAL geo math)

  const project = makeProjector(routeArr);

  // ── marshalling only: node layout on a unit circle (render scaffold) ──
  const nodeCount = harm.length; // == n
  let maxIntensity = 0;
  for (const v of harm) if (v > maxIntensity) maxIntensity = v;
  if (maxIntensity <= 0) maxIntensity = 1; // display-scale guard only

  const vertexCount = nodeCount + 1; // + courier marker
  const vertices = new Float32Array(vertexCount * FLOATS_PER_VERTEX);
  for (let i = 0; i < nodeCount; i++) {
    const ang = (2 * Math.PI * i) / Math.max(nodeCount, 1);
    const o = i * FLOATS_PER_VERTEX;
    vertices[o + 0] = Math.cos(ang); // x (layout scaffold)
    vertices[o + 1] = Math.sin(ang); // y (layout scaffold)
    vertices[o + 2] = harm[i]; // intensity (REAL harmonic centrality)
    vertices[o + 3] = 0.03 + 0.10 * (harm[i] / maxIntensity); // radius (display-scale)
  }
  // Courier marker — real geo position projected into the field disk.
  const [cx, cy] = project(courierLat, courierLng);
  const co = nodeCount * FLOATS_PER_VERTEX;
  vertices[co + 0] = cx;
  vertices[co + 1] = cy;
  vertices[co + 2] = 1.0; // intensity (bright)
  vertices[co + 3] = 0.06; // radius

  // ── uniform block: REAL kernel scalars drive the whole palette ───────
  const uniforms = new Float32Array(UNIFORM_FLOATS);
  uniforms[0] = rho; // spectral radius (drives wave amplitude)
  uniforms[1] = gap; // spectral gap
  uniforms[2] = fiedler; // Fiedler / algebraic connectivity
  uniforms[3] = driftCode; // 0 Damped / 1 Resonant / 2 Unstable → palette
  uniforms[4] = nodeCount; // instance count for the courier test
  uniforms[5] = maxIntensity; // display-scale denom
  uniforms[6] = courierRemaining; // meters remaining (REAL)
  uniforms[7] = courierSeg; // current segment (REAL)
  // [8] frame is written by the render loop; [9..15] reserved / zero

  return {
    vertices,
    uniforms,
    vertexCount,
    floatsPerVertex: FLOATS_PER_VERTEX,
    layout: { fields: ['x', 'y', 'intensity', 'radius'] },
  };
}
