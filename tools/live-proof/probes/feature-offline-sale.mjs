// FEATURE offline-sale (W-OFFSALE, OF3; contract ../contracts/feature-offline-sale.json).
//
// A room tablet loses its network, sells QA Water for cash, prints the
// receipt without NIVF, is reloaded OFFLINE (the room service worker serves
// the shell), gets the network back, and the sale lands in the venue ONCE,
// with its own time, and in the fiscal pane with a deadline 48 h from the
// sale. Then the TEST sale is ended. MAIN RUNS THIS AFTER THE DEPLOY:
//
//   node tools/live-proof/probes/feature-offline-sale.mjs
//
// The owner's token signs the room (staff_at takes an owner token for
// Cap::TakePayment), so no waiter credential is needed. One Chromium, one
// context, closed in a finally (e2e/flows/page.mjs). Service workers are
// ALLOWED here (the flows' PHONE blocks them): the offline reload is the point.
import fs from 'node:fs';
import path from 'node:path';

const HERE = path.dirname(new URL(import.meta.url).pathname);
const CONTRACT = JSON.parse(fs.readFileSync(path.join(HERE, '../contracts/feature-offline-sale.json'), 'utf8'));
const DEADLINE_MS = 48 * 3600 * 1000;
const PRODUCT = 'qa-water';

