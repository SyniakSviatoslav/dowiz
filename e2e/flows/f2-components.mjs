// F2's COMPONENT steps (operator 2026-09-30: "the bug with deleting or
// changing a component is still there"): a dish's recipe and a ПФ card's
// lines are edited the way a person does it -- type a new quantity, then tap
// the x of ANOTHER line, Save -- then reopened; the page AND the API must both
// show the change.
//
// The dish recipe step is RED on the build deployed 2026-09-30 (22ff93e8):
// the qty box redrew every line on blur, the x the finger pressed was gone
// before its click landed, and the save carried the removed line again
// (admin/menu.js, fixed by W-FLOWS; node proof admin/menu-dom.test.mjs).
//
// Also the console helpers the owner flow shares: a tab, and a dish sheet
// that is really THIS dish's (a closed sheet keeps its old inputs in the DOM,
// hidden: waiting for a bare `#d-name` matched the previous dish's box, which
// is what the "rename back" RED of the first gate runs really was).
import { LOC, RUN, S, save, Fail, own } from './lib.mjs';
import { clean, inspect, until } from './page.mjs';

const HEAD = { orders: 'tabOrders', menu: 'tabMenu', stock: 'inv_title' };
export async function tab(p, name) {
  const b = await p.$(`#nav [data-tab="${name}"]`);
  if (!b) throw new Fail(`the console has no "${name}" tab`);
  const drawn = () => until(p, k => !!document.querySelector(`#app h1[data-t="${k}"]`), HEAD[name], 20000);
  await b.click();
  if (await drawn()) return;
  // Measured 2026-09-30 (run FLOWS-muo9h3om): the Menu tab lit up and the
  // Orders screen stayed. One more tap is what a person does; printed.
  console.log(`  !! F2: tapping the "${name}" tab did not draw its screen in 20 s; tapping again`);
  await b.click();
  if (!(await drawn())) throw new Fail(`tapping the "${name}" tab twice did not draw its screen (heading ${HEAD[name]} absent)`);
}

/// The menu tab, the dish's folded category opened, the dish tapped: answers
/// once the OPEN sheet is this dish's.
export async function openDish(p, id, cat) {
  await tab(p, 'menu');
  const head = `.cat-h[data-cat="${cat}"]`;
  if (!(await p.waitForSelector(head, { timeout: 30000 }).then(() => true, () => false))) throw new Fail(`menu editor shows no category ${cat}`);
  if (await p.$eval(head, e => e.getAttribute('aria-expanded')) !== 'true') await p.click(head);
  if (!(await p.waitForSelector(`[data-p="${id}"]`, { timeout: 10000 }).then(() => true, () => false))) throw new Fail(`menu editor lists no ${id} under ${cat}`);
  await p.click(`[data-p="${id}"]`);
  if (!(await p.waitForSelector('#sheet.show[data-name="dish"] #d-name', { timeout: 20000 }).then(() => true, () => false))) throw new Fail(`tapping ${id} opened no dish sheet`);
}

/// POST responses to `re` while `fn` runs: [{ status, body }].
async function posts(p, re, fn, ms = 12000) {
  const got = [];
  const on = r => { if (r.request().method() === 'POST' && re.test(r.url())) { let body = null; try { body = JSON.parse(r.request().postData() || 'null'); } catch {} got.push({ status: r.status(), body }); } };
  p.on('response', on);
  await fn();
  for (let i = 0; i < ms / 250 && !got.length; i++) await p.waitForTimeout(250);
  p.off('response', on);
  return got;
}
const pairs = lines => JSON.stringify((lines || []).map(l => [l.supply ?? l.item, l.qty]));

