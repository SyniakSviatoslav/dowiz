// F2 OWNER, in the console at 390x844: sign in; the order F1 placed is on the
// board; accept it and it moves (page row and API both CONFIRMED); rename a QA
// dish and rename it back through the dish sheet (API read-back each time);
// open the stock screen, the semi-finished (ПФ) filter and the ПФ editor --
// and save nothing there. F4 refunds the accepted order.
import { HOST, LOC, RUN, S, save, Fail, creds, own, ownerOrder } from './lib.mjs';
import { launch, go, PHONE, routeLocal, localServed, guard, clean, inspect, shot, until } from './page.mjs';
import { tab, openDish, recipeStep, prepStep } from './f2-components.mjs';

const RENAME = 'qa-water';
const productOf = async id => {
  const r = await own(`/api/owner/products?location_id=${LOC}`);
  if (r.status !== 200) throw new Fail(`GET /api/owner/products ${r.status}`);
  return (r.body?.products || r.body || []).find(x => x.id === id) || null;
};
const prepCount = async () => {
  const r = await own(`/api/owner/preps?location_id=${LOC}`);
  if (r.status !== 200) throw new Fail(`GET /api/owner/preps ${r.status}: ${r.text.slice(0, 100)}`);
  return (r.body?.preps || []).length;
};
const sheetName = p => p.evaluate(() => document.getElementById('sheet')?.dataset.name || '');
async function closeSheet(p) {
  await p.evaluate(() => document.getElementById('scrim')?.click());
  if (!(await until(p, () => !document.getElementById('sheet')?.classList.contains('open') && !document.getElementById('sheet')?.dataset.name, null, 4000))) {
    await p.keyboard.press('Escape'); await p.waitForTimeout(800);
  }
}
async function rename(p, g, to, cat) {
  await openDish(p, RENAME, cat);
  await inspect(p, `F2 dish sheet (${to})`);
  const shown = await p.$eval('#d-name', e => e.value);
  await p.fill('#d-name', to);
  const boxes = await p.$$eval('#d-name', els => els.map(e => e.value));
  let saved = null, sent = null;
  const onResp = r => { if (r.url().includes(`/owner/products/${RENAME}`) && r.request().method() === 'POST') { saved = r.status(); try { sent = JSON.parse(r.request().postData() || '{}'); } catch {} } };
  p.on('response', onResp);
  await p.click('#dSave');
  for (let i = 0; i < 40 && saved === null; i++) await p.waitForTimeout(250);
  p.off('response', onResp);
  if (saved !== 200) throw new Fail(`saving the dish name answered ${saved ?? 'nothing in 10 s'}`);
  const api = (await productOf(RENAME))?.name;
  // The evidence a wrong name needs: what the field showed when the sheet
  // opened, and whether the save carried a name at all.
  if (api !== to) throw new Fail(`renamed to "${to}" on the page, the API says "${api}" (the sheet opened showing "${shown}"; #d-name boxes after typing: ${JSON.stringify(boxes)}; the save sent ${sent && 'name' in sent ? `name "${sent.name}"` : 'NO name'}, translations ${JSON.stringify(sent?.translations || null).slice(0, 120)})`);
  if (!(await until(p, ([id, n]) => (document.querySelector(`[data-p="${id}"]`)?.innerText || '').includes(n), [RENAME, to], 10000)))
    throw new Fail(`the menu row does not show the new name "${to}"`);
  clean(g, `F2 rename to "${to}"`);
}

