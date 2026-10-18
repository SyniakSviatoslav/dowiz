// F10 FAILURES, simulated in the browser only (Playwright's setOffline / route / routeWebSocket), never
// on the server -- except the venue pause, which is the owner's own switch and is put back at once.
//   a) the guest's connection drops at the checkout submit; back online, one more tap: ONE order
//   b) the order POST answers 503 once; the guest retries: ONE order, the page does not claim success
//   c) courier offline between picked-up and delivered   -- run inside F7 on guest B, judged here
//   d) waiter offline while placing a round               -- run inside F8 on table 92, judged here
//   e) the owner console's socket dropped: a new order still appears, without a reload, within the
//      console's own refresh (admin/core.js POLL_MS 15 s busy / POLL_IDLE_MS 60 s idle)
//   f) an 86'd dish: the API refuses it and no order exists
//   g) refusals, in words: refused at the door (F7 D, money back), the kitchen's reason (F6 K), and
//      a PAUSED venue: the storefront refuses in words and the API with 409; the pause is undone.
import { HOST, LOC, RUN, S, save, Fail, api, own, menu, dishes, opened, ownerOrders } from './lib.mjs';
import { launch, go, PHONE, routeLocal, guard, clean, inspect, shot } from './page.mjs';
import { C, checkoutUI, setLang, openConsole, openTab, info } from './cycle-lib.mjs';

const named = async name => (await ownerOrders()).filter(o => o.contact?.name === name);
const JSONISH = /Unexpected token|is not valid JSON|JSON\.parse|SyntaxError/;

