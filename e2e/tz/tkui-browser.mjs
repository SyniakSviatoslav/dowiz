// TZ 2026-10-12, W-TKUI items in a REAL browser (operator: "усі зміни обов'язково
// перевірити в реальному браузері"). Chromium on qa-durres, the owner console.
//
//   TZ4  tick a component in the recipe picker, type a search that hides it, clear
//        the search: the tick is still there.
//   TZ2  taste/aroma/texture sit in ONE closed accordion; opened, it stays open
//        after a reload on this device.
//   TZ6  a dish save is followed by ONE fresh menu read (not four), and the list
//        row shows the new price without a reload. The price is put back after.
//
// FLOWS_LOCAL=<workers/api/public> serves the local UI over the deployed Worker,
// so the UI half can be proven before a deploy (page.mjs routeLocal). Without it
// the run measures what is deployed. qa-* venues only (lib.mjs refuses others).
//
//   FLOWS_LOCAL=/root/dowiz/workers/api/public node e2e/tz/tkui-browser.mjs
import { HOST, LOC, creds, Fail, own } from '../flows/lib.mjs';
import { launch, PHONE, routeLocal, localServed, guard, go, clean, shot, until } from '../flows/page.mjs';
import { tab, openDish } from '../flows/f2-components.mjs';

const DISH = 'qa-tuna-roll', CAT = 'qa-rolls';
const log = m => console.log(`  tkui: ${m}`);
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

async function tz4(p) {
  await openDish(p, DISH, CAT);
  // The sheet redraws its recipe once the stored dish arrives: tap after that, not before.
  if (!(await p.waitForSelector('#sheet.show #rcLines .rc-line', { timeout: 30000 }).then(() => true, () => false))) throw new Fail('the recipe lines never drew');
  await p.waitForTimeout(1500);
  await p.click('#sheet.show #rcPick > summary');
  if (!(await until(p, () => document.querySelector('#sheet.show #rcPick')?.open, null, 5000))) await p.click('#sheet.show #rcPick > summary');
  if (!(await p.waitForSelector('#sheet.show #rcBook [data-rs]', { state: 'attached', timeout: 20000 }).then(() => true, () => false)))
    throw new Fail(`the recipe picker listed no component (open=${await p.$eval('#sheet.show #rcPick', d => d.open).catch(() => '?')}, book: ${(await p.$eval('#sheet.show #rcBook', e => e.textContent).catch(() => '?')).slice(0, 120)})`);
  const id = await p.$eval('#sheet.show #rcBook [data-rs]', e => e.dataset.rs);
  // The box is a zero-size input under the drawn switch: a person taps the label.
  await p.click(`#sheet.show label:has([data-rs="${id}"])`);
  check(await p.$eval(`#sheet.show [data-rs="${id}"]`, e => e.checked), `TZ4 tapping the switch ticked ${id}`);
  await p.fill('#sheet.show #rcQ', 'zzzz-no-such-component');
  await p.waitForTimeout(400);
  const hidden = !(await p.$(`#sheet.show [data-rs="${id}"]`));
  await p.fill('#sheet.show #rcQ', '');
  await p.waitForTimeout(400);
  const kept = await p.$eval(`#sheet.show [data-rs="${id}"]`, e => e.checked).catch(() => false);
  check(hidden, `TZ4 the search hid the ticked component ${id}`);
  check(kept, `TZ4 after clearing the search ${id} is still ticked`);
  await p.keyboard.press('Escape'); await p.waitForTimeout(500);
}

