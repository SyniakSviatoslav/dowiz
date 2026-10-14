// TZ 2026-10-12, the items no browser had driven yet (operator: "усі зміни
// обов'язково перевірити в реальному браузері"). Chromium on qa-durres.
//
//   TZ1  FRACTIONAL: a delivery of 0,75, a write-off of 0,25 and a count of
//        1,25 on one ingredient; a 2,5 line in a dish's tech card and in a ПФ
//        card. After save + a page reload the exact fraction is on screen.
//   TZ3  CASCADE: an ingredient used in a recipe is renamed (no lock) and the
//        recipe shows the new name; deleted, the dish wears "recipe needs
//        review (ingredient removed: X)"; the recipe saved without the ghost
//        line, the badge is gone.
//   TZ5  FOOD COST CHAIN: ingredient -> ПФ -> dish. The ingredient's price is
//        changed on its form; the dish sheet's food cost and the kitchen
//        numbers' portion cost both move, with normal navigation (no F5).
//   TZ7  SESSION: the background renewer calls the refresh route before the
//        token ends and the console stays signed in; a session that ends
//        (access AND refresh token refused) mid-edit sends the person to the
//        sign-in, and after it the typed text is back in the form.
//   TZ8  LOGIN CENTRED: the sign-in form of /admin/ (the owner's AND the
//        kitchen's door), /courier/ and /room/ (the till), at a phone and at a
//        desktop size: its box's centre is the viewport's, within a few px.
//
// Everything this run makes is named qa-tz-<run> and is deleted at the end;
// the one qa dish it borrows (qa-salmon-roll, for the kitchen numbers, which
// list only dishes SOLD in the window) gets its recipe back in a finally.
//
// FLOWS_LOCAL=<workers/api/public> serves the local UI over the deployed Worker
// (page.mjs routeLocal). Without it the run measures what is deployed.
//
//   node e2e/tz/tz-rest-browser.mjs
//   FLOWS_LOCAL=/root/dowiz/workers/api/public node e2e/tz/tz-rest-browser.mjs
import { HOST, LOC, creds, Fail, own } from '../flows/lib.mjs';
import { launch, PHONE, routeLocal, localServed, guard, go, shot, until } from '../flows/page.mjs';
import { tab, openDish } from '../flows/f2-components.mjs';

const log = m => console.log(`  tz-rest: ${m}`);
const fails = [];
const check = (ok, what) => { if (ok) log(`ok   ${what}`); else { fails.push(what); log(`FAIL ${what}`); } };

const TAG = `qa-tz-${Date.now().toString(36)}`;
const SUP_A = `${TAG}-a`, SUP_B = `${TAG}-b`, SUP_C = `${TAG}-c`, PREP_A = `${TAG}-pfa`, PREP_C = `${TAG}-pfc`;
const NAME = { [SUP_A]: `${TAG} A`, [SUP_B]: `${TAG} B`, [SUP_C]: `${TAG} C`, [PREP_A]: `${TAG} PF-A`, [PREP_C]: `${TAG} PF-C` };
const CAT = 'qa-rolls', SOLD = 'qa-salmon-roll';
let DISH = null;          // the throwaway dish's id, made in setup
let soldBom = null;       // SOLD's recipe as found, put back in the finally

// ── plumbing ─────────────────────────────────────────────────────────────────
const ok200 = r => r && r.status === 200;
const posted = (p, re, ms = 20000) => p.waitForResponse(r => r.request().method() === 'POST' && re.test(r.url()), { timeout: ms });
const sheetOpen = (p, name, ms = 20000) => p.waitForSelector(`#sheet.show[data-name="${name}"]`, { timeout: ms }).then(() => true, () => false);
const esc = async p => { await p.keyboard.press('Escape').catch(() => {}); await p.waitForTimeout(400); };
// Spaces folded the way costText folds them (the formatter puts a no-break space after the code).
const moneyIn = (p, n) => p.evaluate(async x => (await import('/admin/core.js')).money(x), n).then(s => s.replace(/\s+/g, ' '));
const reB = s => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

async function signIn(p, g) {
  await go(p, `${HOST}/admin/`, g);
  // The console boots from the stored token, or draws the sign-in: whichever comes first.
  await p.waitForSelector('#nav:not([hidden]), #e', { timeout: 60000 });
  if (await p.$('#nav:not([hidden])')) { await p.waitForTimeout(1000); return; }
  await typeLogin(p);
}
async function typeLogin(p) {
  await p.fill('#e', creds.OWNER_EMAIL); await p.fill('#p', creds.OWNER_PASSWORD);
  await p.click('#go');
  if (!(await p.waitForSelector('#nav:not([hidden])', { timeout: 60000 }).then(() => true, () => false))) throw new Fail('the console did not sign in');
  await p.waitForTimeout(1500);
}
async function reload(p, g) { await p.reload({ waitUntil: 'domcontentloaded' }).catch(() => {}); await signIn(p, g); }

