// H: room Floor with a plan -- tables by zone, with states (a TEST booking holds 91).
import { HOST, OUT, creds, say, watch, csp, browser, api, staffToken, finish } from './_lib.mjs';
const b = await browser();
try {
  const ctx = await b.newContext({ viewport: { width: 400, height: 860 }, serviceWorkers: 'block' });
  const p = await ctx.newPage(); watch(p, 'room');
  await p.goto(`${HOST}/room/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await p.waitForSelector('form[data-form="login"]', { timeout: 40000 });
  await p.fill('input[name="email"]', creds.OWNER_EMAIL); await p.fill('input[name="password"]', creds.OWNER_PASSWORD);
  await p.click('form[data-form="login"] button[type="submit"]');
  await p.waitForSelector('[data-act="floor"]', { timeout: 40000 }); await p.waitForTimeout(1500);
  await p.click('[data-act="floor"]'); await p.waitForTimeout(4000);
  const fl = await p.evaluate(() => ({
    zones: [...document.querySelectorAll('.floor-zone')].map(z => ({ name: z.querySelector('h3')?.textContent, tables: [...z.querySelectorAll('g.ft')].map(g => g.className.baseVal + ' | ' + g.getAttribute('aria-label')) })),
  }));
  say(fl.zones.length === 1 && fl.zones[0].tables.some(t => /fs-booked/.test(t) && /91/.test(t)), 'floor draws the zone; TEST booking shows table 91 booked', JSON.stringify(fl));
  const box = await p.$eval('svg.floor-plan', s => { const r = s.getBoundingClientRect(); return `${Math.round(r.width)}x${Math.round(r.height)}`; }).catch(() => 'no svg');
  say(null, 'plan svg size', box);
  await p.screenshot({ path: `${OUT}/h1-floor-plan.png`, fullPage: true });
  await csp(p, 'room-floor-plan');
  const f = await api(`/api/staff/floor?location_id=sushi-durres`, { token: await staffToken() });
  say(null, 'API floor', JSON.stringify(f.body).slice(0, 500));
} finally { await b.close(); finish(); }
