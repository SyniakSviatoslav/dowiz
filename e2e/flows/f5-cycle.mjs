// F5-F9: THE ORDER CYCLE, every role in its own screen, one chromium at a time.
//   F5 GUESTS     six random guests through the storefront UI (random dishes, quantities, language,
//                 phone), each READING prices in a different currency (EUR / USD / ALL: display only,
//                 the charge is the venue's ALL, src/services/ordering/rates.rs), with and without a tip:
//                 A delivery EUR +tip (map pin), B delivery USD no tip, C pickup ALL, D delivery ALL +tip
//                 (refused at the door in F7), R delivery (the owner REJECTS with a reason), K pickup (the
//                 KITCHEN rejects with a reason in F6). The owner confirms A, B, C, D in the console.
//   F6 KITCHEN    a kitchen member (invite + claim) works A-D: seen -> preparing -> ready, rejects K with
//                 a reason; no customer name or phone on its screens or its API. The guest's own phone
//                 then reads R's and K's status page: REJECTED, and the reason.
//   F7 COURIER    f7-couriers.mjs.  F8 IN-VENUE f8-room.mjs.  F9 OWNER f9-owner.mjs.
// Every name carries RUN (FLOWS-...) so F4 can close what is left. Each step asserts the page AND the API.
import { HOST, LOC, RUN, OUT, S, save, Fail, api, own, menu, dishes, opened, ownerOrder } from './lib.mjs';
import { launch, go, PHONE, routeLocal, localServed, guard, clean, inspect, shot, until } from './page.mjs';
import { rnd, randPhone, C, digits, softFail, endSoft, info, dismiss, closeSheet, setLang, setCurrency, checkoutUI,
  openConsole, openTab, ownerRow, makeStaff, staffJwt } from './cycle-lib.mjs';
export { checkoutUI, setLang } from './cycle-lib.mjs';
export { f7 } from './f7-couriers.mjs';
export { f8 } from './f8-room.mjs';
export { f9 } from './f9-owner.mjs';

const sheetNameOf = p => p.evaluate(() => document.getElementById('sheet')?.dataset.name || '');
/// The API's money for one order: total = subtotal - discount + fee + tip, in the venue's minor units.
function sums(b) {
  const n = k => Number(b?.[k] ?? 0) || 0;
  return { total: n('total'), subtotal: n('subtotal'), fee: n('delivery_fee'), tip: n('tip'), cut: n('discount') };
}

/// THE SHELF THE SUITE EATS IS REFILLED BY THE SUITE (2026-10-21): every run
/// serves tuna and nothing ever received it again, so qa-tuna reached 0 and
/// F5's first order was refused ("qa-tuna: 80 wanted, 0 available"), taking
/// F6-F11 down with it. F11 receives salmon and rice priced; a supply never
/// priced is received here WITHOUT a price, so it stays unpriced for F11's
/// stockUnpriced check. Named, not "every low row": qa-lost is kept short on
/// purpose, for the lost-sale check.
const FLOOR = 1500, REFILL = 3000, REFILLED = ['qa-tuna'];
async function topUp(log) {
  const r = await own(`/api/owner/stock?location_id=${LOC}`);
  if (r.status !== 200) throw new Fail(`reading the shelf: ${r.status}`);
  for (const s of (r.body.supplies || []).filter(s => REFILLED.includes(s.id) && s.costPerBasis == null && !(Number(s.wac) > 0) && Number(s.available) < FLOOR)) {
    const rc = await own(`/api/owner/stock/received?location_id=${LOC}`, { item: s.id, qty: REFILL, supplier: 'FLOWS refill', doc: `FLOWS-${RUN}-refill` });
    if (rc.status !== 200) throw new Fail(`refilling ${s.id}: ${rc.status} ${rc.text.slice(0, 160)}`);
    log(`refilled ${s.id}: ${s.available} available + ${REFILL} (unpriced)`);
  }
}

