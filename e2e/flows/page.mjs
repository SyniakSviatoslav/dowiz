// WHAT A FLOW WATCHES ON EVERY PAGE, and the checks it runs at every stop.
//
// guard(): any console error, any uncaught exception, any 4xx/5xx the flow
// did not name in advance, any request that failed. inspect(): images that
// did not load, icons that draw blank (a `.ti` whose mask resolved to nothing
// is a grey square -- W-VERIFY 2026-09-30 -- and a mask whose data URL does not
// decode is an empty box -- memory icons-data-url-hash-broke-every-icon),
// anything wider than the viewport, and a translation key shown raw.
//
// ONE CHROMIUM, ONE CONTEXT. The launch flags are walk/_lib.mjs's (a
// single-process Chromium is ~2 procs instead of ~8 on a box that dies at 32),
// and a second context in that Chromium kills the first (measured 2026-09-24),
// so each flow launches its own and closes it in a finally.
import fs from 'node:fs';
import path from 'node:path';
import { HOST, OUT, Fail } from './lib.mjs';

let pw;
try { pw = await import('playwright'); } catch { pw = await import('/root/dowiz/node_modules/playwright/index.mjs'); }

export class NoLaunch extends Error {}
export async function launch() {
  try {
    return await pw.chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage', '--no-zygote', '--single-process',
      '--renderer-process-limit=1', '--disable-gpu', '--disable-3d-apis', '--disable-extensions',
      // HTTP/3 from this box to Cloudflare broke mid-transfer (measured
      // 2026-09-30: net::ERR_QUIC_PROTOCOL_ERROR on /lib/map/maplibre-gl.js);
      // the box's transport is not the product. TCP only.
      '--disable-quic'] });
  } catch (e) { throw new NoLaunch(`could not launch chromium: ${String(e.message).split('\n')[0]}`); }
}

/// A phone: 390x844 at dpr 1 (dpr > 1 made screenshots fail while the Sea
/// canvas ran away; memory sea-canvas-doubles-every-frame).
export const PHONE = { viewport: { width: 390, height: 844 }, deviceScaleFactor: 1, isMobile: true, hasTouch: true,
  serviceWorkers: 'block', reducedMotion: 'reduce', userAgent: 'Mozilla/5.0 (Linux; Android 14; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Mobile Safari/537.36' };

/// FLOWS_LOCAL=<dir of workers/api/public>: every .js/.css the page asks the
/// venue for is served from that tree instead. This is how the gate is proven
/// to bite: a deliberately broken copy must turn it red.
export async function routeLocal(ctx) {
  const dir = process.env.FLOWS_LOCAL; if (!dir) return;
  let served = 0;
  await ctx.route(u => u.origin === new URL(HOST).origin && /\.(js|css)$/.test(u.pathname), route => {
    const f = path.join(dir, new URL(route.request().url()).pathname);
    if (!fs.existsSync(f)) return route.continue();
    served++;
    return route.fulfill({ status: 200, contentType: f.endsWith('.css') ? 'text/css' : 'text/javascript', body: fs.readFileSync(f) });
  });
  ctx.localServed = () => served;
}
/// A local route that served nothing proves nothing: say so, loudly.
export function localServed(ctx, flow) {
  if (process.env.FLOWS_LOCAL && !(ctx.localServed?.() > 0)) throw new Fail(`${flow}: FLOWS_LOCAL=${process.env.FLOWS_LOCAL} served no file -- the run measured the deployed UI, not the local one`);
  if (process.env.FLOWS_LOCAL) console.log(`  ${flow}: ${ctx.localServed()} .js/.css files served from FLOWS_LOCAL`);
}

// Not the product: Cloudflare injects a bot script into every page at the
// edge, whose inline <script> our policy refuses (our HTML ships none), and
// WebGL is switched off in this Chromium (memory webgl-renders-nothing-on-the-box).
const NOISE = /policy is report-only|Executing inline script violates|WebGL|GPU process|THREE\.WebGLRenderer/;

