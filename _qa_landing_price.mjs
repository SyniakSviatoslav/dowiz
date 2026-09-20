// The landing's price, in the three languages it is written in.
import { chromium } from 'playwright';
const OUT = '/tmp/claude-0/-root/0ee0cdf2-e369-472c-a954-cbca99cc3bfd/scratchpad';
const b = await chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage'] });
const ctx = await b.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
const p = await ctx.newPage();
const issues = [];
p.on('pageerror', e => issues.push(`PAGEERROR ${e.message.slice(0, 140)}`));
p.on('console', m => { if (m.type() === 'error' && !/CF\$cv|Content Security|WebGL|GPU|favicon|404/i.test(m.text())) issues.push(`console: ${m.text().slice(0, 120)}`); });
await p.goto('http://127.0.0.1:8799/platform/index.html', { waitUntil: 'domcontentloaded', timeout: 60000 });
await p.waitForTimeout(2500);
for (const [lang, want] of [['uk', 'на місяць'], ['en', 'a month'], ['sq', 'në muaj']]) {
  const btn = await p.$(`#lang-${lang}`);
  if (!btn) { issues.push(`${lang}: no language button`); continue; }
  await btn.click();
  await p.waitForTimeout(700);
  const price = await p.evaluate(() => {
    const el = document.querySelector('.term.ours .price');
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return { text: el.innerText.replace(/\s+/g, ' ').trim(), w: r.width, h: r.height,
             vis: !!(r.width && r.height && getComputedStyle(el).visibility !== 'hidden') };
  });
  if (!price) { issues.push(`${lang}: no .price element`); continue; }
  if (!price.vis) issues.push(`${lang}: price not visible`);
  if (!price.text.includes('$50')) issues.push(`${lang}: price text "${price.text}" has no $50`);
  if (!price.text.includes(want)) issues.push(`${lang}: price text "${price.text}" missing "${want}"`);
  const body = await p.evaluate(() => document.body.innerText);
  if (!body.includes('$50')) issues.push(`${lang}: $50 absent from the page text`);
  const overflow = await p.evaluate(() => [document.documentElement.scrollWidth, innerWidth]);
  if (overflow[0] > overflow[1] + 1) issues.push(`${lang}: horizontal overflow ${overflow[0]}>${overflow[1]}`);
  console.log(`${lang}: ${price.text}`);
}
await p.evaluate(() => document.querySelector('#terms')?.scrollIntoView());
// The counters animate: the aggregator's counts UP to -35 and dowiz's counts
// DOWN from 35 to 0. Settled is what a visitor reads, so settled is what is
// asserted -- a screenshot at 1.5 s catches "-1%" and means nothing.
await p.waitForTimeout(3500);
const settled = await p.evaluate(() => ({
  agg: document.querySelector('.term:not(.ours) .n')?.textContent.trim(),
  ours: document.querySelector('.term.ours .n')?.textContent.trim(),
}));
console.log('counters settled:', JSON.stringify(settled));
if (settled.agg !== '−35%') issues.push(`aggregator counter settled at ${settled.agg}`);
if (settled.ours !== '0%') issues.push(`dowiz counter settled at ${settled.ours}`);
await p.screenshot({ path: `${OUT}/landing-price.png` }).catch(() => {});
console.log(issues.length ? `ISSUES: ${issues.join(' | ')}` : 'landing price: OK');
await b.close();
