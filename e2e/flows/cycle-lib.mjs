// THE CYCLE'S SHARED STEPS (F5-F10): the storefront checkout as a guest does it, the owner console
// sign-in, staff made through the product's own invite + claim, the courier's credential, and the
// soft check that records a failure and lets the cycle go on (one product defect must not hide the rest).
import { HOST, LOC, RUN, OUT, S, save, Fail, api, own, courierCreds, creds } from './lib.mjs';
import { go, routeLocal, guard, inspect, until } from './page.mjs';

export const rnd = a => a[Math.floor(Math.random() * a.length)];
export const randPhone = () => `+35569${String(1000000 + Math.floor(Math.random() * 8999999))}`;
export const sheetName = p => p.evaluate(() => document.getElementById('sheet')?.dataset.name || '');
export const C = () => S.cycle || (S.cycle = {});
export const digits = s => String(s || '').replace(/\D/g, '');

/// A check that does not stop the flow: its reason is kept and the flow fails at the end, naming all.
export function softFail(list, ok, msg) { if (ok) { console.log(`  ok   ${msg}`); return true; } console.log(`  FAIL ${msg}`); list.push(msg); return false; }
export function endSoft(list, flow) { if (list.length) throw new Fail(`${list.length} check(s) failed in ${flow}: ${list.slice(0, 4).join(' | ')}${list.length > 4 ? ` (+${list.length - 4})` : ''}`); }
export const info = msg => console.log(`  info ${msg}`);

export async function dismiss(p) {
  for (let i = 0; i < 6; i++) {
    const n = await sheetName(p);
    if (!n || n === 'track' || n === 'bookMine' || n === 'book') return;
    const later = await p.$('#insLater');
    if (later) await later.click().catch(() => {}); else await p.evaluate(() => document.getElementById('scrim')?.click());
    await p.waitForTimeout(600);
  }
}
export async function closeSheet(p) {
  await p.evaluate(() => document.getElementById('scrim')?.click());
  if (!(await until(p, () => !document.getElementById('sheet')?.dataset.name, null, 5000))) {
    await p.keyboard.press('Escape');
    if (!(await until(p, () => !document.getElementById('sheet')?.dataset.name, null, 5000))) throw new Fail(`sheet "${await sheetName(p)}" does not close`);
  }
}
export async function setLang(p, L) {
  await p.click('#langBtn');
  if (!(await until(p, () => document.getElementById('sheet')?.dataset.name === 'lang', null, 8000))) throw new Fail('the language button opened no sheet');
  await p.click(`#sheetIn [data-l="${L}"]`);
  if (!(await until(p, l => document.documentElement.lang === l, L, 8000))) throw new Fail(`language ${L} did not take`);
  await p.waitForTimeout(1200);
  await closeSheet(p).catch(() => {});
}
/// The READING currency (store/nav.js openCurrency): display only, the charge stays in the venue's.
export async function setCurrency(p, code) {
  await dismiss(p);
  await p.click('#curBtn');
  if (!(await until(p, () => document.getElementById('sheet')?.dataset.name === 'currency', null, 8000))) throw new Fail('the currency button opened no sheet');
  await p.click(`#sheetIn [data-c="${code}"]`);
  await p.waitForTimeout(2500);
  await closeSheet(p).catch(() => {});
}
/// What the checkout's totals block says (store/cart.js totalsBlock): the grand total in the reading
/// currency, and the "you will be charged" line in the venue's, present only when the two differ.
export const readTotals = p => p.evaluate(() => {
  const box = document.getElementById('totalsBox'); if (!box) return null;
  const tx = sel => box.querySelector(sel)?.textContent.trim() || null;
  return { grand: tx('.row.grand .money'), grandMinor: Number(box.querySelector('.row.grand [data-money]')?.dataset.money ?? NaN),
    charged: tx('.row.note .money'), text: box.innerText.replace(/\s+/g, ' ').trim() };
});