export function guard(page, tag) {
  const bad = [], expected = [];
  const g = {
    bad,
    /// Name a response the flow expects to be refused: method, url regex, status.
    expect(method, re, status) { expected.push({ method, re, status, hit: 0 }); },
    noise: 0,
  };
  page.on('pageerror', e => bad.push(`${tag}: uncaught ${e.message.split('\n')[0].slice(0, 200)}`));
  page.on('console', m => {
    if (m.type() !== 'error') return;
    const t = m.text();
    if (NOISE.test(t)) { g.noise++; return; }
    // Redundant with the response / requestfailed listeners, which name the URL.
    if (/^Failed to load resource/.test(t)) return;
    bad.push(`${tag}: console error ${t.slice(0, 200)}`);
  });
  page.on('response', r => {
    if (r.status() < 400) return;
    const m = r.request().method(), u = r.url();
    const e = expected.find(x => x.method === m && x.re.test(u) && x.status === r.status());
    if (e) { e.hit++; return; }
    bad.push(`${tag}: HTTP ${r.status()} ${m} ${u.replace(HOST, '').slice(0, 120)}`);
  });
  page.on('requestfailed', r => {
    const why = r.failure()?.errorText || '';
    if (/ERR_ABORTED/.test(why)) return;   // a navigation or a reload cancelled it
    bad.push(`${tag}: request failed ${why} ${r.url().replace(HOST, '').slice(0, 120)}`);
  });
  return g;
}

/// Throw the first thing the guard saw, if anything.
export function clean(g, where) {
  if (g.bad.length) throw new Fail(`${where}: ${g.bad[0]}${g.bad.length > 1 ? ` (+${g.bad.length - 1} more)` : ''}`);
}

/// The page checks, run in the page. Answers a list of problems.
export async function inspect(page, where) {
  const r = await page.evaluate(async () => {
    const out = [];
    const vw = document.documentElement.clientWidth;
    const shown = e => { if (!e.getClientRects().length) return false; const cs = getComputedStyle(e); return cs.visibility !== 'hidden' && cs.display !== 'none'; };
    const name = e => `${e.tagName.toLowerCase()}${e.id ? '#' + e.id : ''}${typeof e.className === 'string' && e.className ? '.' + e.className.trim().split(/\s+/).slice(0, 2).join('.') : ''}`;
    // 1. wider than the viewport
    if (document.documentElement.scrollWidth > vw + 1) out.push(`page scrolls sideways: scrollWidth ${document.documentElement.scrollWidth} > ${vw}`);
    for (const e of document.querySelectorAll('body *')) {
      const w = e.getBoundingClientRect().width;
      if (w > vw + 1 && shown(e)) { out.push(`${name(e)} is ${Math.round(w)}px wide, viewport ${vw}`); break; }
    }
    // 2. images that did not load
    for (const i of document.images) {
      if (!i.getAttribute('src') || !shown(i) || !i.complete) continue;
      if (i.naturalWidth === 0) out.push(`image failed to load: ${i.getAttribute('src').slice(0, 100)}`);
    }
    // 3. icons: a .ti with no mask draws a filled square; a mask URL that does
    // not decode draws nothing. Each distinct URL is loaded once.
    const urls = new Map();
    for (const e of document.querySelectorAll('.ti')) {
      if (!shown(e)) continue;
      const cs = getComputedStyle(e);
      const m = cs.maskImage || cs.webkitMaskImage || 'none';
      const cls = [...e.classList].find(c => c.startsWith('ti-')) || '(no ti- class)';
      if (m === 'none') { out.push(`icon draws blank: .${cls} has no mask image`); continue; }
      const u = /url\("?(.*?)"?\)$/.exec(m)?.[1];
      if (u && !urls.has(u)) urls.set(u, cls);
    }
    for (const [u, cls] of urls) {
      const ok = await new Promise(res => { const im = new Image(); im.onload = () => res(im.naturalWidth > 0); im.onerror = () => res(false); im.src = u; });
      if (!ok) out.push(`icon draws blank: .${cls} mask image does not decode`);
    }
    for (const u of document.querySelectorAll('use')) {
      const h = u.getAttribute('href') || u.getAttribute('xlink:href') || '';
      if (h.startsWith('#') && !document.getElementById(h.slice(1))) out.push(`<use> points at nothing: ${h}`);
    }
    // 4. a translation key shown raw: the i18n layers answer the KEY when a
    // word is missing, so an element whose text IS its key is the tell.
    // Only a key that cannot be a word counts (`inv_title`, `chooseLang`): the
    // English for `kcal` IS "kcal" (measured 2026-09-30, the dish sheet).
    const keyish = k => /[_.]|[a-z][A-Z]/.test(k);
    for (const e of document.querySelectorAll('[data-t]')) {
      if (shown(e) && keyish(e.dataset.t || '') && e.textContent.trim() === e.dataset.t) out.push(`raw i18n key on screen: "${e.dataset.t}"`);
    }
    for (const e of document.querySelectorAll('[data-t-attr]')) for (const p of e.dataset.tAttr.split(/\s+/)) {
      const [a, k] = p.split(':'); if (a && k && keyish(k) && e.getAttribute(a) === k) out.push(`raw i18n key in ${a}: "${k}"`);
    }
    for (const e of document.querySelectorAll('[data-t-st]')) {
      if (shown(e) && e.textContent.trim() === e.dataset.tSt) out.push(`raw status key on screen: "${e.dataset.tSt}"`);
    }
    const snake = (document.body.innerText.match(/\b[a-z][a-z0-9]*(?:_[a-z0-9]+)+\b/g) || []).filter(w => !/^\d/.test(w));
    if (snake.length) out.push(`raw i18n key in the text: "${[...new Set(snake)].slice(0, 3).join('", "')}"`);
    return [...new Set(out)];
  });
  if (r.length) throw new Fail(`${where}: ${r[0]}${r.length > 1 ? ` (+${r.length - 1} more: ${r.slice(1, 3).join(' | ')})` : ''}`);
}