/// The stock tab, drawn with its list.
async function stockTab(p) {
  await tab(p, 'stock');
  if (!(await p.waitForSelector('#stockList .rows, #stockList .inv-item', { timeout: 30000 }).then(() => true, () => false))) throw new Fail('the stock screen drew no list');
}
/// The text of one ingredient's row on the stock screen.
const rowText = (p, id) => p.$eval(`[data-s="${id}"]`, e => e.textContent.replace(/\s+/g, ' ')).catch(() => '');
/// Wait until that row's text matches `re` (the list redraws after a save).
const rowSays = (p, id, re, ms = 20000) => until(p, ([i, s]) => new RegExp(s).test(document.querySelector(`[data-s="${i}"]`)?.textContent.replace(/\s+/g, ' ') || ''), [id, re.source], ms);

/// One movement on one ingredient, typed the way a person does: the row's action, the quantity, Save.
async function move(p, id, kind, qty, extra = async () => {}) {
  await p.click(`[data-act="${kind}"][data-s="${id}"]`);
  if (!(await sheetOpen(p, 'move'))) throw new Fail(`the ${kind} sheet for ${id} did not open`);
  await p.fill('#sheet.show #m-qty', qty);
  await extra();
  const res = posted(p, new RegExp(`/owner/stock/${kind}$`));
  await p.click('#sheet.show #mGo');
  const r = await res.catch(() => null);
  let sent = null; try { sent = JSON.parse(r?.request().postData() || 'null'); } catch {}
  return { status: r?.status() ?? 'none', sent };
}

/// The supply form, from the stock row: card, Edit.
async function supplyForm(p, id) {
  await p.click(`[data-s="${id}"]`);
  if (!(await sheetOpen(p, 'supplyCard'))) throw new Fail(`tapping ${id} opened no card`);
  await p.click('#sheet.show #cEdit');
  if (!(await sheetOpen(p, 'supply'))) throw new Fail(`the card of ${id} opened no form`);
}

/// The ПФ editor of `id`, from the stock row: card, Edit.
async function prepEditor(p, id) {
  if (!(await p.waitForSelector(`[data-s="${id}"]`, { timeout: 30000 }).then(() => true, () => false))) throw new Fail(`the stock list shows no ${id}`);
  await p.click(`[data-s="${id}"]`);
  if (!(await p.waitForSelector('#sheet.show #pfEdit', { timeout: 15000 }).then(() => true, () => false))) throw new Fail(`the ПФ card of ${id} has no edit button`);
  await p.click('#sheet.show #pfEdit');
  if (!(await p.waitForSelector('#sheet.show[data-name="prepEdit"] [data-plq]', { timeout: 15000 }).then(() => true, () => false))) throw new Fail(`the ПФ editor of ${id} drew no lines`);
}

/// The dish sheet with its recipe drawn from the stored dish.
async function dishSheet(p, id) {
  await openDish(p, id, CAT);
  if (!(await p.waitForSelector('#sheet.show #rcLines .rc-line, #sheet.show #rcLines .hint', { timeout: 30000 }).then(() => true, () => false))) throw new Fail(`the recipe of ${id} never drew`);
  await p.waitForTimeout(1200);   // the stored dish lands after the first draw
}
/// The index of the recipe line whose name contains `name`, or -1.
const lineIdx = (p, name) => p.$$eval('#sheet.show #rcLines .rc-line', (ls, n) => ls.findIndex(l => (l.querySelector('b')?.textContent || '').includes(n)), name);
/// The food-cost text under the recipe.
const costText = p => p.$eval('#sheet.show #rcSum', e => e.textContent.replace(/\s+/g, ' ')).catch(() => '');

