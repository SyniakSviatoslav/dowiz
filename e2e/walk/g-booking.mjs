// G: storefront booking as a GUEST (time first), then what the owner/floor see.
import { HOST, OUT, say, watch, csp, browser, api, save, finish } from './_lib.mjs';
import { devices } from 'playwright';
const b = await browser();
try {
  const ctx = await b.newContext({ ...devices['iPhone 14 Pro'], serviceWorkers: 'block' });
  const c = await ctx.newPage(); watch(c, 'store-book');
  const posts = [];
  c.on('response', async r => { if (/\/reservations$/.test(r.url()) && r.request().method() === 'POST') posts.push({ status: r.status(), body: (await r.text().catch(() => '')).slice(0, 300), sentAuth: !!r.request().headers()['authorization'], req: r.request().postData() }); });
  await c.goto(`${HOST}/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await c.waitForSelector('.card', { timeout: 60000 });
  await c.waitForTimeout(3500);
  for (let i = 0; i < 6; i++) { const n = await c.evaluate(() => document.getElementById('sheet')?.dataset.name || ''); if (!n) break; const l = await c.$('#insLater'); if (l) await l.click().catch(() => {}); else await c.evaluate(() => document.getElementById('scrim')?.click()); await c.waitForTimeout(600); }
  const tab = await c.$('[data-tab="book"]');
  say(!!tab, 'storefront nav has Book');
  await tab?.click(); await c.waitForTimeout(1500);
  const times = await c.$$eval('[data-min]', els => els.map(e => e.dataset.min + '=' + e.textContent.trim()));
  say(times.length > 0, 'booking offers times', times.slice(0, 6).join(' '));
  // the first offered time today (closest to now) keeps the hold within the floor's 90-min window
  const first = await c.$('[data-min]');
  await first?.click(); await c.waitForTimeout(3500);
  const plan = await c.evaluate(() => ({ tabs: [...document.querySelectorAll('.bk-tab')].map(e => e.textContent), tables: [...document.querySelectorAll('.bk-t')].map(e => e.getAttribute('aria-label')), text: document.querySelector('.bk')?.innerText.replace(/\s+/g, ' ').slice(0, 300) }));
  say(plan.tables.length > 0, 'plan for the chosen time', JSON.stringify(plan));
  const t91 = await c.$('.bk-t[data-n="91"]');
  await t91?.click(); await c.waitForTimeout(700);
  const foot = await c.$eval('.bk-foot', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
  say(null, 'chosen line', foot);
  await c.click('#bkGo').catch(e => say(false, 'book button', e.message.slice(0, 80)));
  await c.waitForTimeout(4000);
  const toast = await c.evaluate(() => [...document.querySelectorAll('.toast, #toast')].map(e => e.textContent).join(' | '));
  say(posts.some(p => p.status === 200), 'guest booking lands', JSON.stringify(posts) + ' toast=' + toast);
  save({ bookingPosts: posts });
  await csp(c, 'store-book');
} finally { await b.close(); finish(); }
