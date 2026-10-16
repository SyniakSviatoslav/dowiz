// TZ 2026-10-16, "enter everything as is, no ingredient lost", in a REAL browser on qa-durres
// (operator: every change is driven in a real browser). Chromium, the owner console, the dish
// sheet's tech card.
//
//   L1  a quantity typed in the kitchen's own words ("80 г") is read and saved as 80.
//   L2  a line typed down to 0 is NOT dropped by the save with "Saved": the save stops, the
//       line is outlined, nothing is sent; the recipe on the hub is unchanged.
//   L3  the recipe picker says how many components are selected while a search hides them.
//
// FLOWS_LOCAL=<workers/api/public> serves the local UI over the deployed Worker (the UI half
// is proven before a deploy); without it the run measures what is deployed. qa-* only.
//
//   FLOWS_LOCAL=/root/dowiz/workers/api/public node e2e/tz/noloss-browser.mjs
import { HOST, LOC, creds, Fail, own } from '../flows/lib.mjs';
import { launch, PHONE, routeLocal, localServed, guard, go, clean, shot } from '../flows/page.mjs';
import { tab, openDish } from '../flows/f2-components.mjs';

const DISH = 'qa-tuna-roll', CAT = 'qa-rolls';
const log = m => console.log(`  noloss: ${m}`);
const fails = [];
const check = (ok, what) => { if (ok) log(`ok   ${what}`); else { fails.push(what); log(`FAIL ${what}`); } };
const bomOf = async () => ((await own(`/api/owner/products?id=${DISH}&location_id=${LOC}`)).body?.products?.find(x => x.id === DISH)?.bom || [])
  .map(l => [l.supply, l.qtyMilli ?? Math.round(Number(l.qty) * 1000)]);

async function signIn(p) {
  await go(p, `${HOST}/admin/`, guard(p, 'signin'));
  if (await p.$('#nav:not([hidden])')) return;
  await p.waitForSelector('#e', { timeout: 60000 });
  await p.fill('#e', creds.OWNER_EMAIL); await p.fill('#p', creds.OWNER_PASSWORD);
  await p.click('#go');
  if (!(await p.waitForSelector('#nav:not([hidden])', { timeout: 60000 }).then(() => true, () => false))) throw new Fail('the console did not sign in');
  await p.waitForTimeout(1500);
}

async function openRecipe(p) {
  await openDish(p, DISH, CAT);
  if (!(await p.waitForSelector('#sheet.show #rcLines .rc-line [data-rq]', { timeout: 30000 }).then(() => true, () => false))) throw new Fail('the recipe lines never drew');
  await p.waitForTimeout(1500);
}

async function saveAndWait(p, ms = 6000) {
  let posted = 0;
  const onReq = r => { if (r.method() === 'POST' && new RegExp(`/owner/products/${DISH}$`).test(r.url())) posted++; };
  p.on('request', onReq);
  await p.click('#sheet.show #dSave');
  await p.waitForTimeout(ms);
  p.off('request', onReq);
  return posted;
}

const b = await launch();
let code = 0;
const before = await bomOf();
try {
  if (!before.length) throw new Fail(`${DISH} has no recipe on qa-durres`);
  const ctx = await b.newContext(PHONE); await routeLocal(ctx);
  const p = await ctx.newPage(); const g = guard(p, 'console');
  await signIn(p); log('signed in');
  await tab(p, 'menu');

  // L1: "N г" (Cyrillic gram) on the first line reads as N.
  await openRecipe(p);
  const [sup0, milli0] = before[0];
  // A value DIFFERENT from the stored one: an unread "N г" kept the old number and still "saved".
  const grams = Math.round(milli0 / 1000) + 1;
  await p.fill('#sheet.show [data-rq="0"]', `${grams} г`);
  const n1 = await saveAndWait(p);
  const after1 = await bomOf();
  check(n1 === 1, `L1 "${grams} г" saved (${n1} POST)`);
  check(after1.find(([s]) => s === sup0)?.[1] === grams * 1000, `L1 the hub stores ${sup0} = ${grams} (got ${JSON.stringify(after1)})`);

  // L2: typed down to 0 -> the save stops and names the line; the hub is unchanged.
  await p.keyboard.press('Escape').catch(() => {}); await p.waitForTimeout(800);
  await openRecipe(p);
  await p.fill('#sheet.show [data-rq="0"]', '0');
  const n2 = await saveAndWait(p, 3000);
  const outlined = await p.$eval('#sheet.show [data-rq="0"]', e => e.closest('.rc-line')?.classList.contains('lost')).catch(() => false);
  const stillOpen = !!(await p.$('#sheet.show #dSave'));
  const after2 = await bomOf();
  check(n2 === 0, `L2 a line typed to 0 is not sent (${n2} POST)`);
  check(outlined, 'L2 the line is outlined as the one that would be lost');
  check(stillOpen, 'L2 the sheet stays open with the typed text');
  check(JSON.stringify(after2) === JSON.stringify(after1), `L2 the hub recipe is unchanged (${JSON.stringify(after2)})`);
  await p.keyboard.press('Escape').catch(() => {}); await p.waitForTimeout(800);

  // L3: ticks hidden by a search are counted.
  await openRecipe(p);
  await p.click('#sheet.show #rcPick > summary');
  if (await p.waitForSelector('#sheet.show #rcBook [data-rs]', { state: 'attached', timeout: 20000 }).then(() => true, () => false)) {
    const id = await p.$eval('#sheet.show #rcBook [data-rs]', e => e.dataset.rs);
    await p.click(`#sheet.show label:has([data-rs="${id}"])`);
    await p.fill('#sheet.show #rcQ', 'zzzz-no-such-component'); await p.waitForTimeout(400);
    const counter = await p.$eval('#sheet.show #rcTicked', e => e.textContent.trim()).catch(() => '');
    check(/1/.test(counter), `L3 with the tick hidden by the search, the sheet still says it is selected ("${counter}")`);
  } else check(false, 'L3 the picker listed no component');
  await p.keyboard.press('Escape').catch(() => {});
  clean(g, 'TZ noloss');
  localServed(ctx, 'TZ noloss');
} catch (e) {
  fails.push(String(e.message));
  const pg = b.contexts()[0]?.pages()[0]; if (pg) log(`screenshot ${await shot(pg, 'tz-noloss-fail').catch(() => 'none')}`);
} finally {
  // Put the recipe back exactly as found.
  const now = await bomOf().catch(() => []);
  if (JSON.stringify(now) !== JSON.stringify(before)) {
    const bom = before.map(([supply, m]) => ({ supply, qty: m % 1000 === 0 ? m / 1000 : String(m / 1000) }));
    const r = await own(`/api/owner/products/${DISH}?location_id=${LOC}`, { bom, location_id: LOC }).catch(e => ({ status: String(e.message) }));
    log(`restore: recipe back -> ${r.status} (${JSON.stringify(await bomOf().catch(() => []))})`);
  }
  await b.close().catch(() => {});
}
if (fails.length) { console.log(`noloss-browser: RED -- ${fails.length} failed: ${fails.join(' | ')}`); code = 1; }
else console.log('noloss-browser: GREEN');
process.exit(code);