// ── setup / teardown (API: the things the checks act on, never the checks) ──
async function must(what, r) { if (!ok200(r)) throw new Fail(`setup: ${what} answered ${r.status} ${r.text.slice(0, 160)}`); return r.body; }
async function setup() {
  for (const [id, cost] of [[SUP_A, 100], [SUP_B, 100], [SUP_C, 1000]])
    await must(`supply ${id}`, await own('/api/owner/supplies', { id, name: NAME[id], unit: 'g', kind: 'food_ingredient', category: TAG, costPerBasis: cost }));
  await must(`ПФ ${PREP_A}`, await own('/api/owner/preps', { location_id: LOC, id: PREP_A, name: NAME[PREP_A], unit: 'g', category: TAG, lines: [{ item: SUP_A, qty: 10 }], yield: 10 }));
  await must(`ПФ ${PREP_C}`, await own('/api/owner/preps', { location_id: LOC, id: PREP_C, name: NAME[PREP_C], unit: 'g', category: TAG, lines: [{ item: SUP_C, qty: 100 }], yield: 100 }));
  const d = await must('dish', await own('/api/owner/products', { location_id: LOC, name: `${TAG} dish`, category_id: CAT, price: 500, description: '' }));
  DISH = d.id;
  if (!DISH) throw new Fail(`setup: making the dish answered no id (${JSON.stringify(d).slice(0, 120)})`);
  await must('dish recipe', await own(`/api/owner/products/${encodeURIComponent(DISH)}`, { location_id: LOC, bom: [{ supply: SUP_A, qty: 10 }, { supply: SUP_B, qty: 10 }] }));
  const prods = (await own(`/api/owner/products?location_id=${LOC}`)).body?.products || [];
  const sold = prods.find(x => x.id === SOLD);
  if (!sold) throw new Fail(`setup: qa-durres has no ${SOLD}`);
  soldBom = (sold.bom || []).map(l => ({ supply: l.supply, qty: l.qty }));
  await must(`${SOLD} recipe -> ПФ`, await own(`/api/owner/products/${encodeURIComponent(SOLD)}`, { location_id: LOC, bom: [{ supply: PREP_C, qty: 100 }] }));
  log(`setup: ${SUP_A} ${SUP_B} ${SUP_C} ${PREP_A} ${PREP_C}, dish ${DISH}; ${SOLD} recipe ${JSON.stringify(soldBom)} -> [${PREP_C} 100] for the run`);
}
async function teardown() {
  const step = async (what, fn) => { try { const r = await fn(); log(`restore: ${what} -> ${r}`); } catch (e) { fails.push(`restore ${what}: ${e.message}`); log(`FAIL restore ${what}: ${e.message}`); } };
  if (soldBom) await step(`${SOLD} recipe back`, async () => {
    const w = await own(`/api/owner/products/${encodeURIComponent(SOLD)}`, { location_id: LOC, bom: soldBom });
    const now = ((await own(`/api/owner/products?location_id=${LOC}`)).body?.products || []).find(x => x.id === SOLD)?.bom || [];
    const same = JSON.stringify(now.map(l => [l.supply, Number(l.qty)])) === JSON.stringify(soldBom.map(l => [l.supply, Number(l.qty)]));
    if (!same) throw new Error(`save ${w.status}, API reads ${JSON.stringify(now.map(l => [l.supply, l.qty]))}`);
    return `ok (${JSON.stringify(soldBom.map(l => [l.supply, l.qty]))})`;
  });
  // The opposite movement: whatever the run put on the shelf of A is counted back to nothing.
  await step(`${SUP_A} shelf counted back to 0`, async () => { const r = await own('/api/owner/stock/stocktake', { item: SUP_A, observed: 0 }); return String(r.status); });
  if (DISH) await step(`dish ${DISH} deleted`, async () => { const r = await own(`/api/owner/products/${encodeURIComponent(DISH)}/delete`, { location_id: LOC }); if (!ok200(r)) throw new Error(`${r.status} ${r.text.slice(0, 100)}`); return '200'; });
  await step('throwaway ПФ + ingredients deleted', async () => {
    const r1 = await own('/api/owner/supplies/delete', { location_id: LOC, ids: [PREP_A, PREP_C], confirmUses: true });
    const r2 = await own('/api/owner/supplies/delete', { location_id: LOC, ids: [SUP_A, SUP_B, SUP_C], confirmUses: true });
    const left = ((await own(`/api/owner/stock?location_id=${LOC}`)).body?.supplies || []).filter(s => s.id.startsWith(TAG)).map(s => s.id);
    if (left.length) throw new Error(`still listed: ${left.join(', ')} (deletes ${r1.status}/${r2.status})`);
    return `${r1.status}/${r2.status}, none left`;
  });
}

