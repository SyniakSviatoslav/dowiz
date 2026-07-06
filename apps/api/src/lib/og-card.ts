// og-card — server-side Open Graph preview card (1200×630) for a storefront `/s/:slug`. Rendered as
// SVG → PNG with sharp (already a dep; no headless browser, no external storage). Referenced from
// <meta property="og:image"> so a pasted link unfurls as a product card (venue name + Google rating +
// a real dish photo + brand accent + optional ▶ play badge) in WhatsApp/Telegram/Messenger/… and can
// be embedded as <img> in outreach email.
//
// FONT NOTE: sharp renders SVG <text> via librsvg, which needs a font installed in the container.
// The deploy image (node:22-slim) ships none, so `fonts-dejavu-core` + `fontconfig` must be present
// (Dockerfile). `DejaVu Sans` is the family referenced below; the local preview used the same font.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import sharp from 'sharp';
import { getImageUrl } from './image-url.js';

// sharp's SVG <text> is rendered by librsvg via fontconfig. The deploy image (node:22-slim) has NO
// system fonts, so we ship DejaVu Sans in apps/api/public/fonts (copied to /app/dist/public/fonts by
// the Dockerfile) and point fontconfig at it via a generated fonts.conf. Without this, card text is
// blank in production (it renders locally only because the dev box has system fonts).
let _fontconfigReady = false;
function ensureFontconfig(): void {
  if (_fontconfigReady) return;
  _fontconfigReady = true;
  if (process.env.FONTCONFIG_FILE) return; // respect an explicit override
  const here =
    typeof __dirname !== 'undefined' ? __dirname : path.dirname(fileURLToPath(import.meta.url));
  const candidates = [
    path.join(here, '..', 'public', 'fonts'), // bundled: dist/api → dist/public
    path.join(here, '..', '..', 'public', 'fonts'), // source: apps/api/src/lib → apps/api/public
    path.join(process.cwd(), 'dist', 'public', 'fonts'),
    path.join(process.cwd(), 'apps', 'api', 'public', 'fonts'),
    path.join(process.cwd(), 'public', 'fonts'),
  ];
  const fontsDir = candidates.find((p) => {
    try { return fs.existsSync(path.join(p, 'DejaVuSans.ttf')); } catch { return false; }
  });
  if (!fontsDir) return; // no bundled fonts found → fall back to system (dev); prod proof will catch it
  try {
    const cacheDir = path.join(os.tmpdir(), 'dowiz-fontconfig');
    fs.mkdirSync(cacheDir, { recursive: true });
    const conf = path.join(cacheDir, 'fonts.conf');
    fs.writeFileSync(
      conf,
      `<?xml version="1.0"?>\n<!DOCTYPE fontconfig SYSTEM "fonts.dtd">\n<fontconfig><dir>${fontsDir}</dir><cachedir>${cacheDir}</cachedir></fontconfig>\n`,
    );
    process.env.FONTCONFIG_FILE = conf;
  } catch { /* leave unset → system fonts */ }
}

// Set at module load (boot, via the route import) so FONTCONFIG_FILE is in place before ANY sharp text
// render — fontconfig caches its config on first init, so a lazy set could lose to another sharp caller.
ensureFontconfig();

export interface OgCardData {
  name: string;
  city: string | null;
  rating: number | null;
  accent: string;
  heroDataUri: string | null;
  play?: boolean;
}

