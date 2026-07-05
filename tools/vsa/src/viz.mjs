// VSA-VIZ — the token-arbitrage visual layer. Renders a logistics/system STATE into a compact,
// high-contrast semantic image a vision model reads "at a glance" for a ~fixed image-token cost,
// instead of linear text-token burn on dense JSON/vector arrays. SEMANTIC, not pixel-noise: the
// dictionary below (shape=kind, size=magnitude, color=urgency, line-thickness=weight) is what a
// vision model reads reliably. Pairs with the text LEGEND (cache it once in the system prompt).
//
// State shape (all fields optional; degrade gracefully):
//   { title, couriers:[{id,lat,lng,load,status}], orders:[{id,lat,lng,value,urgency}],
//     assignments:[{courier,order,weight}], vsaWeights:[{label,weight}] }

import { Canvas, C } from './raster.mjs';

const STATUS_COLOR = { free: C.green, busy: C.amber, offline: C.gray };
const URGENCY_COLOR = { ok: C.green, soon: C.amber, late: C.red, critical: C.red };

// The legend — passed ONCE to the model (prompt-cacheable). This is the decoder ring.
export const LEGEND = `This image is a live logistics-state snapshot (VSA-VIZ). Read it as a map, not pixels:
- SQUARES = couriers. Square SIZE = current load (bigger = more loaded). Color: green=free, amber=busy, gray=offline. Label = courier id + load%.
- CIRCLES = orders. Circle SIZE = order value (bigger = higher value). Color: green=on-plan, amber=due-soon, red=late/critical. Label = order id.
- LINES connect a courier to an assigned order; thicker line = higher priority/weight.
- Position is geographic (x=longitude, y=latitude); closer shapes are physically nearer.
- The bottom HEATMAP strip = VSA vector intensities (each labeled cell; brighter/redder = higher weight).
Answer using ONLY what the image shows.`;

function project(items, w, h, pad, bbox) {
  // lat/lng → canvas px; y is flipped (north = up). Falls back to a grid if no geo.
  const withGeo = items.filter((i) => Number.isFinite(i.lat) && Number.isFinite(i.lng));
  if (!withGeo.length) return null;
  const latMin = bbox?.latMin ?? Math.min(...withGeo.map((i) => i.lat));
  const latMax = bbox?.latMax ?? Math.max(...withGeo.map((i) => i.lat));
  const lngMin = bbox?.lngMin ?? Math.min(...withGeo.map((i) => i.lng));
  const lngMax = bbox?.lngMax ?? Math.max(...withGeo.map((i) => i.lng));
  const spanLat = latMax - latMin || 1;
  const spanLng = lngMax - lngMin || 1;
  return (lat, lng) => ({
    x: pad + ((lng - lngMin) / spanLng) * (w - 2 * pad),
    y: pad + (1 - (lat - latMin) / spanLat) * (h - 2 * pad),
  });
}

// ── Integration primitive ──────────────────────────────────────────────────────────────────
// Turn a state into a ready-to-send vision message. The LEGEND goes in `system` (prompt-cacheable
// → paid once); the PNG rides as a base64 image block. Works for both the Anthropic Messages API
// and OpenRouter's OpenAI-compatible shape (pass style: 'openai'). The orchestrator calls this
// right before a dispatch decision so the model reads the whole background "at a glance".
export function visionMessage(state, { style = 'anthropic', userText = 'Read the current state and answer the query.' } = {}) {
  const { png } = renderState(state);
  const b64 = png.toString('base64');
  if (style === 'openai') {
    return {
      system: LEGEND,
      messages: [{ role: 'user', content: [
        { type: 'text', text: userText },
        { type: 'image_url', image_url: { url: `data:image/png;base64,${b64}` } },
      ] }],
    };
  }
  return {
    system: LEGEND,
    messages: [{ role: 'user', content: [
      { type: 'text', text: userText },
      { type: 'image', source: { type: 'base64', media_type: 'image/png', data: b64 } },
    ] }],
  };
}

