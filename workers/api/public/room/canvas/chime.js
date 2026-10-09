// THE NEW-TICKET CHIME (Wave CV row CV8a): pure, node-tested (chime.test.mjs). ASCII QUOTES ONLY (rule 11).
// Three rules, all tested: NO AudioContext is created before the first pointer or key event (the
// browser's autoplay rule, and a kitchen phone that wakes in a pocket must stay silent); the chime is
// ON by default and the choice is one per-device key (`dw_room_sound`, '0' = off); on Android a new
// ticket also vibrates 12 ms, with the same switch and the same gesture rule.

export const KEY = 'dw_room_sound';
// The kitchen's NEW column (src/board/model.rs `Status::column`): PENDING and CONFIRMED.
export const NEW_COLUMN = ['PENDING', 'CONFIRMED'];

// The ids of the NEW column in `orders`, and how many of them are not in `known` (the last
// snapshot's set; null = the first one, which rings for nothing).
export function arrivals(known, orders) {
  const ids = new Set();
  for (const o of orders || []) if (o && o.id && NEW_COLUMN.includes(o.status)) ids.add(o.id);
  const fresh = known ? [...ids].filter(id => !known.has(id)).length : 0;
  return { ids, fresh };
}

export function makeChime({ AudioCtx = null, vibrate = () => {}, store = { get: () => null, set: () => {} } } = {}) {
  let on = true;
  try { on = store.get(KEY) !== '0'; } catch { on = true; }
  let armed = false;   // a pointer or key event has happened on this page
  let ctx = null;      // created at the first ring after that event, never before

  function sound() {
    if (!ctx) ctx = new AudioCtx();
    if (ctx.state === 'suspended') ctx.resume?.();
    const t = ctx.currentTime;
    for (const [freq, at] of [[880, 0], [1320, 0.16]]) {
      const osc = ctx.createOscillator(), gain = ctx.createGain();
      osc.type = 'sine';
      osc.frequency.value = freq;
      gain.gain.setValueAtTime(0.0001, t + at);
      gain.gain.exponentialRampToValueAtTime(0.25, t + at + 0.01);
      gain.gain.exponentialRampToValueAtTime(0.0001, t + at + 0.25);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(t + at);
      osc.stop(t + at + 0.3);
    }
  }

  return {
    // Wire to pointerdown and keydown (capture): the only thing that arms the chime.
    gesture() { armed = true; },
    get on() { return on; },
    get armed() { return armed; },
    // The sound button: flips the choice, remembers it on this device, and answers the new state.
    toggle() {
      on = !on;
      try { store.set(KEY, on ? '1' : '0'); } catch { /* storage refused: the choice lasts this page */ }
      return on;
    },
    // `n` new tickets arrived in this snapshot: one chime, one vibration.
    ring(n) {
      if (!on || !armed || !n || !AudioCtx) return;
      vibrate(12);
      sound();
    },
  };
}