// ── TZ1 fractional ───────────────────────────────────────────────────────────
async function tz1(p, g) {
  await stockTab(p);
  if (!(await p.waitForSelector(`[data-s="${SUP_A}"]`, { timeout: 20000 }).then(() => true, () => false))) throw new Fail(`the stock list shows no ${SUP_A}`);
  // a delivery of 0,75 (a comma, as a phone keyboard types it)
  const d = await move(p, SUP_A, 'received', '0,75');
  check(d.status === 200 && String(d.sent?.qty) === '0.75', `TZ1 delivery "0,75" saved (${d.status}, sent qty ${JSON.stringify(d.sent?.qty)})`);
  check(await rowSays(p, SUP_A, /\b0\.75\b/), `TZ1 after the delivery the row reads on hand 0.75 (row: ${(await rowText(p, SUP_A)).slice(0, 140)})`);
  await reload(p, g); await stockTab(p);
  check(await rowSays(p, SUP_A, /\b0\.75\b/), `TZ1 after a reload the row still reads 0.75 (row: ${(await rowText(p, SUP_A)).slice(0, 140)})`);
  // a write-off of 0,25 -> 0.5
  const w = await move(p, SUP_A, 'wasted', '0,25', async () => { await p.click('#sheet.show [data-r="spoiled"]'); });
  check(w.status === 200 && String(w.sent?.qty) === '0.25', `TZ1 write-off "0,25" saved (${w.status}, sent qty ${JSON.stringify(w.sent?.qty)})`);
  await reload(p, g); await stockTab(p);
  check(await rowSays(p, SUP_A, /\b0\.5\b/), `TZ1 after the write-off + reload the row reads 0.5 (row: ${(await rowText(p, SUP_A)).slice(0, 140)})`);
  // a count of 1,25
  const c = await move(p, SUP_A, 'stocktake', '1,25');
  check(c.status === 200 && String(c.sent?.observed) === '1.25', `TZ1 count "1,25" saved (${c.status}, sent observed ${JSON.stringify(c.sent?.observed)})`);
  await reload(p, g); await stockTab(p);
  check(await rowSays(p, SUP_A, /\b1\.25\b/), `TZ1 after the count + reload the row reads 1.25 (row: ${(await rowText(p, SUP_A)).slice(0, 140)})`);

  // the ПФ card: its line of A typed to 2,5
  await prepEditor(p, PREP_A);
  await p.fill('#sheet.show [data-plq="0"]', '2,5');
  const pr = posted(p, /\/owner\/preps$/);
  await p.click('#sheet.show #pfSave');
  const prr = await pr.catch(() => null);
  let psent = null; try { psent = JSON.parse(prr?.request().postData() || 'null'); } catch {}
  let pbody = ''; try { pbody = (await prr?.text() || '').slice(0, 160); } catch {}
  check(prr?.status() === 200, `TZ1 ПФ line "2,5" saved (${prr?.status() ?? 'none'}, sent ${JSON.stringify(psent?.lines)}${prr?.status() === 200 ? '' : `, answer ${pbody}`})`);
  await esc(p);
  await reload(p, g); await stockTab(p);
  await prepEditor(p, PREP_A);
  const pv = await p.$eval('#sheet.show [data-plq="0"]', e => e.value).catch(() => '?');
  check(pv === '2.5', `TZ1 after a reload the ПФ editor shows the line as 2.5 (shows "${pv}")`);
  await esc(p);

  // the dish's tech card: its line of A typed to 2,5
  await dishSheet(p, DISH);
  const i = await lineIdx(p, NAME[SUP_A]);
  if (i < 0) throw new Fail(`the recipe of ${DISH} shows no line ${NAME[SUP_A]}`);
  await p.fill(`#sheet.show [data-rq="${i}"]`, '2,5');
  const dr = posted(p, new RegExp(`/owner/products/${reB(DISH)}$`));
  await p.click('#sheet.show #dSave');
  const drr = await dr.catch(() => null);
  let dsent = null; try { dsent = JSON.parse(drr?.request().postData() || 'null'); } catch {}
  const aLine = (dsent?.bom || []).find(l => l.supply === SUP_A);
  check(drr?.status() === 200 && String(aLine?.qty) === '2.5', `TZ1 dish line "2,5" saved (${drr?.status() ?? 'none'}, sent ${JSON.stringify(aLine)})`);
  await reload(p, g);
  await dishSheet(p, DISH);
  const j = await lineIdx(p, NAME[SUP_A]);
  const dv = j < 0 ? '(no line)' : await p.$eval(`#sheet.show [data-rq="${j}"]`, e => e.value).catch(() => '?');
  check(dv === '2.5', `TZ1 after a reload the dish's tech card shows the line as 2.5 (shows "${dv}")`);
  await esc(p);
}

