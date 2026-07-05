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
import { mkdirSync, writeFileSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { checkWolt } from '../tools/demo-builder/aggregator-check.mjs';

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

// ── Stage 2b — VENUE business signal (public data only; NO personal profiling of the owner) ─────────────
// Derives targeting signals from the BUSINESS, not the person: tenure, momentum, quality, price, and the
// sharpest one — is the venue already on a commission aggregator (→ actively paying ~30% → receptive to a
// 0%-commission pitch). This is what "new/old? looking to cut costs?" actually resolves to, done lawfully.
async function buildVenueSignal(d) {
  const sig = [];
  const rc = d.reviews ? Number(String(d.reviews).replace(/[^\d]/g, '')) : null;
  if (rc != null) {
    const tenure = rc < 30 ? 'NEW / early (few reviews) — likely still finding footing, open to tools that bring orders'
      : rc < 200 ? 'ESTABLISHING (moderate review base) — has traction, weighing how to grow without more platform fees'
      : 'ESTABLISHED (large review base) — steady demand; the pitch is keeping margin, not chasing volume';
    sig.push(`- **Tenure/momentum:** ~${rc} reviews → ${tenure}.`);
  }
  if (d.rating) sig.push(`- **Quality:** ${d.rating}★ — lead by appreciating the food/craft (never critique). High rating = they care about the product; a polished own-storefront matches that pride.`);
  // Aggregator presence = the cost-pressure signal (robust: Wolt city-list + token-set match).
  const [lat, lon] = (d.latlng || ',').split(',');
  const w = lat && lon ? await checkWolt(d.name, lat, lon) : { on: null, match: null, listSize: 0 };
  if (w.on === true) sig.push(`- **💡 COST-PRESSURE (HOT):** on Wolt as "${w.match}" → already paying ~30% commission → lead with "porositë drejt te ju, pa komision". Angle: keep the money you already lose.`);
  else if (w.on === false) sig.push(`- **Cost-pressure (angle B):** NOT on Wolt (${w.listSize} venues checked) → likely self-delivers/phone-orders or no online ordering. Angle: get online ordering WITHOUT giving 30% to anyone — a fresh channel, not a switch. (Check Glovo too.)`);
  else sig.push('- **Cost-pressure:** aggregator check inconclusive (no lat/lon or Wolt list unavailable) — verify Wolt/Glovo manually.');
  sig.push('- **Deeper (optional next pass):** read the recent reviews for delivery mentions/complaints and read Maps "popular times" for peak-hours load — both public, both sharpen timing. NOT the owner\'s private life.');
  return sig.join('\n');
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
    `Tungjatjeta! 👋 Ju shkruaj për ${nameGuess}${d.rating ? ` — s'çudi që keni ${d.rating} yje` : ''}.   // BLOCK 1: WHY OPEN — it's about THEIR place, swap in a real detail (a dish/visit)`,
    ``,
    `E mora menunë tuaj dhe e ktheva në një dyqan online, që thjesht ta provoni vetë:   // BLOCK 2: THE HOOK — a thing already MADE for them`,
    `${demoUrl}   // BLOCK 3: the demo link`,
    ``,
    `Kështu porositë — dhe ~30% që u shkojnë aplikacioneve — vijnë drejt te ju; klientët mbeten tuajt.   // BLOCK 4: WHAT'S IN IT FOR ME`,
    ``,
    `Nëse ju pëlqen, ta tregoj për 15 min si niset. Nëse jo, thjesht më thoni — pa problem. 🙏   // BLOCK 5: WHY RESPOND — done already, easy yes/no`,
  ].join('\n');
}

