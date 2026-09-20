// Storefront across devices and engines: the whole customer flow short of placing an order.
import { chromium, firefox, devices } from 'playwright';
const OUT = '/tmp/claude-0/-root/0ee0cdf2-e369-472c-a954-cbca99cc3bfd/scratchpad';
const HOST = 'https://sushi-durres.dowiz.org/';
const MATRIX = [
  ['chromium', 'iPhone SE', devices['iPhone SE']], ['chromium', 'Galaxy S24', devices['Galaxy S24']], ['chromium', 'iPhone 14 Pro', devices['iPhone 14 Pro']],
  ['chromium', 'Pixel 7', devices['Pixel 7']], ['chromium', 'iPhone 15 Pro Max', devices['iPhone 15 Pro Max']], ['chromium', 'iPad Mini', devices['iPad Mini']],
  ['chromium', 'desktop 1280', { viewport: { width: 1280, height: 800 } }], ['firefox', 'phone 360 (firefox)', { viewport: { width: 360, height: 780 } }], ['firefox', 'desktop 1280 (firefox)', { viewport: { width: 1280, height: 800 } }],
];
const gl = ['--no-sandbox', '--disable-dev-shm-usage', '--use-gl=swiftshader', '--enable-webgl', '--ignore-gpu-blocklist', '--enable-unsafe-swiftshader'];
const browsers = { chromium: await chromium.launch({ args: gl }), firefox: await firefox.launch() };
for (const [engine, name, dev] of MATRIX.filter(([, n]) => !process.env.ONLY || n === process.env.ONLY)) {
  const opts = { ...dev }; if (engine === 'firefox') { delete opts.isMobile; delete opts.hasTouch; delete opts.defaultBrowserType; } else delete opts.defaultBrowserType;
  const ctx = await browsers[engine].newContext(opts); const p = await ctx.newPage();
  const issues = []; let phase = 'load';
  p.on('pageerror', e => issues.push(`${phase}: PAGEERROR ${e.message.slice(0, 120)}`)); p.on('console', m => { if (m.type() === 'error' && !/CF\$cv|Content Security|WebGL|GPU|swiftshader/i.test(m.text())) issues.push(`${phase}: ${m.text().slice(0, 100)}`); });
  p.on('response', r => { if (r.status() >= 400) issues.push(`${phase}: ${r.status()} ${r.url().slice(0, 70)}`); });
  const overflow = async tag => { const w = await p.evaluate(() => [document.documentElement.scrollWidth, innerWidth]); if (w[0] > w[1] + 1) issues.push(`${tag}: overflow ${w[0]}>${w[1]}`); };
  try {
    await p.goto(HOST, { waitUntil: 'networkidle', timeout: 90_000 }); await p.waitForSelector('.card', { timeout: 40_000 }); await p.waitForTimeout(800);
    const cards = await p.$$('.card'); if (cards.length < 100) issues.push(`load: only ${cards.length} cards`); await overflow('load');
    phase = 'search'; const q = await p.$('input[type=search]'); if (q) { await q.fill('sake'); await p.waitForTimeout(700); const n = await p.evaluate(() => [...document.querySelectorAll('.card')].filter(c => c.offsetParent !== null && !c.hidden).length); if (n < 1 || n > 60) issues.push(`search: ${n} visible for "sake"`); await q.fill(''); await p.waitForTimeout(500); } else issues.push('search: no input');
    phase = 'lang'; const before = await p.evaluate(() => document.body.innerText.slice(0, 400)); await p.click('#langBtn'); await p.waitForSelector('[data-l="en"]', { timeout: 10000 }).catch(() => {}); await p.waitForTimeout(300); const en = await p.$('[data-l="en"]'); if (en) { await en.click(); await p.waitForTimeout(900); await p.mouse.click(8, 200); await p.waitForTimeout(500); const after = await p.evaluate(() => document.body.innerText.slice(0, 400)); if (after === before) issues.push('lang: text unchanged after EN'); } else issues.push('lang: no chip');
    if (process.env.ONLY) console.log('lang-state', JSON.stringify(await p.evaluate(() => ({ sheet: document.getElementById('sheet').dataset.name, scrim: document.getElementById('scrim').className, chips: document.querySelectorAll('[data-l]').length }))));
    phase = 'currency'; if (await p.evaluate(() => document.getElementById('scrim').classList.contains('show'))) { issues.push('lang: sheet re-shown after close (observed)'); await p.evaluate(() => document.getElementById('scrim').click()); await p.waitForTimeout(500); } await p.click('#curBtn'); await p.waitForSelector('[data-c="EUR"]', { timeout: 10000 }).catch(() => {}); await p.waitForTimeout(300); const eur = await p.$('[data-c="EUR"]'); if (eur) { await eur.click(); await p.waitForTimeout(900); const has = await p.evaluate(() => /€|EUR/.test(document.body.innerText)); if (!has) issues.push('currency: no € after EUR'); await p.click('#curBtn'); await p.waitForTimeout(400); const all = await p.$('[data-c="ALL"]'); if (all) await all.click(); await p.waitForTimeout(400); await p.mouse.click(8, 200); await p.waitForTimeout(500); } else issues.push('currency: no EUR control');
    phase = 'dish'; await (await p.$('.card')).click(); await p.waitForTimeout(1000); const dn = await p.evaluate(() => document.getElementById('sheet')?.dataset.name); if (dn !== 'dish') issues.push(`dish: sheet=${dn}`); await overflow('dish');
    const add = await p.$('#dadd'); if (add) { await add.click(); await p.waitForTimeout(800); } else issues.push('dish: no add button');
    await p.keyboard.press('Escape'); await p.waitForTimeout(400);
    phase = 'cart'; const cnt = await p.evaluate(() => { const el = document.getElementById('cartPill'); return el && !el.hidden && el.offsetParent !== null ? el.innerText.replace(/\s+/g, ' ').slice(0, 40) : null; }); if (!cnt) issues.push('cart: pill not shown after add');
    const cartOpen = await p.$('#cartPill'); if (cartOpen) { await cartOpen.click({ timeout: 5000 }).catch(() => issues.push('cart: bar not clickable')); await p.waitForTimeout(900); const cn = await p.evaluate(() => document.getElementById('sheet')?.dataset.name); if (cn !== 'cart') issues.push(`cart: sheet=${cn}`); await overflow('cart'); const plus = await p.$('[data-a]'); if (plus) { await plus.click(); await p.waitForTimeout(400); } }
    phase = 'checkout'; const co = await p.$('#toCheckout'); if (co) { await co.click({ timeout: 5000 }).catch(() => issues.push('checkout: button not clickable')); await p.waitForTimeout(1200); const cn = await p.evaluate(() => document.getElementById('sheet')?.dataset.name); if (!/checkout|address|order/.test(cn || '')) issues.push(`checkout: sheet=${cn}`); await overflow('checkout'); const fields = await p.evaluate(() => [...document.querySelectorAll('#sheetIn input, #sheetIn select, #sheetIn button')].length); if (fields < 4) issues.push(`checkout: only ${fields} controls`); } else issues.push('checkout: no button');
    await p.keyboard.press('Escape'); await p.waitForTimeout(300);
    phase = 'track'; await p.evaluate(async () => { const s = await import('/store/state.js'); const m = await import('/store/track.js'); const lat = s.state.loc.lat, lng = s.state.loc.lng; m.openTracking({ id: 'probemap02', status: 'IN_DELIVERY', total: 1800, subtotal: 1500, payment: 'cash', fulfilment: { kind: 'delivery', address: { line: 'Rruga 1', lat_udeg: Math.round((lat + .01) * 1e6), lon_udeg: Math.round((lng + .01) * 1e6) } }, eta: { range: '8–12', courierAt: { latUdeg: Math.round((lat + .004) * 1e6), lonUdeg: Math.round((lng + .003) * 1e6) } } }); });
    await p.waitForTimeout(3500); const tr = await p.evaluate(() => ({ st: document.querySelector('.ep-title')?.innerText, markers: document.querySelectorAll('.tm').length, ocean: !!document.querySelector('.ocean-cv'), map: !!document.querySelector('#epMap canvas') })); if (tr.markers !== 3) issues.push(`track: markers=${tr.markers}`); if (!tr.ocean) issues.push('track: no ocean'); if (!tr.map) issues.push('track: no map canvas'); await overflow('track');
    await p.screenshot({ path: `${OUT}/mx-store-${name.replace(/[^a-z0-9]+/gi, '_')}.png` }).catch(() => {});
    await p.keyboard.press('Escape');
  } catch (e) { const why = String(e.message).split('\n').filter(l => /intercepts|not visible|viewport|stable|retrying|element is/.test(l)).slice(0, 4).join(' / '); issues.push(`${phase}: THREW ${String(e.message).split('\n')[0].slice(0, 80)} :: ${why.slice(0, 260)}`); }
  console.log(`${engine} · ${name}: ${issues.length ? issues.join(' | ') : 'OK'}`);
  await ctx.close();
}
for (const b of Object.values(browsers)) await b.close();
