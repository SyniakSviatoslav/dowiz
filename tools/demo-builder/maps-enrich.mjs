// maps-enrich — scrape REAL public venue data from Google Maps (no API key) so a demo storefront looks like
// the venue's real identity, not a cuisine-guess. This is the enrichment the demo-builder loop was missing:
// logo/hero photo, weekly hours, address/lat/lng, Google rating, and a brand colour pulled from the sign.
//
// Public business-listing data only (address/hours/rating/photos a venue publishes on its Maps profile).
// Re-hosting a Maps photo is an operator-accepted ToS risk (see memory storefront-venue-data-maps-scrape).
//
// Recipe (proven headless, unauthenticated): search-url keeps the place panel; consent dismissed; extract from
// the panel DOM + the settled URL. Full weekly hours are sign-in-gated → we read today's row and, when only
// today is available, synthesize a sane weekly default around it (owner edits on claim). The hero/sign photo
// is the first lh3 googleusercontent img, size-bumped. Brand colour = dominant saturated colour of that photo.
//
// Usage (standalone probe): node tools/demo-builder/maps-enrich.mjs "Apollonia" durres
// Programmatic: import { enrichFromMaps } from './maps-enrich.mjs'; const packet = await enrichFromMaps({name, city})

import { chromium } from '@playwright/test';
import { createRequire } from 'node:module';

const DAYS = ['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday'];

// sharp isn't a direct dep of this tool; resolve it from the app packages that DO depend on it (pnpm-strict).
function loadSharp() {
  for (const base of ['/root/dowiz/apps/api/', '/root/dowiz/packages/ui/', '/root/dowiz/']) {
    try { return createRequire(base + 'noop.js')('sharp'); } catch { /* try next */ }
  }
  return null;
}

/** Best-effort dominant, reasonably-saturated colour of a photo buffer → "#rrggbb". Uses sharp when available
 *  (bundled app node_modules); returns null if sharp can't be loaded from a standalone process. */
export async function dominantBrandColor(buf) {
  const sharp = loadSharp();
  if (!sharp) return null;
  // Downscale to a small thumb, walk pixels, bucket by hue, pick the most-common vivid bucket.
  const w = 48, h = 48;
  const { data } = await sharp(buf).resize(w, h, { fit: 'cover' }).removeAlpha().raw().toBuffer({ resolveWithObject: true });
  const buckets = new Map(); // hueBucket -> {r,g,b,n} accumulator of vivid pixels
  for (let i = 0; i < data.length; i += 3) {
    const r = data[i], g = data[i + 1], b = data[i + 2];
    const max = Math.max(r, g, b), min = Math.min(r, g, b);
    const sat = max === 0 ? 0 : (max - min) / max;
    const lum = (max + min) / 2;
    if (sat < 0.35 || lum < 40 || lum > 225) continue; // skip greys, near-black, near-white
    // hue bucket (0..11)
    let hue = 0; const d = max - min;
    if (d !== 0) {
      if (max === r) hue = ((g - b) / d) % 6;
      else if (max === g) hue = (b - r) / d + 2;
      else hue = (r - g) / d + 4;
      hue = Math.round(hue * 60); if (hue < 0) hue += 360;
    }
    const key = Math.floor(hue / 30);
    const acc = buckets.get(key) || { r: 0, g: 0, b: 0, n: 0 };
    acc.r += r; acc.g += g; acc.b += b; acc.n++;
    buckets.set(key, acc);
  }
  let best = null;
  for (const acc of buckets.values()) if (!best || acc.n > best.n) best = acc;
  if (!best || best.n < 4) return null; // not enough vivid signal → keep cuisine palette
  const r = best.r / best.n, g = best.g / best.n, b = best.b / best.n;
  // Final vividness gate: a muddy/greyish average is a worse "brand" than the tasteful cuisine seed, so only
  // override when the averaged colour is genuinely saturated (a real sign/awning colour, not decor beige).
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  const sat = max === 0 ? 0 : (max - min) / max;
  if (sat < 0.45) return null;
  const toHex = (n) => Math.min(255, Math.max(0, Math.round(n))).toString(16).padStart(2, '0');
  return '#' + toHex(r) + toHex(g) + toHex(b);
}

/** Synthesize a weekly hours_json from a single "today" open/close (the unauthenticated-view limit). */
function weeklyFromToday(open, close) {
  if (!open || !close) return null;
  const row = { isOpen: true, open, close };
  return Object.fromEntries(DAYS.map((d) => [d, { ...row }]));
}