// ── TZ3 cascade ──────────────────────────────────────────────────────────────
async function tz3(p, g) {
  const renamed = `${TAG} B renamed`;
  // The owner looks at the dish first (the menu tab has read the shelf), then goes to rename B.
  await dishSheet(p, DISH);
  check(await lineIdx(p, NAME[SUP_B]) >= 0, `TZ3 the dish's recipe lists ${NAME[SUP_B]} before the edit`);
  await esc(p);
  await stockTab(p);
  await supplyForm(p, SUP_B);
  await p.fill('#sheet.show #s-name', renamed);
  const sv = posted(p, /\/owner\/supplies$/);
  await p.click('#sheet.show #sSave');
  const svr = await sv.catch(() => null);
  check(svr?.status() === 200, `TZ3 an ingredient used in a recipe saves with no lock (${svr?.status() ?? 'none'})`);
  await p.waitForTimeout(800);
  await dishSheet(p, DISH);
  const shown = await lineIdx(p, renamed) >= 0;
  const names = await p.$$eval('#sheet.show #rcLines .rc-line b', bs => bs.map(b => b.textContent));
  let note = '';
  if (!shown) {
    // Diagnostic only: is it the page's own copy (a cache) or the data?
    await esc(p); await reload(p, g); await dishSheet(p, DISH);
    note = (await lineIdx(p, renamed) >= 0) ? '; after F5 it does -- the console kept an old copy of the shelf' : '; not even after F5';
  }
  check(shown, `TZ3 the linked recipe shows the new name "${renamed}" without F5 (lines: ${JSON.stringify(names)}${note})`);
  await esc(p);

  // delete B through its form, the in-use confirmation answered yes
  await stockTab(p);
  await supplyForm(p, SUP_B);
  await p.click('#sheet.show #sDelete');
  if (!(await sheetOpen(p, 'confirm', 20000))) throw new Fail('deleting an ingredient in use asked nothing');
  const askText = await p.$eval('#sheet.show', e => e.textContent.replace(/\s+/g, ' ')).catch(() => '');
  check(askText.includes(`${TAG} dish`), `TZ3 the delete confirmation names the dish that uses it ("${askText.slice(0, 160)}")`);
  const dl = posted(p, /\/owner\/supplies\/delete$/);
  await p.click('#sheet.show #cfYes');
  const dlr = await dl.catch(() => null);
  check(dlr?.status() === 200, `TZ3 the ingredient in use is deleted (${dlr?.status() ?? 'none'})`);
  await p.waitForTimeout(800);

  // the menu: the dish's row wears the badge; its sheet names the removed ingredient
  await tab(p, 'menu');
  const head = `.cat-h[data-cat="${CAT}"]`;
  await p.waitForSelector(head, { timeout: 30000 });
  if (await p.$eval(head, e => e.getAttribute('aria-expanded')) !== 'true') await p.click(head);
  const rowBadge = await until(p, id => !!document.querySelector(`[data-p="${id}"] .review`), DISH, 10000);
  const rowTitle = rowBadge ? await p.$eval(`[data-p="${DISH}"] .review`, e => `${e.textContent.trim()} / title "${e.getAttribute('title') || ''}"`) : '(none)';
  check(rowBadge, `TZ3 the dish's row in the menu wears the review badge without F5 (${rowTitle})`);
  await dishSheet(p, DISH);
  const badge = await p.$eval('#sheet.show #rcReview', e => e.textContent.trim()).catch(() => '');
  check(!!badge && badge.includes(renamed), `TZ3 the dish sheet says "${badge || '(no badge)'}" (want it to name ${renamed})`);
  // take the ghost line out (x) and save
  const ghost = await p.$$eval('#sheet.show #rcLines .rc-line', ls => ls.findIndex(l => !!l.querySelector('.ui-pill--bad, .pill-bad, [data-t="rv_removed"]') || /removed|видалено|i hequr|удалён/.test(l.querySelector('[data-rmeta]')?.textContent || '')));
  if (ghost < 0) throw new Fail(`the dish sheet shows no ghost line for the removed ingredient (lines: ${JSON.stringify(await p.$$eval('#sheet.show #rcLines .rc-line', ls => ls.map(l => l.textContent.replace(/\s+/g, ' ').trim().slice(0, 60))))})`);
  await p.click(`#sheet.show [data-rx="${ghost}"]`);
  await p.waitForTimeout(300);
  const sr = posted(p, new RegExp(`/owner/products/${reB(DISH)}$`));
  await p.click('#sheet.show #dSave');
  const srr = await sr.catch(() => null);
  let ssent = null; try { ssent = JSON.parse(srr?.request().postData() || 'null'); } catch {}
  check(srr?.status() === 200 && !(ssent?.bom || []).some(l => l.supply === SUP_B), `TZ3 the recipe saved without the ghost line (${srr?.status() ?? 'none'}, bom ${JSON.stringify((ssent?.bom || []).map(l => [l.supply, l.qty]))})`);
  const gone = await until(p, id => !document.querySelector(`[data-p="${id}"] .review`), DISH, 10000);
  check(gone, 'TZ3 after the save the row\'s review badge is gone');
  await dishSheet(p, DISH);
  const after = await p.$eval('#sheet.show #rcReview', e => e.textContent.trim()).catch(() => '');
  check(!after, `TZ3 reopened, the dish sheet has no review badge (${after || 'none'})`);
  await esc(p);
}