// ── Stage 5b — LLM compose: auto-draft the per-venue Albanian message from the packet (optional) ────────
// All the cultural rules are baked into the prompt. Returns null on any failure → the scaffold stands.
// Needs OPENROUTER_API_KEY in env (run `node --env-file=.env scripts/offer-builder.mjs …`). Still native-review.
function loadOpenrouterKey() {
  if (process.env.OPENROUTER_API_KEY) return process.env.OPENROUTER_API_KEY;
  try { const t = readFileSync(resolve('.env'), 'utf8'); const m = t.match(/^OPENROUTER_API_KEY=(.+)$/m); return m ? m[1].trim() : null; } catch { return null; }
}
async function composeDraft(d, venueSignal, demoUrl) {
  const key = loadOpenrouterKey();
  if (!key) return null;
  // Paid primary (needs OpenRouter credits) → free fallbacks (work when not rate-limited). Override with
  // OPENROUTER_MODEL. Current 2026 slugs; refresh from https://openrouter.ai/api/v1/models if they 404.
  const models = (process.env.OPENROUTER_MODEL ? [process.env.OPENROUTER_MODEL]
    : ['google/gemini-3.5-flash', 'meta-llama/llama-3.3-70b-instruct:free', 'qwen/qwen3-next-80b-a3b-instruct:free']);
  const prompt = `You are a NATIVE Albanian (Tosk, Albania) copywriter helping a founder write ONE warm outreach DM to a
restaurant owner who ALREADY agreed once to test a product (warm lead, prior consent).

WRITE IT FROM THE OWNER'S POV. Before writing, satisfy the 4 questions a busy owner asks in 2 seconds:
  • Why would I even OPEN this? → line 1 must prove it's about MY place specifically, from someone real —
    not a blast. Name the venue + one true detail.
  • What grabs my attention? → a concrete thing ALREADY MADE for me (my own menu, live), not "an offer".
  • What's in it for ME? → the money I lose to Glovo/Wolt stays mine; my customers stay mine. In my terms.
  • Why RESPOND? → it's already done, one tap to see it, dead-easy yes/no, zero pressure.

Then write ONE message in natural, warm, HUMAN Albanian — SHORT, CLEAR, HONEST. Max 5 short lines, one role each:
1) recognition of THEIR place + one real detail (rating if given) — proves it's personal;
2) the hook = a made thing: "I turned YOUR menu into a working online shop, so you could just try it";
3) the demo link;
4) what's in it for them: orders (and the ~30% you pay the apps) come straight to you, your customers stay yours;
5) soft two-way CTA: like it → I'll show you in 15 min how to start; if not → just say, no problem.

HARD RULES: never "falas"/"kursim"/"mundësi" (post-1997 scam triggers); never question their competence or imply
anything's wrong on their side; no religion; no pressure; no marketing-speak or adjectives-as-value — let the
made thing carry it; be honest (it's a demo, not live yet). Output ONLY the Albanian message (1-2 tasteful emojis ok).

VENUE: ${d.name} · ${d.address || ''} · rating ${d.rating || '—'}${d.reviews ? ` (${d.reviews})` : ''}
DEMO LINK: ${demoUrl}
SIGNAL (for your framing, do NOT quote verbatim): ${venueSignal.replace(/\n/g, ' ').slice(0, 400)}`;
  for (const model of models) {
    for (let attempt = 1; attempt <= 2; attempt++) {
      try {
        const res = await fetch('https://openrouter.ai/api/v1/chat/completions', {
          method: 'POST',
          headers: { Authorization: `Bearer ${key}`, 'Content-Type': 'application/json' },
          body: JSON.stringify({ model, max_tokens: 600, temperature: 0.7, messages: [{ role: 'user', content: prompt }] }),
        });
        if (res.status === 429 && attempt === 1) { console.error(`[offer-builder] compose ${model} → 429, retrying`); await new Promise((r) => setTimeout(r, 4000)); continue; }
        if (!res.ok) { console.error(`[offer-builder] compose ${model} → ${res.status}, next`); break; }
        const j = await res.json();
        const text = j?.choices?.[0]?.message?.content?.trim();
        if (text) return { text, model };
        break;
      } catch (e) { console.error(`[offer-builder] compose ${model} err: ${e.message}`); break; }
    }
  }
  return null;
}

function packet(d, reach, socials, demoUrl, demoLive, venueSignal, composed) {
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

## 4 · Venue signal (public BUSINESS data — targeting, not personal profiling)
${venueSignal}

## 5 · Communication strategy (how to talk to THIS venue)
${buildStrategy(d)}

## 6 · DM draft — Albanian · ⚠️ native review required before sending
${composed ? `_Auto-composed (${composed.model}). A draft, not final — a native speaker still verifies tone (insider, not outsider)._
\`\`\`
${composed.text}
\`\`\`

<details><summary>Fallback scaffold (5 blocks, if you prefer to write it yourself)</summary>

\`\`\`
${draftAlbanian(d, reach, demoUrl)}
\`\`\`
</details>` : `_(No auto-draft — OPENROUTER_API_KEY missing, out of credits, or rate-limited. Scaffold below; add
credits or set OPENROUTER_MODEL to a model your key can use, then re-run.)_
\`\`\`
${draftAlbanian(d, reach, demoUrl)}
\`\`\``}
**English gloss:** Hi! I'm writing about <venue> — I really liked your place/menu. · I made something for you,
no strings. · I turned your menu into an online shop you can try yourself: <demo>. · This way orders come
straight to you — your customers and data, no Glovo commission. · If you like it, I'll show you in 15 min how
to launch; if not, just tell me — no problem.

## 7 · Send checklist
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
  const venueSignal = await buildVenueSignal(d);
  const composed = argv.includes('--no-compose') ? null : await composeDraft(d, venueSignal, demoUrl);
  console.error(`[offer-builder] compose: ${composed ? 'ok (' + composed.model + ')' : 'skipped/failed → scaffold'}`);
  const md = packet(d, reach, socials, demoUrl, demoLive, venueSignal, composed);
  const dir = resolve('loops/offers'); mkdirSync(dir, { recursive: true });
  const out = resolve(dir, `${useSlug}-offer.md`);
  writeFileSync(out, md);
  console.error(`[offer-builder] wrote ${out} (demo ${demoLive ? 'LIVE' : 'not built'})`);
  console.log(md);
})().catch((e) => { console.error('[offer-builder] FATAL:', e.message); process.exit(1); });