export async function f10(log) {
  const bad = [];
  const check = (ok, msg) => { if (ok) console.log(`  ok   ${msg}`); else { console.log(`  FAIL ${msg}`); bad.push(msg); } return ok; };
  const on = dishes(await menu()).filter(d => d.available && !String(d.id).startsWith('flows-'));
  if (on.length < 4) throw new Fail(`qa-durres lists ${on.length} dishes on sale; F10 needs 4`);
  let veg = null, paused = false;
  let b = await launch();
  try {
    const ctx = await b.newContext(PHONE); await routeLocal(ctx);
    const p = await ctx.newPage(); const g = guard(p, 'store-f10');
    await go(p, `${HOST}/`, g);
    await p.waitForSelector('.card', { timeout: 60000 }).catch(() => {});
    await p.waitForTimeout(2500);
    await setLang(p, 'en');

    // a) offline at the submit, then online and one more tap
    const nameA = `${RUN} offline A`;
    const seen = g.bad.length;
    let r1 = null;
    try { r1 = await checkoutUI(p, { how: 'pickup', ids: [on[0].id], qty: 1, name: nameA, phone: '+355690000071', allowFail: true, beforePlace: () => ctx.setOffline(true) }); } catch (e) { bad.push(`a) ${e.message}`); }
    const shownA = await p.evaluate(() => (document.getElementById('f-err')?.innerText || '').trim());
    await ctx.setOffline(false);
    g.bad.splice(seen);   // the POST the network never carried is the simulated failure itself
    if (r1?.id) opened(r1.id, r1.access_token, 'F10');
    check((await named(nameA)).length === 0 && shownA.length > 0, `a) offline at submit: no order, and the page says so: "${shownA.slice(0, 80)}"`);
    await p.click('#place').catch(() => {}); await p.waitForTimeout(5000);
    const listA = await named(nameA);
    for (const o of listA) opened(o.id, null, 'F10');
    check(listA.length === 1, `a) back online, one more tap (same Idempotency-Key, store/order-key.js): exactly ONE order (api ${listA.length})`);

    // b) one 503 on the order POST, then the guest's retry
    const nameB = `${RUN} retry B`;
    let hits = 0;
    await p.route(u => /\/api\/public\/locations\/[^/]+\/orders$/.test(u.pathname), async route => {
      if (route.request().method() === 'POST' && hits++ === 0) return route.fulfill({ status: 503, contentType: 'application/json', body: '{"error":"QA flows F10: simulated 503"}' });
      return route.continue();
    });
    g.expect('POST', /\/orders$/, 503);
    await p.evaluate(() => document.getElementById('scrim')?.click()).catch(() => {});
    await p.waitForTimeout(800);
    let r2 = null;
    try { r2 = await checkoutUI(p, { how: 'pickup', ids: [on[1].id], qty: 1, name: nameB, phone: '+355690000072', allowFail: true }); } catch (e) { bad.push(`b) ${e.message}`); }
    const shownB = await p.evaluate(() => (document.getElementById('f-err')?.innerText || '').trim());
    const sheetB = await p.evaluate(() => document.getElementById('sheet')?.dataset.name || '');
    check(!r2?.id && sheetB === 'checkout' && /503/.test(shownB), `b) the 503 is said on the checkout, not a status page: "${shownB.slice(0, 80)}"`);
    await p.click('#place').catch(() => {}); await p.waitForTimeout(5000);
    await p.unroute(u => /\/orders$/.test(u.pathname)).catch(() => {});
    const listB = await named(nameB);
    for (const o of listB) opened(o.id, null, 'F10');
    check(hits >= 2 && listB.length === 1, `b) the retry made exactly ONE order (POSTs ${hits}, api ${listB.length})`);

    // f) an 86'd dish cannot be ordered
    veg = on.find(d => d.id === 'qa-veg-roll') || on[2];
    const off = await own(`/api/owner/products/${veg.id}`, { location_id: LOC, available: false });
    check(off.status === 200, `f) the owner takes ${veg.id} off sale (${off.status})`);
    const pub = await api(`/api/public/locations/${LOC}/menu?fresh=1`);
    const still = dishes(pub.body || {}).find(d => d.id === veg.id);
    check(!still || still.available === false, `f) the public menu no longer offers ${veg.id}`);
    const tryOrder = await api(`/api/public/locations/${LOC}/orders`, { method: 'POST', headers: { 'idempotency-key': `${RUN}-f10-86` }, body: {
      contact: { name: `${RUN} 86 F`, phone: '+355690000073' }, fulfilment: { kind: 'pickup' }, items: [{ product_id: veg.id, quantity: 1 }], payment: 'cash' } });
    check(tryOrder.status >= 400 && tryOrder.status < 500, `f) the API refuses an order for the 86'd dish (${tryOrder.status} ${tryOrder.text.slice(0, 100)})`);
    check((await named(`${RUN} 86 F`)).length === 0, `f) no order was created for the 86'd dish`);
    await own(`/api/owner/products/${veg.id}`, { location_id: LOC, available: true }); veg = null;

    // g) the venue paused: the storefront refuses in words; the API with 409
    const was = (await menu()).location || {};
    if (S.restore.deliveryPaused === undefined) { S.restore.deliveryPaused = !!was.deliveryPaused; save(); }
    const pz = await own('/api/owner/location', { location_id: LOC, delivery_paused: true }); paused = pz.status === 200;
    check(paused, `g) the owner pauses the venue (${pz.status})`);
    await go(p, `${HOST}/`, g);
    await p.waitForSelector('.card', { timeout: 60000 }).catch(() => {});
    await p.waitForTimeout(2500);
    const head = await p.evaluate(() => document.body.innerText.replace(/\s+/g, ' ').slice(0, 400));
    g.expect('POST', /\/orders$/, 409);
    const nameP = `${RUN} paused P`;
    let rp = null, stopped = '';
    try { rp = await checkoutUI(p, { how: 'pickup', ids: [on[0].id], qty: 1, name: nameP, phone: '+355690000074', allowFail: true }); }
    catch (e) { stopped = e.message; }
    const shownP = await p.evaluate(() => (document.getElementById('f-err')?.innerText || document.getElementById('toast')?.innerText || '').trim());
    const fP = await shot(p, 'F10g-paused');
    const apiP = await api(`/api/public/locations/${LOC}/orders`, { method: 'POST', body: { contact: { name: nameP, phone: '+355690000074' }, fulfilment: { kind: 'pickup' }, items: [{ product_id: on[0].id, quantity: 1 }], payment: 'cash' } });
    check(apiP.status === 409 && (await named(nameP)).length === 0 && !rp?.id, `g) paused: the API refuses (${apiP.status} "${apiP.text.slice(0, 60)}") and no order exists`);
    info(`g) paused storefront: header reads "${head.slice(0, 120)}"; ${stopped ? `checkout stopped: ${stopped.slice(0, 100)}` : `checkout said "${shownP.slice(0, 120)}"`}`);
    check(!!(stopped || shownP) && !JSONISH.test(shownP), `PRODUCT g) the paused storefront refuses in words: "${(shownP || stopped).slice(0, 120)}"${JSONISH.test(shownP) ? ' -- a JSON parse error: checkout.js reads the hub\'s plain-text refusal with r.json()' : ''} [${fP}]`);
    const un = await own('/api/owner/location', { location_id: LOC, delivery_paused: S.restore.deliveryPaused });
    if (un.status === 200) { paused = false; delete S.restore.deliveryPaused; save(); }
    check(un.status === 200 && !(await menu()).location?.deliveryPaused, `g) the pause is undone (${un.status})`);
    clean(g, 'F10 storefront');
    await inspect(p, 'F10');
  } catch (e) {
    { const pg = b.contexts()[0]?.pages()[0]; if (pg) e.shot = await shot(pg, 'F10-fail'); }
    bad.push(String(e.message || e).slice(0, 200));
  } finally {
    if (veg) await own(`/api/owner/products/${veg.id}`, { location_id: LOC, available: true }).catch(() => {});
    if (paused) await own('/api/owner/location', { location_id: LOC, delivery_paused: false }).then(r => { if (r.status === 200) { delete S.restore.deliveryPaused; save(); } }).catch(() => {});
    await b.close().catch(() => {});
  }

  // e) the owner console with its socket dropped
  b = await launch();
  try {
    const { p, g } = await openConsole(b, 'owner-F10e');
    let block = false; const socks = [];
    await p.routeWebSocket(/\/api\/live/, ws => { if (block) { ws.close(); return; } ws.connectToServer(); socks.push(ws); });
    await p.reload({ waitUntil: 'domcontentloaded' });
    await p.waitForSelector('#nav:not([hidden])', { timeout: 60000 });
    await openTab(p, 'orders');
    await p.waitForTimeout(3000);
    const opened0 = socks.length;
    block = true; for (const s of socks) await s.close().catch(() => {});
    await p.waitForTimeout(1500);
    const nameE = `${RUN} socket E`;
    const t0 = Date.now();
    const pe = await api(`/api/public/locations/${LOC}/orders`, { method: 'POST', headers: { 'idempotency-key': `${RUN}-f10e` }, body: {
      contact: { name: nameE, phone: '+355690000075' }, fulfilment: { kind: 'pickup' }, items: [{ product_id: on[0].id, quantity: 1 }], payment: 'cash' } });
    if (pe.body?.id) opened(pe.body.id, pe.body.access_token, 'F10');
    const appeared = pe.body?.id ? await p.waitForSelector(`.orow[data-o="${pe.body.id}"]`, { timeout: 80000 }).then(() => true, () => false) : false;
    const secs = Math.round((Date.now() - t0) / 1000);
    const fE = await shot(p, 'F10e-socket-dropped');
    check(opened0 > 0, `e) the console had opened its live socket (${opened0}) before it was dropped`);
    check(appeared && secs <= 75, `e) socket dropped: the new order appears without a reload in ${secs} s (console poll 15 s busy / 60 s idle) [${fE}]`);
    C().f10e = { socketsBefore: opened0, appeared, secs }; save();
    g.bad.splice(0).forEach(x => info(`e) while the socket was down: ${x.slice(0, 120)}`));
  } catch (e) { bad.push(`e) ${String(e.message || e).slice(0, 160)}`); }
  finally { await b.close().catch(() => {}); }

  // c, d, and the other refusals: what F6, F7 and F8 recorded
  const c = C().f10c, d = C().f10d, r = C().refusedAtDoor, k = C().rejectSeen?.K;
  check(!!c?.ok, `c) courier offline between picked-up and delivered: ${c ? `queued ${JSON.stringify(c.queued).slice(0, 100)}; API offline ${c.apiOffline} -> ${c.apiAfter}, deliveries ${c.deliveries}` : 'not run (F7 had no order B)'}`);
  check(!!d?.ok, `d) waiter offline while placing a round: ${d ? `said "${String(d.said).slice(0, 60)}", queued-and-landed ${d.queuedLanded}, rounds on the table ${d.rounds92}` : 'not run (F8 did not reach it)'}`);
  check(!!r && ['REFUNDING', 'COMPENSATED_REFUND'].includes(r.status) && r.refund?.reason === 'refused_at_door', `g) refused at the door, money back recorded: ${r ? `${r.status} ${JSON.stringify(r.refund).slice(0, 100)}` : 'not run'}`);
  check(!!k?.api && k.api === 'REJECTED', `g) the kitchen's rejection reaches the guest: API ${k?.api ?? 'not run'}, reason on the page: ${k?.pageHasReason ?? 'not run'} (judged in F6)`);
  if (bad.length) throw new Fail(`${bad.length} F10 check(s) failed: ${bad.slice(0, 3).join(' | ')}`);
  log('every failure case answered honestly');
}
