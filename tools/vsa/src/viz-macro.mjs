// VSA-VIZ MACRO tier — the low-resolution / minimum-token layer (operator directive 2026-07-05:
// drive 1600→~300 tok). Three levers:
//   1. RESOLUTION HACK — render small (256²/384²). Claude tokenizes an image at ~(w·h)/750, so
//      256² ≈ 87 tok, 384² ≈ 197 tok, vs 1024²'s 1399. The honest catch: that same formula is the
//      INFORMATION ceiling — ~87 perceptual tokens can't carry 450 distinct micro-states, so the
//      macro tier drops the per-order 3×3 matrix and encodes only the DECISION signal (zone heat +
//      per-driver status + spares), which stays legible when shrunk.
//   2. MINIFIED LEGEND — telegraphic, machine-readable (LEGEND_MIN, ~40 tok vs ~235 prose).
//   3. DELTA FRAMES — when <10% of drivers changed, send a tiny patch strip of only the changed
//      drivers (renderDelta); the model holds the global frame from prompt cache.

import { Canvas, C } from './raster.mjs';

const URG = { ok: C.green, soon: C.amber, late: C.red, critical: C.red };
const URG_RANK = { ok: 0, soon: 1, late: 2, critical: 2 };

// Machine-readable legend — dense syntax an LLM parses fine. Cache once in `system`.
export const LEGEND_MIN =
  'IMG=macro dispatch grid. Cell=zone(hub). Cell border RED=zone ALERT(driver deficit). ' +
  'Cell bg tint=zone heat(green→red = share of late orders). Inside each cell: a row of driver ' +
  'squares — color=driver worst order [grn=ok, amb=soon, red=late], HOLLOW/gray square=empty ' +
  'driver(spare). Task: find RED-bordered zone(s) with red driver-squares, move nearest HOLLOW ' +
  '(spare) toward it. Reply JSON reallocations. Read only what is shown.';

// Aggregate a driver's order list → its display status + load. Color by the SHARE of late orders
// (not the single worst — that saturates to red the moment any order slips), so drivers spread
// across green/amber/red and the truly-critical ones (mostly-late) stand out.
function driverStatus(v) {
  if (!v.orders || v.orders.length === 0) return { spare: true, color: C.gray, load: 0 };
  const late = v.orders.filter((o) => URG_RANK[o] >= 2).length / v.orders.length;
  const color = late >= 0.5 ? C.red : late > 0 ? C.amber : C.green;
  return { spare: false, color, load: v.orders.length };
}

// Zone heat = share of late orders across its drivers → green..red.
function zoneHeat(z) {
  const orders = (z.vehicles ?? []).flatMap((v) => v.orders ?? []);
  if (!orders.length) return [246, 248, 250, 255];
  const late = orders.filter((o) => URG_RANK[o] >= 2).length / orders.length;
  // interpolate very-light-green → very-light-red
  const r = Math.round(238 + late * 16);
  const g = Math.round(248 - late * 40);
  const b = Math.round(240 - late * 40);
  return [r, g, b, 255];
}

/**
 * Low-res macro render. Each zone = a grid cell; drivers = a strip of status squares inside it.
 * Deliberately NO per-order matrix, NO text at tiny sizes — those don't survive the shrink.
 */
