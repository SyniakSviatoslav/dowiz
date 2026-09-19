// Source footage for the 30-second cut: the storefront, tracking and console at 9:16, dark, English.
// Run from the repo root (Playwright resolves from there): node marketing/promo-30s/capture/capture-surfaces.mjs <out-dir>
import { chromium } from 'playwright';
import fs from 'node:fs';
const OUT = process.argv[2] || 'public/promo';
const env = Object.fromEntries(fs.readFileSync('/root/.dowiz_owner', 'utf8').split('\n').filter(l => l.startsWith('export ')).map(l => l.slice(7).split('=').map(s => s.trim())));
const H = 'https://sushi-durres.dowiz.org';
const VP = { width: 540, height: 1170 };
const b = await chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage', '--use-gl=angle', '--use-angle=swiftshader', '--enable-webgl', '--ignore-gpu-blocklist', '--enable-unsafe-swiftshader'] });
const record = async (name, fn) => {
  const ctx = await b.newContext({ viewport: VP, deviceScaleFactor: 2, isMobile: true, hasTouch: true, colorScheme: 'dark', locale: 'en-US', recordVideo: { dir: OUT, size: { width: 540, height: 1170 } } });
  await ctx.addInitScript(() => { try { localStorage.setItem('dw_lang', 'en'); localStorage.setItem('dw_admin_lang', 'en'); } catch {} });
  const p = await ctx.newPage();
  try { await fn(p); } catch (e) { console.log(name, 'THREW', String(e.message).split('\n')[0].slice(0, 120)); }
  const v = p.video(); await ctx.close(); const path = await v.path(); fs.renameSync(path, `${OUT}/${name}.webm`); console.log('clip', name);
};
const login = async p => { await p.goto(`${H}/admin/`, { waitUntil: 'networkidle', timeout: 90_000 }); await p.fill('#e', env.OWNER_EMAIL); await p.fill('#p', env.OWNER_PASSWORD); await p.click('#go'); await p.waitForSelector('#nav:not([hidden])', { timeout: 30_000 }); await p.waitForTimeout(1200); };
const storeEn = async () => {};
await record('store-loader-grid-dish', async p => {
  await p.goto(`${H}/`, { waitUntil: 'commit', timeout: 90_000 }); await p.waitForSelector('.card', { timeout: 60_000 }); await p.waitForTimeout(2400); await storeEn(p);
  await p.evaluate(() => window.scrollBy({ top: 460, behavior: 'smooth' })); await p.waitForTimeout(1500);
  const cards = await p.$$('.card'); await cards[3].click(); await p.waitForTimeout(2400); await p.click('#dadd'); await p.waitForTimeout(1300); await p.evaluate(() => document.getElementById('scrim').click()); await p.waitForTimeout(1600);
});
await record('track-ocean-map', async p => {
  await p.goto(`${H}/`, { waitUntil: 'networkidle', timeout: 90_000 }); await p.waitForSelector('.card'); await p.waitForTimeout(800); await storeEn(p);
  const open = (st, eta, courier) => p.evaluate(async ({ st, eta, courier }) => { const s = await import('/store/state.js'); const m = await import('/store/track.js'); const lat = s.state.loc.lat, lng = s.state.loc.lng; m.openTracking({ id: 'promo00001', status: st, total: 1800, subtotal: 1500, payment: 'cash', fulfilment: { kind: 'delivery', address: { line: 'Rruga Taulantia 12', lat_udeg: Math.round((lat + .011) * 1e6), lon_udeg: Math.round((lng + .009) * 1e6) } }, eta: { range: eta, ...(courier ? { courierAt: { latUdeg: Math.round((lat + courier[0]) * 1e6), lonUdeg: Math.round((lng + courier[1]) * 1e6) } } : {}) } }); }, { st, eta, courier });
  await open('PREPARING', '12–18', null); await p.waitForTimeout(4000); await open('IN_DELIVERY', '6–9', [.003, .002]); await p.waitForTimeout(2500); await open('IN_DELIVERY', '3–5', [.008, .007]); await p.waitForTimeout(3500);
});
await record('admin-orders', async p => { await login(p); await p.waitForTimeout(1200); const row = await p.$('.orow'); if (row) { await row.click(); await p.waitForTimeout(2400); await p.keyboard.press('Escape'); } await p.waitForTimeout(900); });
await record('admin-posts', async p => { await login(p); await p.click('#nav .tab[data-tab="more"]'); await p.waitForSelector('[data-open="posts"]'); await p.waitForTimeout(600); await p.click('[data-open="posts"]'); await p.waitForTimeout(2500); const d = await p.$('#postDraft'); if (d) { await d.click(); await p.waitForTimeout(2500); } const first = await p.$('[data-post]'); if (first) { await first.click(); await p.waitForTimeout(2400); } });
await record('admin-assistant', async p => { await login(p); await p.click('#nav .tab[data-tab="more"]'); await p.waitForSelector('[data-open="assistant"]'); await p.click('[data-open="assistant"]'); await p.waitForTimeout(1500); await p.type('#as-q', 'How much did we make yesterday?', { delay: 45 }); await p.waitForTimeout(500); await p.evaluate(() => { document.getElementById('asOut').innerHTML = '<div class="answer mt-3">Yesterday: 23 orders, ALL 41,300 in revenue, average check ALL 1,795. Philadelphia Premium was ordered most (7). Two orders were cancelled before cooking.</div><p class="hint">Answered by the model on this server</p>'; }); await p.waitForTimeout(3000); });
await record('admin-integrations', async p => { await login(p); await p.click('#nav .tab[data-tab="more"]'); await p.waitForSelector('[data-open="integrations"]'); await p.click('[data-open="integrations"]'); await p.waitForTimeout(1800); await p.click('#igAll'); await p.waitForTimeout(6000); });
await b.close();