export default async function (ctx) {
  const { lib, check, must, note } = ctx;
  const { launch, until, go, shot } = await import('../../../e2e/flows/page.mjs');
  const token = await lib.owner();
  const menu = await lib.menu();
  must(lib.dishes(menu).some(d => d.id === PRODUCT && d.available !== false), `the QA menu has no available ${PRODUCT} (run e2e/walk/qa-setup.mjs)`);
  const before = (await lib.ownerOrders()).filter(o => String(o.id).startsWith('offline:')).map(o => o.id);

  const browser = await launch();
  let key = null;
  try {
    const context = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 1, isMobile: true, hasTouch: true,
      serviceWorkers: 'allow', reducedMotion: 'reduce', userAgent: lib.UA });
    const session = { jwt: token, staff: { id: 'owner', locationId: lib.LOC, role: 'owner', caps: 'take_orders,take_payment,open_till', expiresMs: Date.now() + 3600_000 } };
    await context.addInitScript(s => { try { if (!localStorage.getItem('dw_room_session')) localStorage.setItem('dw_room_session', s); } catch {} }, JSON.stringify(session));
    const page = await context.newPage();
    // An uncaught error anywhere on the way is a failure (offline request failures are the point, so only these count).
    const uncaught = [];
    page.on('pageerror', e => uncaught.push(String(e.message).split('\n')[0].slice(0, 160)));
    await go(page, `${lib.HOST}/room/`);
    must(await until(page, () => !!localStorage.getItem('dw_room_menu'), null, 30000), 'online: the room never saved its menu (dw_room_menu)');
    // The service worker must CONTROL the page before the network goes, or the offline reload has nothing to serve it.
    must(await until(page, () => navigator.serviceWorker?.ready.then(() => true), null, 30000), 'the room service worker never became ready');
    await page.reload({ waitUntil: 'domcontentloaded' });
    must(await until(page, () => !!navigator.serviceWorker?.controller, null, 20000), 'the room service worker does not control the page');

    // ── OFFLINE ──
    await context.setOffline(true);
    must(await until(page, () => !document.querySelector('#offlineBanner')?.hidden, null, 15000), 'offline: the banner never showed');
    must(await until(page, () => !!document.querySelector('[data-act="sell"]'), null, 15000), 'offline: no Cash sale button in the room');
    await page.click('[data-act="sell"]');
    must(await until(page, id => !!document.querySelector(`[data-act="pick"][data-id="${id}"]`), PRODUCT, 15000), `offline: ${PRODUCT} is not in the cached menu`);
    await page.click(`[data-act="pick"][data-id="${PRODUCT}"]`);
    await page.click('[data-act="send"]');
    must(await until(page, () => !!document.querySelector('[data-act="sellCommit"]'), null, 10000), 'offline: no confirmation screen');
    await page.click('[data-act="sellCommit"]');
    must(await until(page, () => (document.querySelector('pre.receipt')?.textContent || '').includes('pa NIVF'), null, 10000), 'offline: the receipt does not say pa NIVF');
    note(`receipt: ${(await page.textContent('pre.receipt')).replace(/\s+/g, ' ').slice(0, 160)}`);
    await shot(page, 'offline-receipt');
    const sale = await page.evaluate(() => new Promise((res, rej) => {
      const r = indexedDB.open('dowiz.room.sales', 1);
      r.onsuccess = () => { const q = r.result.transaction('sales').objectStore('sales').getAll(); q.onsuccess = () => res(q.result); q.onerror = () => rej(q.error); };
      r.onerror = () => rej(r.error);
    }));
    const mine = sale.filter(s => s.status === 'queued' && s.lines?.some(l => l.product_id === PRODUCT)).sort((a, b) => b.sold_at_ms - a.sold_at_ms)[0];
    must(mine, `the sale is not in the tablet journal (IndexedDB dowiz.room.sales): ${JSON.stringify(sale).slice(0, 160)}`);
    key = mine.sale_key;
    const { status: _s, said: _w, ...body } = mine;
    check('request_schema', body);
    // The receipt is drawn BEFORE the send is tried (sell.js), so the outbox entry follows it: wait for it.
    const inOutbox = k => new Promise(res => {
      const r = indexedDB.open('dowiz.room.outbox'); r.onsuccess = () => { const q = r.result.transaction('queue').objectStore('queue').getAll(); q.onsuccess = () => res(q.result.some(e => e.key === k)); q.onerror = () => res(false); };
      r.onerror = () => res(false);
    });
    must(await until(page, inOutbox, key, 10000), `the sale ${key} never reached the outbox (dowiz.room.outbox)`);

    // ── RELOAD, STILL OFFLINE: the shell must render from the service worker ──
    await page.reload({ waitUntil: 'domcontentloaded' }).catch(e => { throw new ctx.Fail(`offline reload: ${String(e.message).split('\n')[0]}`); });
    must(await until(page, () => (document.querySelector('#app')?.children.length || 0) > 0 && !document.querySelector('#offlineBanner')?.hidden, null, 20000),
      'offline reload: the shell did not render (service worker)');
    await shot(page, 'offline-reload');

    // ── ONLINE AGAIN: exactly once ──
    await context.setOffline(false);
    let landed = [];
    for (let i = 0; i < 30 && !landed.length; i++) {
      await page.waitForTimeout(2000);
      landed = (await lib.ownerOrders()).filter(o => o.id === `offline:${key}`);
    }
    must(landed.length === 1, `online: offline:${key} landed ${landed.length} time(s) in /api/owner/orders`);
    const o = landed[0];
    const at = o.created_at_ms ?? o.createdAtMs ?? o.offline?.sold_at_ms;
    must(o.status === 'PICKED_UP' && at === mine.sold_at_ms,
      `the order is ${o.status} at ${at}; wanted PICKED_UP at the sale's own ${mine.sold_at_ms}`);
    await page.reload({ waitUntil: 'domcontentloaded' });
    await page.waitForTimeout(5000);

    // A REPLAY under the same key: the same sale, still one order.
    const again = await lib.api('/api/staff/offline_sales', { method: 'POST', token, headers: { 'idempotency-key': key }, body });
    must(again.status === 200 && again.body?.replayed === true, `replay: ${again.status} ${again.text}`);
    check('response_schema', again.body);
    const after = (await lib.ownerOrders()).filter(x => x.id === `offline:${key}`).length;
    const fresh = (await lib.ownerOrders()).filter(x => String(x.id).startsWith('offline:') && !before.includes(x.id)).length;
    must(after === 1 && fresh === 1, `after a reload and a replay: ${after} order(s) offline:${key}, ${fresh} new offline order(s)`);

    // The fiscal queue: its deadline is 48 h from the SALE.
    const pane = await lib.own(`/api/owner/offline_sales?location_id=${lib.LOC}`);
    must(pane.status === 200, `GET /api/owner/offline_sales ${pane.status} ${pane.text}`);
    check('readback_schema', pane.body);
    const it = (pane.body.items || []).find(x => x.order_id === `offline:${key}`);
    must(it && it.deadline_ms === mine.sold_at_ms + DEADLINE_MS, `pane: ${JSON.stringify(it)}; wanted deadline_ms = ${mine.sold_at_ms + DEADLINE_MS}`);
    must(pane.body.send_enabled === false, 'the platform send switch reads ON');
    const fx = await lib.own(`/api/owner/fiscal?location_id=${lib.LOC}`);
    const inQueue = JSON.stringify(fx.body?.waiting || []).includes(`offline:${key}`);
    note(`landed once as offline:${key} (PICKED_UP, created_at = sold_at ${mine.sold_at_ms}); replay answered replayed=${again.body.replayed}; ` +
      `pane fiscal=${it.fiscal} deadline=${it.deadline_ms}; /api/owner/fiscal waiting ${inQueue ? 'lists it' : 'does NOT list it'}; conflicts ${JSON.stringify(it.conflicts)}`);
    must(it.fiscal === 'queued' && inQueue, `the fiscal queue does not hold it (pane fiscal=${it.fiscal}, fiscal pane lists it: ${inQueue})`);
    must(!uncaught.length, `the room threw: ${uncaught.join(' | ')}`);
    await context.close();
  } finally {
    await browser.close().catch(() => {});
    if (key) {
      // END THE TEST SALE: the refund route is the kernel's exit for an order that took money.
      // PICKED_UP is TERMINAL (e2e/flows/f4-cleanup.mjs: DELIVERED stays in the history);
      // if the route refuses, the answer is recorded and the TEST sale stays, named by its key.
      const r = await lib.api(`/api/staff/orders/${encodeURIComponent('offline:' + key)}/refund`, { method: 'POST', token,
        headers: { 'idempotency-key': `${lib.RUN}-refund-${key}` }, body: { location_id: lib.LOC, reason: 'venue_cancelled', note: 'live-proof offline-sale cleanup' } });
      note(`cleanup: refund of offline:${key} answered ${r.status} ${String(r.text).slice(0, 100)}`);
    }
  }
}

