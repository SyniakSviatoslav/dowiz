// TZ 2026-10-12 item 6 in a REAL browser (lane W-TZFRESH): "automatic data sync
// after saving, so the page never needs F5". Chromium on qa-durres, the owner
// console, ONE page, never reloaded:
//
//   rename  a qa-tz- supply is renamed on the Stock screen; the dish that uses
//           it is opened -> the recipe line shows the NEW name (and the console
//           asked the hub for the shelf again: a fresh GET /owner/stock).
//   price   its list price is changed -> the recipe line shows the NEW cost.
//   delete  it is deleted (in use) -> the menu row wears the review badge, the
//           dish sheet names the removed ingredient.
//
// On HEAD before this lane every one of those showed the OLD answer for up to
// 60 s (menu.js SUPPLY_BOOK_MS / REVIEW_MS). Each step is timed: a step that
// took longer than the old cache's life proves nothing and FAILS loudly.
//
// FLOWS_LOCAL=<workers/api/public> serves the local UI over the deployed Worker
// (page.mjs routeLocal). qa-* venues only. Everything made here is deleted after.
//
//   FLOWS_LOCAL=/root/lanes/w-tzfresh/workers/api/public node e2e/tz/tzfresh-browser.mjs
import { HOST, LOC, creds, Fail, own } from '../flows/lib.mjs';
import { launch, PHONE, routeLocal, localServed, guard, go, clean, shot, until } from '../flows/page.mjs';
import { tab, openDish } from '../flows/f2-components.mjs';

const CAT = 'qa-rolls';
const RUN = Date.now().toString(36);
const SUP = `qa-tz-fresh-${RUN}`;
const NAME_A = `QA tzfresh A ${RUN}`, NAME_B = `QA tzfresh B ${RUN}`;
const COST_A = 100, COST_B = 250;   // list price per 100 g; the recipe takes 100 g, so the line's cost IS the list price
const QTY_G = 100;
/// The old cache's life: a step slower than this would have passed on HEAD too.
const OLD_CACHE_MS = 60_000, STEP_MAX_MS = 50_000;
const log = m => console.log(`  tzfresh: ${m}`);
const fails = [];
const check = (ok, what) => { if (ok) log(`ok   ${what}`); else { fails.push(what); log(`FAIL ${what}`); } };

async function signIn(p, g) {
  await go(p, `${HOST}/admin/`, g);
  if (await p.$('#nav:not([hidden])')) return;
  await p.waitForSelector('#e', { timeout: 60000 });
  await p.fill('#e', creds.OWNER_EMAIL); await p.fill('#p', creds.OWNER_PASSWORD);
  await p.click('#go');
  if (!(await p.waitForSelector('#nav:not([hidden])', { timeout: 60000 }).then(() => true, () => false))) throw new Fail('the console did not sign in');
  await p.waitForTimeout(1500);
}

/// The supply and a dish of one line on it, by the API; returns the dish id.
async function setUp() {
  const s = await own('/api/owner/supplies', { id: SUP, name: NAME_A, unit: 'g', kind: 'food_ingredient', category: 'QA', costPerBasis: COST_A, location_id: LOC });
  if (s.status !== 200) throw new Fail(`making the supply answered ${s.status}: ${s.text}`);
  const d = await own('/api/owner/products', { location_id: LOC, category_id: CAT, name: `QA tzfresh dish ${RUN}`, price: 500, available: false });   // off sale: an undeclared dish may not go on sale (W-MR0)
  if (d.status !== 200 || !d.body?.id) throw new Fail(`making the dish answered ${d.status}: ${d.text}`);
  const r = await own(`/api/owner/products/${d.body.id}`, { location_id: LOC, bom: [{ supply: SUP, qty: QTY_G }], allergens: [], available: true });   // declared, so it sits in its category row, not only in the undeclared panel
  if (r.status !== 200) throw new Fail(`writing the recipe answered ${r.status}: ${r.text}`);
  return d.body.id;
}
async function tearDown(dish) {
  if (dish) { const r = await own(`/api/owner/products/${dish}/delete`, { location_id: LOC }); log(`cleanup: dish ${dish} delete -> ${r.status}`); }
  const r = await own('/api/owner/supplies/delete', { ids: [SUP], location_id: LOC, confirmUses: true });
  log(`cleanup: supply ${SUP} delete -> ${r.status} (404/409 = already gone)`);
  const left = (await own('/api/owner/stock')).body?.supplies?.some(s => s.id === SUP);
  check(!left, `cleanup: ${SUP} is gone from the shelf`);
}

