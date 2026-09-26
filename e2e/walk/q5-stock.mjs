// Q5 STOCK on the QA hub -- what an order does to its recipe's ingredients, today (before W-INV lands).
// QA Salmon roll = 80 g QA Salmon + 120 g QA Sushi rice (qa-setup.mjs). One order of one roll is walked
// PENDING -> CONFIRMED -> PREPARING -> READY, a second is rejected; the shelf is read after every step,
// and the console's stock tab is looked at once.
//   HOST=https://qa-durres.dowiz.org LOC=qa-durres OUT=<dir> slot.sh qa node e2e/walk/q5-stock.mjs
import { HOST, OUT, LOC, browser, csp, step, end, api } from './_lib.mjs';
import { PHONE, qaOwnerToken, placeOrder, ownerOrder, stockOf, ownerConsole } from './_qa.mjs';

if (!/qa-durres/.test(HOST)) { console.log('Q5 writes: QA hub only'); process.exit(2); }
const shot = n => `${OUT}/q5-${n}.png`;
const ITEMS = ['qa-salmon', 'qa-rice'];
const RECIPE = { 'qa-salmon': 80, 'qa-rice': 120 };
let b = null;
try {
  const tok = await qaOwnerToken();
  const shelf = async () => Object.fromEntries(await Promise.all(ITEMS.map(async i => { const s = await stockOf(tok, i); return [i, s && { onHand: s.onHand, reserved: s.reserved, available: s.available, counted: s.counted }]; })));
  const act = async (id, action, extra = {}) => { let r; for (let i = 0; i < 6; i++) { r = await api(`/api/owner/orders/${id}/action`, { method: 'POST', token: tok, body: { location_id: LOC, action, ...extra } }); if (r.status !== 503) break; } return r; };
  const diff = (a, z) => ITEMS.map(i => `${i}: onHand ${a[i]?.onHand}->${z[i]?.onHand} (${z[i]?.onHand - a[i]?.onHand}), reserved ${a[i]?.reserved}->${z[i]?.reserved} (${z[i]?.reserved - a[i]?.reserved})`).join('; ');

  const s0 = await shelf();
  step(ITEMS.every(i => s0[i]), 'the shelf before', JSON.stringify(s0));
  let o = null; for (let i = 0; i < 6 && !o?.id; i++) o = await placeOrder('q5');
  const s1 = await shelf();
  step(null, 'PLACED (PENDING): what one salmon roll did to the shelf', diff(s0, s1));
  const heldAtPlace = ITEMS.every(i => s1[i].reserved - s0[i].reserved === RECIPE[i]);
  step(heldAtPlace, 'placing reserves the recipe (80 g salmon, 120 g rice)', diff(s0, s1));
  let prev = s1;
  for (const a of ['confirm', 'preparing', 'ready']) {
    const r = await act(o.id, a);
    const sn = await shelf();
    step(r.status === 200, `${a}: ${(await ownerOrder(tok, o.id))?.status}`, `${r.status} :: ${diff(prev, sn)}`);
    prev = sn;
  }
  const consumed = ITEMS.every(i => s0[i].onHand - prev[i].onHand === RECIPE[i] || prev[i].reserved - s0[i].reserved === RECIPE[i]);
  step(consumed, 'at READY the recipe is still held or taken off the shelf, never lost', diff(s0, prev));

  // A rejected order gives its reservation back.
  let x = null; for (let i = 0; i < 6 && !x?.id; i++) x = await placeOrder('q5x');
  const sx = await shelf();
  const rj = await act(x.id, 'reject', { reason: 'QA walk Q5: release check' });
  const sy = await shelf();
  step(rj.status === 200 && ITEMS.every(i => sy[i].reserved === sx[i].reserved - RECIPE[i]), 'a rejected order gives its reservation back', `${rj.status} :: ${diff(sx, sy)}`);

  b = await browser();
  const ctx = await b.newContext(PHONE);
  const c = await ownerConsole(ctx, 'q5');
  await c.click('#nav [data-tab="stock"]'); await c.waitForTimeout(3500);
  const txt = await c.$eval('#app', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
  await c.screenshot({ path: shot('stock-tab'), fullPage: true });
  step(/QA Salmon/.test(txt) && /QA Sushi rice/.test(txt), 'the stock tab lists the QA supplies', txt.slice(0, 300));
  await csp(c, 'q5');
} catch (e) {
  step(false, 'Q5 threw', e.stack?.slice(0, 400));
}
await end(b);