// ── TZ5 food cost chain ──────────────────────────────────────────────────────
async function kitchenPortion(p, dishName) {
  await stockTab(p);
  // The sheet loads its range on open; the 30-day chip loads again. Each read is
  // awaited before the next, so a late answer cannot draw over a newer one.
  const ka = (re = /\/owner\/analytics\/kitchen\?/) => p.waitForResponse(r => r.request().method() === 'GET' && re.test(r.url()), { timeout: 60000 }).catch(() => null);
  const first = ka();
  await p.click('[data-tile="numbers"]');
  if (!(await sheetOpen(p, 'kitchen'))) throw new Fail('the kitchen numbers did not open');
  const r0 = await first;
  if (!r0 || !/days=30/.test(r0.url())) {
    const again = ka(/\/owner\/analytics\/kitchen\?days=30/);
    await p.click('#sheet.show [data-kd="30"]');
    if (!(await again)) throw new Fail('the 30-day chip read no numbers');
  }
  if (!(await p.waitForSelector('#sheet.show #kaOut table', { timeout: 60000 }).then(() => true, () => false))) throw new Fail('the kitchen numbers drew no table');
  await p.waitForTimeout(800);
  const cell = await p.evaluate(name => {
    for (const t of document.querySelectorAll('#sheet.show #kaOut table')) {
      const ths = [...t.querySelectorAll('thead th')].map(h => h.dataset.t);
      const ix = ths.indexOf('ka_portionCost'); if (ix < 0) continue;
      for (const tr of t.querySelectorAll('tbody tr')) if ((tr.cells[0]?.textContent || '').trim().startsWith(name)) return tr.cells[ix]?.textContent.replace(/\s+/g, " ").trim() ?? null;
    }
    return null;
  }, dishName);
  await esc(p);
  return cell;
}
async function tz5(p) {
  const soldName = (((await own(`/api/owner/products?location_id=${LOC}`)).body?.products) || []).find(x => x.id === SOLD)?.name || SOLD;
  const was = await moneyIn(p, 1000), want = await moneyIn(p, 2000);
  // before: the dish sheet and the numbers
  await dishSheet(p, SOLD);
  const c0 = await costText(p);
  await esc(p);
  const k0 = await kitchenPortion(p, soldName);
  check(c0.includes(was), `TZ5 before: the dish sheet of ${SOLD} reads food cost ${was} (rcSum: "${c0.slice(-150)}")`);
  check(k0 === was, `TZ5 before: the kitchen numbers read portion cost ${was} for ${soldName} (cell: ${k0})`);
  // the price of the raw ingredient, on its own form
  await stockTab(p);
  await supplyForm(p, SUP_C);
  await p.fill('#sheet.show #s-cost', '2000');
  const sv = posted(p, /\/owner\/supplies$/);
  await p.click('#sheet.show #sSave');
  const svr = await sv.catch(() => null);
  check(svr?.status() === 200, `TZ5 the price of ${SUP_C} saved 1000 -> 2000 per 100 g (${svr?.status() ?? 'none'})`);
  await p.waitForTimeout(800);
  // after, by normal navigation: the numbers tile, then the menu
  const k1 = await kitchenPortion(p, soldName);
  check(k1 === want, `TZ5 after: the kitchen numbers read portion cost ${want} (cell: ${k1})`);
  await dishSheet(p, SOLD);
  const c1 = await costText(p);
  let note = '';
  if (!c1.includes(want)) {
    await esc(p); await reload(p, null); await dishSheet(p, SOLD);
    note = (await costText(p)).includes(want) ? '; after F5 it does -- the console kept an old copy of the shelf' : '; not even after F5';
  }
  check(c1.includes(want), `TZ5 after: the dish sheet of ${SOLD} reads food cost ${want} without F5 (rcSum: "${c1.slice(-150)}"${note})`);
  await esc(p);
}

