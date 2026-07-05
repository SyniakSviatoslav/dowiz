// VSA-VIZ FRACTAL — hierarchical 3-level encoding that speaks Vision-Transformer's native language
// (attention resolves patches→local→global, so a nested container image maps onto how the model
// already sees). Three levels, each drawn at RELATIVE offsets (the rasterizer analogue of nested
// SVG <g transform="translate">): near-zero compute, ~fixed image-token cost.
//
//   L1 MACRO  = zones/hubs      → big thick-outlined boxes; RED outline = zone alert (driver deficit)
//   L2 MESO   = vehicles        → shape = type: TRIANGLE=car, DIAMOND=van, placed inside its zone
//   L3 MICRO  = orders/VSA      → a 3×3 matrix inside the vehicle; filled cells = load, cell color
//                                 = deadline (green ok / amber soon / red late); empty = outlined slot
//
// State: { title, zones:[{ id, alert:bool, vehicles:[{ id, type:'car'|'van', orders:['ok'|'soon'|'late',…≤9] }] }] }

import { Canvas, C } from './raster.mjs';

const DEADLINE = { ok: C.green, soon: C.amber, late: C.red, critical: C.red };

export const FRACTAL_LEGEND = `You are an L5 logistics-orchestration core. The image is a FRACTAL state snapshot — read it hierarchically, outer→inner:
1. LARGE OUTLINED BOXES = geographic zones/hubs. A RED thick outline = that zone is in ALERT (driver deficit / overload). A gray outline = normal.
2. SHAPES inside a box = active vehicles in that zone. TRIANGLE = car, DIAMOND = van. Shape label = vehicle id.
3. The 3×3 COLOR MATRIX inside each shape = that vehicle's current orders/load. Filled cells = orders on board (count = load); EMPTY outlined cells = free capacity. Cell color = deadline: green=on-time, amber=due-soon, red=late/critical.
Task: find the pressure — a zone with a RED outline containing a vehicle whose matrix is mostly/all RED (critical overload) — and the slack — a vehicle with an EMPTY matrix (a spare) in a calm zone. Output JSON reallocation commands that minimize red micro-cells (move the nearest spare toward the alert zone). Answer using ONLY what the image shows.`;

function gridDims(n) {
  const cols = Math.ceil(Math.sqrt(n));
  const rows = Math.ceil(n / cols);
  return { cols, rows };
}

// Draw a vehicle (meso) + its order matrix (micro) at absolute centre (cx,cy). Pure relative offsets.
function drawVehicle(cv, cx, cy, v, glyph = 128) {
  const status = v.orders?.some((o) => o === 'late' || o === 'critical')
    ? C.red
    : v.orders?.length ? C.amber : C.gray;
  const border = { color: status, t: 6 };
  const lightFill = [244, 246, 248, 255];
  if (v.type === 'van') cv.diamond(cx, cy, glyph, lightFill, border);
  else cv.triangle(cx, cy, glyph, lightFill, border);

  // 3×3 micro-matrix, centred (triangle biased downward into its wide base).
  const cell = 24;
  const gap = 6;
  const side = 3 * cell + 2 * gap; // 84
  const mx = cx - side / 2;
  const my = cy - side / 2 + (v.type === 'van' ? 0 : 18);
  const colors = Array.from({ length: 9 }, (_, i) => {
    const o = v.orders?.[i];
    return o ? DEADLINE[o] ?? C.blue : null;
  });
  cv.matrix(mx, my, cell, gap, 3, colors);
  cv.text(cx - 24, cy + glyph / 2 - 6, `${v.id}`.slice(0, 6), C.ink, 2);
}

export function renderFractal(state, opts = {}) {
  const W = opts.width ?? 1024;
  const H = opts.height ?? 1024;
  const cv = new Canvas(W, H, C.bg);
  const zones = state.zones ?? [];

  // Title
  cv.fillRect(0, 0, W, 72, C.ink);
  cv.text(24, 18, (state.title ?? 'FRACTAL DISPATCH').slice(0, 26), C.white, 4);
  const vehCount = zones.reduce((s, z) => s + (z.vehicles?.length ?? 0), 0);
  const alertN = zones.filter((z) => z.alert).length;
  cv.text(24, 50, `${zones.length} ZONES  ${vehCount} VEHICLES  ${alertN} ALERT`, [200, 208, 220, 255], 2);

  // Macro grid
  const top = 84;
  const { cols, rows } = gridDims(zones.length);
  const zw = Math.floor((W - 24) / cols) - 16;
  const zh = Math.floor((H - top - 60) / rows) - 16;
  zones.forEach((z, i) => {
    const zx = 16 + (i % cols) * (zw + 16);
    const zy = top + Math.floor(i / cols) * (zh + 16);
    // Zone box: thick outline red on alert, muted otherwise; faint fill tint on alert.
    if (z.alert) cv.fillRect(zx, zy, zw, zh, [254, 242, 242, 255]);
    cv.rectOutline(zx, zy, zw, zh, z.alert ? C.red : [176, 184, 194, 255], z.alert ? 8 : 4);
    cv.text(zx + 14, zy + 12, `${z.id}${z.alert ? ' ALERT' : ''}`.slice(0, 18), z.alert ? C.red : C.muted, 3);

    // Meso: vehicles in a sub-grid inside the zone
    const vs = z.vehicles ?? [];
    const vg = gridDims(Math.max(1, vs.length));
    const cw = zw / vg.cols;
    const ch = (zh - 46) / vg.rows;
    vs.forEach((v, j) => {
      const vx = zx + (j % vg.cols) * cw + cw / 2;
      const vy = zy + 46 + Math.floor(j / vg.cols) * ch + ch / 2;
      drawVehicle(cv, Math.round(vx), Math.round(vy), v, Math.min(150, Math.round(Math.min(cw, ch) * 0.78)));
    });
  });

  // Drawn legend strip
  const ly = H - 34;
  cv.triangle(40, ly, 26, [244, 246, 248, 255], { color: C.ink, t: 3 });
  cv.text(58, ly - 8, 'CAR', C.ink, 2);
  cv.diamond(150, ly, 26, [244, 246, 248, 255], { color: C.ink, t: 3 });
  cv.text(170, ly - 8, 'VAN', C.ink, 2);
  cv.fillRect(270, ly - 10, 20, 20, C.red);
  cv.text(296, ly - 8, 'LATE', C.ink, 2);
  cv.fillRect(400, ly - 10, 20, 20, C.green);
  cv.text(426, ly - 8, 'OK', C.ink, 2);

  return { png: cv.toPNG(), legend: FRACTAL_LEGEND, meta: { w: W, h: H, zones: zones.length, vehicles: vehCount, alerts: alertN } };
}

export function visionMessageFractal(state, { style = 'anthropic', userText = 'Analyze the fractal and output reallocation commands.' } = {}) {
  const { png } = renderFractal(state);
  const b64 = png.toString('base64');
  const img = style === 'openai'
    ? { type: 'image_url', image_url: { url: `data:image/png;base64,${b64}` } }
    : { type: 'image', source: { type: 'base64', media_type: 'image/png', data: b64 } };
  return { system: FRACTAL_LEGEND, messages: [{ role: 'user', content: [{ type: 'text', text: userText }, img] }] };
}