const xml = (s: unknown): string =>
  String(s ?? '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

// Curated on-brand accents. A known demo cuisine wins; otherwise a stable hash of the slug picks one
// (generalises to any tenant without a per-slug table).
const CUISINE_ACCENT: Record<string, string> = {
  italian: '#3f7d4f', pizzeria: '#e0542b', seafood: '#1f9ac2', mediterranean: '#2f9d92',
  traditional: '#c1873f', turkish: '#c0392b', sushi: '#d64545',
};
const DEMO_CUISINE: Record<string, string> = {
  demo: 'sushi', apollonia: 'mediterranean', aragosta: 'seafood', artepasta: 'italian',
  'casa-mia': 'italian', 'dyrrah-mare': 'seafood', 'eljos-pizza': 'pizzeria', idua: 'traditional',
  lamuse: 'mediterranean', liriada: 'traditional', otantik: 'turkish', ventus: 'seafood',
};
const PALETTE = ['#e0542b', '#2f9d92', '#1f9ac2', '#3f7d4f', '#c1873f', '#c0392b', '#7c5cbf', '#d64545'];

export function accentFor(slug: string, _name?: string): string {
  const cuisine = DEMO_CUISINE[slug];
  if (cuisine && CUISINE_ACCENT[cuisine]) return CUISINE_ACCENT[cuisine];
  let h = 0;
  for (let i = 0; i < slug.length; i++) h = (h * 31 + slug.charCodeAt(i)) >>> 0;
  return PALETTE[h % PALETTE.length] as string;
}

/** City name only (never the country — appended once in the card), or null. */
export function cityOf(address: string | null | undefined): string | null {
  if (!address) return null;
  if (/durr/i.test(address)) return 'Durrës';
  const parts = address.split(',').map((s) => s.trim()).filter(Boolean);
  const guess = parts.length >= 2 ? (parts[parts.length - 2] as string).replace(/\d+/g, '').trim() : '';
  return guess || null;
}

/** First real FOOD photo URL from a public/preview menu payload (drinks/desserts deprioritised). */
export function pickHeroUrl(menu: any): string | null {
  const cats: any[] = Array.isArray(menu?.categories) ? menu.categories : [];
  const isDrink = (n: string) => /pije|drink|ëmbëls|embels|dessert|kafe/i.test(n || '');
  const ordered = cats.slice().sort((a, b) => (isDrink(a?.name) ? 1 : 0) - (isDrink(b?.name) ? 1 : 0));
  for (const c of ordered) {
    for (const p of (Array.isArray(c?.products) ? c.products : [])) {
      const attr = p?.attributes?.image_url;
      if (typeof attr === 'string' && /^https?:\/\//.test(attr)) return attr;
      const resolved = getImageUrl(p?.image_key ?? p?.imageKey ?? p?.imageUrl);
      if (resolved && /^https?:\/\//.test(resolved)) return resolved;
    }
  }
  return null;
}

/** Fetch an image and inline it as a data URI (baked into the PNG — no live hotlink in the final card). */
export async function fetchImageDataUri(url: string, timeoutMs = 4000): Promise<string | null> {
  try {
    const r = await fetch(url, { headers: { 'user-agent': 'Mozilla/5.0 dowiz-og' }, signal: AbortSignal.timeout(timeoutMs) });
    if (!r.ok) return null;
    const ct = r.headers.get('content-type') || 'image/jpeg';
    if (!ct.startsWith('image/')) return null;
    const buf = Buffer.from(await r.arrayBuffer());
    if (buf.length > 4_000_000) return null; // 4MB guard
    return `data:${ct};base64,${buf.toString('base64')}`;
  } catch {
    return null;
  }
}

export function buildOgCardSvg(d: OgCardData): string {
  const { name, city, rating, accent, heroDataUri, play } = d;
  const place = city ? `${city}, Shqipëri` : 'Shqipëri';
  const nameSize = name.length > 15 ? 58 : name.length > 11 ? 68 : 78;
  const hero = heroDataUri
    ? `<image x="520" y="0" width="680" height="630" href="${heroDataUri}" preserveAspectRatio="xMidYMid slice" clip-path="url(#rc)"/>`
    : `<rect x="560" y="0" width="640" height="630" fill="url(#hg)"/>`;
  const ratingPill = rating
    ? `<g transform="translate(56,196)"><rect width="188" height="52" rx="26" fill="${accent}"/><text x="26" y="35" font-size="26" fill="#0e0f10" font-weight="bold">★</text><text x="58" y="35" font-size="26" fill="#0e0f10" font-weight="bold">${xml(Number(rating).toFixed(1))}</text><text x="112" y="34" font-size="17" fill="#0e0f10" opacity="0.72" font-weight="bold">Google</text></g>`
    : '';
  const playBadge = play
    ? `<g transform="translate(880,315)"><circle r="60" fill="#0e0f10" fill-opacity="0.55" stroke="#fff" stroke-width="3"/><path d="M-18 -28 L34 0 L-18 28 Z" fill="#fff"/></g>`
    : '';
  return `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630" font-family="DejaVu Sans, Liberation Sans, sans-serif">
  <defs>
    <clipPath id="rc"><rect x="560" y="0" width="640" height="630"/></clipPath>
    <linearGradient id="fade" x1="0" y1="0" x2="1" y2="0"><stop offset="0.30" stop-color="#0e0f10" stop-opacity="1"/><stop offset="0.52" stop-color="#0e0f10" stop-opacity="0.72"/><stop offset="0.66" stop-color="#0e0f10" stop-opacity="0"/></linearGradient>
    <linearGradient id="hg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${accent}"/><stop offset="1" stop-color="#0e0f10"/></linearGradient>
  </defs>
  <rect width="1200" height="630" fill="#0e0f10"/>
  ${hero}
  <rect width="1200" height="630" fill="url(#fade)"/>
  ${playBadge}
  <g transform="translate(64,60)"><circle cx="6" cy="14" r="7" fill="${accent}"/><text x="22" y="22" font-size="27" font-weight="bold" fill="#fff">dowiz</text></g>
  <g transform="translate(456,52)"><rect width="98" height="38" rx="19" fill="none" stroke="#3a3d42" stroke-width="1.5"/><text x="49" y="25" font-size="16" fill="#c9cdd2" text-anchor="middle">Demo</text></g>
  ${ratingPill}
  <text x="56" y="322" font-size="${nameSize}" font-weight="bold" fill="#fff" letter-spacing="-1">${xml(name)}</text>
  <text x="56" y="376" font-size="24" fill="#aab0b6">${xml(place)}</text>
  <text x="56" y="512" font-size="23" font-weight="bold" fill="#fff">Menu Digjitale · Porosi pa Komision</text>
  <rect x="56" y="524" width="400" height="3" fill="${accent}"/>
  <text x="56" y="560" font-size="19" fill="#8b9198"><tspan fill="#e6e9ec" font-weight="bold">0% komision</tspan> · të dhënat e klientëve mbeten <tspan fill="#e6e9ec" font-weight="bold">tuajat</tspan></text>
</svg>`;
}

export async function renderOgCardPng(d: OgCardData): Promise<Buffer> {
  ensureFontconfig();
  return sharp(Buffer.from(buildOgCardSvg(d))).png({ compressionLevel: 9 }).toBuffer();
}
