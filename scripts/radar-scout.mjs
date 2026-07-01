#!/usr/bin/env node
// ══════════════════════════════════════════════════════════════════════════════════════════════════════
// radar-scout — sweep a city's food vendors from Google Maps, SCORE by public BUSINESS signals, rank by
// "likelihood of a successful warm DM", and emit a shortlist to feed demo-builder + offer-builder.
//
// Signals (PUBLIC BUSINESS only — no personal profiling): rating (product pride), review volume (real &
// active vs chain-scale), category fit (food vendor). Optional enrich: on Wolt/Glovo = already paying ~30%
// commission = strongest receptivity. Ethics identical to offer-builder: business data, human decides who to
// contact, nothing auto-sent.
//
// Usage:
//   node scripts/radar-scout.mjs --city "Durrës" [--queries "restaurants,cafe,fast food,pizza"] [--scroll 30] [--top 25]
// ══════════════════════════════════════════════════════════════════════════════════════════════════════
import { chromium } from '@playwright/test';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i >= 0 && process.argv[i + 1] && !process.argv[i + 1].startsWith('--') ? process.argv[i + 1] : d; };
const CITY = arg('city', 'Durrës');
const QUERIES = arg('queries', 'restaurants,cafe,fast food,pizza,traditional food').split(',').map((s) => s.trim());
const SCROLL = Number(arg('scroll', '28'));
const TOP = Number(arg('top', '25'));

// ── scrape one Maps search's result feed (list cards carry name/rating/reviews/category — no per-place nav) ─
async function scrapeFeed(page, query) {
  await page.goto(`https://www.google.com/maps/search/${encodeURIComponent(query + ' ' + CITY)}?hl=en`, { waitUntil: 'domcontentloaded', timeout: 45000 });
  await page.waitForTimeout(3000);
  try { const c = page.locator('button:has-text("Accept all"), button:has-text("Reject all"), form[action*="consent"] button').first(); if (await c.isVisible({ timeout: 3000 })) { await c.click(); await page.waitForTimeout(3000); } } catch {}
  const feed = page.locator('div[role="feed"]').first();
  await feed.waitFor({ timeout: 12000 }).catch(() => {});
  // scroll the virtualized feed to load more cards
  for (let i = 0; i < SCROLL; i++) {
    await feed.evaluate((el) => el.scrollBy(0, 1600)).catch(() => {});
    await page.waitForTimeout(650);
  }
  return await page.evaluate(() => {
    const out = [];
    document.querySelectorAll('div[role="feed"] > div').forEach((card) => {
      const a = card.querySelector('a.hfpxzc');
      const name = card.querySelector('.qBF1Pd')?.textContent?.trim() || a?.getAttribute('aria-label')?.trim();
      if (!name) return;
      const rating = parseFloat((card.querySelector('.MW4etd')?.textContent || '').replace(',', '.')) || null;
      const reviews = parseInt((card.querySelector('.UY7F9')?.textContent || '').replace(/[^\d]/g, ''), 10) || null;
      const meta = [...card.querySelectorAll('.W4Efsd')].map((e) => e.textContent).join(' ');
      const href = a?.getAttribute('href') || null;
      const ll = href && href.match(/!3d(-?\d+\.\d+)!4d(-?\d+\.\d+)/);
      out.push({ name, rating, reviews, meta: meta.replace(/\s+/g, ' ').trim().slice(0, 120), href, latlng: ll ? `${ll[1]},${ll[2]}` : null });
    });
    return out;
  });
}

