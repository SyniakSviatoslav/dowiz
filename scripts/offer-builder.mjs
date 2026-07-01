#!/usr/bin/env node
// ══════════════════════════════════════════════════════════════════════════════════════════════════════
// offer-builder — CALLABLE LOOP: 1 Google-Maps location → a ready-to-review warm-outreach OFFER PACKET.
//
// It gathers PUBLIC BUSINESS data (name/address/phone/hours/rating/website/socials + a live fact), derives
// the owner DM channels (WhatsApp from the phone, Instagram/Facebook if public), reads a COMMUNICATION
// STRATEGY from observable signals + the Albanian warm-list playbook, and emits a packet with a slot-filled
// Albanian DM draft. The final creative/native pass is a human/model step (flagged in the packet).
//
// ETHICS (baked in, not optional): PUBLIC BUSINESS info only — no covert individual psychological profiling,
// no auto-send. The packet is PREPARED for a human to review, personalize the one live fact, and send. Warm
// list = prior consent; still one soft follow-up max, honour opt-outs. Mirrors the repo's outreach posture
// (preview-by-default, Art-14 notice as a separate human act).
//
// Usage:
//   node scripts/offer-builder.mjs "ArtePasta Durres" --slug artepasta [--base https://dowiz-staging.fly.dev]
//   (needs @playwright/test's chromium; run from repo root)
// ══════════════════════════════════════════════════════════════════════════════════════════════════════
import { chromium } from '@playwright/test';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const argv = process.argv.slice(2);
const query = argv.find((a) => !a.startsWith('--'));
const slug = (argv[argv.indexOf('--slug') + 1] && !argv[argv.indexOf('--slug') + 1].startsWith('--')) ? argv[argv.indexOf('--slug') + 1] : null;
const BASE = (argv.includes('--base') ? argv[argv.indexOf('--base') + 1] : null) || 'https://dowiz-staging.fly.dev';
if (!query) { console.error('usage: node scripts/offer-builder.mjs "<maps query>" --slug <demo-slug> [--base <url>]'); process.exit(1); }

const onlyDigits = (s) => (s || '').replace(/[^\d]/g, '');