// ── TZ7 session ──────────────────────────────────────────────────────────────
// The page's clock and its five-minute timer are steerable (installed before
// any script): `__tzShift` moves Date.now, a 300000 ms interval ticks every 1.5 s.
const CLOCK = () => {
  const real = Date.now.bind(Date); let shift = 0;
  Object.defineProperty(window, '__tzShift', { get: () => shift, set: v => { shift = v; } });
  Date.now = () => real() + shift;
  const si = window.setInterval.bind(window);
  window.setInterval = (fn, ms, ...a) => si(fn, ms === 300000 ? 1500 : ms, ...a);
};
const expOfTok = t => { try { return JSON.parse(Buffer.from(t.split('.')[1].replace(/-/g, '+').replace(/_/g, '/'), 'base64').toString()).exp; } catch { return null; } };

async function tz7(p, g) {
  // 1. background renewal: the clock is moved to an hour before the token ends.
  const t0 = await p.evaluate(() => sessionStorage.getItem('dw_at'));
  const exp0 = expOfTok(t0);
  if (!exp0) throw new Fail('TZ7: the console holds no readable access token');
  const realNow = await p.evaluate(() => Date.now() - window.__tzShift);
  const ref = p.waitForResponse(r => r.request().method() === 'POST' && /\/api\/auth\/refresh$/.test(r.url()), { timeout: 20000 }).catch(() => null);
  await p.evaluate(s => { window.__tzShift = s; }, exp0 - realNow - 3600_000);
  const rr = await ref;
  await p.evaluate(() => { window.__tzShift = 0; });
  // The answer lands before the page stores it: wait for the stored token to move (5 s).
  await p.waitForFunction(t => sessionStorage.getItem('dw_at') !== t, t0, { timeout: 5000 }).catch(() => {});
  const t1 = await p.evaluate(() => sessionStorage.getItem('dw_at'));
  check(rr?.status() === 200, `TZ7 an hour before the token ends the renewer calls POST /api/auth/refresh by itself (${rr ? rr.status() : 'not called in 20 s'})`);
  check(!!t1 && t1 !== t0 && (expOfTok(t1) || 0) > exp0, `TZ7 the renewed token is a new one ending later (exp ${exp0} -> ${expOfTok(t1)})`);
  await tab(p, 'stock');
  check(!(await p.$('#e')) && !!(await p.$('#nav:not([hidden])')), 'TZ7 after the renewal the console is still signed in and draws a tab');

  // 2. the session ends mid-edit; the typed text survives the sign-in.
  const typed = `${TAG} draft — typed before the session ended`;
  await dishSheet(p, DISH);
  await p.fill('#sheet.show #d-desc', typed);
  await p.waitForTimeout(300);
  const kept = await p.evaluate(s => Object.keys(localStorage).filter(k => k.startsWith('dw_draft_')).some(k => (localStorage.getItem(k) || '').includes(s)), typed);
  const book = kept ? '' : await p.evaluate(() => Object.keys(localStorage).filter(k => k.startsWith('dw_draft_')).map(k => `${k}=${(localStorage.getItem(k) || '').slice(0, 160)}`).join(' ; ') || 'no dw_draft_ key');
  check(kept, `TZ7 what is typed in the dish sheet is kept on the device as it is typed${kept ? '' : ` (${book})`}`);
  // What an ended session is to this page: the access token is refused, and so is the refresh token.
  g.expect('POST', new RegExp(`/api/owner/products/${reB(DISH)}$`), 401);
  g.expect('POST', /\/api\/auth\/refresh$/, 401); g.expect('POST', /\/api\/auth\/refresh$/, 400);
  g.expect('GET', /\/api\//, 401);
  await p.evaluate(() => {
    const t = sessionStorage.getItem('dw_at'); const [h, b, s] = t.split('.');
    const c = JSON.parse(atob(b.replace(/-/g, '+').replace(/_/g, '/'))); c.exp = Date.now() - 60_000;
    sessionStorage.setItem('dw_at', `${h}.${btoa(JSON.stringify(c)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')}.${s}`);
    localStorage.setItem('dw_rt', 'expired-refresh-token');
  });
  await p.click('#sheet.show #dSave');
  const login = await p.waitForSelector('#e', { timeout: 30000 }).then(() => true, () => false);
  const sheetOver = await p.$('#sheet.show');
  check(login, `TZ7 the save on an ended session leads to the sign-in screen${sheetOver ? ' (the dish sheet stayed open over it)' : ''}`);
  if (!login) return;
  if (sheetOver) await esc(p);
  const loginVisible = await p.$eval('#e', e => { const r = e.getBoundingClientRect(); const x = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2); return x === e || e.contains(x) || !!x?.closest?.('.login'); }).catch(() => false);
  check(loginVisible, 'TZ7 the sign-in form is on top, nothing covers it');
  await typeLogin(p);
  await dishSheet(p, DISH);
  const back = await p.$eval('#sheet.show #d-desc', e => e.value).catch(() => '');
  const noteShown = await p.$('#sheet.show .draft-note');
  check(back === typed, `TZ7 after signing in again the dish sheet has the typed text back (${JSON.stringify(back.slice(0, 80))})`);
  check(!!noteShown, 'TZ7 the sheet says the draft was restored (with its discard button)');
  if (noteShown) await p.click('#sheet.show [data-draft-discard]').catch(() => {});
  await esc(p);
}