/// One storefront order through the real checkout.
/// o: {how: delivery|pickup|table, ids, qty, name, phone, street?, house?, pin?, tip?, allowFail?, beforePlace?}
/// Answers the placed order (with `shown`: the totals as the guest read them), or null when allowFail.
export async function checkoutUI(p, o) {
  let placed = null;
  const h = async r => {
    if (/\/orders$/.test(r.url()) && r.request().method() === 'POST' && r.status() < 500) {
      try { placed = { status: r.status(), ...(await r.json()) }; } catch { placed = { status: r.status() }; }
    }
  };
  p.on('response', h);
  try {
    await dismiss(p);
    // One unit per visit to the dish sheet: "add" closes it (store/dish.js).
    for (const [i, id] of o.ids.entries()) for (let q = 0; q < (i === 0 ? o.qty : 1); q++) {
      await p.click(`.card[data-p="${id}"]`);
      if (!(await until(p, () => document.getElementById('sheet')?.dataset.name === 'dish', null, 10000))) throw new Fail(`dish ${id} opened no sheet`);
      const n0 = Number(await p.$eval('#pillCount', e => e.textContent).catch(() => '0')) || 0;
      await p.click('#dadd');
      if (!(await until(p, n => Number(document.getElementById('pillCount')?.textContent) === n + 1, n0, 8000))) throw new Fail(`adding ${id}: the cart count did not rise from ${n0}`);
      if (await sheetName(p)) await closeSheet(p);
    }
    await p.click('#cartPill');
    if (!(await until(p, () => document.getElementById('sheet')?.dataset.name === 'cart', null, 10000))) throw new Fail('the cart pill opened no cart');
    // The checkout is a dynamic import (> 20 s on this box's network, F1). A tap that landed while the
    // cart sheet was still rising is tapped once more, said, never silently.
    await p.waitForTimeout(600);
    await p.click('#toCheckout');
    if (!(await until(p, () => document.getElementById('sheet')?.dataset.name === 'checkout', null, 30000))) {
      console.log(`  !! ${o.name}: checkout not open 30 s after the tap; tapping "checkout" once more`);
      await p.click('#toCheckout').catch(() => {});
      if (!(await until(p, () => document.getElementById('sheet')?.dataset.name === 'checkout', null, 45000))) throw new Fail(`${o.name}: checkout did not open (two taps, 75 s)`);
    }
    if (o.how === 'pickup') {
      await p.click('[data-how="pickup"]');
      await until(p, () => document.getElementById('addrBox')?.hidden === true, null, 5000);
    } else if (o.how === 'delivery') {
      if (await p.$('[data-how="delivery"]')) await p.click('[data-how="delivery"]');
      if (!(await until(p, () => document.getElementById('addrBox')?.hidden === false, null, 5000))) throw new Fail(`${o.name}: choosing delivery did not show the address`);
      await p.fill('#f-street', o.street);
      await p.fill('#f-house', o.house);
      if (o.pin) o.pinResult = await pinOnMap(p);
    }
    await p.fill('#f-name', o.name);
    await p.fill('#f-phone', o.phone);
    await p.fill('#f-note', 'QA flows gate -- not a real order');   // never the name: the kitchen reads this note
    await p.click('[data-pay="cash"]');
    // The tip is chosen EVERY time: the checkout pre-selects the last one (store/state.js state.tip).
    if (o.how === 'delivery' && await p.$('#tipBox:not([hidden])')) {
      const tb = await p.$(`#tipBox:not([hidden]) [data-tip="${o.tip || 0}"]`);
      if (!tb) throw new Fail(`${o.name}: the checkout offers no tip of ${o.tip} (tips switched off for the venue?)`);
      await tb.click(); await p.waitForTimeout(400);
    }
    await inspect(p, `checkout (${o.name})`);
    const shown = await readTotals(p);
    if (o.beforePlace) await o.beforePlace();
    await p.click('#place');
    for (let i = 0; i < (o.allowFail ? 16 : 80) && !placed; i++) await p.waitForTimeout(500);
    if (!placed?.id && o.allowFail) return null;
    if (!placed?.id) throw new Fail(`no order placed for ${o.name} (${o.how}): ${await p.$eval('#f-err', e => e.innerText).catch(() => 'no #f-err')}`);
    return { ...placed, shown };
  } finally { p.off('response', h); }
}

/// The map pin (store/address.js pickOnMap): the sheet's #pickOk confirms; the checkout re-opens.
/// WebGL is off in this Chromium (memory webgl-renders-nothing-on-the-box), so the map may not draw:
/// the confirm still answers with the venue's point. Answers what happened, never throws.
async function pinOnMap(p) {
  await p.click('#pinGo');
  if (!(await until(p, () => document.getElementById('sheet')?.dataset.name === 'map', null, 15000))) return 'the map sheet did not open';
  await p.waitForTimeout(2500);
  const ok = await p.$('#pickOk');
  if (!ok) return 'no #pickOk on the map sheet';
  await ok.click();
  if (!(await until(p, () => document.getElementById('sheet')?.dataset.name === 'checkout', null, 15000))) return 'confirming the pin did not re-open the checkout';
  const line = await p.evaluate(() => !document.getElementById('pinLine')?.hidden);
  return line ? 'pin confirmed (checkout shows the pin line)' : 'checkout re-opened without the pin line';
}

