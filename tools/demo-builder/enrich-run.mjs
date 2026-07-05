// enrich-run — batch the Maps enrichment for a list of already-provisioned demo venues, convert the scraped
// hero/sign photo into the storefront's webp assets (landscape hero + square logo), and emit one wiring packet
// per venue for the in-container R2 + DB seam. Standalone from the provisioning loop so it can also RE-enrich.
//
// Input JSON (argv[2]): [{ slug, name, location_id, cuisine, city?, seedPrimary? }]
// Output: writes <out>/packets.json ([{ slug, location_id, address, lat, lng, phone, googleRating,
//   googleReviewCount, googleMapsUrl, hoursJson, primaryColor, heroWebpB64, logoWebpB64, notes[] }]) — the
//   image bytes are base64 so the container seam can PutObject them without a second scrape.

import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { createRequire } from 'node:module';
import { enrichFromMaps } from './maps-enrich.mjs';

function loadSharp() {
  for (const base of ['/root/dowiz/apps/api/', '/root/dowiz/packages/ui/', '/root/dowiz/']) {
    try { return createRequire(base + 'noop.js')('sharp'); } catch { /* next */ }
  }
  return null;
}
const sharp = loadSharp();

// A demo-default weekly (daily 11:00–23:00) when Maps hours are gated — owner edits on claim (matches ArtePasta).
const DAYS = ['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday'];
const demoHours = () => Object.fromEntries(DAYS.map((d) => [d, { isOpen: true, open: '11:00', close: '23:00' }]));

async function main() {
  const file = process.argv[2];
  const outDir = process.argv[3] || '.';
  if (!file) { console.error('usage: enrich-run.mjs <venues.json> <outDir>'); process.exit(1); }
  if (!sharp) { console.error('FATAL: sharp not resolvable (needed for webp conversion)'); process.exit(2); }
  const venues = JSON.parse(readFileSync(resolve(file), 'utf8'));
  const packets = [];
  const out = resolve(outDir, 'packets.json');
  const withTimeout = (pr, ms) => Promise.race([pr, new Promise((res) => setTimeout(() => res({ ok: false, reason: `timeout ${ms}ms` }), ms))]);

  for (const v of venues) {
    const notes = [];
    const p = await withTimeout(enrichFromMaps({ name: v.name, city: v.city || 'durres' }), 90000);
    if (!p.ok) { notes.push(`scrape-failed: ${p.reason || 'no match'}`); packets.push({ slug: v.slug, location_id: v.location_id, notes }); console.error(`[${v.slug}] SCRAPE FAIL: ${p.reason || 'no match'}`); continue; }

    let heroWebpB64 = null, logoWebpB64 = null;
    if (p._heroBuf) {
      try {
        const hero = await sharp(p._heroBuf).resize(1600, 1200, { fit: 'cover', position: 'attention' }).webp({ quality: 82 }).toBuffer();
        heroWebpB64 = hero.toString('base64');
        const logo = await sharp(p._heroBuf).resize(512, 512, { fit: 'cover', position: 'attention' }).webp({ quality: 88 }).toBuffer();
        logoWebpB64 = logo.toString('base64');
      } catch (e) { notes.push(`webp-convert-failed: ${String(e?.message || e)}`); }
    } else notes.push('no hero photo scraped');

    const primaryColor = p.brandColor || v.seedPrimary || null; // brand override only when vivid; else keep cuisine seed
    if (p.brandColor) notes.push(`brand-color from sign: ${p.brandColor}`); else notes.push('brand-color not vivid → keep cuisine seed');
    if (!p.hoursJson) notes.push('hours gated → demo default 11:00–23:00');

    packets.push({
      slug: v.slug, location_id: v.location_id, matchedName: p.matchedName,
      address: p.address || null, lat: p.lat ?? null, lng: p.lng ?? null, phone: p.phone || null,
      googleRating: p.googleRating ?? null, googleReviewCount: p.googleReviewCount ?? null, googleMapsUrl: p.googleMapsUrl || null,
      hoursJson: p.hoursJson || demoHours(),
      primaryColor, heroWebpB64, logoWebpB64, notes,
    });
    console.error(`[${v.slug}] OK rating=${p.googleRating ?? '-'} hero=${heroWebpB64 ? Math.round(heroWebpB64.length * 0.75 / 1024) + 'kb' : 'none'} color=${primaryColor} latlng=${p.lat ? 'y' : 'n'} hours=${p.hoursJson ? 'scraped' : 'default'}`);
    writeFileSync(out, JSON.stringify(packets)); // incremental: survive a mid-batch kill
  }

  writeFileSync(out, JSON.stringify(packets));
  // a compact human summary (no base64) alongside
  writeFileSync(resolve(outDir, 'packets-summary.json'), JSON.stringify(packets.map(({ heroWebpB64, logoWebpB64, ...r }) => ({ ...r, hero: heroWebpB64 ? 'yes' : 'no', logo: logoWebpB64 ? 'yes' : 'no' })), null, 2));
  console.error(`\nwrote ${packets.length} packets → ${out}`);
}
main().catch((e) => { console.error('FATAL', e?.message || e); process.exit(1); });