// ── F5 ────────────────────────────────────────────────────────────────────
export async function f5(log) {
  const bad = [];
  const m0 = await menu();
  // qa-veg-roll is taken off sale and back by F10; nothing else here depends on which dish.
  const on = dishes(m0).filter(d => d.available && !String(d.id).startsWith('flows-'));   // not another lane's temporary dish
  if (on.length < 4) throw new Fail(`qa-durres lists ${on.length} dishes on sale; the cycle needs 4 (run e2e/walk/qa-setup.mjs)`);
  await topUp(log);
  if (!m0.location?.pickup) {
    if (S.restore.pickup === undefined) { S.restore.pickup = false; save(); }
    const r = await own('/api/owner/location', { location_id: LOC, pickup: true });
    if (r.status !== 200) throw new Fail(`switching pickup on: ${r.status}`);
  }
  const base = m0.location?.currencyCode || 'ALL';
  const rates = (await api(`/api/public/rates?base=${base}`)).body || {};
  log(`venue currency ${base}; rates ppm ${JSON.stringify(rates.ppm || {})}${rates.stale ? ' (STALE: identity rate)' : ''}`);
  const L = () => rnd(['sq', 'en', 'uk']);
  const q = () => 1 + Math.floor(Math.random() * 2);
  const G = [
    { key: 'A', how: 'delivery', cur: 'EUR', tip: 200, pin: true, street: 'Rruga Taulantia', house: String(1 + Math.floor(Math.random() * 90)), ids: [rnd(on).id], qty: q() },
    { key: 'B', how: 'delivery', cur: 'USD', tip: 0, street: 'Rruga Kavajes', house: String(1 + Math.floor(Math.random() * 90)), ids: [on[1].id, on[2].id], qty: q() },
    { key: 'C', how: 'pickup', cur: 'ALL', tip: 0, ids: [rnd(on).id], qty: q() },
    { key: 'D', how: 'delivery', cur: 'ALL', tip: 100, street: 'Rruga Durresi', house: String(1 + Math.floor(Math.random() * 90)), ids: [rnd(on).id], qty: 1 },
    { key: 'R', how: 'delivery', cur: 'ALL', tip: 0, street: 'Rruga Durresi', house: '3', ids: [on[3].id], qty: 1 },
    { key: 'K', how: 'pickup', cur: 'ALL', tip: 0, ids: [rnd(on).id], qty: 1 },
  ].map(x => ({ ...x, name: `${RUN} guest ${x.key}`, phone: randPhone(), lang: L() }));
  C().guests = G.map(({ key, how, name, phone, lang, ids, qty, cur, tip }) => ({ key, how, name, phone, lang, ids, qty, cur, tip }));
  save();
  log(`random guests: ${G.map(g => `${g.key}:${g.how}/${g.lang}/${g.cur}${g.tip ? `+tip${g.tip}` : ''}/${g.ids.join('+')}x${g.qty}`).join('  ')}`);

  const b = await launch();
  try {
    const ctx = await b.newContext(PHONE); await routeLocal(ctx);
    const p = await ctx.newPage(); const g = guard(p, 'guest');
    // qa-durres has no coordinates of its own: with a map pin the ETA quote answers 409 "this venue has
    // not set its own location" (src/eta.rs) and store/eta.js shows no estimate. Venue data, named here.
    g.expect('POST', /\/eta$/, 409);
    await go(p, `${HOST}/`, g);
    if (!(await p.waitForSelector('.card', { timeout: 60000 }).then(() => true, () => false))) throw new Fail('the menu never drew a dish card');
    await p.waitForTimeout(2500);
    for (const x of G) {
      await setLang(p, x.lang);
      await setCurrency(p, x.cur);
      const placed = await checkoutUI(p, x);
      opened(placed.id, placed.access_token, 'F5');
      const o = await api(`/api/order/${placed.id}`, { token: placed.access_token });
      const st = o.body?.status, kind = o.body?.fulfilment?.kind, m = sums(o.body);
      Object.assign(C().guests.find(c => c.key === x.key), { id: placed.id, token: placed.access_token, total: m.total, shown: placed.shown, pin: x.pinResult });
      save();
      softFail(bad, st === 'PENDING' && kind === x.how, `${x.name}: placed, API ${st} ${kind} (want PENDING ${x.how})`);
      softFail(bad, m.total === m.subtotal - m.cut + m.fee + m.tip && m.tip === x.tip,
        `${x.name}: API money exact in ${base}: total ${m.total} = subtotal ${m.subtotal} - ${m.cut} + fee ${m.fee} + tip ${m.tip} (tip asked ${x.tip})`);
      const sh = placed.shown || {};
      if (x.cur === base) {
        softFail(bad, !sh.charged && digits(sh.grand) === String(m.total), `${x.name} reads ${base}: page total "${sh.grand}" = API ${m.total}, no second "charged" line`);
      } else {
        const ppm = rates.ppm?.[x.cur];
        const want = ppm ? Math.round(m.total * ppm / 10000) : null;     // ALL (0 decimals) -> 2-decimal minor units
        const got = Number(digits(sh.grand));
        softFail(bad, digits(sh.charged) === String(m.total), `${x.name} reads ${x.cur}: the page says which figure is charged: "${sh.charged}" = API ${m.total} ${base}`);
        softFail(bad, sh.grandMinor === m.total && (rates.stale || (want != null && Math.abs(got - want) <= 1)),
          `${x.name} reads ${x.cur}: total shown "${sh.grand}" (${got} minor) is the reading of ${m.total} ${base} at ${ppm} ppm (want ${want})`);
      }
      if (x.pinResult) info(`${x.name}: map pin: ${x.pinResult}`);
      // the status page is a full sheet over the bar: "done" closes it, as a guest would
      await p.waitForTimeout(1200);
      if (await p.$('#closeTrack')) await p.click('#closeTrack').catch(() => {});
      if (await sheetNameOf(p)) await closeSheet(p).catch(() => {});
      await dismiss(p);
    }
    softFail(bad, !g.bad.length, `F5 storefront: no console error or failed request (${g.bad.slice(0, 2).join(' | ') || 'none'})`);
    await inspect(p, 'F5 storefront').catch(e => softFail(bad, false, e.message));
    await shot(p, 'F5-storefront-placed');
    await ctx.storageState({ path: `${OUT}/guest-state.json` }).catch(() => {});
    S.guestState = `${OUT}/guest-state.json`; save();
    localServed(ctx, 'F5');
  } catch (e) {
    { const pg = b.contexts()[0]?.pages()[0]; if (pg) e.shot = await shot(pg, 'F5-fail'); }
    throw e;
  } finally { await b.close().catch(() => {}); }

  // ── the owner confirms A-D, rejects R with a reason; K is left for the kitchen ──
  const ob = await launch();
  try {
    const { p, g } = await openConsole(ob, 'owner-F5');
    await openTab(p, 'orders');
    for (const x of C().guests.filter(x => x.id && x.key !== 'K')) {
      if (!softFail(bad, await ownerRow(p, x.id), `owner list shows ${x.name}`)) continue;
      if (x.key === 'R') {
        await p.click(`.orow [data-act="reject"][data-o="${x.id}"]`);
        await p.waitForSelector('#cf-reason', { timeout: 10000 });
        await p.fill('#cf-reason', 'QA flows gate: out of stock for this dish');
        await p.click('#cfYes');
        await until(p, id => !document.querySelector(`.orow[data-o="${id}"]`), x.id, 15000);
        const o = await ownerOrder(x.id);
        softFail(bad, o?.status === 'REJECTED', `owner REJECTS ${x.name}: API ${o?.status}`);
        softFail(bad, /out of stock/.test(String(o?.rejection_reason ?? '')), `the owner's reason is stored: ${JSON.stringify(o?.rejection_reason ?? null)}`);
      } else {
        await p.click(`.orow [data-act="confirm"][data-o="${x.id}"]`);
        await until(p, id => document.querySelector(`.orow[data-o="${id}"]`)?.dataset.st === 'CONFIRMED', x.id, 15000);
        const o = await ownerOrder(x.id);
        softFail(bad, o?.status === 'CONFIRMED', `owner CONFIRMS ${x.name}: row ${await p.$eval(`.orow[data-o="${x.id}"]`, e => e.dataset.st).catch(() => 'gone')}, API ${o?.status}`);
      }
    }
    clean(g, 'F5 owner');
    await inspect(p, 'F5 owner orders');
    await shot(p, 'F5-owner-done');
  } catch (e) {
    { const pg = ob.contexts()[0]?.pages()[0]; if (pg) e.shot = await shot(pg, 'F5-owner-fail'); }
    throw e;
  } finally { await ob.close().catch(() => {}); }
  endSoft(bad, 'F5');
}

