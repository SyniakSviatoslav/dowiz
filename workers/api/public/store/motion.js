// Motion the way a game does it: things MOVE, nothing is cut.
//
// MORPH -- a sort or a filter. The cards on screen are measured, the change
// is applied to the DOM once, and then three things happen at the same time:
// a card that left scatters -- it lifts, turns a little, drifts off in its
// own direction and dissolves like ink in water; a card that stayed but moved
// slides from where it was to where it is now, lifted, as if a hand dealt it
// across the table; a card that arrived is dealt in from just below, turned a
// degree, one beat after the one before it. The Web Animations API drives
// all of it, so the DOM is settled at once and the screen catches up.
//
// SWEEP -- a change of language or currency. A bank of fog rolls across the
// screen, left to right, with a hairline of the accent on its leading edge;
// the words and the numbers are replaced while the fog is over them, and each
// node that changed gets a wisp of its own -- the old value blurs and drifts,
// the new one settles -- so what emerges from the fog is the new page, and
// the eye saw the weather, not a cut.
//
// Money is never counted up or tweened: it is replaced under the fog, whole.
// With reduced motion every one of these is the plain change.

import { $$, reduced } from '/store/ui.js';

/// The morph's beats.
const LEAVE_MS = 340;
const MOVE_MS = 460;
const ENTER_MS = 380;
const ENTER_STAGGER_MS = 28;
const ENTER_STAGGER_CAP_MS = 420;
/// A leaving card drifts this far and turns this much, at most.
const SCATTER_PX = 90;
const SCATTER_DEG = 9;
/// A dealt card comes from this far below, turned this much.
const DEAL_PX = 28;
const DEAL_DEG = -2.5;
/// Only cards near the screen are animated; the rest change under the fold.
const NEAR_SCREEN_PX = 400;
/// The sweep's beats: the fog crosses in this long and the values swap when
/// its thick middle is over the screen's middle.
const SWEEP_MS = 980;
const SWEEP_SWAP_AT = 0.46;
const WISP_MS = 520;
const WISP_PX = 10;
const EASE_TIDE = 'cubic-bezier(.37,0,.63,1)';
const EASE_SNAP = 'cubic-bezier(.32,.72,0,1)';

const near = r => r.bottom > -NEAR_SCREEN_PX && r.top < innerHeight + NEAR_SCREEN_PX;

/// Apply `change` to the cards, then animate what it did.
export function morph(change, { quick = false } = {}){
  if (reduced()) { change(); return Promise.resolve(); }
  const cards = $$('.card');
  const before = new Map();
  for (const el of cards) if (!el.hidden) { const r = el.getBoundingClientRect(); if (near(r)) before.set(el, r); }
  change();
  const scale = quick ? 0.55 : 1;
  const done = [];
  let dealt = 0;
  for (const el of cards) {
    const was = before.get(el);
    if (el.hidden) {
      if (!was) continue;
      // Scatter: shown again out of flow, exactly where it was, and dissolved.
      const host = el.parentElement;
      const hr = host.getBoundingClientRect();
      el.hidden = false;
      el.classList.add('leaving');
      el.style.position = 'absolute';
      el.style.left = `${was.left - hr.left}px`; el.style.top = `${was.top - hr.top}px`;
      el.style.width = `${was.width}px`;
      const dir = (Math.random() * 2 - 1);
      const a = el.animate([
        { transform: 'translate(0,0) rotate(0) scale(1)', opacity: 1, filter: 'blur(0)' },
        { transform: `translate(${dir * SCATTER_PX}px, ${SCATTER_PX * 0.4}px) rotate(${dir * SCATTER_DEG}deg) scale(.92)`, opacity: 0, filter: 'blur(6px)' },
      ], { duration: LEAVE_MS * scale, easing: EASE_TIDE, fill: 'forwards' });
      done.push(a.finished.catch(() => {}).then(() => {
        el.classList.remove('leaving'); el.style.position = el.style.left = el.style.top = el.style.width = '';
        a.cancel(); el.hidden = true;
      }));
      continue;
    }
    const now = el.getBoundingClientRect();
    if (!near(now)) continue;
    if (was) {
      const dx = was.left - now.left, dy = was.top - now.top;
      if (Math.abs(dx) < 1 && Math.abs(dy) < 1) continue;
      // Dealt across the table: from the old place, lifted, to the new.
      const turn = Math.max(-4, Math.min(4, dx / 60));
      done.push(el.animate([
        { transform: `translate(${dx}px, ${dy}px) rotate(${turn}deg) scale(1.02)`, boxShadow: '0 18px 40px -12px rgba(0,0,0,.6)', zIndex: 3 },
        { transform: 'translate(0,0) rotate(0) scale(1)', boxShadow: '0 1px 2px rgba(0,0,0,.06)', zIndex: 3 },
      ], { duration: MOVE_MS * scale, easing: EASE_TIDE }).finished.catch(() => {}));
    } else {
      // Dealt in from below, one beat after the last.
      const delay = Math.min(dealt++ * ENTER_STAGGER_MS, ENTER_STAGGER_CAP_MS) * scale;
      done.push(el.animate([
        { transform: `translateY(${DEAL_PX}px) rotate(${DEAL_DEG}deg) scale(.97)`, opacity: 0, filter: 'blur(3px)' },
        { transform: 'translateY(0) rotate(0) scale(1)', opacity: 1, filter: 'blur(0)' },
      ], { duration: ENTER_MS * scale, delay, easing: EASE_SNAP, fill: 'backwards' }).finished.catch(() => {}));
    }
  }
  return Promise.all(done);
}

/// The fog: one element, made once, kept.
function mist(){
  let el = document.getElementById('mist');
  if (!el) {
    el = document.createElement('div'); el.id = 'mist'; el.className = 'mist'; el.setAttribute('aria-hidden', 'true');
    el.innerHTML = '<i class="mist-edge"></i>';
    document.body.appendChild(el);
  }
  return el;
}

/// Roll the fog across and run `swap` when it covers the middle. `swap` may
/// return a promise; the fog's thick part waits for nothing -- data must be
/// in hand before the sweep starts, so the swap is a DOM write.
export function sweep(swap){
  if (reduced()) { swap(); return Promise.resolve(); }
  const el = mist();
  const snapshot = () => {
    const out = new Map();
    for (const n of document.querySelectorAll('[data-money],[data-t],[data-t-tag],[data-t-st],.card-name,.card-desc,.sec-name,.rail-chip')) out.set(n, n.textContent);
    return out;
  };
  const beforeText = snapshot();
  el.classList.add('on');
  const run = el.animate([
    { transform: 'translateX(-105%)' }, { transform: 'translateX(105%)' },
  ], { duration: SWEEP_MS, easing: EASE_TIDE, fill: 'forwards' });
  return new Promise(resolve => {
    setTimeout(async () => {
      await swap();
      // The wisps: every node whose value changed drifts from the old to the new.
      for (const [n, was] of beforeText) {
        if (!n.isConnected || n.textContent === was) continue;
        const r = n.getBoundingClientRect();
        if (r.bottom < 0 || r.top > innerHeight) continue;
        n.animate([
          { opacity: .1, filter: 'blur(4px)', transform: `translateX(${WISP_PX}px)` },
          { opacity: 1, filter: 'blur(0)', transform: 'translateX(0)' },
        ], { duration: WISP_MS, easing: EASE_TIDE });
      }
      run.finished.catch(() => {}).then(() => { el.classList.remove('on'); run.cancel(); resolve(); });
    }, SWEEP_MS * SWEEP_SWAP_AT);
  });
}
