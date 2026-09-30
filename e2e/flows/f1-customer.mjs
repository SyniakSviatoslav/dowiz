// F1 CUSTOMER, on the storefront at 390x844: the menu, the category rail, two
// dishes into the cart, checkout as PICKUP paying CASH, the status page at
// PENDING -- the page's word and the API's must agree -- and the language
// switch through sq/en/uk/ru with no raw key anywhere.
//
// qa-durres has pickup switched off, so the flow switches it on through the
// owner's own route first and F4 puts back what it found (S.restore.pickup).
// No dish on qa-durres has a modifier group; the flow says so from the API
// rather than pretending to have tested one.
import { HOST, LOC, RUN, S, save, Fail, api, own, menu, dishes, opened, statusOf } from './lib.mjs';
import { launch, go, PHONE, routeLocal, localServed, guard, clean, inspect, shot, until } from './page.mjs';

const sheetName = p => p.evaluate(() => document.getElementById('sheet')?.dataset.name || '');

/// Close what the storefront opens by itself on a first visit (install offer,
/// consent) -- never a sheet a tap opened. Bounded: six taps.
async function dismiss(p) {
  for (let i = 0; i < 6; i++) {
    const n = await sheetName(p);
    if (!n) return;
    const later = await p.$('#insLater');
    if (later) await later.click().catch(() => {}); else await p.evaluate(() => document.getElementById('scrim')?.click());
    await p.waitForTimeout(600);
  }
  throw new Fail(`a sheet opened by itself will not close: ${await sheetName(p)}`);
}
async function closeSheet(p) {
  await p.evaluate(() => document.getElementById('scrim')?.click());
  if (!(await until(p, () => !document.getElementById('sheet')?.dataset.name, null, 5000))) {
    await p.keyboard.press('Escape');
    if (!(await until(p, () => !document.getElementById('sheet')?.dataset.name, null, 5000))) throw new Fail(`sheet "${await sheetName(p)}" does not close`);
  }
}

async function addDish(p, g, id) {
  await p.click(`.card[data-p="${id}"]`);
  if (!(await until(p, () => document.getElementById('sheet')?.dataset.name === 'dish'))) throw new Fail(`tapping dish ${id} opened no dish sheet`);
  await inspect(p, `F1 dish sheet ${id}`);
  const n0 = Number(await p.$eval('#pillCount', e => e.textContent).catch(() => '0')) || 0;
  await p.click('#dadd');
  if (!(await until(p, n => Number(document.getElementById('pillCount')?.textContent) === n + 1, n0))) throw new Fail(`adding ${id}: cart count did not go ${n0} -> ${n0 + 1}`);
  if (await sheetName(p)) await closeSheet(p);
  clean(g, `F1 add ${id}`);
}