/// Open a page. The box's own network drops connections to Cloudflare
/// (measured 2026-09-30: net::ERR_CONNECTION_CLOSED on the first navigation,
/// connect timeouts on node's fetch), so a NAVIGATION that never got an answer
/// is tried at most three times -- each one printed. An answer, any answer, is
/// not retried: the guard judges it.
export async function go(page, url, g) {
  for (let i = 1; ; i++) {
    const seen = g ? g.bad.length : 0;
    try { return await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 90000 }); } catch (e) {
      const why = String(e.message).split('\n')[0];
      // The failed attempt's own error page ("chrome-error://") can interrupt
      // the next one, so that counts as the same transport failure.
      if (i >= 3 || !/net::ERR_(CONNECTION|TIMED_OUT|NAME|NETWORK|INTERNET|SSL)|interrupted by another navigation to "chrome-error/.test(why)) throw new Fail(`opening ${url.replace(HOST, '') || '/'}: ${why}`);
      await page.goto('about:blank').catch(() => {});
      await page.waitForTimeout(2000);
      // The failed document request itself is the retried thing, not a finding.
      const dropped = g ? g.bad.splice(seen) : [];
      console.log(`  !! navigation to ${url.replace(HOST, '') || '/'} failed (${why.slice(0, 80)}), try ${i + 1} of 3${dropped.length ? `; set aside: ${dropped.join(' | ').slice(0, 160)}` : ''}`);
    }
  }
}

let shots = 0;
export async function shot(page, label) {
  const f = `${OUT}/${String(++shots).padStart(2, '0')}-${label.replace(/[^a-z0-9-]+/gi, '_').slice(0, 60)}.png`;
  try { await page.screenshot({ path: f }); return f; } catch (e) { return `(screenshot failed: ${String(e.message).slice(0, 80)})`; }
}

/// Wait for a condition on the page, bounded; answers whether it came true.
export const until = (page, fn, arg, ms = 20000) => page.waitForFunction(fn, arg, { timeout: ms, polling: 250 }).then(() => true, () => false);