async function tz2(p, g) {
  await openDish(p, DISH, CAT);
  const one = await p.$$eval('#sheet.show details#sxFold', d => d.length);
  check(one === 1, `TZ2 one accordion #sxFold on the dish sheet (found ${one})`);
  const inside = await p.$$eval('#sheet.show [data-t="sx_taste"], #sheet.show [data-t="sx_texture"], #sheet.show [data-t="sx_aroma"]',
    es => es.length > 0 && es.every(e => !!e.closest('details#sxFold')));
  check(inside, 'TZ2 taste, texture and aroma are all inside it');
  const closed = await p.$eval('#sheet.show details#sxFold', d => !d.open).catch(() => false);
  check(closed, 'TZ2 it starts closed');
  if (!one) { await p.keyboard.press('Escape'); await p.waitForTimeout(500); return; }
  await p.click('#sheet.show details#sxFold > summary');
  await p.keyboard.press('Escape'); await p.waitForTimeout(500);
  await p.reload(); await signIn(p, g);
  await openDish(p, DISH, CAT);
  const kept = await p.$eval('#sheet.show details#sxFold', d => d.open).catch(() => false);
  check(kept, 'TZ2 opened, it is still open after a reload (remembered on this device)');
  await p.click('#sheet.show details#sxFold > summary');   // leave it as found
  await p.keyboard.press('Escape'); await p.waitForTimeout(500);
}

async function savePrice(p, to) {
  await openDish(p, DISH, CAT);
  await p.fill('#sheet.show #d-price', String(to));
  const reads = [];
  let loads = 0;
  const onReq = r => { if (r.method() === 'GET' && /\/menu\?/.test(r.url())) reads.push(r.url()); };
  const onLoad = () => { loads++; };
  p.on('request', onReq); p.on('load', onLoad);
  const posted = p.waitForResponse(r => r.request().method() === 'POST' && new RegExp(`/owner/products/${DISH}$`).test(r.url()), { timeout: 20000 });
  await p.click('#sheet.show #dSave');
  const st = (await posted).status();
  await p.waitForTimeout(4000);
  p.off('request', onReq); p.off('load', onLoad);
  return { st, reads, loads };
}

async function tz6(p) {
  const before = (await own(`/api/owner/products?location_id=${LOC}`)).body?.products?.find(x => x.id === DISH);
  if (!before) throw new Fail(`qa-durres has no ${DISH}`);
  const was = before.price, to = was + 1;
  try {
    const r = await savePrice(p, to);
    check(r.st === 200, `TZ6 the save answered ${r.st}`);
    check(r.reads.length === 1 && /fresh=1/.test(r.reads[0]), `TZ6 one fresh menu read after the save (got ${r.reads.length}: ${r.reads.map(u => new URL(u).search).join(' ')})`);
    check(r.loads === 0, `TZ6 no page reload (${r.loads})`);
    await p.keyboard.press('Escape').catch(() => {});
    const row = await until(p, ([id, n]) => (document.querySelector(`[data-p="${id}"]`)?.textContent || '').replace(/\D/g, '').includes(String(n)), [DISH, to], 10000);
    check(row, `TZ6 the list row shows the new price ${to} without F5`);
  } finally {
    const back = await savePrice(p, was).catch(e => ({ st: String(e.message) }));
    const now = (await own(`/api/owner/products?location_id=${LOC}`)).body?.products?.find(x => x.id === DISH)?.price;
    check(now === was, `restore: price back to ${was} (API reads ${now}, save ${back.st})`);
  }
}

const b = await launch();
let code = 0;
try {
  const ctx = await b.newContext(PHONE); await routeLocal(ctx);
  const p = await ctx.newPage(); const g = guard(p, 'console');
  await signIn(p, g); log('signed in');
  await tab(p, 'menu');
  await tz4(p);
  await tz2(p, g);
  await tz6(p);
  clean(g, 'TZ tkui');
  localServed(ctx, 'TZ tkui');
} catch (e) {
  fails.push(String(e.message));
  const pg = b.contexts()[0]?.pages()[0]; if (pg) log(`screenshot ${await shot(pg, 'tz-tkui-fail').catch(() => 'none')}`);
} finally { await b.close().catch(() => {}); }
if (fails.length) { console.log(`tkui-browser: RED -- ${fails.length} failed: ${fails.join(' | ')}`); code = 1; }
else console.log('tkui-browser: GREEN');
process.exit(code);