/// The recipe line of the dish as the sheet draws it: { name, metaDigits }.
async function recipeLine(p, dish) {
  await openDish(p, dish, CAT);
  if (!(await p.waitForSelector('#sheet.show #rcLines .rc-line', { timeout: 30000 }).then(() => true, () => false))) throw new Fail('the recipe lines never drew');
  await p.waitForTimeout(800);
  const line = await p.$eval('#sheet.show #rcLines .rc-line', l => ({ name: l.querySelector('.t b')?.textContent || '', meta: l.querySelector('[data-rmeta]')?.textContent || '' }));
  const review = await p.$eval('#sheet.show #rcReviewBox', b => b.textContent.trim()).catch(() => '');
  await p.keyboard.press('Escape'); await p.waitForTimeout(500);
  return { ...line, metaDigits: line.meta.replace(/\D/g, ''), review };
}

/// Open the supply's editor from the Stock screen (search, row, card, Edit).
async function openSupplyEditor(p) {
  await tab(p, 'stock');
  await p.fill('#sq', SUP); await p.waitForTimeout(600);
  if (!(await p.waitForSelector(`[data-s="${SUP}"]`, { timeout: 20000 }).then(() => true, () => false))) throw new Fail(`the stock screen lists no ${SUP}`);
  await p.click(`[data-s="${SUP}"]`);
  // One more tap is what a person does when the first lands on a list being redrawn (as f2-components tab()).
  if (!(await p.waitForSelector('#sheet.show #cEdit', { timeout: 20000 }).then(() => true, () => false))) {
    log('!! the supply card did not open on the first tap; tapping the row again');
    await p.click(`[data-s="${SUP}"]`);
    if (!(await p.waitForSelector('#sheet.show #cEdit', { timeout: 20000 }).then(() => true, () => false))) throw new Fail('the supply card did not open');
  }
  await p.click('#sheet.show #cEdit');
  if (!(await p.waitForSelector('#sheet.show #s-name', { timeout: 20000 }).then(() => true, () => false))) throw new Fail('the supply editor did not open');
}
/// Save the open editor; the POST's status.
async function saveSupply(p, re = /\/owner\/supplies$/) {
  const posted = p.waitForResponse(r => r.request().method() === 'POST' && re.test(r.url()), { timeout: 20000 });
  await p.click('#sheet.show #sSave');
  const st = (await posted).status();
  await p.waitForTimeout(800);
  return st;
}

