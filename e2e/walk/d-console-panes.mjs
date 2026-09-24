// D: console -- Refund the TEST round (REFUNDING -> money handed back), then
// the Printer, Exceptions, e-bills and Health panes (read only).
import { HOST, OUT, creds, say, watch, csp, browser, api, ownerToken, st, finish } from './_lib.mjs';
const ID = process.env.ORDER || st().roomOrder;
const DO_REFUND = process.env.REFUND !== '0';
const b = await browser();
try {
  const ctx = await b.newContext({ viewport: { width: 420, height: 900 }, serviceWorkers: 'block' });
  const o = await ctx.newPage(); watch(o, 'console');
  const refunds = [];
  o.on('response', async r => { if (/\/refund$/.test(r.url())) { let body; try { body = await r.json(); } catch { body = await r.text().catch(() => ''); } refunds.push({ status: r.status(), body: JSON.stringify(body).slice(0, 300) }); } });
  await o.goto(`${HOST}/admin/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await o.waitForSelector('#e', { timeout: 40000 });
  await o.fill('#e', creds.OWNER_EMAIL); await o.fill('#p', creds.OWNER_PASSWORD); await o.click('#go');
  await o.waitForSelector('#nav:not([hidden])', { timeout: 60000 }).catch(() => {});
  await o.waitForTimeout(3000);
  const tok = await ownerToken();
  const orderStatus = async () => { const r = await api(`/api/owner/orders?location_id=sushi-durres`, { token: tok }); const l = Array.isArray(r.body) ? r.body : (r.body.orders || []); return l.find(x => x.id === ID)?.status ?? `(not listed; http ${r.status})`; };
  if (DO_REFUND) {
    const row = await o.waitForSelector(`.orow[data-o="${ID}"]`, { timeout: 20000 }).catch(() => null);
    say(!!row, 'TEST round row on console');
    await row?.click(); await o.waitForTimeout(1500);
    const rb = await o.$(`[data-refund="${ID}"]`);
    say(!!rb, 'order sheet offers Refund on READY');
    await o.screenshot({ path: `${OUT}/d1-order.png` });
    await rb?.click(); await o.waitForTimeout(1000);
    await o.selectOption('#rf-why', 'other').catch(e => say(false, 'refund reason select', e.message.slice(0, 80)));
    await o.fill('#rf-note', 'TEST walk 2026-09-24, not a real sale');
    await o.screenshot({ path: `${OUT}/d2-refund-sheet.png` });
    await o.click('#rfGo'); await o.waitForTimeout(4000);
    say(null, 'refund responses', JSON.stringify(refunds));
    let s = await orderStatus();
    say(s === 'REFUNDING' || s === 'COMPENSATED_REFUND', 'refund started', `status=${s}`);
    const mb = await o.$(`[data-moneyback="${ID}"]`);
    say(!!mb || s === 'COMPENSATED_REFUND', 'order sheet offers Money handed back', mb ? 'yes' : 'no button');
    if (mb) { await mb.click(); await o.waitForTimeout(4000); }
    s = await orderStatus();
    say(s === 'COMPENSATED_REFUND', 'refund complete', `status=${s}; responses=${JSON.stringify(refunds)}`);
    await o.screenshot({ path: `${OUT}/d3-refunded.png` });
    await o.keyboard.press('Escape'); await o.waitForTimeout(600);
  }
  // MORE tab
  const navs = await o.$$eval('#nav [data-tab], #nav button', els => els.map(e => (e.dataset.tab || '') + ':' + e.textContent.trim()));
  say(null, 'nav', navs.join(' | '));
  const more = await o.$('#nav [data-tab="more"]') || (await o.$$('#nav button')).slice(-1)[0];
  await more?.click(); await o.waitForTimeout(2000);
  for (const key of ['printer', 'exceptions', 'ebills', 'health']) {
    const tile = await o.$(`[data-open="${key}"]`);
    if (!tile) { say(false, `tile ${key}`, 'missing'); continue; }
    const before = (await import('./_lib.mjs')).issues.length;
    await tile.click(); await o.waitForTimeout(4000);
    const txt = await o.$eval('#sheetIn', e => e.innerText.replace(/\s+/g, ' ').slice(0, 900)).catch(() => '(no sheet)');
    const ctrls = await o.$$eval('#sheetIn button, #sheetIn input, #sheetIn select', els => els.map(e => e.id || e.dataset.t || e.tagName)).catch(() => []);
    say(null, `pane ${key}`, `${txt}\n      controls: ${ctrls.join(',')}`);
    await o.screenshot({ path: `${OUT}/d-${key}.png`, fullPage: true });
    if (key === 'exceptions') {
      await o.waitForTimeout(1500);
      const legs = await o.$eval('#exLegs', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '(no #exLegs)');
      say(null, 'exceptions wallet legs', legs);
      const chk = await o.$('#exLegsCheck');
      if (chk) { await chk.click(); await o.waitForTimeout(3000); say(null, 'legs dry-run', await o.$eval('#exLegsOut', e => e.innerText).catch(() => '')); }
    }
    await csp(o, `pane-${key}`);
    await o.keyboard.press('Escape'); await o.waitForTimeout(800);
  }
  const x = await api(`/api/owner/exceptions?location_id=sushi-durres&from_ms=${Date.now() - 3 * 3600e3}`, { token: tok });
  say(null, 'API exceptions', JSON.stringify(x).slice(0, 700));
  const pj = await api(`/api/owner/print/jobs?location_id=sushi-durres`, { token: tok });
  say(null, 'API print jobs', JSON.stringify(pj).slice(0, 300));
} finally { await b.close(); finish(); }
