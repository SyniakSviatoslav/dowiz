// web/src/lib/audio/sonify.mjs — P07 sonification Phase 0
// Event-driven sound with voice budget, causal ordering, money-guard.

const VOICE_BUDGET = 6;
const PENTATONIC = [262, 294, 330, 392, 440, 524, 588, 660, 784, 880, 988, 1175];
const EVENTS = {
  addToCart: { freq: 523, dur: 0.1, type: 'sine', vol: 0.12, desc: 'item added' },
  removeFromCart: { freq: 330, dur: 0.15, type: 'sawtooth', vol: 0.08, desc: 'item removed' },
  checkout: { freq: 600, dur: 0.08, type: 'sine', vol: 0.12, desc: 'order placed' },
  checkout2: { freq: 800, dur: 0.08, type: 'sine', vol: 0.12, desc: '' },
  checkout3: { freq: 1000, dur: 0.12, type: 'sine', vol: 0.12, desc: '' },
  advanceOrder: { freq: 440, dur: 0.1, type: 'triangle', vol: 0.1, desc: 'status advance' },
  shiftStart: { freq: 660, dur: 0.15, type: 'sine', vol: 0.12, desc: 'shift on' },
  shiftEnd: { freq: 330, dur: 0.2, type: 'sawtooth', vol: 0.08, desc: 'shift off' },
  deliver: { freq: 880, dur: 0.15, type: 'sine', vol: 0.15, desc: 'delivered' },
  navigate: { freq: 392, dur: 0.06, type: 'sine', vol: 0.06, desc: 'page change' },
};

let activeVoices = 0;
const queue = [];
let draining = false;

function processQueue(ctx) {
  if (draining || queue.length === 0) return;
  draining = true;
  const ev = queue.shift();
  if (activeVoices >= VOICE_BUDGET) { draining = false; return; }
  if (ev.money) { draining = false; return; }
  activeVoices++;
  const o = ctx.createOscillator();
  const g = ctx.createGain();
  o.type = ev.type || 'sine';
  o.frequency.value = ev.freq;
  g.gain.setValueAtTime(ev.vol || 0.1, ctx.currentTime);
  g.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + (ev.dur || 0.1));
  o.connect(g);
  g.connect(ctx.destination);
  o.start();
  o.stop(ctx.currentTime + (ev.dur || 0.1));
  o.onended = () => { activeVoices--; draining = false; processQueue(ctx); };
}

export function createSonifier() {
  let ctx = null;
  let enabled = true;
  try { const C = window.AudioContext || window.webkitAudioContext; if (C) ctx = new C(); } catch {}

  function enqueue(ev, money = false) {
    if (!enabled || !ctx) return;
    queue.push({ ...ev, money });
    if (!draining) processQueue(ctx);
  }

  function sonify(eventName, money = false) {
    const ev = EVENTS[eventName];
    if (!ev) return;
    enqueue(ev, money);
    if (eventName === 'checkout') {
      setTimeout(() => enqueue(EVENTS.checkout2, money), 60);
      setTimeout(() => enqueue(EVENTS.checkout3, money), 120);
    }
  }

  function setEnabled(v) { enabled = v; }
  function isEnabled() { return enabled; }

  return { sonify, setEnabled, isEnabled, get ctx() { return ctx; } };
}