// ── F6 ────────────────────────────────────────────────────────────────────
export async function f6(log) {
  const bad = [];
  const guests = (C().guests || []).filter(x => x.id);
  if (!guests.length) throw new Fail('not run: F5 placed no order');
  const k = await makeStaff('kitchen');
  log(`kitchen member invited and claimed through /api/owner/staff/invite and /api/staff/claim (${k.email})`);
  const work = guests.filter(x => ['A', 'B', 'C', 'D'].includes(x.key));
  const K = guests.find(x => x.key === 'K');
  const b = await launch();
  try {
    const ctx = await b.newContext({ viewport: { width: 390, height: 844 }, serviceWorkers: 'block' }); await routeLocal(ctx);
    const p = await ctx.newPage(); const g = guard(p, 'kitchen');
    // EXPECTED, BY DESIGN: admin/signin.js asks the owner door first; a member of staff is refused there
    // (403 "no active owner membership") and then signed in at /api/staff/login.
    g.expect('POST', /\/api\/auth\/login$/, 403);
    await go(p, `${HOST}/admin/`, g);
    await p.waitForSelector('#e', { timeout: 60000 });
    await p.fill('#e', k.email); await p.fill('#p', k.password); await p.click('#go');
    const landed = await until(p, () => { const n = document.getElementById('nav'); return !!n && !n.hidden; }, null, 30000);
    if (!softFail(bad, landed, 'the kitchen member signs in through the /admin form')) throw new Fail(bad[0]);
    const tabs = await p.$$eval('#nav [data-tab]', els => els.map(e => e.dataset.tab));
    softFail(bad, tabs.includes('kitchen') && !tabs.some(t => ['orders', 'couriers'].includes(t)), `the kitchen has its tab and no owner-only tab (orders, couriers): ${tabs.join(',')}`);
    await openTab(p, 'kitchen');
    for (const x of [...work, K].filter(Boolean)) {
      softFail(bad, await p.waitForSelector(`.kds-ticket[data-o="${x.id}"]`, { timeout: 20000 }).then(() => true, () => false), `the kitchen board shows ${x.key}'s ticket`);
    }
    const pii = guests.flatMap(x => [x.name, x.phone.slice(-7)]).concat(guests.filter(x => x.street).map(x => `${x.street} ${x.house}`));
    const txt = await p.evaluate(() => document.body.innerText);
    const seen = pii.filter(w => txt.includes(w));
    softFail(bad, seen.length === 0, `no customer name, phone or street on the kitchen screen (leaks: ${seen.join(', ') || 'none'})`);
    await shot(p, 'F6-board');
    for (const x of work) {
      await p.click(`[data-seen="${x.id}"]`, { timeout: 8000 }).catch(() => {});
      await p.waitForTimeout(1500);
      const o = await ownerOrder(x.id);
      softFail(bad, !!(o?.kitchen && Object.keys(o.kitchen).length), `${x.key}: the "seen" tap is read back by the owner (${JSON.stringify(o?.kitchen || null).slice(0, 80)})`);
      for (const want of ['PREPARING', 'READY']) {
        // the ticket redraws after every tap (kitchen.js refresh): tap the bump that names THIS step
        const sel = `[data-bump="${x.id}"][data-act="${want.toLowerCase()}"]`;
        await p.waitForSelector(sel, { timeout: 15000 }).catch(() => {});
        await p.click(sel, { timeout: 8000 }).catch(e => console.log(`  !! bump ${x.key}: ${String(e.message).split('\n')[0]}`));
        let s = null;
        for (let i = 0; i < 6 && s !== want; i++) { await p.waitForTimeout(1500); s = (await ownerOrder(x.id))?.status; }
        softFail(bad, s === want, `${x.key}: kitchen bump -> ${want}: API ${s}`);
      }
    }
    // K: the kitchen rejects with a reason (admin/kitchen.js askReason)
    if (K) {
      await p.click(`[data-stop="${K.id}"]`);
      await p.waitForSelector('#kdsWhy', { timeout: 10000 });
      await p.fill('#kdsWhy', 'QA flows gate: the fryer is down');
      await p.click('[data-ui-close="yes"]');
      let o = null;
      for (let i = 0; i < 6 && o?.status !== 'REJECTED'; i++) { await p.waitForTimeout(1500); o = await ownerOrder(K.id); }
      softFail(bad, o?.status === 'REJECTED' && /fryer is down/.test(String(o?.rejection_reason || '')), `K: the kitchen REJECTS with a reason: API ${o?.status} ${JSON.stringify(o?.rejection_reason ?? null)}`);
    }
    const kt = await api(`/api/staff/kitchen?location_id=${LOC}`, { token: await staffJwt(k) });
    const kj = JSON.stringify(kt.body);
    const leak = pii.filter(w => w && kj.includes(w));
    softFail(bad, kt.status === 200 && leak.length === 0, `the kitchen API carries no customer name, phone or street (${kt.status}${leak.length ? `, leaks ${leak.join(', ')}` : ''})`);
    clean(g, 'F6 kitchen');
    await inspect(p, 'F6 kitchen');
    await shot(p, 'F6-kitchen-ready');
  } catch (e) {
    { const pg = b.contexts()[0]?.pages()[0]; if (pg) e.shot = await shot(pg, 'F6-fail'); }
    throw e;
  } finally { await b.close().catch(() => {}); }
  await guestSeesRejections(bad, guests.filter(x => x.key === 'R' || x.key === 'K'));
  endSoft(bad, 'F6');
}

