// F: console Customers -- two spellings are one row; Link to... then Unlink.
import { HOST, OUT, creds, say, watch, csp, browser, api, ownerToken, save, finish } from './_lib.mjs';
const tok = await ownerToken();
const list = async () => (await api('/api/owner/customers?sort=recent&location_id=sushi-durres', { token: tok })).body.customers || [];
const b = await browser();
try {
  const ctx = await b.newContext({ viewport: { width: 420, height: 900 }, serviceWorkers: 'block' });
  const o = await ctx.newPage(); watch(o, 'console');
  const writes = [];
  o.on('response', async r => { if (/\/customers\/[^/]+\/(link|unlink)/.test(r.url())) writes.push(`${r.status()} ${r.url().replace(HOST, '')} ${(await r.text().catch(() => '')).slice(0, 200)}`); });
  await o.goto(`${HOST}/admin/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await o.waitForSelector('#e', { timeout: 40000 });
  await o.fill('#e', creds.OWNER_EMAIL); await o.fill('#p', creds.OWNER_PASSWORD); await o.click('#go');
  await o.waitForSelector('#nav:not([hidden])', { timeout: 60000 }).catch(() => {});
  await o.waitForTimeout(2500);
  const openList = async () => {
    await o.click('#nav [data-tab="more"]'); await o.waitForTimeout(1500);
    await o.click('[data-open="customers"]'); await o.waitForTimeout(3500);
    return o.$$eval('#cuBody [data-key]', els => els.map(e => ({ key: e.dataset.key, text: e.innerText.replace(/\s+/g, ' ') })));
  };
  let rows = await openList();
  const tests = rows.filter(r => /T\. W\. C\./.test(r.text));
  say(tests.length === 2, 'UI: three TEST orders over two people show as two rows (0011 twice = one row)', JSON.stringify(tests));
  await o.screenshot({ path: `${OUT}/f1-list.png` });
  const L = await list();
  const r11 = L.find(c => /11$/.test(c.phone) && /T\. W\. C\./.test(c.name));
  const r12 = L.find(c => /12$/.test(c.phone) && /T\. W\. C\./.test(c.name));
  say(!!r11 && r11.orders === 2 && (r11.linked || []).length === 1, 'API: 0011 row carries both spellings', JSON.stringify(r11));
  save({ cust11: r11?.key, cust12: r12?.key, cust11linked: r11?.linked });
  // open 0012 -> card -> Link to... -> pick 0011
  await o.click(`#cuBody [data-key="${r12.key}"]`); await o.waitForTimeout(1500);
  await o.click('#rvCard'); await o.waitForTimeout(1500);
  await o.fill('#cd-lreason', 'TEST walk: same person');
  await o.click('#cdLink'); await o.waitForTimeout(3000);
  const picks = await o.$$eval('#cdPick [data-to]', els => els.map(e => e.dataset.to));
  say(picks.includes(r11.key), 'Link to... lists the 0011 row', `${picks.length} candidates`);
  await o.screenshot({ path: `${OUT}/f2-pick.png` });
  await o.click(`#cdPick [data-to="${r11.key}"]`); await o.waitForTimeout(3000);
  say(null, 'link writes', JSON.stringify(writes));
  await o.keyboard.press('Escape'); await o.waitForTimeout(800);
  await o.keyboard.press('Escape'); await o.waitForTimeout(800);
  rows = await openList();
  const t2 = rows.filter(r => /T\. W\. C\./.test(r.text));
  say(t2.length === 1, 'UI after link: one row', JSON.stringify(t2));
  const L2 = await list(); const m = L2.find(c => (c.linked || []).includes(r12.key) || c.key === r11.key);
  say(!!m && (m.linked || []).includes(r12.key), 'API after link: 0012 linked into 0011', JSON.stringify(m));
  await o.screenshot({ path: `${OUT}/f3-linked.png` });
  // unlink from the merged row's card
  await o.click(`#cuBody [data-key="${m.key}"]`); await o.waitForTimeout(1500);
  await o.click('#rvCard'); await o.waitForTimeout(1500);
  await o.fill('#cd-lreason', 'TEST walk: undo');
  const ub = await o.$(`[data-unlink="${r12.key}"]`);
  say(!!ub, 'card lists the linked 0012 with Unlink', await o.$$eval('[data-unlink]', e => e.map(x => x.dataset.unlink).join(',')));
  await o.screenshot({ path: `${OUT}/f4-card.png`, fullPage: true });
  await ub?.click(); await o.waitForTimeout(3000);
  say(null, 'unlink writes', JSON.stringify(writes));
  await o.keyboard.press('Escape'); await o.waitForTimeout(800); await o.keyboard.press('Escape'); await o.waitForTimeout(800);
  rows = await openList();
  const t3 = rows.filter(r => /T\. W\. C\./.test(r.text));
  say(t3.length === 2, 'UI after unlink: two rows', JSON.stringify(t3));
  const L3 = await list();
  say(null, 'API after unlink', JSON.stringify(L3.filter(c => /T\. W\. C\./.test(c.name))));
  await csp(o, 'customers');
} finally { await b.close(); finish(); }
