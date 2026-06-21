// ── Brand-signal extraction ────────────────────────────────────────────────
// Turn whatever brand evidence a restaurant already has — an existing website
// and/or a logo image — into the three SEED colours the storefront needs
// (primary, bg, text) plus a logo URL and display name. The client expands
// these into a full coherent palette (derivePalette), so the server only
// produces seeds and never imports the React theme util.
//
// Everything here is pure string parsing except extractLogoColor() which uses
// sharp (already an API dependency). Network fetches are SSRF-guarded.

import dns from 'dns/promises';
import net from 'net';

export interface BrandSignal {
  primary?: string;
  bg?: string;
  text?: string;
  logoUrl?: string;
  name?: string;
  /** which signals contributed, for UI transparency / debugging */
  sources: string[];
}

// ── colour helpers ─────────────────────────────────────────────────────────

export function normalizeHex(input: string | undefined | null): string | undefined {
  if (!input) return undefined;
  let h = input.trim().toLowerCase();
  const m3 = h.match(/^#([0-9a-f])([0-9a-f])([0-9a-f])$/);
  if (m3) return `#${m3[1]}${m3[1]}${m3[2]}${m3[2]}${m3[3]}${m3[3]}`;
  if (/^#[0-9a-f]{6}$/.test(h)) return h;
  const rgb = h.match(/^rgba?\(\s*(\d+)[,\s]+(\d+)[,\s]+(\d+)/);
  if (rgb) {
    const hex = (n: string) => Math.min(255, Math.max(0, parseInt(n, 10))).toString(16).padStart(2, '0');
    return `#${hex(rgb[1]!)}${hex(rgb[2]!)}${hex(rgb[3]!)}`;
  }
  return undefined;
}

function rgbOf(hex: string): { r: number; g: number; b: number } {
  return { r: parseInt(hex.slice(1, 3), 16), g: parseInt(hex.slice(3, 5), 16), b: parseInt(hex.slice(5, 7), 16) };
}
function saturation({ r, g, b }: { r: number; g: number; b: number }): number {
  const mx = Math.max(r, g, b) / 255, mn = Math.min(r, g, b) / 255;
  const l = (mx + mn) / 2;
  if (mx === mn) return 0;
  const d = mx - mn;
  return l > 0.5 ? d / (2 - mx - mn) : d / (mx + mn);
}
function lightness({ r, g, b }: { r: number; g: number; b: number }): number {
  return (Math.max(r, g, b) + Math.min(r, g, b)) / 510;
}
const isNeutral = (hex: string) => saturation(rgbOf(hex)) < 0.12;

// ── CSS / HTML parsing ──────────────────────────────────────────────────────

const BG_KEYS = ['background', 'bg', 'base', 'page', 'canvas', 'body-bg', 'surface-0'];
const PRIMARY_KEYS = ['primary', 'accent', 'brand', 'gold', 'cta', 'action', 'highlight', 'main', 'theme'];
const TEXT_KEYS = ['text', 'foreground', 'fg', 'ink', 'cream', 'copy', 'body-color', 'content', 'on-bg'];

/** Map of `--var-name` → normalized hex for every colour custom property. */
export function extractCssVars(css: string): Record<string, string> {
  const out: Record<string, string> = {};
  const re = /--([a-z0-9-]+)\s*:\s*(#[0-9a-fA-F]{3,6}|rgba?\([^)]+\))/gi;
  let m: RegExpExecArray | null;
  while ((m = re.exec(css))) {
    const hex = normalizeHex(m[2]);
    if (hex) out[m[1]!.toLowerCase()] = hex;
  }
  return out;
}

function pickByKeys(vars: Record<string, string>, keys: string[], exclude: Set<string>): string | undefined {
  for (const key of keys) {
    for (const [name, hex] of Object.entries(vars)) {
      if (name.includes(key) && !exclude.has(hex)) return hex;
    }
  }
  return undefined;
}

/** Extract brand seeds from a page's HTML (+ any inline/collected CSS). */
export function extractFromHtml(html: string, css: string, baseUrl?: string): BrandSignal {
  const sources: string[] = [];
  const sig: BrandSignal = { sources };
  const allCss = css + '\n' + html;
  const vars = extractCssVars(allCss);
  const used = new Set<string>();

  // 1) Background: prefer a bg-named var, else an explicit body{background}.
  let bg = pickByKeys(vars, BG_KEYS, used);
  if (!bg) {
    const bodyBg = allCss.match(/body[^{]*\{[^}]*background(?:-color)?\s*:\s*([^;}]+)/i);
    bg = normalizeHex(bodyBg?.[1]?.match(/#[0-9a-fA-F]{3,6}|rgba?\([^)]+\)/)?.[0]);
  }
  if (bg) { sig.bg = bg; used.add(bg); sources.push('css:bg'); }

  // 2) Primary: a brand/accent var, else <meta theme-color>, else most saturated.
  let primary = pickByKeys(vars, PRIMARY_KEYS, used);
  if (!primary) {
    const themeColor = html.match(/<meta[^>]+name=["']theme-color["'][^>]+content=["']([^"']+)["']/i)
      || html.match(/<meta[^>]+content=["']([^"']+)["'][^>]+name=["']theme-color["']/i);
    primary = normalizeHex(themeColor?.[1]);
    if (primary) sources.push('meta:theme-color');
  } else {
    sources.push('css:primary');
  }
  if (!primary) {
    // Fallback: the most saturated, mid-light colour anywhere in the CSS vars.
    const cand = Object.values(vars)
      .filter(h => !used.has(h) && !isNeutral(h) && lightness(rgbOf(h)) > 0.18 && lightness(rgbOf(h)) < 0.82)
      .sort((a, b) => saturation(rgbOf(b)) - saturation(rgbOf(a)))[0];
    if (cand) { primary = cand; sources.push('css:saturated'); }
  }
  if (primary) { sig.primary = primary; used.add(primary); }

  // 3) Text: a text/fg var, else the readable extreme opposite the bg.
  let text = pickByKeys(vars, TEXT_KEYS, used);
  if (text) sources.push('css:text');
  if (text) sig.text = text;

  // 4) Logo: og:image, then apple-touch-icon / icon link, made absolute.
  const og = html.match(/<meta[^>]+property=["']og:image["'][^>]+content=["']([^"']+)["']/i)
    || html.match(/<meta[^>]+content=["']([^"']+)["'][^>]+property=["']og:image["']/i);
  const iconLink = html.match(/<link[^>]+rel=["'][^"']*(?:apple-touch-icon|icon)[^"']*["'][^>]+href=["']([^"']+)["']/i)
    || html.match(/<link[^>]+href=["']([^"']+)["'][^>]+rel=["'][^"']*(?:apple-touch-icon|icon)[^"']*["']/i);
  const logoRaw = og?.[1] || iconLink?.[1];
  if (logoRaw) {
    sig.logoUrl = absolutize(logoRaw, baseUrl);
    sources.push(og ? 'og:image' : 'link:icon');
  }

  // 5) Name: og:site_name, then <title> (trimmed of trailing "| Menu" noise).
  const siteName = html.match(/<meta[^>]+property=["']og:site_name["'][^>]+content=["']([^"']+)["']/i);
  const title = html.match(/<title[^>]*>([^<]+)<\/title>/i);
  const rawName = (siteName?.[1] || title?.[1] || '').trim()
    .replace(/\s*[|\-–—]\s*(menu|home|official site|restaurant).*$/i, '')
    .replace(/\s+(menu|restaurant|official)$/i, '').trim();
  if (rawName) { sig.name = rawName.slice(0, 80); sources.push(siteName ? 'og:site_name' : 'title'); }

  return sig;
}

function absolutize(href: string, base?: string): string {
  try { return base ? new URL(href, base).toString() : href; } catch { return href; }
}

// ── SSRF-guarded fetch ──────────────────────────────────────────────────────

function isPrivateIp(ip: string): boolean {
  if (net.isIPv4(ip)) {
    const [a, b] = ip.split('.').map(Number) as [number, number];
    return a === 10 || a === 127 || a === 0 || (a === 192 && b === 168) ||
      (a === 172 && b >= 16 && b <= 31) || (a === 169 && b === 254) || a >= 224;
  }
  const v = ip.toLowerCase();
  return v === '::1' || v.startsWith('fc') || v.startsWith('fd') || v.startsWith('fe80') || v === '::';
}

/** Throw unless `url` is a public http(s) address (blocks SSRF to internal hosts). */
export async function assertPublicUrl(url: string): Promise<URL> {
  let u: URL;
  try { u = new URL(url); } catch { throw new Error('Invalid URL'); }
  if (u.protocol !== 'http:' && u.protocol !== 'https:') throw new Error('Only http(s) URLs allowed');
  const host = u.hostname.replace(/^\[|\]$/g, '');
  if (host === 'localhost') throw new Error('Refusing to fetch internal host');
  const addrs = net.isIP(host) ? [{ address: host }] : await dns.lookup(host, { all: true });
  for (const a of addrs) if (isPrivateIp(a.address)) throw new Error('Refusing to fetch internal address');
  return u;
}

async function fetchText(url: string, timeoutMs = 6000, maxBytes = 1_500_000): Promise<string> {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), timeoutMs);
  try {
    const res = await fetch(url, { signal: ctrl.signal, redirect: 'follow', headers: { 'User-Agent': 'dowiz-brand-extractor/1.0' } });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const reader = res.body?.getReader();
    if (!reader) return (await res.text()).slice(0, maxBytes);
    let received = 0; const chunks: Uint8Array[] = [];
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      received += value.length; chunks.push(value);
      if (received > maxBytes) { await reader.cancel(); break; }
    }
    return Buffer.concat(chunks).toString('utf8');
  } finally { clearTimeout(timer); }
}

/** Fetch a public website (+ up to 3 same-origin stylesheets) and extract seeds. */
export async function extractFromWebsite(rawUrl: string): Promise<BrandSignal> {
  const u = await assertPublicUrl(rawUrl);
  const html = await fetchText(u.toString());
  // Collect a few same-origin stylesheets so CSS-var palettes are seen.
  const cssHrefs = [...html.matchAll(/<link[^>]+rel=["']stylesheet["'][^>]+href=["']([^"']+)["']/gi)]
    .map(m => absolutize(m[1]!, u.toString()))
    .filter(href => { try { return new URL(href).origin === u.origin; } catch { return false; } })
    .slice(0, 3);
  let css = '';
  for (const href of cssHrefs) {
    try { css += '\n' + await fetchText(href, 5000, 800_000); } catch { /* skip unreadable sheet */ }
  }
  return extractFromHtml(html, css, u.toString());
}

// ── Logo colour sampling (sharp) ────────────────────────────────────────────

/**
 * Pick a representative brand colour from a logo image: downscale, drop
 * transparent / near-white / near-black / near-grey pixels, then return the
 * average of the most-saturated remaining cluster. Returns undefined when the
 * logo is essentially monochrome (no usable accent).
 */
export async function extractLogoColor(buffer: Buffer): Promise<string | undefined> {
  const sharp = (await import('sharp')).default;
  const size = 48;
  const { data } = await sharp(buffer)
    .resize(size, size, { fit: 'inside' })
    .ensureAlpha()
    .raw()
    .toBuffer({ resolveWithObject: true });
  // Bucket saturated pixels by coarse hue; accumulate rgb sums per bucket.
  const buckets = new Map<number, { r: number; g: number; b: number; n: number; s: number }>();
  for (let i = 0; i < data.length; i += 4) {
    const r = data[i]!, g = data[i + 1]!, b = data[i + 2]!, a = data[i + 3]!;
    if (a < 128) continue;
    const px = { r, g, b };
    const s = saturation(px), l = lightness(px);
    if (s < 0.18 || l < 0.12 || l > 0.92) continue; // skip neutral / extreme pixels
    const mx = Math.max(r, g, b), mn = Math.min(r, g, b), d = mx - mn || 1;
    let hue = 0;
    if (mx === r) hue = ((g - b) / d) % 6;
    else if (mx === g) hue = (b - r) / d + 2;
    else hue = (r - g) / d + 4;
    const key = ((Math.round(hue * 60) % 360) + 360) % 360 / 30 | 0; // 12 buckets
    const cur = buckets.get(key) || { r: 0, g: 0, b: 0, n: 0, s: 0 };
    cur.r += r; cur.g += g; cur.b += b; cur.n += 1; cur.s += s;
    buckets.set(key, cur);
  }
  if (buckets.size === 0) return undefined;
  // Prefer the bucket with the highest saturation-weighted population.
  let best: { r: number; g: number; b: number; n: number; s: number } | undefined;
  let bestScore = -1;
  for (const v of buckets.values()) {
    const score = (v.s / v.n) * Math.log2(v.n + 1);
    if (score > bestScore) { bestScore = score; best = v; }
  }
  if (!best) return undefined;
  const hex = (n: number) => Math.round(n).toString(16).padStart(2, '0');
  return `#${hex(best.r / best.n)}${hex(best.g / best.n)}${hex(best.b / best.n)}`;
}
