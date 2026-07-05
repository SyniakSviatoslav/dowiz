#!/usr/bin/env node
// VSA-VIZ CLI — render a system state to a semantic PNG + report the token arbitrage.
//   node tools/vsa/viz-cli.mjs demo [out.png]          generate a demo dispatch + render
//   node tools/vsa/viz-cli.mjs render <state.json> [out.png]
// Prints: image-token estimate (near-fixed) vs the JSON-text token cost (linear in state size).

import fs from 'node:fs';
import { renderState, LEGEND } from './src/viz.mjs';
import { renderFractal, FRACTAL_LEGEND } from './src/viz-fractal.mjs';
import { countTokens } from './src/tokens.mjs';

// Claude vision cost ≈ (w*h)/750 tokens (docs), capped ~1600 for a 1092px tile.
const imageTokens = (w, h) => Math.min(1600, Math.ceil((w * h) / 750));

// A realistic Tirana dispatch snapshot with KNOWN ground truth (for the verification test).
function demoState() {
  return {
    title: 'TIRANA DISPATCH',
    bbox: { latMin: 41.31, latMax: 41.35, lngMin: 19.79, lngMax: 19.84 },
    couriers: [
      { id: 'C1', lat: 41.327, lng: 19.818, load: 0.9, status: 'busy' },
      { id: 'C2', lat: 41.34, lng: 19.802, load: 0.2, status: 'free' },
      { id: 'C3', lat: 41.318, lng: 19.83, load: 0.0, status: 'offline' },
      { id: 'C4', lat: 41.345, lng: 19.835, load: 0.55, status: 'busy' },
    ],
    orders: [
      { id: 'O10', lat: 41.33, lng: 19.82, value: 2400, urgency: 'late' },
      { id: 'O11', lat: 41.322, lng: 19.808, value: 900, urgency: 'ok' },
      { id: 'O12', lat: 41.338, lng: 19.828, value: 1500, urgency: 'soon' },
      { id: 'O13', lat: 41.315, lng: 19.815, value: 600, urgency: 'ok' },
      { id: 'O14', lat: 41.348, lng: 19.8, value: 3100, urgency: 'late' },
    ],
    assignments: [
      { courier: 'C1', order: 'O10', weight: 0.95 },
      { courier: 'C4', order: 'O12', weight: 0.5 },
      { courier: 'C1', order: 'O14', weight: 0.8 },
    ],
    vsaWeights: [
      { label: 'DEMAND', weight: 0.82 },
      { label: 'SUPPLY', weight: 0.35 },
      { label: 'LATEUR', weight: 0.71 },
      { label: 'CASH', weight: 0.44 },
      { label: 'ETAVAR', weight: 0.6 },
    ],
  };
}

// Fractal demo — the operator's scenario: SE hub in ALERT with an all-red overloaded VAN, a spare
// EMPTY CAR sitting in the calm SW hub. Ground truth for the read-back test + the reallocation call.
function fractalState() {
  return {
    title: 'FRACTAL DISPATCH',
    zones: [
      { id: 'HUB-NW', alert: false, vehicles: [{ id: 'T1', type: 'car', orders: ['ok', 'ok'] }] },
      { id: 'HUB-NE', alert: false, vehicles: [{ id: 'V1', type: 'van', orders: ['ok', 'soon', 'ok', 'soon'] }] },
      { id: 'HUB-SW', alert: false, vehicles: [{ id: 'T2', type: 'car', orders: [] }] },
      { id: 'HUB-SE', alert: true, vehicles: [{ id: 'V2', type: 'van', orders: ['late', 'late', 'late', 'late', 'late', 'late', 'late', 'late', 'late'] }] },
    ],
  };
}

const [cmd, arg1, arg2] = process.argv.slice(2);
let state, out, renderer = renderState, legendText = LEGEND;
if (cmd === 'demo') {
  state = demoState();
  out = arg1 ?? 'tools/vsa/bench/vsa-viz-demo.png';
} else if (cmd === 'fractal') {
  state = fractalState();
  out = arg1 ?? 'tools/vsa/bench/vsa-viz-fractal.png';
  renderer = renderFractal;
  legendText = FRACTAL_LEGEND;
} else if (cmd === 'render') {
  state = JSON.parse(fs.readFileSync(arg1, 'utf8'));
  out = arg2 ?? arg1.replace(/\.json$/, '.png');
} else {
  console.error('usage: viz-cli.mjs demo|render [args]');
  process.exit(1);
}

const t0 = process.hrtime.bigint();
const { png, meta } = renderer(state);
const ms = Number(process.hrtime.bigint() - t0) / 1e6;
fs.writeFileSync(out, png);

const jsonText = JSON.stringify(state);
const jsonTok = await countTokens(jsonText);
const imgTok = imageTokens(meta.w, meta.h);
const legendTok = await countTokens(legendText);

console.log(JSON.stringify({
  wrote: out,
  bytes: png.length,
  render_ms: Math.round(ms * 10) / 10,
  dims: `${meta.w}x${meta.h}`,
  entities: meta,
  tokens: {
    json_text: jsonTok,
    image_estimate: imgTok,
    legend_once_cached: legendTok,
    saved_vs_json: jsonTok - imgTok,
    reduction_pct: Math.round((1 - imgTok / jsonTok) * 1000) / 10,
  },
  note: 'image cost is ~FIXED regardless of entity count; JSON grows linearly — arbitrage widens with scale',
}, null, 2));
