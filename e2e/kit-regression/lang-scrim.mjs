// After a customer changes the language, can they still tap anything?
import { chromium } from 'playwright';
const b = await chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage'] });
const ctx = await b.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
const p = await ctx.newPage();
const log = [];
await p.goto('https://sushi-durres.dowiz.org/', { waitUntil: 'domcontentloaded', timeout: 90000 });
await p.waitForSelector('.card', { timeout: 60000 });
await p.waitForTimeout(1200);
const state = () => p.evaluate(() => ({
  sheet: document.getElementById('sheet')?.dataset.name || '',
  scrim: document.getElementById('scrim')?.className || '',
  scrimPE: getComputedStyle(document.getElementById('scrim')).pointerEvents,
  bodyOpen: document.body.className,
}));
log.push(['before', await state()]);
await p.click('#langBtn'); await p.waitForTimeout(700);
log.push(['sheet open', await state()]);
const en = await p.$('[data-l="en"]');
if (!en) { console.log('no EN chip'); process.exit(1); }
await en.click();
await p.waitForTimeout(1500);
log.push(['after EN', await state()]);
// The real question: can a customer now tap a dish?
let clicked = 'no';
try { await p.click('.card', { timeout: 6000 }); clicked = 'yes'; } catch (e) { clicked = String(e.message).split('\n')[0].slice(0, 90); }
log.push(['card click', clicked]);
log.push(['after click', await state()]);
// What a customer actually does: tap outside the sheet.
await p.evaluate(() => document.getElementById('scrim').click());
await p.waitForTimeout(800);
log.push(['after scrim tap', await state()]);
let clicked2 = 'no';
try { await p.click('.card', { timeout: 8000 }); clicked2 = 'yes'; } catch (e) { clicked2 = String(e.message).split('\n')[0].slice(0, 90); }
log.push(['card click after dismiss', clicked2]);
log.push(['sheet now', await state()]);
// And the language really changed?
log.push(['html lang', await p.evaluate(() => document.documentElement.lang)]);
for (const [k, v] of log) console.log(k, JSON.stringify(v));
await b.close();