// ── score: likelihood of a successful warm DM for a 0%-commission storefront (CONTINUOUS, so it ranks) ────
const clamp = (n, lo, hi) => Math.max(lo, Math.min(hi, n));
function score(v) {
  const why = [];
  const rating = v.rating ?? 0, reviews = v.reviews ?? 0;
  // Credibility of the rating grows with volume (a 5.0 on 5 reviews is unproven). log-scaled, full by ~400.
  const revConf = clamp(Math.log10(reviews + 1) / Math.log10(400), 0, 1);
  // Quality: normalise rating 3.5→0 .. 4.9→1, weighted by credibility. (55)
  const ratingNorm = clamp((rating - 3.5) / (4.9 - 3.5), 0, 1);
  const qualityScore = 55 * ratingNorm * (0.35 + 0.65 * revConf);
  // Reachable & real: enough of a footprint to actually respond. (25)
  const activityScore = 25 * revConf;
  // Independent-scale: chains (very high volume) are less likely to switch — taper above ~1500.
  const chainPenalty = reviews > 1500 ? clamp((reviews - 1500) / 4000, 0, 0.45) : 0;
  // Fit: a food vendor we can serve. (20 / partial)
  const hay = (v.meta + ' ' + v.name).toLowerCase();
  const isFood = /restaurant|cafe|coffee|pizz|bar|fast food|bakery|bistro|trattoria|kebab|burger|food|grill|pasticeri|resto/i.test(hay);
  const fitScore = isFood ? 20 : 8;
  // DELIVERY-INTENT (the decisive weight for a delivery-storefront pitch): food that travels + venues that
  // already do delivery rank up; destination dine-in (seaside/fine-dining/steakhouse) ranks down.
  let intent = 0.7, intentTag = 'mixed (could deliver)';
  if (/pizz|piceri|pasta|pastarell|burger|kebab|fast food|sushi|fried|wok|sandwich|bakery|pasticeri|gelato|dessert|byrek|street|fixhtim|snack/.test(hay)) { intent = 1.0; intentTag = 'delivery-native'; }
  else if (/seaside|harbor|harbour|marine|\bmare\b|fish|seafood|peshk|aragost|lobster|steakhouse|fine|lounge|rooftop|beach|resort/.test(hay)) { intent = 0.45; intentTag = 'destination dine-in (weak delivery fit)'; }
  const intentMult = 0.5 + 0.5 * intent; // 0.45→0.725, 0.7→0.85, 1.0→1.0
  const raw = (qualityScore + activityScore + fitScore) * (1 - chainPenalty) * intentMult;
  if (rating) why.push(`${rating}★${revConf < 0.5 ? ' (few reviews → unproven)' : ''}`);
  if (reviews) why.push(`${reviews} rev${reviews > 1500 ? ' (chain-scale ↓)' : reviews < 40 ? ' (new ↑receptive)' : ''}`);
  why.push(`delivery: ${intentTag}`);
  return { score: Math.round(raw * 10) / 10, deliveryIntent: intent, why };
}

(async () => {
  const b = await chromium.launch({ args: ['--lang=en-US', '--disable-blink-features=AutomationControlled'] });
  const ctx = await b.newContext({ locale: 'en-US', viewport: { width: 1280, height: 1400 }, userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36' });
  const page = await ctx.newPage();
  const byName = new Map();
  try {
    for (const q of QUERIES) {
      console.error(`[radar] sweeping: ${q} ${CITY}`);
      const rows = await scrapeFeed(page, q).catch((e) => { console.error(`  ${q} failed: ${e.message}`); return []; });
      for (const r of rows) { const k = r.name.toLowerCase(); if (!byName.has(k) || (r.reviews || 0) > (byName.get(k).reviews || 0)) byName.set(k, r); }
      console.error(`  +${rows.length} (unique so far: ${byName.size})`);
    }
  } finally { await b.close(); }

  const ranked = [...byName.values()].map((v) => ({ ...v, ...score(v) })).sort((a, b2) => b2.score - a.score);
  const top = ranked.slice(0, TOP);

  const md = `# Radar · food vendors in ${CITY} — ranked by warm-DM likelihood

_Swept from Google Maps (${QUERIES.join(' · ')}). PUBLIC business signals only — no personal profiling.
Score = quality (product pride) + activity/tenure + food-vendor fit. Next step per row: run demo-builder to
build /s/<slug>, then offer-builder for the packet (it confirms phone/WhatsApp + aggregator cost-pressure)._

**Swept:** ${byName.size} unique vendors · **shortlist:** top ${top.length}

| # | Vendor | Score | Rating | Reviews | Why it ranks | Maps |
|---|--------|-------|--------|---------|--------------|------|
${top.map((v, i) => { const url = v.href ? (v.href.startsWith('http') ? v.href : `https://www.google.com${v.href.startsWith('/') ? '' : '/'}${v.href}`) : null; return `| ${i + 1} | ${v.name} | ${v.score} | ${v.rating ?? '—'} | ${v.reviews ?? '—'} | ${v.why.join('; ')} | ${url ? `[map](${url})` : '—'} |`; }).join('\n')}

## How to work the list
- **Top of list first** — highest product-pride + active + independent-scale = most likely to value a
  0%-commission own storefront.
- **Enrich before DMing:** run \`offer-builder\` per shortlisted vendor — it confirms the DM channel
  (WhatsApp/Instagram) and the **aggregator cost-pressure** signal (already on Wolt/Glovo = paying ~30% =
  hottest). That signal isn't in the Maps list, so it's not in this score yet — it's the tie-breaker.
- **Warm list only:** contact those who already said "yes" to testing. Cold ones → question-first (SPER), not
  a link from a stranger.
`;
  const dir = resolve('loops/offers'); mkdirSync(dir, { recursive: true });
  const out = resolve(dir, `radar-${CITY.toLowerCase().replace(/[^a-z0-9]+/g, '-')}.md`);
  writeFileSync(out, md);
  writeFileSync(resolve(dir, `radar-${CITY.toLowerCase().replace(/[^a-z0-9]+/g, '-')}.json`), JSON.stringify(ranked, null, 1));
  console.error(`[radar] wrote ${out} (+ .json raw) — ${byName.size} vendors, top ${top.length}`);
  console.log(md);
})().catch((e) => { console.error('[radar] FATAL:', e.message); process.exit(1); });