// RUN DIRECTLY: one JSON line in run.mjs's shape, exit 0 only on LIVE-PROVEN.
if (import.meta.url === `file://${process.argv[1]}`) {
  process.env.FLOWS_HOST ||= process.env.LIVE_HOST || 'https://qa-durres.dowiz.org';
  const lib = await import('../../../e2e/flows/lib.mjs');
  const { validate } = await import('../lib/schema.mjs');
  class Fail extends Error {}
  const line = { row: CONTRACT.id, name: CONTRACT.name, status: 'FAILED', evidence: [], at: new Date().toISOString(), contract: CONTRACT.contract_version };
  const v = await lib.api('/api/version');
  line.build = v.body?.commit || `unknown (${v.status})`;
  let checked = 0; const bad = [];
  const ctx = { lib, run: lib.RUN, Fail, note: s => line.evidence.push(s),
    check(name, value) { checked++; const e = validate(value, CONTRACT[name]); if (e.length) bad.push(`${name}: ${e.slice(0, 4).join('; ')}`); return value; },
    must(ok, why) { if (!ok) throw new Fail(why); } };
  try {
    await (await import(import.meta.url)).default(ctx);
    line.status = bad.length || !checked ? 'FAILED' : 'LIVE-PROVEN';
  } catch (e) { line.evidence.push(`${e instanceof Fail ? '' : 'CRASH '}${e.message}`); }
  if (bad.length) line.evidence.push(`SCHEMA ${bad.join(' | ')}`);
  console.log(JSON.stringify(line));
  process.exit(line.status === 'LIVE-PROVEN' ? 0 : 1);
}