/// The owner console, signed in through its own form.
export async function openConsole(b, tag, viewport = { width: 390, height: 844 }) {
  const ctx = await b.newContext({ viewport, serviceWorkers: 'block' });
  await routeLocal(ctx);
  const p = await ctx.newPage(); const g = guard(p, tag);
  await go(p, `${HOST}/admin/`, g);
  await p.waitForSelector('#e', { timeout: 60000 });
  for (let i = 0; i < 4; i++) {
    await p.fill('#e', creds.OWNER_EMAIL); await p.fill('#p', creds.OWNER_PASSWORD);
    await p.click('#go').catch(() => {});
    if (await until(p, () => { const n = document.getElementById('nav'); return !!n && !n.hidden; }, null, 20000)) break;
  }
  await p.waitForTimeout(1500);
  if (!(await p.$('#nav:not([hidden])'))) throw new Fail(`${tag}: the owner console did not sign in`);
  return { ctx, p, g };
}
export const openTab = async (p, tab) => { await p.click(`#nav [data-tab="${tab}"]`); await until(p, t => document.getElementById('app')?.dataset.tab === t, tab, 10000); await p.waitForTimeout(1500); };
export const ownerRow = async (p, id, ms = 30000) => p.waitForSelector(`.orow[data-o="${id}"]`, { timeout: ms }).then(() => true, () => false);

/// A member of staff, made through the product's own invite and claim routes (no database writes).
export async function makeStaff(role) {
  const email = `qa-${role}-${Date.now().toString(36)}@example.com`;
  const password = `Qa-${RUN.slice(6)}-${role}-${Math.random().toString(36).slice(2, 8)}!`;
  const inv = await own('/api/owner/staff/invite', { email, name: `${RUN} ${role}`, role, location_id: LOC });
  if (inv.status !== 200 || !inv.body?.code) throw new Fail(`inviting the ${role}: ${inv.status} ${inv.text.slice(0, 120)}`);
  const cl = await api('/api/staff/claim', { method: 'POST', body: { email, code: inv.body.code, password } });
  if (cl.status !== 200) throw new Fail(`the ${role} invite did not claim: ${cl.status} ${cl.text.slice(0, 120)}`);
  const s = { email, password, role, jwt: cl.body?.jwt };
  C()[role] = { email, password }; save();
  return s;
}
export async function staffJwt(s) {
  const r = await api('/api/staff/login', { method: 'POST', body: { email: s.email, password: s.password } });
  return r.body?.jwt;
}

/// The courier credential for THIS venue; failing that, one invited and claimed through the product.
export async function courierLogin() {
  const { phone, password } = courierCreds();
  if (phone && password) {
    const lg = await api('/api/courier/auth/login', { method: 'POST', body: { phone, password } });
    if (lg.status === 200 && lg.body?.courier?.locationId === LOC) return { phone, password, jwt: lg.body.jwt, id: lg.body.courier.id };
  }
  const ph = randPhone();
  const pw = `Qa-${RUN.slice(6)}-courier-${Math.random().toString(36).slice(2, 8)}!`;
  const inv = await own('/api/owner/couriers/invite', { phone: ph, name: `${RUN} courier` });
  if (inv.status !== 200 || !inv.body?.code) throw new Fail(`inviting a courier: ${inv.status} ${inv.text.slice(0, 120)}`);
  const cl = await api('/api/courier/auth/claim', { method: 'POST', body: { phone: ph, code: inv.body.code, password: pw } });
  if (cl.status !== 200) throw new Fail(`the courier invite did not claim: ${cl.status} ${cl.text.slice(0, 120)}`);
  const lg = await api('/api/courier/auth/login', { method: 'POST', body: { phone: ph, password: pw } });
  C().courier = { phone: ph, password: pw }; save();
  return { phone: ph, password: pw, jwt: lg.body?.jwt, id: lg.body?.courier?.id };
}

/// The room app, signed in as `s` (already claimed: no claim toggle).
export async function roomIn(b, s, tag) {
  const ctx = await b.newContext({ viewport: { width: 400, height: 860 }, serviceWorkers: 'block', reducedMotion: 'reduce' }); await routeLocal(ctx);
  const p = await ctx.newPage(); const g = guard(p, tag);
  await go(p, `${HOST}/room/`, g);
  await p.waitForSelector('form[data-form="login"]', { timeout: 60000 });
  await p.fill('input[name="email"]', s.email); await p.fill('input[name="password"]', s.password);
  await p.click('form[data-form="login"] button[type="submit"]');
  const ok = await until(p, () => !document.querySelector('form[data-form="login"]') && !!document.querySelector('.bar'), null, 30000);
  return { ctx, p, g, ok };
}
export { OUT };