export function renderState(state, opts = {}) {
  const W = opts.width ?? 1024;
  const H = opts.height ?? 1024;
  const cv = new Canvas(W, H, C.bg);
  const couriers = state.couriers ?? [];
  const orders = state.orders ?? [];
  const assignments = state.assignments ?? [];
  const vsa = state.vsaWeights ?? [];

  const mapTop = 92;
  const mapBottom = vsa.length ? H - 140 : H - 60;
  const pad = 70;

  // Title bar
  cv.fillRect(0, 0, W, 76, C.ink);
  cv.text(24, 20, (state.title ?? 'DISPATCH STATE').slice(0, 26), C.white, 4);
  cv.text(24, 52, `${couriers.length} COURIERS  ${orders.length} ORDERS  ${assignments.length} LINKS`, [200, 208, 220, 255], 2);

  // Geographic projection over both couriers+orders so scales match.
  const all = [...couriers, ...orders];
  const projFn = project(all, W, mapBottom - mapTop, pad, state.bbox);
  const gridPos = (idx, n, band) => ({
    x: pad + ((idx + 0.5) / Math.max(1, n)) * (W - 2 * pad),
    y: mapTop + band,
  });
  const place = (item, idx, n, band) => {
    if (projFn && Number.isFinite(item.lat) && Number.isFinite(item.lng)) {
      const p = projFn(item.lat, item.lng);
      return { x: p.x, y: mapTop + p.y * ((mapBottom - mapTop) / (mapBottom - mapTop)) };
    }
    return gridPos(idx, n, band);
  };
  // Precompute positions keyed by id
  const cPos = new Map();
  couriers.forEach((c, i) => cPos.set(c.id, place(c, i, couriers.length, 60)));
  const oPos = new Map();
  orders.forEach((o, i) => oPos.set(o.id, place(o, i, orders.length, mapBottom - mapTop - 120)));

  // De-collision: nearby geo points (e.g. a courier at its assigned order) would overlap and
  // clip labels / hide links — a vision model then mis-reads. Relax overlapping node pairs apart
  // a few iterations (radius ≈ drawn size). Keeps geography approximately, restores legibility.
  {
    const nodes = [
      ...couriers.map((c) => ({ p: cPos.get(c.id), r: 26 + (c.load ?? 0) * 42 })),
      ...orders.map((o) => ({ p: oPos.get(o.id), r: 24 + ((o.value ?? 1) / Math.max(1, ...orders.map((x) => x.value ?? 1))) * 30 })),
    ].filter((n) => n.p);
    for (let iter = 0; iter < 24; iter++) {
      for (let i = 0; i < nodes.length; i++) {
        for (let j = i + 1; j < nodes.length; j++) {
          const a = nodes[i].p;
          const b = nodes[j].p;
          const dx = b.x - a.x;
          const dy = b.y - a.y;
          const dist = Math.hypot(dx, dy) || 0.01;
          const min = nodes[i].r + nodes[j].r + 26; // + label gutter
          if (dist < min) {
            const push = (min - dist) / 2;
            const ux = dx / dist;
            const uy = dy / dist;
            a.x -= ux * push;
            a.y -= uy * push;
            b.x += ux * push;
            b.y += uy * push;
          }
        }
      }
    }
    // clamp back inside the map band
    for (const n of nodes) {
      n.p.x = Math.max(pad, Math.min(W - pad, n.p.x));
      n.p.y = Math.max(mapTop + 30, Math.min(mapBottom, n.p.y));
    }
  }

  // Assignment lines FIRST (under the nodes)
  for (const a of assignments) {
    const cp = cPos.get(a.courier);
    const op = oPos.get(a.order);
    if (!cp || !op) continue;
    const th = 2 + Math.round((a.weight ?? 0.4) * 10);
    cv.line(cp.x, cp.y, op.x, op.y, C.line, th);
  }

  // Orders (circles): size ∝ value, color ∝ urgency
  const maxVal = Math.max(1, ...orders.map((o) => o.value ?? 1));
  for (const o of orders) {
    const p = oPos.get(o.id);
    const r = 12 + Math.round(((o.value ?? 1) / maxVal) * 30);
    cv.circle(Math.round(p.x), Math.round(p.y), r, URGENCY_COLOR[o.urgency] ?? C.blue, C.white);
    cv.text(Math.round(p.x) - 14, Math.round(p.y) + r + 4, String(o.id).slice(0, 5), C.ink, 2);
  }

  // Couriers (squares): size ∝ load, color ∝ status
  for (const c of couriers) {
    const p = cPos.get(c.id);
    const s = 26 + Math.round((c.load ?? 0) * 42);
    cv.square(Math.round(p.x), Math.round(p.y), s, STATUS_COLOR[c.status] ?? C.blue, C.ink);
    cv.text(Math.round(p.x) - 20, Math.round(p.y) - s / 2 - 20, `${String(c.id).slice(0, 4)}:${Math.round((c.load ?? 0) * 100)}%`, C.ink, 2);
  }

  // VSA heatmap strip (bottom): each labeled cell colored by weight (green→amber→red)
  if (vsa.length) {
    const y = H - 120;
    cv.text(24, y - 26, 'VSA WEIGHTS', C.muted, 2);
    const cw = Math.min(120, Math.floor((W - 48) / vsa.length));
    vsa.forEach((v, i) => {
      const t = Math.max(0, Math.min(1, v.weight ?? 0));
      const col = t < 0.5 ? [Math.round(34 + t * 2 * 183), 163, 74, 255] : [220, Math.round(163 - (t - 0.5) * 2 * 125), 74 - Math.round((t - 0.5) * 2 * 36), 255];
      cv.fillRect(24 + i * cw, y, cw - 6, 70, col);
      cv.text(28 + i * cw, y + 26, String(v.label ?? i).slice(0, 6), C.white, 2);
      cv.text(28 + i * cw, y + 46, `${Math.round(t * 100)}`, C.white, 2);
    });
  }

  // Drawn legend strip (bottom edge) — redundant with the text legend, helps the model self-anchor
  const ly = H - 40;
  cv.square(38, ly, 22, C.green, C.ink);
  cv.text(58, ly - 8, 'COURIER', C.ink, 2);
  cv.circle(220, ly, 12, C.red, C.white);
  cv.text(240, ly - 8, 'ORDER', C.ink, 2);
  cv.line(360, ly, 420, ly, C.line, 8);
  cv.text(430, ly - 8, 'ASSIGNED', C.ink, 2);

  return { png: cv.toPNG(), legend: LEGEND, meta: { w: W, h: H, couriers: couriers.length, orders: orders.length, assignments: assignments.length } };
}