/** Scrape one venue. Returns a packet (never throws — returns { ok:false, reason } on failure). */
export async function enrichFromMaps({ name, city = 'durres', headless = true, debugShot = null } = {}) {
  const out = { name, city, ok: false };
  let browser;
  try {
    browser = await chromium.launch({ headless, args: ['--lang=en-US', '--disable-blink-features=AutomationControlled'] });
    const ctx = await browser.newContext({ locale: 'en-US', viewport: { width: 1280, height: 1000 }, userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126 Safari/537.36' });
    const page = await ctx.newPage();
    // Fast-fail every implicit locator wait: a zero-match getAttribute otherwise blocks the DEFAULT 30s each,
    // and several of those in a row blow past the caller's per-venue budget (the batch-timeout root cause).
    page.setDefaultTimeout(4000);
    const q = encodeURIComponent(`${name} ${city}`);
    await page.goto(`https://www.google.com/maps/search/${q}?hl=en`, { waitUntil: 'domcontentloaded', timeout: 45000 });

    // consent (may be a top-level page OR a consent.google.com interstitial)
    for (const label of [/^Accept all$/i, /^Reject all$/i, /^Accept$/i, /^I agree$/i]) {
      const btn = page.getByRole('button', { name: label });
      if (await btn.count().catch(() => 0)) { await btn.first().click().catch(() => {}); break; }
    }
    // wait for the place panel to render the venue name
    await page.getByRole('main').getByRole('heading', { level: 1 }).first().waitFor({ state: 'visible', timeout: 15000 }).catch(() => {});
    await page.waitForTimeout(1500);

    // If the search returned a LIST (not a single place), click the first result matching the name.
    const h1 = (await page.locator('h1').first().innerText().catch(() => '')) || '';
    if (!h1 || /results/i.test(h1)) {
      const first = page.locator('a[href*="/place/"]').first();
      if (await first.count().catch(() => 0)) { await first.click().catch(() => {}); await page.waitForTimeout(3000); }
    }
    out.matchedName = (await page.locator('h1').first().innerText().catch(() => '')) || null;

    // lat/lng + canonical place url — from the place link's !3d<lat>!4d<lng> (cheap: one attr read, no full
    // page.content() scan which is very slow on the Maps DOM). Fall back to the settled URL's @lat,lng.
    const placeHref = await page.locator('a[href*="/maps/place/"]').first().getAttribute('href').catch(() => null);
    const src = placeHref || page.url();
    let m = src.match(/!3d(-?\d+\.\d+)!4d(-?\d+\.\d+)/);
    if (m) { out.lat = Number(m[1]); out.lng = Number(m[2]); }
    else { const u = src.match(/@(-?\d+\.\d+),(-?\d+\.\d+)/); if (u) { out.lat = Number(u[1]); out.lng = Number(u[2]); } }
    out.googleMapsUrl = (placeHref ? (placeHref.startsWith('http') ? placeHref : 'https://www.google.com' + placeHref) : page.url()).split('?')[0];

    // address + phone from the info buttons
    out.address = await page.locator('button[data-item-id="address"]').getAttribute('aria-label').catch(() => null);
    if (out.address) out.address = out.address.replace(/^Address:\s*/i, '').trim();
    const phoneBtn = await page.locator('button[data-item-id^="phone:tel:"]').getAttribute('data-item-id').catch(() => null);
    if (phoneBtn) out.phone = phoneBtn.replace(/^phone:tel:/, '');

    // rating + review count (F7nice block; role=img aria-label "4.6 stars" + "(361)")
    const ratingTxt = await page.locator('div.F7nice').first().innerText().catch(() => '');
    const rm = ratingTxt.match(/(\d[.,]\d)/);
    const cm = ratingTxt.match(/\(([\d,\.]+)\)/) || ratingTxt.match(/([\d,\.]+)\s*review/i);
    if (rm) out.googleRating = Number(rm[1].replace(',', '.'));
    if (cm) out.googleReviewCount = Number(cm[1].replace(/[.,]/g, ''));

    // Hours. Best source = the full weekly table behind the hours button (often reachable unauthenticated).
    // Try to expand it and read per-day rows; else fall back to today's "Closes HH:MM" + a sane default open.
    const to24 = (s) => { if (!s) return null; const t = String(s).trim().toUpperCase().match(/(\d{1,2})(?::(\d{2}))?\s*([AP]M)/); if (!t) return null; let hr = Number(t[1]) % 12; if (t[3] === 'PM') hr += 12; return String(hr).padStart(2, '0') + ':' + (t[2] || '00'); };
    try {
      const hoursBtn = page.locator('button[data-item-id="oh"], [jsaction*="openhours"], button:has-text("See more hours")').first();
      if (await hoursBtn.count()) { await hoursBtn.click({ timeout: 4000 }).catch(() => {}); await page.waitForTimeout(1200); }
      // the weekly table: rows are <tr> with a day cell + an hours cell; scrape all
      const rows = await page.locator('table tr').evaluateAll((trs) => trs.map((tr) => tr.innerText.replace(/\s+/g, ' ').trim()).filter(Boolean)).catch(() => []);
      const week = {};
      for (const line of rows) {
        const dm = line.match(/^(Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday)/i);
        if (!dm) continue;
        const day = dm[1].toLowerCase();
        if (/closed/i.test(line)) { week[day] = { isOpen: false, open: '00:00', close: '00:00' }; continue; }
        const rng = line.match(/(\d{1,2}(?::\d{2})?\s*[AP]M)\s*(?:to|[–-])\s*(\d{1,2}(?::\d{2})?\s*[AP]M)/i);
        if (rng) week[day] = { isOpen: true, open: to24(rng[1]), close: to24(rng[2]) };
      }
      if (Object.keys(week).length >= 5) out.weekScraped = week;
    } catch { /* hours best-effort */ }
    // today's line as a fallback signal
    const hoursLabel = (await page.locator('button[data-item-id="oh"], [jsaction*="openhours"]').first().getAttribute('aria-label').catch(() => '')) || '';
    const closeM = hoursLabel.match(/Closes\s+(\d{1,2}(?::\d{2})?\s*[AP]M)/i);
    const openM = hoursLabel.match(/Opens\s+(\d{1,2}(?::\d{2})?\s*[AP]M)/i);
    if (closeM || openM) out.hoursToday = { open: to24(openM?.[1]) || '10:00', close: to24(closeM?.[1]) || '23:00' };

    // hero/sign photo — the header hero button's background image, else first lh3 img. Bump size.
    let heroUrl = null;
    const heroBtnImg = await page.locator('button[jsaction*="heroHeaderImage"] img, button[aria-label*="Photo"] img, div[role="img"][style*="googleusercontent"]').first();
    heroUrl = await heroBtnImg.getAttribute('src').catch(() => null);
    if (!heroUrl) {
      const style = await page.locator('div[role="img"][style*="googleusercontent"]').first().getAttribute('style').catch(() => '');
      const um = style && style.match(/url\("?(https:\/\/[^")]+googleusercontent[^")]+)"?\)/);
      if (um) heroUrl = um[1];
    }
    if (!heroUrl) {
      const anyImg = await page.locator('img[src*="googleusercontent"], img[src*="lh3"]').first().getAttribute('src').catch(() => null);
      heroUrl = anyImg;
    }
    if (heroUrl) out.heroUrl = heroUrl.replace(/=w\d+-h\d+[^&]*/, '=w1600-h1200-k-no').replace(/=s\d+[^&]*/, '=s1600');

    if (debugShot) await page.screenshot({ path: debugShot, fullPage: false }).catch(() => {});

    // fetch the hero bytes + derive brand colour
    if (out.heroUrl) {
      try {
        const res = await fetch(out.heroUrl, { signal: AbortSignal.timeout(15000) });
        if (res.ok) {
          const buf = Buffer.from(await res.arrayBuffer());
          out.heroBytes = buf.length;
          out._heroBuf = buf; // consumed by the caller for R2 upload (not serialized)
          out.brandColor = await dominantBrandColor(buf);
        }
      } catch { /* hero fetch best-effort */ }
    }

    // hours precedence: real weekly table > today's close+default open > null (caller may apply a demo default)
    out.hoursJson = out.weekScraped
      ? Object.fromEntries(DAYS.map((d) => [d, out.weekScraped[d] || { isOpen: false, open: '00:00', close: '00:00' }]))
      : (out.hoursToday ? weeklyFromToday(out.hoursToday.open, out.hoursToday.close) : null);
    out.ok = Boolean(out.matchedName && (out.address || out.lat || out.heroUrl));
    await browser.close();
    return out;
  } catch (e) {
    if (browser) await browser.close().catch(() => {});
    return { ...out, ok: false, reason: String(e?.message || e) };
  }
}

// standalone probe
if (process.argv[1] && process.argv[1].endsWith('maps-enrich.mjs')) {
  const name = process.argv[2] || 'Apollonia';
  const city = process.argv[3] || 'durres';
  const packet = await enrichFromMaps({ name, city, debugShot: process.env.MAPS_SHOT || null });
  const { _heroBuf, ...printable } = packet;
  console.log(JSON.stringify(printable, null, 2));
}