// ── Stage 1 — scrape the Google-Maps listing (public business data) ─────────────────────────────────────
async function scrapeMaps(q) {
  const b = await chromium.launch({ args: ['--lang=en-US', '--disable-blink-features=AutomationControlled'] });
  const ctx = await b.newContext({ locale: 'en-US', userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36' });
  const p = await ctx.newPage();
  try {
    await p.goto(`https://www.google.com/maps/search/${encodeURIComponent(q)}?hl=en`, { waitUntil: 'domcontentloaded', timeout: 45000 });
    await p.waitForTimeout(3000);
    try { const c = p.locator('button:has-text("Accept all"), button:has-text("Reject all"), form[action*="consent"] button').first(); if (await c.isVisible({ timeout: 3000 })) { await c.click(); await p.waitForTimeout(3000); } } catch {}
    await p.locator('h1').first().waitFor({ timeout: 15000 }).catch(() => {});
    const name = await p.locator('h1').first().innerText().catch(() => q);
    const address = (await p.locator('button[data-item-id="address"]').first().getAttribute('aria-label').catch(() => null) || '').replace(/^Address:\s*/, '').trim() || null;
    const phone = (await p.locator('button[data-item-id^="phone:tel:"]').first().getAttribute('aria-label').catch(() => null) || '').replace(/^Phone:\s*/, '').trim() || null;
    const website = await p.locator('a[data-item-id="authority"]').first().getAttribute('href').catch(() => null);
    const panelText = await p.locator('[role="main"]').first().innerText().catch(() => '');
    const ratingM = panelText.match(/\b([1-5][.,]\d)\b\s*\n?\s*\(?([\d,.]+)?\)?/);
    const hoursM = panelText.match(/(Open|Closed)[^\n]*?(\d{1,2}\s?[ap]m[^\n]*)/i);
    const url = p.url();
    const llM = url.match(/@(-?\d+\.\d+),(-?\d+\.\d+)/);
    const headerPhoto = await p.evaluate(() => { const i = [...document.querySelectorAll('img')].map(x => x.src).find(s => /lh3\.googleusercontent|lh5\.googleusercontent/.test(s)); return i || null; });
    return { name, address, phone, website, rating: ratingM ? ratingM[1].replace(',', '.') : null, reviews: ratingM && ratingM[2] ? ratingM[2] : null, hours: hoursM ? hoursM[0].replace(/\s+/g, ' ').trim() : null, latlng: llM ? `${llM[1]},${llM[2]}` : null, headerPhoto, placeUrl: url.split('?')[0] };
  } finally { await b.close(); }
}

// ── Stage 2 — socials from the venue website (best-effort; public links only) ───────────────────────────
async function scrapeSocials(website) {
  if (!website) return {};
  try {
    const res = await fetch(website, { headers: { 'user-agent': 'Mozilla/5.0' } });
    const html = await res.text();
    const grab = (re) => { const m = html.match(re); return m ? m[0] : null; };
    return {
      instagram: grab(/https?:\/\/(www\.)?instagram\.com\/[A-Za-z0-9_.]+/i),
      facebook: grab(/https?:\/\/(www\.)?facebook\.com\/[A-Za-z0-9_.\-/]+/i),
      tiktok: grab(/https?:\/\/(www\.)?tiktok\.com\/@[A-Za-z0-9_.]+/i),
    };
  } catch { return {}; }
}

// ── Stage 3 — communication strategy from observable signals + the Albanian warm-list playbook ──────────
function buildStrategy(d) {
  const s = [];
  s.push('- **Frame:** warm (prior "yes" to testing). One message, demo first, call as a soft P.S. — no hook-question, you already have permission.');
  s.push('- **Language:** Albanian only. Get the name exactly right (ë/ç). English/Italian default reads as "outsider who didn’t bother".');
  s.push('- **Lead with a made thing, not a promise.** Value = the demo of THEIR menu. Never open with falas/kursim/mundësi (post-1997 scam trigger).');
  s.push('- **Never touch competence.** No "ti humbet / e bën gabim". Frame is always "ja çfarë bëra për ty", not "ja çfarë s’shkon te ti".');
  if (d.rating && Number(d.rating) >= 4.6) s.push(`- **Real fact to weave in:** their ${d.rating}★${d.reviews ? ` (${d.reviews})` : ''} rating — appreciate the food/craft, it signals you actually looked.`);
  s.push('- **Value in their terms:** "porositë vijnë drejt te ti, pa komision" (orders straight to you, no aggregator cut) — not a feature list.');
  s.push('- **Soft two-way door:** "nëse të intereson, ta tregoj për 15 min; nëse jo, thjesht më thuaj." Permission to say no lowers pressure, raises replies.');
  s.push('- **No religion. No pressure. One gentle follow-up after a few days, never three.**');
  return s.join('\n');
}

// ── Stage 4 — Albanian DM draft (SCAFFOLD — 5 blocks, slot-filled; native pass required before sending) ──
function draftAlbanian(d, reach, demoUrl) {
  const nameGuess = d.name || 'restoranti';
  return [
    `Përshëndetje! Ju shkruaj për ${nameGuess} — e kam pasur qejf vendin/menunë tuaj. 👋   // BLOCK 1: warm recognition — swap in a REAL detail (a dish, a visit, the ${d.rating || ''}★)`,
    ``,
    `Bëra diçka për ju, pa asnjë detyrim.   // BLOCK 2: bridge — "I made something", not "I want something"`,
    ``,
    `E ktheva menunë tuaj në një dyqan online që mund ta provoni vetë këtu: ${demoUrl}   // BLOCK 3: the demo — THEIR menu, try it yourself`,
    ``,
    `Kështu porositë vijnë drejt te ju — klientët dhe të dhënat tuaja, pa komisionin e Glovo-s.   // BLOCK 4: value in their terms`,
    ``,
    `Nëse ju pëlqen, ta tregoj për 15 minuta si niset; nëse jo, thjesht më thoni — pa problem. 🙏   // BLOCK 5: soft, two-way CTA`,
  ].join('\n');
}

function packet(d, reach, socials, demoUrl, demoLive) {
  const wa = reach.whatsapp;
  return `# Offer packet · ${d.name}

_Generated by offer-builder from a single Google-Maps location. PUBLIC business data only. Review, drop in
one real personal detail, get the Albanian verified by a native, then YOU send. Do not auto-send._

## 1 · Target
| | |
|---|---|
| Venue | ${d.name} |
| Address | ${d.address || '—'} |
| Category/fact | ${d.rating ? `${d.rating}★${d.reviews ? ` (${d.reviews})` : ''}` : '—'} |
| Hours (today) | ${d.hours || '—'} |
| Google Maps | ${d.placeUrl || '—'} |
| Website | ${d.website || '—'} |

## 2 · Demo (the personalization)
- **Link:** ${demoUrl}  ${demoLive ? '✅ live' : '⚠️ NOT built yet — run demo-builder first'}
- Test it on a PHONE, no login, before sending. A broken first click = closed contact forever on this market.

## 3 · Owner DM channels (public)
- **WhatsApp:** ${wa || '— (no phone found)'}${d.phone ? `  (from ${d.phone})` : ''}
- **Instagram:** ${socials.instagram || '⚠️ not public — search the handle & verify before using'}
- **Facebook:** ${socials.facebook || '—'}
- **Phone:** ${d.phone || '—'}
- _Owner personal name: not reliably public from OSINT — keep it to the venue name unless you truly know it._

## 4 · Communication strategy (how to talk to THIS venue)
${buildStrategy(d)}

## 5 · DM draft — Albanian (scaffold, 5 blocks) · ⚠️ native review required
\`\`\`
${draftAlbanian(d, reach, demoUrl)}
\`\`\`
**English gloss:** Hi! I'm writing about <venue> — I really liked your place/menu. · I made something for you,
no strings. · I turned your menu into an online shop you can try yourself: <demo>. · This way orders come
straight to you — your customers and data, no Glovo commission. · If you like it, I'll show you in 15 min how
to launch; if not, just tell me — no problem.

## 6 · Send checklist
- [ ] Demo opens & works on a phone (no login)
- [ ] One REAL detail swapped into block 1 (a dish, a visit) — never identical copy across the 50
- [ ] Name spelled with correct ë/ç
- [ ] Albanian verified by a native speaker (tone: warm/insider, not outsider)
- [ ] Sent from a channel they'll recognise · one soft follow-up max after a few days
`;
}

// ── run ─────────────────────────────────────────────────────────────────────────────────────────────────
(async () => {
  console.error(`[offer-builder] scraping Google Maps for: ${query}`);
  const d = await scrapeMaps(query);
  const socials = await scrapeSocials(d.website);
  const reach = { whatsapp: d.phone ? `https://wa.me/${onlyDigits(d.phone)}` : null, ...socials };
  const useSlug = slug || (d.name || 'venue').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
  const demoUrl = `${BASE}/s/${useSlug}`;
  let demoLive = false;
  try { const r = await fetch(`${BASE}/public/locations/${useSlug}/menu`); demoLive = r.ok; } catch {}
  const md = packet(d, reach, socials, demoUrl, demoLive);
  const dir = resolve('loops/offers'); mkdirSync(dir, { recursive: true });
  const out = resolve(dir, `${useSlug}-offer.md`);
  writeFileSync(out, md);
  console.error(`[offer-builder] wrote ${out} (demo ${demoLive ? 'LIVE' : 'not built'})`);
  console.log(md);
})().catch((e) => { console.error('[offer-builder] FATAL:', e.message); process.exit(1); });
