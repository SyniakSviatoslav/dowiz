// F9 OWNER SCREENS: every tab of the owner's bar (orders, kitchen, menu, stock, couriers, more) and,
// from More, Analytics and Customers, each drawn with no console error and no failed request; then the
// RAPID TAB SWITCH, on a desktop (1366x860) and a phone: every tab tapped ~40 ms apart, four rounds,
// then one target -- within 8 s #app must be that tab and the bar must light it, with no page error.
// (2026-10-17: the live console drew the wrong screen under the tapped tab and threw "Transition was
// skipped"; fixed in admin/app.js 371d39a2, NOT deployed when this was written -- red until deployed.)
import { Fail } from './lib.mjs';
import { launch, inspect, shot, until, localServed } from './page.mjs';
import { C, softFail, endSoft, openConsole, openTab } from './cycle-lib.mjs';

const drawnOk = txt => txt.length > 20 && !/\bundefined\b|\bNaN\b|\[object /.test(txt);

async function rapid(bad, label, viewport) {
  // its own browser: a second context in this single-process Chromium kills the first (page.mjs)
  const b = await launch();
  try { await rapidIn(bad, b, label, viewport); }
  catch (e) { softFail(bad, false, `rapid tab switch (${label}): ${String(e.message).split('\n')[0]}`); }
  finally { await b.close().catch(() => {}); }
}
async function rapidIn(bad, b, label, viewport) {
  const { p, g, ctx } = await openConsole(b, `owner-F9-${label}`, viewport);
  const errs = [];
  p.on('pageerror', e => errs.push(e.message.split('\n')[0].slice(0, 160)));
  const tabs = await p.$$eval('#nav [data-tab]', els => els.map(e => e.dataset.tab));
  const target = tabs.includes('menu') ? 'menu' : tabs[0];
  await p.evaluate(async ({ tabs, rounds, gap }) => {
    const w = ms => new Promise(r => setTimeout(r, ms));
    for (let i = 0; i < rounds; i++) for (const t of tabs) { document.querySelector(`#nav [data-tab="${t}"]`)?.click(); await w(gap); }
  }, { tabs, rounds: 4, gap: 40 });
  await p.click(`#nav [data-tab="${target}"]`);
  const ok = await until(p, t => document.getElementById('app')?.dataset.tab === t
    && document.querySelector('#nav [aria-current="page"]')?.dataset.tab === t, target, 8000);
  await p.waitForTimeout(1000);
  const st = await p.evaluate(() => ({ app: document.getElementById('app')?.dataset.tab, lit: document.querySelector('#nav [aria-current="page"]')?.dataset.tab }));
  const f = await shot(p, `F9-rapid-${label}`);
  localServed(ctx, `F9 rapid ${label}`);
  softFail(bad, ok && errs.length === 0 && g.bad.length === 0,
    `rapid tab switch (${label} ${viewport.width}x${viewport.height}, ${tabs.length} tabs x4 at 40 ms, then ${target}): #app=${st.app} lit=${st.lit}; page errors: ${errs.slice(0, 2).join(' | ') || 'none'}${g.bad.length ? `; ${g.bad[0]}` : ''} [${f}]`);
}

export async function f9(log) {
  const bad = [];
  const b = await launch();
  try {
    const { p, g, ctx } = await openConsole(b, 'owner-F9');
    const tabs = await p.$$eval('#nav [data-tab]', els => els.map(e => e.dataset.tab));
    log(`owner bar: ${tabs.join(', ')}`);
    for (const t of tabs) {
      const n0 = g.bad.length;
      await openTab(p, t);
      const txt = await p.$eval('#app', e => e.innerText).catch(() => '');
      softFail(bad, drawnOk(txt), `${t} opens and draws (${txt.length} chars)`);
      softFail(bad, g.bad.length === n0, `${t}: no console error or failed request (${g.bad.slice(n0, n0 + 2).join(' | ') || 'none'})`);
      await inspect(p, `F9 ${t}`).catch(e => softFail(bad, false, e.message));
      await shot(p, `F9-${t}`);
    }
    // Analytics and Customers live under More (admin/more.js GROUPS)
    for (const key of ['analytics', 'customers']) {
      await openTab(p, 'more');
      const n0 = g.bad.length;
      const tile = await p.$(`#app [data-open="${key}"]`);
      if (!softFail(bad, !!tile, `More offers ${key}`)) continue;
      await tile.click();
      await until(p, () => !!document.getElementById('sheet')?.dataset.name, null, 15000);
      await p.waitForTimeout(3500);
      const txt = await p.$eval('#sheetIn', e => e.innerText).catch(() => '');
      softFail(bad, drawnOk(txt), `${key} opens from More and draws (${txt.length} chars)`);
      softFail(bad, g.bad.length === n0, `${key}: no console error or failed request (${g.bad.slice(n0, n0 + 2).join(' | ') || 'none'})`);
      await shot(p, `F9-${key}`);
      await p.keyboard.press('Escape').catch(() => {});
      await p.evaluate(() => document.getElementById('scrim')?.click()).catch(() => {});
      await p.waitForTimeout(800);
    }
    await openTab(p, 'orders');
    const rows = await p.$$eval('.orow[data-o]', e => e.map(x => x.dataset.o));
    await p.click('[data-mode="history"]').catch(() => {});
    await p.waitForTimeout(1500);
    rows.push(...await p.$$eval('.orow[data-o]', e => e.map(x => x.dataset.o)));
    const have = (C().guests || []).filter(x => x.id).map(x => x.id);
    const missing = have.filter(id => !rows.includes(id));
    softFail(bad, !have.length || missing.length === 0, `Orders (live + history) lists this run's ${have.length} guest orders (missing ${missing.map(x => x.slice(0, 8)).join(',') || 'none'})`);
  } catch (e) {
    { const pg = b.contexts()[0]?.pages()[0]; if (pg) e.shot = await shot(pg, 'F9-fail'); }
    throw e;
  } finally { await b.close().catch(() => {}); }
  await rapid(bad, 'desktop', { width: 1366, height: 860 });
  await rapid(bad, 'phone', { width: 390, height: 844 });
  endSoft(bad, 'F9');
}