export async function f2(log) {
  const order = S.orders.find(o => o.by === 'F1');
  if (!order) throw new Fail('no order from F1 to accept (F1 failed before placing one)');
  const before = await ownerOrder(order.id);
  if (before?.status !== 'PENDING') throw new Fail(`F1's order is ${before?.status ?? 'missing'} in the owner's list, want PENDING`);
  const dish = await productOf(RENAME);
  if (!dish) throw new Fail(`qa-durres has no ${RENAME} to rename`);
  if (S.restore.dishName === undefined) { S.restore.dishName = dish.name; save(); }
  const preps0 = await prepCount();

  const b = await launch();
  try {
    const ctx = await b.newContext(PHONE); await routeLocal(ctx);
    const p = await ctx.newPage(); const g = guard(p, 'console');
    await go(p, `${HOST}/admin/`, g);
    await p.waitForSelector('#e', { timeout: 60000 });
    await p.fill('#e', creds.OWNER_EMAIL); await p.fill('#p', creds.OWNER_PASSWORD);
    await p.click('#go');
    if (!(await p.waitForSelector('#nav:not([hidden])', { timeout: 60000 }).then(() => true, () => false))) throw new Fail('the console did not sign in');
    await p.waitForTimeout(2000);
    clean(g, 'F2 sign-in');
    log('console signed in');

    // ── the order F1 placed, accepted ─────────────────────────────────────
    await tab(p, 'orders');
    if (!(await p.waitForSelector(`.orow[data-o="${order.id}"]`, { timeout: 45000 }).then(() => true, () => false))) throw new Fail(`F1's order ${order.id} never reached the console board`);
    const st0 = await p.$eval(`.orow[data-o="${order.id}"]`, e => e.dataset.st);
    if (st0 !== 'PENDING') throw new Fail(`the board shows F1's order as ${st0}, the API as PENDING`);
    await inspect(p, 'F2 orders board');
    const accept = await p.$(`[data-act="confirm"][data-o="${order.id}"]`);
    if (!accept) throw new Fail('the PENDING row has no accept button');
    await accept.click();
    if (!(await until(p, id => document.querySelector(`.orow[data-o="${id}"]`)?.dataset.st === 'CONFIRMED', order.id, 20000)))
      throw new Fail(`after accept the row still says ${await p.$eval(`.orow[data-o="${order.id}"]`, e => e.dataset.st).catch(() => 'gone')}`);
    const st1 = (await ownerOrder(order.id))?.status;
    if (st1 !== 'CONFIRMED') throw new Fail(`page says CONFIRMED, API says ${st1}`);
    if (!(await p.$(`[data-act="preparing"][data-o="${order.id}"]`))) throw new Fail('the accepted row offers no next step (preparing)');
    await inspect(p, 'F2 order accepted');
    clean(g, 'F2 accept');
    log(`order ${order.id}: accepted, row and API both CONFIRMED, next step offered`);

    // ── stock, the ПФ filter, the ПФ editor (nothing saved) ───────────────
    await tab(p, 'stock');
    if (!(await p.waitForSelector('#addPrep', { timeout: 30000 }).then(() => true, () => false))) throw new Fail('the stock screen drew no ПФ button');
    await p.waitForTimeout(3000);
    await inspect(p, 'F2 stock');
    const pf = await p.$('[data-k="prep"]');
    if (!pf) throw new Fail('the stock screen has no semi-finished (ПФ) filter chip');
    await pf.click(); await p.waitForTimeout(1500);
    await inspect(p, 'F2 stock ПФ filter');
    await p.click('#addPrep');
    if (!(await p.waitForSelector('#pf-name', { timeout: 15000 }).then(() => true, () => false))) throw new Fail('the ПФ button opened no editor');
    await inspect(p, 'F2 ПФ editor');
    await closeSheet(p);
    if (await sheetName(p) === 'prepEdit') throw new Fail('the ПФ editor does not close');
    const preps1 = await prepCount();
    if (preps1 !== preps0) throw new Fail(`opening the ПФ editor changed the number of ПФ: ${preps0} -> ${preps1}`);
    clean(g, 'F2 stock');
    log(`stock + ПФ filter + ПФ editor opened; ПФ count unchanged (${preps1})`);

    // ── components: a ПФ card's lines, a dish's recipe ─────────────────────
    await prepStep(p, g, log, async () => {
      await tab(p, 'stock');
      if (!(await p.waitForSelector('#addPrep', { timeout: 30000 }).then(() => true, () => false))) throw new Fail('the stock screen drew no ПФ button');
    });
    await recipeStep(p, g, log);
    // ── rename a QA dish and back ─────────────────────────────────────────
    // (The first gate runs reported the rename BACK as a product bug. It was
    // this flow: it typed into the previous, closed sheet's box. openDish now
    // waits for the OPEN sheet -- see f2-components.mjs.)
    await rename(p, g, `${dish.name} ${RUN}`, dish.categoryId);
    await rename(p, g, dish.name, dish.categoryId);
    log(`dish ${RENAME}: renamed to "${dish.name} ${RUN}" and back, API read-back both times`);

    localServed(ctx, 'F2');
  } catch (e) {
    if (e instanceof Fail) { const pg = b.contexts()[0]?.pages()[0]; if (pg) e.shot = await shot(pg, 'F2-fail'); }
    throw e;
  } finally { await b.close().catch(() => {}); }
}
