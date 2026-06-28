import { assertPublicUrl } from '../../lib/brand-extractor.js';

// P6-3 — MenuSource.locate: fetch a restaurant's public menu page/PDF and classify it, so MenuExtractor
// can feed it to the parser port. Net-new (brand-extractor only fetches brand signals). Council guards:
//  • SSRF — reuse assertPublicUrl per hop (resolve redirects manually; refuse private/link-local IPs).
//    Residual (shared with brand-extractor): the DNS-rebind window between lookup and connect is not
//    fully closed without a pinned-IP dispatcher — tracked as M3; same posture as the existing scraper.
//  • robots.txt — honor Disallow for our UA before fetching the menu (operator decision: honor robots).
//  • bounds — timeout + byte cap (large menus may truncate → flagged, never silently "full").

export type MenuKind = 'html' | 'pdf' | 'image' | 'none';
export interface LocateResult {
  kind: MenuKind;
  bytes?: Buffer;
  finalUrl?: string;
  truncated?: boolean;
  note?: string;
}

const UA = 'dowiz-menu-bot';
const MAX_BYTES = 4_000_000; // menus (esp. PDFs) run large; cap + flag truncation (M4)
const TIMEOUT_MS = 8000;

/** Minimal robots.txt check: is `path` Disallowed for our UA (or `*`)? Fail-open only on fetch error. */
async function robotsAllows(origin: string, path: string): Promise<boolean> {
  try {
    await assertPublicUrl(origin + '/robots.txt');
    const res = await fetch(origin + '/robots.txt', { headers: { 'user-agent': UA }, signal: AbortSignal.timeout(5000) });
    if (!res.ok) return true; // no robots → allowed
    const txt = await res.text();
    // Walk groups; collect Disallow rules under a matching User-agent (* or our UA).
    let applies = false;
    const disallows: string[] = [];
    for (const lineRaw of txt.split('\n')) {
      const line = lineRaw.split('#')[0]!.trim();
      const [k, ...rest] = line.split(':');
      const key = (k ?? '').toLowerCase();
      const val = rest.join(':').trim();
      if (key === 'user-agent') applies = val === '*' || val.toLowerCase() === UA;
      else if (key === 'disallow' && applies && val) disallows.push(val);
    }
    return !disallows.some((d) => path.startsWith(d));
  } catch {
    return true; // robots unreachable → do not block (matches common crawler behavior)
  }
}

/** Fetch with per-hop SSRF re-validation on redirects (mirrors brand-extractor's manual-redirect guard). */
async function guardedFetchBytes(rawUrl: string): Promise<{ bytes: Buffer; contentType: string; finalUrl: string; truncated: boolean }> {
  let currentUrl = (await assertPublicUrl(rawUrl)).toString();
  for (let hop = 0; hop < 5; hop++) {
    await assertPublicUrl(currentUrl);
    const res = await fetch(currentUrl, {
      headers: { 'user-agent': UA, accept: 'text/html,application/pdf,image/*' },
      redirect: 'manual',
      signal: AbortSignal.timeout(TIMEOUT_MS),
    });
    if (res.status >= 300 && res.status < 400 && res.headers.get('location')) {
      currentUrl = new URL(res.headers.get('location')!, currentUrl).toString();
      continue;
    }
    const contentType = (res.headers.get('content-type') || '').toLowerCase();
    const buf = Buffer.from(await res.arrayBuffer());
    const truncated = buf.length > MAX_BYTES;
    return { bytes: truncated ? buf.subarray(0, MAX_BYTES) : buf, contentType, finalUrl: currentUrl, truncated };
  }
  throw new Error('too many redirects');
}

function classify(contentType: string): MenuKind {
  if (contentType.includes('application/pdf')) return 'pdf';
  if (contentType.startsWith('image/')) return 'image';
  if (contentType.includes('text/html') || contentType.includes('text/plain') || contentType === '') return 'html';
  return 'none';
}

/**
 * Locate a menu document at `websiteUrl`. Returns kind 'none' (with a note) on robots-disallow, SSRF
 * refusal, or unfetchable — the caller routes 'none' to MENU_NOT_FOUND (required reason). Never throws
 * for an expected miss; throws only on programmer error.
 */
export async function locate(websiteUrl: string): Promise<LocateResult> {
  let origin: string;
  let path: string;
  try {
    const u = new URL(websiteUrl);
    origin = u.origin;
    path = u.pathname || '/';
  } catch {
    return { kind: 'none', note: 'invalid website_url' };
  }
  try {
    if (!(await robotsAllows(origin, path))) return { kind: 'none', note: 'robots.txt disallows' };
    const { bytes, contentType, finalUrl, truncated } = await guardedFetchBytes(websiteUrl);
    const kind = classify(contentType);
    if (kind === 'none') return { kind: 'none', finalUrl, note: `unsupported content-type: ${contentType}` };
    return { kind, bytes, finalUrl, truncated };
  } catch (e) {
    return { kind: 'none', note: `fetch failed: ${(e as Error).message}` };
  }
}
