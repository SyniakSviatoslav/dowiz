// "Tell me on my phone" (W-PUSH): the opt-in control the storefront's order
// page, the courier app and the owner console share.
//
// PERMISSION IS ASKED ONLY ON THE TAP. Nothing here calls
// `Notification.requestPermission()` except `turnOn`, and `turnOn` is only
// bound to the button -- a page that asks on load is the page people block.
//
// WHO IS TOLD WHAT IS THE TOKEN'S BUSINESS (`/api/push/subscribe`): the
// customer's order token subscribes that order, a courier's their runs, an
// owner's or staff member's the venue's new orders. This file only hands the
// browser's subscription (endpoint + keys) and the language over.
//
// iOS: Safari shows web notifications only to a page added to the Home Screen
// (iOS 16.4+). Not installed there, the control SAYS SO instead of offering a
// button that cannot work.

import { pushWords } from './push-words.js';
import { button, esc } from './ui/index.js';

const API = '/api';
/// The last view per scope and token, so a redraw does not re-ask the hub.
const known = new Map();

export const supported = () =>
  typeof navigator !== 'undefined' && 'serviceWorker' in navigator
  && typeof window !== 'undefined' && 'PushManager' in window && 'Notification' in window;

/// An Apple touch device whose page is not on the Home Screen.
export const appleNotInstalled = () =>
  typeof navigator !== 'undefined' && navigator.vendor === 'Apple Computer, Inc.'
  && matchMedia('(hover: none) and (pointer: coarse)').matches
  && !(navigator.standalone === true || matchMedia('(display-mode: standalone)').matches);

/// What the control shows. PURE, so the four states are tested in node.
///   'ios'          Apple, not installed: the Home Screen instruction
///   'unsupported'  no push in this browser
///   'blocked'      the person said no; only the browser's settings can undo it
///   'on' / 'off'
export function viewOf({ isSupported, apple, permission, on }){
  if (!isSupported) return apple ? 'ios' : 'unsupported';
  if (permission === 'denied') return 'blocked';
  return on && permission === 'granted' ? 'on' : 'off';
}

/// base64url -> bytes, for `applicationServerKey`.
export function keyBytes(b64url){
  const b64 = b64url.replace(/-/g, '+').replace(/_/g, '/') + '==='.slice((b64url.length + 3) % 4);
  const raw = atob(b64);
  return Uint8Array.from(raw, c => c.charCodeAt(0));
}

async function post(path, token, body){
  const r = await fetch(API + path, {
    method: 'POST',
    headers: { 'content-type': 'application/json', authorization: 'Bearer ' + token },
    body: JSON.stringify(body),
  });
  const d = await r.json().catch(() => ({}));
  if (!r.ok) throw new Error(d.error || ('HTTP ' + r.status));
  return d;
}

async function registration(sw, scope){
  const have = await navigator.serviceWorker.getRegistration(scope);
  if (have) return have;
  return navigator.serviceWorker.register(sw, { scope });
}

/// The browser's subscription for `scope`, made with OUR key. A subscription
/// made with another key (a rotated one) is replaced, or every push to it
/// would be refused by the push service.
async function subscription(reg, key){
  const want = keyBytes(key);
  const have = await reg.pushManager.getSubscription();
  if (have) {
    const k = have.options?.applicationServerKey;
    const same = k && new Uint8Array(k).every((b, i) => b === want[i]) && new Uint8Array(k).length === want.length;
    if (same || !k) return have;
    await have.unsubscribe().catch(() => {});
  }
  return reg.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: want });
}

/// The tap. Returns the new view.
export async function turnOn({ token, lang, sw, scope }){
  const perm = await Notification.requestPermission();
  if (perm !== 'granted') return perm === 'denied' ? 'blocked' : 'off';
  const reg = await registration(sw, scope);
  await navigator.serviceWorker.ready;
  const { key } = await (await fetch(API + '/push/key')).json();
  const sub = await subscription(reg, key);
  await post('/push/subscribe', token, { ...sub.toJSON(), lang });
  return 'on';
}

/// The other tap. The browser's subscription is KEPT: another order or role
/// on this device may still use it; the hub forgets this one.
export async function turnOff({ token, scope }){
  const reg = await navigator.serviceWorker.getRegistration(scope);
  const sub = reg && await reg.pushManager.getSubscription();
  if (sub) await post('/push/unsubscribe', token, { endpoint: sub.endpoint });
  return 'off';
}

async function isOn({ token, scope }){
  if (Notification.permission !== 'granted') return false;
  const reg = await navigator.serviceWorker.getRegistration(scope);
  const sub = reg && await reg.pushManager.getSubscription();
  if (!sub) return false;
  return !!(await post('/push/state', token, { endpoint: sub.endpoint })).on;
}

/// The control's markup for a view. PURE.
export function markup(view, lang, why, tour){
  const w = pushWords(lang);
  const line = { ios: w.pushIos, unsupported: w.pushUnsupported, blocked: w.pushBlocked, on: w.pushIsOn, off: w[why] || '' }[view];
  const btn = view === 'on' ? button({ id: 'pushToggle', variant: 'ghost', icon: 'x', label: w.pushOff, block: true, attrs: { 'data-tour': tour } })
    : view === 'off' ? button({ id: 'pushToggle', variant: 'secondary', icon: 'message-2', label: w.pushOn, block: true, attrs: { 'data-tour': tour } })
    : '';
  return `<section class="push-opt" data-push="${esc(view)}">
    <p class="eyebrow">${esc(w.pushTitle)}</p>
    <p class="muted small">${esc(line)}</p>
    ${btn}
  </section>`;
}

/// Put the control at the end of `host`. `why` is the words key saying what
/// this role is told (pushWhyCustomer / pushWhyCourier / pushWhyStaff).
export async function mount(host, { token, lang, sw, scope, why, tour, toast }){
  if (!host || !token) return;
  let el = host.querySelector(':scope > .push-host');
  if (!el) { el = document.createElement('div'); el.className = 'push-host'; host.appendChild(el); }
  const memo = `${scope}|${token}`;
  const show = view => {
    known.set(memo, view);
    el.innerHTML = markup(view, lang, why, tour);
    const b = el.querySelector('#pushToggle');
    if (!b) return;
    b.onclick = async () => {
      b.disabled = true;
      try { show(view === 'on' ? await turnOff({ token, scope }) : await turnOn({ token, lang, sw, scope })); }
      catch (e) { b.disabled = false; if (toast) toast(`${pushWords(lang).pushFail}: ${e.message || e}`); }
    };
  };
  const base = { isSupported: supported(), apple: appleNotInstalled(), permission: supported() ? Notification.permission : 'default' };
  // A screen that redraws every few seconds asks the hub ONCE per token.
  if (known.has(memo) && base.permission !== 'denied') return show(known.get(memo));
  show(viewOf({ ...base, on: false }));
  if (base.isSupported && base.permission === 'granted') {
    try { show(viewOf({ ...base, on: await isOn({ token, scope }) })); } catch { /* the off view stays: tapping it re-subscribes */ }
  }
}