const DISH = 'qa-tuna-roll', DISH_CAT = 'qa-rolls';
/// Dish recipe: type the first line down to 70, tap the x of the second, Save, reopen.
export async function recipeStep(p, g, log) {
  const before = (await own(`/api/owner/products?location_id=${LOC}`)).body?.products?.find(x => x.id === DISH);
  if (!(before?.bom?.length >= 2)) throw new Fail(`${DISH} needs a recipe of 2+ components (has ${pairs(before?.bom)})`);
  if (S.restore.bom === undefined) { S.restore.bom = before.bom.map(l => ({ supply: l.supply, qty: l.qty })); save(); }
  const want = [[before.bom[0].supply, 70]];
  await openDish(p, DISH, DISH_CAT);
  if (!(await p.waitForSelector('#sheet.show [data-rx="1"]', { timeout: 30000 }).then(() => true, () => false))) throw new Fail('the recipe lines never drew');
  await inspect(p, 'F2 recipe');
  await p.fill('[data-rq="0"]', '70');
  await p.click('[data-rx="1"]');       // the finger leaves the qty box for ANOTHER line's x
  await p.waitForTimeout(500);
  const shown = await p.$$eval('#sheet.show .rc-line', ls => ls.filter(l => !l.classList.contains('off')).length);
  if (shown !== 1) throw new Fail(`typed 70 then tapped x on the second component: ${shown} lines still on the page (the tap did not land)`);
  const sent = await posts(p, new RegExp(`/owner/products/${DISH}$`), () => p.click('#dSave'));
  if (sent[0]?.status !== 200) throw new Fail(`saving the recipe answered ${sent[0]?.status ?? 'nothing'}`);
  if (pairs(sent[0].body?.bom) !== JSON.stringify(want)) throw new Fail(`the save sent bom ${pairs(sent[0].body?.bom)}, the page showed ${JSON.stringify(want)}`);
  const api = (await own(`/api/owner/products?location_id=${LOC}`)).body?.products?.find(x => x.id === DISH)?.bom;
  if (pairs(api) !== JSON.stringify(want)) throw new Fail(`recipe saved, the API reads ${pairs(api)}, want ${JSON.stringify(want)}`);
  await openDish(p, DISH, DISH_CAT);
  if (!(await until(p, () => document.querySelectorAll('#sheet.show [data-rq]').length === 1, null, 20000))) throw new Fail('reopened: the removed component is back on the page');
  const q = await p.$eval('#sheet.show [data-rq="0"]', e => e.value);
  if (q !== '70') throw new Fail(`reopened: the kept component shows ${q}, want 70`);
  await p.keyboard.press('Escape');
  clean(g, 'F2 recipe');
  log(`recipe of ${DISH}: typed 70 + x on the other line -> save sent ${pairs(sent[0].body.bom)}, API ${pairs(api)}, reopened shows 1 line at 70`);
}

/// ПФ card: made by the API with two lines; x on the second, first typed to 70, Save, reopen.
export async function prepStep(p, g, log, openStock) {
  const id = `${RUN.toLowerCase()}-pf`;
  const mk = await own('/api/owner/preps', { location_id: LOC, id, name: `${RUN} ПФ`, unit: 'g', lines: [{ item: 'qa-rice', qty: 100 }, { item: 'qa-salmon', qty: 50 }], yield: 150 });
  if (mk.status !== 200) throw new Fail(`making the QA ПФ: ${mk.status} ${mk.text.slice(0, 100)}`);
  await openStock();
  const open = async () => {
    if (!(await p.waitForSelector(`[data-s="${id}"]`, { timeout: 30000 }).then(() => true, () => false))) throw new Fail(`the stock list shows no ${id}`);
    await p.click(`[data-s="${id}"]`);
    if (!(await p.waitForSelector('#sheet.show #pfEdit', { timeout: 15000 }).then(() => true, () => false))) throw new Fail('the ПФ card has no edit button');
    await p.click('#pfEdit');
    if (!(await p.waitForSelector('#sheet.show[data-name="prepEdit"] [data-plq="0"]', { timeout: 15000 }).then(() => true, () => false))) throw new Fail('the ПФ editor drew no lines');
  };
  await open();
  await inspect(p, 'F2 ПФ card editor');
  await p.fill('[data-plq="0"]', '70');
  await p.click('[data-plx="1"]');
  const sent = await posts(p, /\/owner\/preps$/, () => p.click('#pfSave'));
  const want = JSON.stringify([['qa-rice', 70]]);
  if (sent[0]?.status !== 200 || pairs(sent[0].body?.lines) !== want) throw new Fail(`ПФ save: ${sent[0]?.status ?? 'nothing'} lines ${pairs(sent[0]?.body?.lines)}, want ${want}`);
  const api = (await own(`/api/owner/preps?location_id=${LOC}`)).body?.preps?.find(x => x.id === id)?.lines;
  if (pairs(api) !== want) throw new Fail(`ПФ saved, the API reads ${pairs(api)}, want ${want}`);
  await p.waitForTimeout(2500);
  await open();
  const qs = await p.$$eval('#sheet.show [data-plq]', e => e.map(x => x.value));
  if (JSON.stringify(qs) !== '["70"]') throw new Fail(`reopened ПФ editor shows ${JSON.stringify(qs)}, want ["70"]`);
  await p.keyboard.press('Escape');
  clean(g, 'F2 ПФ card');
  log(`ПФ ${id}: typed 70 + x on the other line -> save sent ${pairs(sent[0].body.lines)}, API ${pairs(api)}, reopened shows ["70"]`);
}
