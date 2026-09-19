// The ambient layer, in two registers.
//
// AT REST: gold dust (/lib/dust.js). A page with nothing happening on it is a
// dark garden at night -- a few motes of the venue's accent drift and
// twinkle, and that is all. Adding a dish is a small rise of sparks from the
// button the finger left; checkout stills them to almost nothing.
//
// WHILE WAITING: the Sea (/lib/tide-sea.js), the developing ocean of the
// "Tide over Bedrock" direction. It exists only while there is an order to
// wait for, drawn in the tracking sheet's own window, and its phase is
// where that order is in its life: a placed order is a young scattered
// sea, a courier on the road is an aligned teal swell, Delivered is a full
// gold sea, a rejection one magenta glimmer. The Sea carries no text, ever:
// content, words, prices and decisions belong to the Sheet.

import { on, state } from '/store/state.js';

let dust = null;
let calm = false;
let ocean = null;

/// Where in its life an order is, as the sea's phase, 0..1.
export const PHASE = {
  PENDING: 0.12, CONFIRMED: 0.26, PREPARING: 0.42, READY: 0.58,
  IN_DELIVERY: 0.76, DELIVERED: 1.0, REJECTED: 0.12, CANCELLED: 0.12,
};
/// The sea's energy while an order is live, and after it has ended.
const OCEAN_ENERGY_LIVE = 0.3;
const OCEAN_ENERGY_DONE = 0.16;
/// The dust: how much of it shows at rest and while checkout holds still.
const DUST_REST = 1.0;
const DUST_CALM = 0.15;
/// Sparks per added dish, from a card and from the sheet.
const SPARKS_MIN = 4;
const SPARKS_PER_PARTICLE = 0.25;
const SPARKS_MAX = 14;
/// How long a rejection's glimmer holds on the sea.
const ANOMALY_MS = 3200;
const FAILED = new Set(['REJECTED', 'CANCELLED']);

/// The dust, on the page's own canvas. Off must mean the module is never fetched.
export async function initSea({ colour: given, leaf: givenLeaf } = {}){
  if (!on('sea') || dust) return dust;
  try {
    const { createDust } = await import('/lib/dust.js');
    const d = createDust();
    const colour = given || state.loc?.stage?.warm || getComputedStyle(document.documentElement).getPropertyValue('--brand-primary').trim();
    if (!d.init(document.getElementById('sea'), { colour, leaf: givenLeaf || state.loc?.stage?.sage })) return null;
    d.setReducedMotion(matchMedia('(prefers-reduced-motion: reduce)').matches);
    dust = d;
  } catch { dust = null; }
  return dust;
}

/// One event, one small rise of sparks. `n` is the old particle budget.
export function seaEvent(kind, n, at){
  if (calm || !dust) return;
  const count = Math.max(SPARKS_MIN, Math.min(SPARKS_MAX, Math.round((n || 0) * SPARKS_PER_PARTICLE)));
  const x = at?.x ?? innerWidth / 2, y = at?.y ?? innerHeight * 0.8;
  try { dust.spark(x, y, count); } catch {}
}

/// A touch under a finger: sparks from that point.
export function seaTouch(clientX, clientY){
  if (calm || !dust) return;
  try { dust.spark(clientX, clientY, SPARKS_MIN); } catch {}
}

export function seaArrive(){ try { dust && dust.setEnergy(DUST_REST); } catch {} }

/// Checkout stillness: the dust fades to almost nothing until the sheet closes.
export function seaCalm(onoff){
  calm = !!onoff;
  const cv = document.getElementById('sea');
  if (cv) cv.classList.toggle('calm', calm);
  try { dust && dust.setEnergy(calm ? DUST_CALM : DUST_REST); } catch {}
}
export function seaRest(){ closeOcean(); seaArrive(); }

// ── the Sea, while waiting ──────────────────────────────────────────────────
/// Draw the ocean into a canvas the tracking sheet owns. One at a time; a
/// second call for another canvas closes the first.
export async function openOcean(canvas, status){
  if (!on('sea')) return null;
  if (ocean && ocean.canvas !== canvas) closeOcean();
  if (!ocean) {
    try {
      const { createTideSea } = await import('/lib/tide-sea.js');
      const s = createTideSea();
      const st = state.loc?.stage || {};
      const gold = getComputedStyle(document.documentElement).getPropertyValue('--brand-primary').trim();
      if (!s.init(canvas, { gold, warm: st.warm, sage: st.sage })) return null;
      s.setReducedMotion(matchMedia('(prefers-reduced-motion: reduce)').matches);
      ocean = { sea: s, canvas, status: null };
    } catch { return null; }
  }
  oceanStatus(status);
  return ocean.sea;
}
/// The order moved: the sea eases to the new phase; a failure is the one beat.
export function oceanStatus(status){
  if (!ocean || !(status in PHASE)) return;
  if (ocean.status === status) return;
  ocean.status = status;
  try {
    ocean.sea.setPhase(PHASE[status]);
    ocean.sea.setEnergy(status === 'DELIVERED' || FAILED.has(status) ? OCEAN_ENERGY_DONE : OCEAN_ENERGY_LIVE);
    if (FAILED.has(status)) ocean.sea.accent('anomaly', ANOMALY_MS);
  } catch {}
}
export function closeOcean(){
  if (!ocean) return;
  try { ocean.sea.destroy(); } catch {}
  ocean = null;
}
/// The sea's phase for a status, for the card beside it.
export const phaseOf = status => PHASE[status] ?? 0;