export function renderMacro(state, opts = {}) {
  const W = opts.width ?? 384;
  const H = opts.height ?? 384;
  const label = opts.label ?? W >= 384; // labels only when there's room
  const cv = new Canvas(W, H, C.bg);
  const zones = state.zones ?? [];
  const cols = Math.ceil(Math.sqrt(zones.length));
  const rows = Math.ceil(zones.length / cols);
  const pad = Math.max(3, Math.round(W / 128));
  const zw = Math.floor(W / cols);
  const zh = Math.floor(H / rows);

  zones.forEach((z, i) => {
    const zx = (i % cols) * zw;
    const zy = Math.floor(i / cols) * zh;
    // zone cell: heat fill + alert border
    cv.fillRect(zx + pad, zy + pad, zw - 2 * pad, zh - 2 * pad, zoneHeat(z));
    const bt = z.alert ? Math.max(3, Math.round(W / 100)) : Math.max(1, Math.round(W / 340));
    cv.rectOutline(zx + pad, zy + pad, zw - 2 * pad, zh - 2 * pad, z.alert ? C.red : C.gray, bt);
    if (label && W >= 384) cv.text(zx + pad + 5, zy + pad + 4, String(z.id).slice(0, 6), z.alert ? C.red : C.muted, 2);

    // driver squares in a grid inside the zone
    const vs = z.vehicles ?? [];
    const inX = zx + pad + 6;
    const inY = zy + pad + (label && W >= 384 ? 22 : 6);
    const availW = zw - 2 * pad - 12;
    const availH = zh - 2 * pad - (label && W >= 384 ? 28 : 12);
    const dcols = Math.max(1, Math.ceil(Math.sqrt((vs.length * availW) / Math.max(1, availH))));
    const drows = Math.ceil(vs.length / dcols);
    const sq = Math.max(4, Math.min(Math.floor(availW / dcols) - 2, Math.floor(availH / drows) - 2));
    vs.forEach((v, j) => {
      const dx = inX + (j % dcols) * (sq + 2);
      const dy = inY + Math.floor(j / dcols) * (sq + 2);
      const st = driverStatus(v);
      if (st.spare) {
        cv.fillRect(dx, dy, sq, sq, [252, 252, 253, 255]);
        cv.rectOutline(dx, dy, sq, sq, C.gray, Math.max(1, Math.round(sq / 8)));
      } else {
        cv.fillRect(dx, dy, sq, sq, st.color);
      }
    });
  });
  return { png: cv.toPNG(), legend: LEGEND_MIN, meta: { w: W, h: H, zones: zones.length, vehicles: zones.reduce((s, z) => s + (z.vehicles?.length ?? 0), 0) } };
}

// ── Delta frames ────────────────────────────────────────────────────────────────────────────
// Diff two states → the drivers whose display-status changed. If few changed, send a strip.

export function diffDrivers(prev, next) {
  const key = (v) => {
    const st = driverStatus(v);
    return `${st.spare ? 'S' : ''}${st.color.join('.')}:${st.load}`;
  };
  const prevMap = new Map();
  for (const z of prev.zones ?? []) for (const v of z.vehicles ?? []) prevMap.set(v.id, key(v));
  const changed = [];
  for (const z of next.zones ?? []) {
    for (const v of z.vehicles ?? []) {
      if (prevMap.get(v.id) !== key(v)) changed.push({ zone: z.id, v });
    }
  }
  const total = (next.zones ?? []).reduce((s, z) => s + (z.vehicles?.length ?? 0), 0);
  return { changed, total, fraction: total ? changed.length / total : 0 };
}

/** Integration primitive: a ready-to-send macro vision message (minified legend → cached system). */
export function visionMessageMacro(state, { style = 'anthropic', width = 256, userText = 'Read the macro grid and output reallocations.' } = {}) {
  const { png } = renderMacro(state, { width, height: width });
  const b64 = png.toString('base64');
  const img = style === 'openai'
    ? { type: 'image_url', image_url: { url: `data:image/png;base64,${b64}` } }
    : { type: 'image', source: { type: 'base64', media_type: 'image/png', data: b64 } };
  return { system: LEGEND_MIN, messages: [{ role: 'user', content: [{ type: 'text', text: userText }, img] }] };
}

/** A tiny patch strip of ONLY the changed drivers (id label + status square). ~64px tall. */
export function renderDelta(changed, opts = {}) {
  const n = changed.length;
  const cols = Math.min(n, opts.cols ?? 8);
  const rows = Math.ceil(n / cols);
  const cell = 40;
  const W = opts.width ?? Math.max(cols * cell + 8, 64);
  const H = rows * cell + 8;
  const cv = new Canvas(W, H, C.bg);
  changed.forEach(({ v }, i) => {
    const x = 4 + (i % cols) * cell;
    const y = 4 + Math.floor(i / cols) * cell;
    const st = driverStatus(v);
    if (st.spare) {
      cv.fillRect(x, y, 24, 24, [252, 252, 253, 255]);
      cv.rectOutline(x, y, 24, 24, C.gray, 3);
    } else cv.fillRect(x, y, 24, 24, st.color);
    cv.text(x, y + 26, String(v.id).slice(0, 5), C.ink, 1);
  });
  return { png: cv.toPNG(), legend: 'IMG=DELTA patch: only drivers whose status changed since the cached frame. Same square colors as the macro legend; apply these updates to your held state.', meta: { w: W, h: H, changed: n } };
}