// ── TZ8 login centred ────────────────────────────────────────────────────────
const LOGINS = [['/admin/', '#app .login', 'owner + kitchen'], ['/courier/', 'form.login#loginForm', 'courier'], ['/room/', 'form[data-form="login"]', 'room / till']];
const TOL = 4;
async function tz8() {
  for (const [vw, opts] of [['phone 390x844', PHONE], ['desktop 1280x800', { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1, serviceWorkers: 'block', reducedMotion: 'reduce' }]]) {
    // A browser per viewport: --single-process chromium goes down with its last context.
    const b = await launch(); const ctx = await b.newContext(opts); await routeLocal(ctx);
    try {
      const p = await ctx.newPage(); const g = guard(p, `login ${vw}`);
      for (const [path, sel, who] of LOGINS) {
        await go(p, `${HOST}${path}`, g);
        if (!(await p.waitForSelector(sel, { timeout: 45000 }).then(() => true, () => false))) { check(false, `TZ8 ${who} ${path} at ${vw}: no sign-in form (${sel})`); continue; }
        await p.waitForTimeout(1200);   // fonts and the layout settle
        const m = await p.$eval(sel, e => { const r = e.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2, w: r.width, h: r.height, vw: innerWidth, vh: innerHeight, sy: scrollY }; });
        const dx = Math.round(m.x - m.vw / 2), dy = Math.round(m.y - m.vh / 2);
        const f = await shot(p, `tz8-${who}-${vw}`);
        check(Math.abs(dx) <= TOL && Math.abs(dy) <= TOL, `TZ8 ${who} ${path} at ${vw}: form centre off the viewport centre by dx=${dx} dy=${dy} px (form ${Math.round(m.w)}x${Math.round(m.h)}, viewport ${m.vw}x${m.vh}, scrollY ${m.sy}; ${f})`);
      }
      check(!g.bad.length, `TZ8 ${vw}: no console error / failed request on the sign-in pages${g.bad.length ? ` (${g.bad.slice(0, 2).join(' | ')})` : ''}`);
    } finally { await ctx.close().catch(() => {}); await b.close().catch(() => {}); }
  }
}

// ── the run ──────────────────────────────────────────────────────────────────
const b = await launch();
let code = 0;
const step = async (name, p, fn) => {
  try { await fn(); } catch (e) {
    fails.push(`${name}: ${e.message}`); log(`FAIL ${name}: ${e.message}`);
    if (p) { log(`screenshot ${await shot(p, `tz-rest-${name}-fail`).catch(() => 'none')}`); await esc(p); }
  }
};
try {
  let setUp = false;
  try { await setup(); setUp = true; } catch (e) { fails.push(String(e.message)); log(`FAIL ${e.message}`); }
  if (setUp) {
    const ctx = await b.newContext(PHONE); await routeLocal(ctx); await ctx.addInitScript(CLOCK);
    const p = await ctx.newPage(); const g = guard(p, 'console');
    try {
      await signIn(p, g); log('signed in');
      await step('TZ1', p, () => tz1(p, g));
      await step('TZ3', p, () => tz3(p, g));
      await step('TZ5', p, () => tz5(p));
      await step('TZ7', p, () => tz7(p, g));
      check(!g.bad.length, `console: no unexpected console error / HTTP error / failed request${g.bad.length ? ` (${g.bad.length}: ${g.bad.slice(0, 3).join(' | ')})` : ''}`);
      try { localServed(ctx, 'TZ rest'); } catch (e) { fails.push(e.message); }
    } catch (e) {
      fails.push(String(e.message)); log(`screenshot ${await shot(p, 'tz-rest-fail').catch(() => 'none')}`);
    } finally { await ctx.close().catch(() => {}); }
  }
  // Its own browser: --single-process chromium goes down with its last context
  // (10-13: "Target page, context or browser has been closed"), as each flow launches its own.
  await step('TZ8', null, () => tz8());
} finally {
  await teardown();
  await b.close().catch(() => {});
}
if (fails.length) { console.log(`tz-rest-browser: RED -- ${fails.length} failed: ${fails.join(' | ')}`); code = 1; }
else console.log('tz-rest-browser: GREEN');
process.exit(code);