export async function f1(log) {
  // ── setup: the modifier question, and pickup on ─────────────────────────
  const m0 = await menu();
  const all = dishes(m0).filter(d => d.available);
  if (all.length < 2) throw new Fail(`qa-durres lists ${all.length} dishes on sale; the flow needs 2 (run e2e/walk/qa-setup.mjs)`);
  const withMod = all.find(d => (d.modifierGroups || []).length);
  log(withMod ? `dish with a modifier: ${withMod.id}` : `no dish on ${LOC} has a modifier group (menu API: modifierGroups null on all ${all.length}) -- modifier step not applicable`);
  if (S.restore.pickup === undefined) { S.restore.pickup = !!m0.location?.pickup; save(); }
  if (!m0.location?.pickup) {
    const r = await own('/api/owner/location', { location_id: LOC, pickup: true });
    if (r.status !== 200) throw new Fail(`switching pickup on: ${r.status} ${r.text.slice(0, 120)}`);
    if (!(await menu()).location?.pickup) throw new Fail('pickup switched on but the public menu still says pickup:false');
  }
  const cats = (m0.categories || []).filter(c => (c.products || []).some(d => d.available));
  const [d1, d2] = [cats[0].products.find(d => d.available), (cats[1] || cats[0]).products.filter(d => d.available).slice(-1)[0]];

  const b = await launch();
  try {
    const ctx = await b.newContext(PHONE); await routeLocal(ctx);
    const p = await ctx.newPage(); const g = guard(p, 'store');
    let placed = null;
    p.on('response', async r => {
      if (/\/orders$/.test(r.url()) && r.request().method() === 'POST' && r.status() < 400) { try { placed = await r.json(); } catch {} }
    });
    await go(p, `${HOST}/`, g);
    if (!(await p.waitForSelector('.card', { timeout: 60000 }).then(() => true, () => false))) throw new Fail('the menu never drew a dish card');
    await p.waitForTimeout(3500);
    await dismiss(p);
    const cards = await p.$$eval('.card[data-p]', e => e.length);
    if (cards !== all.length) throw new Fail(`menu shows ${cards} dish cards, the API lists ${all.length} on sale`);
    await inspect(p, 'F1 menu');
    log(`menu: ${cards} cards, matching the API`);

    // ── category nav ──────────────────────────────────────────────────────
    const chips = await p.$$eval('.rail-chip[data-c]', e => e.map(x => x.dataset.c));
    if (chips.length !== cats.length) throw new Fail(`category rail has ${chips.length} chips, the menu has ${cats.length} categories`);
    const last = chips[chips.length - 1];
    await p.click(`.rail-chip[data-c="${last}"]`);
    if (!(await until(p, c => { const s = document.getElementById('c-' + c); return !!s && s.getBoundingClientRect().top < 260; }, last, 8000)))
      throw new Fail(`tapping category ${last} did not bring its section to the top`);
    if (!(await until(p, c => document.querySelector(`.rail-chip[data-c="${c}"]`)?.getAttribute('aria-current') === 'true', last, 5000)))
      throw new Fail(`category chip ${last} is not marked current after the tap`);
    log(`category nav: ${chips.length} chips, ${last} scrolled into view and marked current`);

    // ── two dishes, the cart ──────────────────────────────────────────────
    await addDish(p, g, d2.id);
    await addDish(p, g, d1.id);
    await p.click('#cartPill');
    if (!(await until(p, () => document.getElementById('sheet')?.dataset.name === 'cart'))) throw new Fail('the cart pill opened no cart');
    const cartText = await p.$eval('#sheetIn', e => e.innerText);
    for (const d of [d1, d2]) if (!cartText.includes(d.name)) throw new Fail(`the cart does not list ${d.name}`);
    await inspect(p, 'F1 cart');
    await p.click('#toCheckout');
    // The checkout is a dynamic import: on this box's network it took > 20 s.
    if (!(await until(p, () => document.getElementById('sheet')?.dataset.name === 'checkout', null, 60000))) throw new Fail('checkout did not open in 60 s');

    // ── checkout: pickup + cash ───────────────────────────────────────────
    if (!(await p.$('[data-how="pickup"]'))) throw new Fail('checkout offers no pickup choice although the venue has pickup on');
    await p.click('[data-how="pickup"]');
    if (!(await until(p, () => document.getElementById('addrBox')?.hidden === true, null, 5000))) throw new Fail('choosing pickup did not hide the address');
    await p.fill('#f-name', `${RUN} guest`);
    await p.fill('#f-phone', '+355690000009');
    await p.fill('#f-note', 'QA flows gate -- not a real order');
    const cash = await p.$('[data-pay="cash"]');
    if (!cash) throw new Fail('checkout offers no cash payment');
    await cash.click();
    await inspect(p, 'F1 checkout');
    await p.click('#place');
    for (let i = 0; i < 60 && !placed; i++) await p.waitForTimeout(500);
    if (!placed?.id) throw new Fail(`no order placed: ${await p.$eval('#f-err', e => e.innerText).catch(() => '')}`);
    opened(placed.id, placed.access_token, 'F1');
    log(`order placed: ${placed.id}`);

    // ── the status page, and the API ──────────────────────────────────────
    if (!(await until(p, () => !!document.querySelector('.tsheet[data-status]'), null, 20000))) throw new Fail('no status page after placing');
    const onPage = await p.$eval('.tsheet', e => e.dataset.status);
    const inApi = await statusOf(placed.id);
    if (onPage !== 'PENDING' || inApi !== 'PENDING') throw new Fail(`status: page=${onPage} api=${inApi}, want PENDING both`);
    const o = await api(`/api/order/${placed.id}`, { token: placed.access_token });
    if (o.body?.fulfilment?.kind !== 'pickup' || !/cash/.test(JSON.stringify(o.body?.payment ?? o.body?.payment_method ?? ''))) throw new Fail(`the stored order is not pickup+cash: ${o.text.slice(0, 160)}`);
    await inspect(p, 'F1 status page');
    clean(g, 'F1 status page');
    log(`status page PENDING = API PENDING, stored as pickup + cash`);

    // ── the language switch ───────────────────────────────────────────────
    await closeSheet(p);
    for (const L of ['sq', 'en', 'uk', 'ru']) {
      await p.click('#langBtn');
      if (!(await until(p, () => document.getElementById('sheet')?.dataset.name === 'lang', null, 8000))) throw new Fail('the language button opened no sheet');
      const btn = await p.$(`#sheetIn [data-l="${L}"]`);
      if (!btn) throw new Fail(`the language sheet offers no ${L}`);
      await btn.click();
      if (!(await until(p, l => document.documentElement.lang === l, L, 8000))) throw new Fail(`choosing ${L}: <html lang> stayed ${await p.evaluate(() => document.documentElement.lang)}`);
      await p.waitForTimeout(1500);
      await inspect(p, `F1 language ${L} sheet`);
      await closeSheet(p);
      await inspect(p, `F1 language ${L} menu`);
    }
    // The status page again, in the last language, from "my orders".
    await p.click('#nav [data-tab="orders"]');
    if (!(await p.waitForSelector('.hist[data-o]', { timeout: 20000 }).then(() => true, () => false))) throw new Fail('"my orders" lists nothing after an order');
    await p.click('.hist[data-o]');
    if (!(await until(p, () => !!document.querySelector('.tsheet[data-status]'), null, 20000))) throw new Fail('the order from "my orders" opened no status page');
    await inspect(p, 'F1 status page (ru)');
    clean(g, 'F1 languages');
    log('language switch sq/en/uk/ru: no raw key, <html lang> followed each');
    localServed(ctx, 'F1');
  } catch (e) {
    if (e instanceof Fail) { const pg = b.contexts()[0]?.pages()[0]; if (pg) e.shot = await shot(pg, 'F1-fail'); }
    throw e;
  } finally { await b.close().catch(() => {}); }
}
