// Helpers shared by the QA-hub walks (q1..q6): the QA owner's token, a staff sign-in to /admin,
// an order placed with the body the storefront itself sent (saved by q2-kitchen.mjs), and one
// line typed into the hub's assistant with its answer read off the screen.
import { HOST, LOC, api, creds, watch, sst } from './_lib.mjs';

export const UA = 'Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Mobile Safari/537.36';
export const PHONE = { viewport: { width: 390, height: 844 }, serviceWorkers: 'block', userAgent: UA };

/// The owner's token for the QA venue (login names it: the owner has several venues).
export async function qaOwnerToken() {
  for (let i = 0; i < 6; i++) {
    const r = await api('/api/auth/login', { method: 'POST', body: { email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD, location_id: LOC } });
    if (r.body?.access_token) return r.body.access_token;
  }
  return null;
}

/// Sign a member of staff in to the hub's /admin with the same form the owner uses.
export async function staffConsoleIn(ctx, email, password, tag = 'staff') {
  const p = await ctx.newPage(); watch(p, tag);
  await p.goto(`${HOST}/admin/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await p.waitForSelector('#e', { timeout: 40000 });
  await p.fill('#e', email); await p.fill('#p', password); await p.click('#go');
  const ok = await p.waitForSelector('#nav:not([hidden])', { timeout: 30000 }).then(() => true).catch(() => false);
  await p.waitForTimeout(3000);
  return { p, ok };
}

/// A fresh order with the storefront's own body (q2 saved it), a new idempotency key each time.
export async function placeOrder(tag = 'x') {
  const raw = sst().orderBody;
  if (!raw) throw new Error('no saved storefront order body: run q2-kitchen.mjs first');
  const body = JSON.parse(raw);
  for (const k of ['idempotency_key', 'idempotencyKey', 'request_id', 'requestId', 'client_id']) if (k in body) body[k] = `${body[k]}-${tag}-${Date.now()}`;
  const r = await api(`/api/public/locations/${LOC}/orders`, { method: 'POST', body, headers: { 'idempotency-key': crypto.randomUUID() } });
  return { status: r.status, id: r.body?.id, body: r.body };
}

/// A read re-asked while the platform answers 503 "exceeded resource limits" (bounded).
export async function read(path, opts = {}) {
  let r; for (let i = 0; i < 6; i++) { r = await api(path, opts); if (r.status !== 503) return r; } return r;
}

export async function ownerOrder(tok, id) {
  const r = await read(`/api/owner/orders?location_id=${LOC}`, { token: tok });
  return (r.body.orders || []).find(o => o.id === id) || null;
}

export async function stockOf(tok, item) {
  const r = await read(`/api/owner/stock?location_id=${LOC}`, { token: tok });
  return (r.body.supplies || []).find(s => s.id === item) || null;
}

export async function dishOf(item) {
  const r = await read(`/api/public/locations/${LOC}/menu?fresh=1`);
  return (r.body.categories || []).flatMap(c => c.products || []).find(p => p.id === item) || null;
}

/// Type one line into the assistant panel (opening it if needed); answers the new bubbles' text and
/// whether the last one is a proposal with a "Yes, do it" button.
export async function ask(p, line, waitMs = 5000) {
  if (!(await p.$('#asQ'))) { await p.click('#asstBtn'); await p.waitForTimeout(1200); }
  const before = await p.$$eval('#asLog .as-msg', els => els.length).catch(() => 0);
  await p.fill('#asQ', line); await p.click('#asGo');
  await p.waitForTimeout(waitMs);
  const msgs = await p.$$eval('#asLog .as-msg', els => els.map(e => e.innerText.replace(/\s+/g, ' ').trim()));
  const fresh = msgs.slice(before);
  const yes = await p.$$eval('#asLog [data-yes]', els => els.map(e => e.dataset.yes));
  return { said: fresh.join(' || '), yes: yes.length ? yes[yes.length - 1] : null };
}

export async function confirmLast(p, i, waitMs = 4000) {
  await p.click(`#asLog [data-yes="${i}"]`); await p.waitForTimeout(waitMs);
  const state = await p.$$eval('#asLog .as-state', els => els.map(e => e.innerText.trim()));
  const toast = await p.$eval('#toast', e => e.innerText.trim()).catch(() => '');
  return { state: state[state.length - 1] || '', toast };
}

/// The storefront, ready to tap. The live page (before lib/retry.js ships) shows "the menu did not load"
/// on a platform 503: tap its own retry, at most five times, and say how many it took.
export async function storeReady(ctx, tag = 'store') {
  const c = await ctx.newPage(); watch(c, tag);
  await c.goto(`${HOST}/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  let taps = 0;
  for (; taps < 6; taps++) {
    const ok = await c.waitForSelector('.card', { timeout: 15000 }).then(() => true).catch(() => false);
    if (ok) break;
    const r = await c.$('#retry'); if (r) await r.click().catch(() => {}); else await c.reload({ waitUntil: 'domcontentloaded' });
  }
  await c.waitForTimeout(3000);
  for (let i = 0; i < 6; i++) {
    const n = await c.evaluate(() => document.getElementById('sheet')?.dataset.name || '');
    if (!n) break;
    const l = await c.$('#insLater'); if (l) await l.click().catch(() => {}); else await c.evaluate(() => document.getElementById('scrim')?.click());
    await c.waitForTimeout(600);
  }
  return { c, taps };
}

/// The owner console, signed in. The form is filled again before every tap: a refused sign-in redraws it
/// empty, and `_lib.consoleIn` tapped an empty form after the first platform 503 (-> 401).
export async function ownerConsole(ctx, tag = 'console') {
  const o = await ctx.newPage(); watch(o, tag);
  await o.goto(`${HOST}/admin/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  for (let i = 0; i < 8; i++) {
    await o.waitForSelector('#e', { timeout: 40000 });
    await o.fill('#e', creds.OWNER_EMAIL); await o.fill('#p', creds.OWNER_PASSWORD);
    await o.click('#go').catch(() => {});
    if (await o.waitForSelector('#nav:not([hidden])', { timeout: 15000 }).then(() => true).catch(() => false)) break;
    console.log(`  !! ${tag}: sign-in did not land (try ${i + 1})`);
  }
  await o.waitForTimeout(2500);
  return o;
}
