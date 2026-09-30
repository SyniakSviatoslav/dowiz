// F3 COURIER, in the courier app at 390x844: a delivery order (placed and
// walked to READY through the API -- F2 already drove the console's buttons)
// reaches the courier's pool; the courier claims it, picks it up and delivers
// it, and after each tap the API must say what the app says.
//
// The credential is QA_HUB_COURIER_* (a qa-durres courier). It is checked to
// belong to THIS venue before a browser opens (memory
// courier-login-venue-from-host): a foreign courier signs in happily and then
// shows another venue's pool, which looks exactly like a product bug.
//
// DELIVERED IS TERMINAL (order_machine.rs allowed_next: Delivered => []): the
// order this flow delivers stays in qa-durres's history as a cash sale. F4
// leaves it there and says so; everything else it closes.
import { HOST, LOC, RUN, S, save, Fail, api, own, owner, menu, dishes, opened, statusOf, courierCreds, ownerOrder } from './lib.mjs';
import { launch, go, PHONE, routeLocal, localServed, guard, clean, inspect, shot, until } from './page.mjs';

export async function f3(log) {
  const { phone, password } = courierCreds();
  if (!phone || !password) throw new Fail('no QA_HUB_COURIER_PHONE/PASSWORD in the credentials file -- the courier flow cannot run');
  const lg = await api('/api/courier/auth/login', { method: 'POST', body: { phone, password } });
  const me = lg.body?.courier;
  if (lg.status !== 200 || me?.locationId !== LOC) throw new Fail(`the QA courier does not belong to ${LOC}: login ${lg.status} at=${me?.locationId ?? '?'}`);
  log(`courier ${String(me.id).slice(0, 8)} belongs to ${LOC}`);

  // ONE SCREEN, ONE JOB: while the courier holds an order the app shows only
  // that order and never the pool (courier/app.js render: `S.mine[0]`). An
  // order another walk left in this courier's hands therefore blocks the flow
  // -- measured 2026-09-30: f5418efd, "QA Guest Arbenita", READY since
  // 2026-09-26. It is NOT this run's to close unless the caller says so.
  const tasks = await api('/api/courier/tasks', { token: lg.body.jwt });
  for (const held of (tasks.body?.mine || []).filter(o => !S.orders.some(x => x.id === o.id))) {
    const who = `${held.id} (${held.status}, "${held.contact?.name ?? ''}")`;
    if (process.env.FLOWS_FREE_COURIER !== '1' || !/^QA /.test(held.contact?.name || ''))
      throw new Fail(`precondition: the QA courier already carries ${who}, not this run's; the app will show nothing else. Free it (console refund) or run with FLOWS_FREE_COURIER=1`);
    const r = await api(`/api/staff/orders/${held.id}/refund`, { method: 'POST', token: await owner(),
      headers: { 'idempotency-key': `${RUN}-free-${held.id}` }, body: { location_id: LOC, reason: 'venue_cancelled', note: 'QA flows gate: freed the QA courier (FLOWS_FREE_COURIER=1)' } });
    if (r.status !== 200) throw new Fail(`freeing the QA courier from ${who}: refund ${r.status} ${r.text.slice(0, 100)}`);
    log(`FLOWS_FREE_COURIER=1: refunded ${who}, which held the QA courier`);
  }

  // ── a delivery order, walked to READY by the owner's API ────────────────
  const dish = dishes(await menu()).find(d => d.available);
  const placed = await api(`/api/public/locations/${LOC}/orders`, { method: 'POST', headers: { 'idempotency-key': `${RUN}-f3` }, body: {
    contact: { name: `${RUN} courier`, phone: '+355690000009' },
    fulfilment: { kind: 'delivery', address: { line: 'Rruga Taulantia 12, Durres', note: 'QA flows gate -- not a real order' } },
    items: [{ product_id: dish.id, quantity: 1 }], payment: 'cash',
  } });
  if (placed.status >= 400 || !placed.body?.id) throw new Fail(`placing the delivery order: ${placed.status} ${placed.text.slice(0, 160)}`);
  const id = placed.body.id;
  opened(id, placed.body.access_token, 'F3');
  for (const a of ['confirm', 'preparing', 'ready']) {
    const r = await own(`/api/owner/orders/${id}/action`, { action: a, location_id: LOC });
    if (r.status !== 200) throw new Fail(`owner ${a} on the delivery order: ${r.status} ${r.text.slice(0, 120)}`);
  }
  if ((await statusOf(id)) !== 'READY') throw new Fail(`the delivery order is ${await statusOf(id)}, want READY`);
  log(`delivery order ${id} placed and READY`);

  const b = await launch();
  try {
    const ctx = await b.newContext({ ...PHONE, permissions: ['geolocation'], geolocation: { latitude: 41.3225, longitude: 19.445 } });
    await routeLocal(ctx);
    const k = await ctx.newPage(); const g = guard(k, 'courier');
    k.on('dialog', d => d.accept());
    await go(k, `${HOST}/courier/`, g);
    await k.waitForSelector('#em', { timeout: 60000 });
    await k.fill('#em', phone); await k.fill('#pw', password); await k.click('#go');
    if (!(await until(k, () => !document.getElementById('em'), null, 30000))) throw new Fail('the courier app did not sign in');
    await k.waitForTimeout(2500);
    // A first visit opens the 5-step tour over the screen; a courier skips it.
    const skip = await k.$('[data-gd-act="skip"]');
    if (skip) { await skip.click(); await k.waitForTimeout(800); log('the first-visit tour was skipped'); }
    const shift = await k.$('#openShift');
    if (shift) {
      if (S.restore.courierShift === undefined) { S.restore.courierShift = 'opened-by-F3'; save(); }
      await shift.click();
      if (!(await until(k, () => !document.getElementById('openShift'), null, 20000))) throw new Fail('opening the shift did not take');
    }
    await inspect(k, 'F3 on shift');
    clean(g, 'F3 sign-in');
    log(`courier signed in, on shift${shift ? ' (opened now)' : ' (was on)'}`);

    // ── claim ─────────────────────────────────────────────────────────────
    if (!(await k.waitForSelector(`[data-sel="${id}"]`, { timeout: 45000 }).then(() => true, () => false)))
      throw new Fail(`the READY delivery ${id} never reached the courier's pool`);
    await k.click(`[data-sel="${id}"]`); await k.waitForTimeout(600);
    await inspect(k, 'F3 pool');
    const take = await k.$('#take');
    if (!take) throw new Fail('the pool card offers no take button');
    await take.click();
    if (!(await k.waitForSelector('#pick', { timeout: 60000 }).then(() => true, () => false))) throw new Fail('after take the app offers no pickup');
    const o1 = await ownerOrder(id);
    if (o1?.courier_id !== me.id) throw new Fail(`app shows the order in hand, API courier_id=${o1?.courier_id} status=${o1?.status}`);
    log(`claimed: API courier_id is this courier, status ${o1.status}`);

    // ── pick up ───────────────────────────────────────────────────────────
    await inspect(k, 'F3 in hand');
    await k.click('#pick');
    if (!(await k.waitForSelector('#done', { timeout: 60000 }).then(() => true, () => false))) throw new Fail('after pickup the app offers no "delivered"');
    const s2 = await statusOf(id);
    if (s2 !== 'IN_DELIVERY') throw new Fail(`app shows the delivery screen, API says ${s2}`);
    await inspect(k, 'F3 in delivery');
    log('picked up: API IN_DELIVERY');

    // ── deliver (cash) ────────────────────────────────────────────────────
    await k.click('#done');
    if (await k.waitForSelector('#got', { timeout: 10000 }).then(() => true, () => false)) {
      await inspect(k, 'F3 cash question');
      await k.click('#confirm');
    }
    if (!(await until(k, () => !document.getElementById('done') && !document.getElementById('got'), null, 60000))) throw new Fail('the delivery screen did not close after delivering');
    let s3 = await statusOf(id);
    if (s3 !== 'DELIVERED') { await k.waitForTimeout(3000); s3 = await statusOf(id); }
    if (s3 !== 'DELIVERED') throw new Fail(`the app closed the delivery, API says ${s3}`);
    if (await k.$(`[data-sel="${id}"]`)) throw new Fail('the delivered order is still offered in the pool');
    await inspect(k, 'F3 delivered');
    clean(g, 'F3 delivered');
    log('delivered: API DELIVERED, the app no longer offers it');
    localServed(ctx, 'F3');
  } catch (e) {
    if (e instanceof Fail) { const pg = b.contexts()[0]?.pages()[0]; if (pg) e.shot = await shot(pg, 'F3-fail'); }
    throw e;
  } finally { await b.close().catch(() => {}); }
}
