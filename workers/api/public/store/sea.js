// The Sea -- the dowiz ambient layer, wired rather than rewritten.
//
// /lib/particle-cloud.js is a WebGL2 field whose vocabulary already maps order
// events to physics: order_created → amber burst, courier_assigned → teal
// stream, delivered → gold bloom, dispatch_failed → blood turbulence,
// pending_aging → slow ember drift. This module decides WHEN the Sea moves and
// how much, following the design language's assignment rule: ambient,
// transition, tracking and feedback belong to the Sea; content, words, prices
// and decisions belong to the Sheet. The Sea carries no text, ever.
//
// THE DRAMATURGY IS RESTRAINT (BLUEPRINTS-DOWIZ-INTERFACES §C). Arrival is a
// calm, brand-graded field. Browsing gets almost nothing. Adding to the cart is
// one small pulse. CHECKOUT IS THE STILLEST MOMENT ON THE PAGE -- the field
// drops to near-nothing so all attention sits on the numbers. Tracking is the
// centrepiece: the tide holds the order's colour and grows with its status,
// terracotta to gold, and Delivered is the one loud beat, earned by the quiet
// before it. Nothing here loops for decoration.

import { on } from '/store/state.js';

let sea = null;
let calm = false;
let tideStatus = null;

const SEA_FOR_STATUS = {
  PENDING: 'pending_aging', CONFIRMED: 'order_created', PREPARING: 'order_created',
  READY: 'courier_assigned', IN_DELIVERY: 'courier_assigned',
  DELIVERED: 'delivered', REJECTED: 'dispatch_failed', CANCELLED: 'dispatch_failed',
};
// Particles per second for the tide, per status: the amplitude budget rises
// monotonically toward the climax and never past it.
const TIDE_RATE = {
  PENDING: 3, CONFIRMED: 4, PREPARING: 5, READY: 6, IN_DELIVERY: 9,
  DELIVERED: 0, REJECTED: 0, CANCELLED: 0,
};

export async function initSea(){
  // A venue whose customers are on older phones can turn the Sea off, and off
  // must mean the module is never fetched.
  if (!on('sea') || sea) return sea;
  try {
    const { createParticleCloud } = await import('/lib/particle-cloud.js');
    sea = createParticleCloud();
    sea.init(document.getElementById('sea'));
    sea.setReducedMotion(matchMedia('(prefers-reduced-motion: reduce)').matches);
  } catch { sea = null; }
  return sea;
}

/// One event, one impulse.
export function seaEvent(kind, n){
  if (calm) return;
  try { sea && sea.burst(kind, n); } catch {}
}

/// The arrival field: a low ember drift, so the page breathes before anything
/// has happened. Not a loop for its own sake -- it is the Sea's resting state.
export function seaArrive(){
  setTide('PENDING', 2);
}

/// The tracking field, held for as long as the order is in this status.
export function seaForOrder(status){
  const kind = SEA_FOR_STATUS[status];
  if (!kind) return;
  if (tideStatus !== status) {
    // The status advance is a beat: one impulse, then the tide settles into
    // the new register. Delivered is the single maximum of the successful
    // branch; a rejection is loud and honest and then goes quiet.
    seaEvent(kind, status === 'DELIVERED' ? 220 : status === 'REJECTED' || status === 'CANCELLED' ? 140 : 70);
    setTide(status, TIDE_RATE[status]);
  }
}

function setTide(status, rate){
  tideStatus = status;
  const kind = SEA_FOR_STATUS[status];
  try { sea && sea.tide(rate > 0 ? kind : null, rate); } catch {}
  const cv = document.getElementById('sea');
  if (cv) cv.dataset.tide = status || '';
}

/// Checkout stillness. The canvas fades to near nothing and no impulse lands
/// until the sheet closes; the outcome of the order is the next thing the Sea
/// says, and it says it from the tracking sheet.
export function seaCalm(onoff){
  calm = !!onoff;
  const cv = document.getElementById('sea');
  if (cv) cv.classList.toggle('calm', calm);
  try { sea && sea.tide(calm ? null : SEA_FOR_STATUS[tideStatus], calm ? 0 : TIDE_RATE[tideStatus] ?? 2); } catch {}
}

/// Back to the resting field after tracking is dismissed.
export function seaRest(){
  tideStatus = null;
  seaArrive();
}