/// The guest's own phone (F5's storage) opens each rejected order from "my orders": the page must say
/// REJECTED and carry the reason the venue gave, as the API does.
async function guestSeesRejections(bad, list) {
  if (!list.length || !S.guestState) return;
  const b = await launch();
  try {
    const ctx = await b.newContext({ ...PHONE, storageState: S.guestState }); await routeLocal(ctx);
    const p = await ctx.newPage(); const g = guard(p, 'guest-rejected');
    await go(p, `${HOST}/`, g);
    await p.waitForSelector('.card', { timeout: 60000 }).catch(() => {});
    await p.waitForTimeout(2000);
    await dismiss(p);
    for (const x of list) {
      const a = await api(`/api/order/${x.id}`, { token: x.token });
      const why = String(a.body?.rejection_reason || '');
      await p.evaluate(() => document.getElementById('scrim')?.click()).catch(() => {});
      await p.click('#nav [data-tab="orders"]');
      // a row is `#<first 8 of the id>` (store/nav.js openHistory; data-o is its index)
      const row = p.locator('button.hist', { hasText: `#${x.id.slice(0, 8)}` });
      const there = await row.first().waitFor({ timeout: 20000 }).then(() => true, () => false);
      if (!softFail(bad, there, `guest ${x.key}: "my orders" lists the rejected order`)) continue;
      await row.first().click();
      await until(p, () => !!document.querySelector('.tsheet[data-status]'), null, 20000);
      const st = await p.$eval('.tsheet', e => e.dataset.status).catch(() => null);
      const page = await p.$eval('.tsheet', e => e.innerText).catch(() => '');
      softFail(bad, st === 'REJECTED' && a.body?.status === 'REJECTED', `guest ${x.key}: status page ${st} = API ${a.body?.status}`);
      const f = await shot(p, `F6-guest-sees-${x.key}`);
      (C().rejectSeen ||= {})[x.key] = { api: a.body?.status, page: st, reason: why || null, pageHasReason: !!why && page.includes(why.slice(0, 20)), shot: f }; save();
      softFail(bad, !!why && page.includes(why.slice(0, 20)),
        `PRODUCT: guest ${x.key} sees the venue's reason on the status page: API rejection_reason ${JSON.stringify(why || null)}; page text has it: ${!!why && page.includes(why.slice(0, 20))} (store/track.js draws no rejection_reason) [${f}]`);
      await closeSheet(p).catch(() => {});
    }
    clean(g, 'F6 guest');
  } finally { await b.close().catch(() => {}); }
}