const b = await launch();
let code = 0, dish = null;
try {
  dish = await setUp(); log(`made ${SUP} (${COST_A}/100 g) and dish ${dish} with ${QTY_G} g of it`);
  const ctx = await b.newContext(PHONE); await routeLocal(ctx);
  const p = await ctx.newPage(); const g = guard(p, 'console');
  // Every shelf read the console makes, with when: the mechanism under test is "a save makes the next ask re-read".
  const shelfReads = [];
  p.on('request', r => { if (r.method() === 'GET' && /\/api\/owner\/stock(\?|$)/.test(r.url())) shelfReads.push(Date.now()); });
  let loads = 0; p.on('load', () => { loads++; });
  await signIn(p, g); log('signed in'); loads = 0;

  // The baseline: the dish shows the supply as made.
  const first = await recipeLine(p, dish);
  check(first.name === NAME_A, `baseline: the recipe line is "${first.name}" (want "${NAME_A}")`);
  check(first.metaDigits.includes(String(COST_A)), `baseline: its cost reads ${COST_A} (meta "${first.meta}")`);
  const lastShelfRead = () => shelfReads[shelfReads.length - 1] || 0;

  // RENAME on the Stock screen, then open the dish: the new name, no reload.
  let t0 = Date.now(), readsBefore = shelfReads.length, since = lastShelfRead();
  await openSupplyEditor(p);
  await p.fill('#sheet.show #s-name', NAME_B);
  const st1 = await saveSupply(p);
  check(st1 === 200, `rename: the save answered ${st1}`);
  const renamed = await recipeLine(p, dish);
  let took = Date.now() - t0, age = Date.now() - since;
  check(took < STEP_MAX_MS && age < OLD_CACHE_MS, `rename: the step took ${took} ms (shelf read ${age} ms old) -- inside the old ${OLD_CACHE_MS} ms cache, so the result is the mechanism's`);
  check(renamed.name === NAME_B, `rename: the dish opened WITHOUT reload shows "${renamed.name}" (want "${NAME_B}")`);
  check(shelfReads.length > readsBefore, `rename: the console asked for the shelf again after the save (${shelfReads.length - readsBefore} new GET /owner/stock)`);

  // PRICE: the list price changes, the recipe line's cost follows.
  t0 = Date.now(); readsBefore = shelfReads.length; since = lastShelfRead();
  await openSupplyEditor(p);
  await p.fill('#sheet.show #s-cost', String(COST_B));
  const st2 = await saveSupply(p);
  check(st2 === 200, `price: the save answered ${st2}`);
  const repriced = await recipeLine(p, dish);
  took = Date.now() - t0; age = Date.now() - since;
  check(took < STEP_MAX_MS && age < OLD_CACHE_MS, `price: the step took ${took} ms (shelf read ${age} ms old)`);
  check(repriced.metaDigits.includes(String(COST_B)) && !repriced.metaDigits.includes(String(COST_A)), `price: the line's cost reads ${COST_B} now (meta "${repriced.meta}")`);
  check(shelfReads.length > readsBefore, `price: a fresh GET /owner/stock after the save (${shelfReads.length - readsBefore})`);

  // DELETE (in use): the menu row wears the review badge, the sheet names the ghost.
  t0 = Date.now();
  await openSupplyEditor(p);
  const deleted = p.waitForResponse(r => r.request().method() === 'POST' && /\/owner\/supplies\/delete$/.test(r.url()), { timeout: 30000 });
  await p.click('#sheet.show #sDelete');
  if (!(await p.waitForSelector('#sheet.show #cfYes', { timeout: 20000 }).then(() => true, () => false))) throw new Fail('deleting an ingredient in use asked no confirmation');
  await p.click('#sheet.show #cfYes');
  const st3 = (await deleted).status();
  check(st3 === 200, `delete: the hub answered ${st3}`);
  await p.waitForTimeout(800);
  await tab(p, 'menu');
  const badge = await until(p, id => !!document.querySelector(`[data-p="${id}"] .review`), dish, 15000);
  took = Date.now() - t0;
  check(took < STEP_MAX_MS, `delete: the step took ${took} ms`);
  check(badge, `delete: the menu row of ${dish} wears the review badge without reload`);
  // The hub drops the deleted line from the recipe and flags the dish: the sheet's review box says which ingredient went.
  await openDish(p, dish, CAT);
  const named = await p.waitForSelector('#sheet.show #rcReviewBox #rcReview', { timeout: 20000 }).then(e => e.textContent().then(x => x.trim()), () => '');
  await p.keyboard.press('Escape'); await p.waitForTimeout(500);
  check(/\S/.test(named), `delete: the dish sheet names the removed ingredient ("${named.slice(0, 80)}")`);
  check(loads === 0, `the page was never reloaded (${loads} loads)`);
  clean(g, 'TZ tzfresh');
  localServed(ctx, 'TZ tzfresh');
} catch (e) {
  fails.push(String(e.message));
  const pg = b.contexts()[0]?.pages()[0]; if (pg) log(`screenshot ${await shot(pg, 'tz-tzfresh-fail').catch(() => 'none')}`);
} finally {
  await tearDown(dish).catch(e => fails.push(`cleanup: ${e.message}`));
  await b.close().catch(() => {});
}
if (fails.length) { console.log(`tzfresh-browser: RED -- ${fails.length} failed: ${fails.join(' | ')}`); code = 1; }
else console.log('tzfresh-browser: GREEN');
process.exit(code);
