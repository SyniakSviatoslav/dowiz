// The Sea -- the dowiz ambient layer, as the developing ocean of the
// "Tide over Bedrock" direction (/lib/tide-sea.js).
//
// ONE PROCESS, NOT A SET OF EVENTS. The sea has a PHASE, 0 to 1, which is
// where the order is in its life: a resting page sits near zero, a placed
// order begins to build, a courier on the road is a strong aligned swell in
// teal, and Delivered is a full gold sea. Every status is a target the field
// eases toward; nothing jumps. Adding a dish is a touch -- a ring that
// spreads and fades -- and a rejection is the one magenta glimmer.
//
// THE DRAMATURGY IS RESTRAINT (BLUEPRINTS-DOWIZ-INTERFACES §C). Arrival is a
// calm, brand-graded field. Browsing gets almost nothing. Adding to the cart
// is one small ring. CHECKOUT IS THE STILLEST MOMENT ON THE PAGE -- the field
// drops to near-nothing so all attention sits on the numbers. Tracking is the
// centrepiece. The Sea carries no text, ever: content, words, prices and
// decisions belong to the Sheet.

import { on } from '/store/state.js';

let sea = null;
let calm = false;
let tideStatus = null;

/// Where in its life an order is, as the sea's phase. Monotonic toward the
/// climax and never past it; the failures fall back to rest after the beat.
const PHASE = {
  REST: 0.14, PENDING: 0.2, CONFIRMED: 0.24, PREPARING: 0.40, READY: 0.55,
  IN_DELIVERY: 0.74, DELIVERED: 1.0, REJECTED: 0.14, CANCELLED: 0.14,
};
/// The energy at rest, while tracking, and while checkout holds still.
const ENERGY_REST = 0.22;
const ENERGY_TRACKING = 0.28;
const ENERGY_CALM = 0.04;
/// A burst's particle count (the old vocabulary) becomes rings: one ring per
/// this many particles, at most a handful, strength rising with the count.
const PARTICLES_PER_RING = 40;
const RINGS_MAX = 5;
const RING_STRENGTH_MIN = 0.25;
const RING_STRENGTH_PER_PARTICLE = 0.002;
/// How long the failure glimmer holds.
const ANOMALY_MS = 3200;
const FAILED = new Set(['dispatch_failed']);

export async function initSea(){
  // A venue whose customers are on older phones can turn the Sea off, and off
  // must mean the module is never fetched.
  if (!on('sea') || sea) return sea;
  try {
    const { createTideSea } = await import('/lib/tide-sea.js');
    const s = createTideSea();
    if (!s.init(document.getElementById('sea'))) return null;
    s.setReducedMotion(matchMedia('(prefers-reduced-motion: reduce)').matches);
    sea = s;
  } catch { sea = null; }
  return sea;
}

/// One event, one touch. `n` is the old particle budget and sets how many
/// rings and how strong; a failure is the magenta accent instead.
export function seaEvent(kind, n){
  if (calm || !sea) return;
  try {
    if (FAILED.has(kind)) return sea.accent('anomaly', ANOMALY_MS);
    const rings = Math.max(1, Math.min(RINGS_MAX, Math.round((n || 0) / PARTICLES_PER_RING)));
    const strength = RING_STRENGTH_MIN + (n || 0) * RING_STRENGTH_PER_PARTICLE;
    for (let i = 0; i < rings; i++) sea.ripple(undefined, undefined, strength);
  } catch {}
}

/// A touch under a finger, for the hero: the page is the sea's surface.
export function seaTouch(clientX, clientY, strength){
  if (calm || !sea) return;
  try { const p = sea.fieldPoint(clientX, clientY); sea.ripple(p.x, p.y, strength); } catch {}
}

/// The arrival field: a low ember drift, so the page breathes before anything
/// has happened. Not a loop for its own sake -- it is the Sea's resting state.
export function seaArrive(){
  setTide(null);
}

/// The tracking field, held for as long as the order is in this status.
export function seaForOrder(status){
  if (!(status in PHASE)) return;
  if (tideStatus !== status) {
    if (status === 'REJECTED' || status === 'CANCELLED') seaEvent('dispatch_failed', 0);
    setTide(status);
  }
}

function setTide(status){
  tideStatus = status;
  try {
    sea && sea.setPhase(PHASE[status] ?? PHASE.REST);
    sea && sea.setEnergy(status && status !== 'DELIVERED' ? ENERGY_TRACKING : ENERGY_REST);
  } catch {}
  const cv = document.getElementById('sea');
  if (cv) cv.dataset.tide = status || '';
}

/// Checkout stillness. The field fades to near nothing and no touch lands
/// until the sheet closes; the outcome of the order is the next thing the Sea
/// says, and it says it from the tracking sheet.
export function seaCalm(onoff){
  calm = !!onoff;
  const cv = document.getElementById('sea');
  if (cv) cv.classList.toggle('calm', calm);
  try { sea && sea.setEnergy(calm ? ENERGY_CALM : (tideStatus ? ENERGY_TRACKING : ENERGY_REST)); } catch {}
}

/// Back to the resting field after tracking is dismissed.
export function seaRest(){
  setTide(null);
}
