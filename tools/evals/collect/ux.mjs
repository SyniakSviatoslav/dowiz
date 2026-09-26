// SUITE nightly · UX: each surface opened in a real headless Chromium at 360 px and 1280 px,
// GET-only (no sign-in, no tap). Per surface: first contentful paint, requests and bytes at
// boot, console errors, uncaught page errors, CSP violations, and text that fails WCAG contrast.
//
// CONTRAST is NOT axe-core (no copy on the box, and installing one is not this lane's call):
// every visible element with its own text node, its computed colour against the first opaque
// background up its ancestors, ratio < 4.5 (< 3 for large text) counted. It cannot see
// gradients or images behind text; the note says "approximate" and the rule is a ratchet.
// CSP violations are counted from `securitypolicyviolation` events in the page itself, so no
// new route is needed for the nightly number (the report-only endpoint of H7 is not built).
import { ind } from '../rules.mjs';

export const SURFACES = { store: '/', admin: '/admin/', room: '/room/', courier: '/courier/' };
export const WIDTHS = [360, 1280];
export const SETTLE_MS = 4000;

/** `rgb(1, 2, 3)` / `rgba(1, 2, 3, 0.5)` → [r, g, b, a]; anything else → null. */
export function parseRgb(s) {
  const m = String(s).match(/rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:[,\s/]+([\d.]+))?/);
  return m ? [+m[1], +m[2], +m[3], m[4] === undefined ? 1 : +m[4]] : null;
}

export function luminance([r, g, b]) {
  const c = v => { const s = v / 255; return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4; };
  return 0.2126 * c(r) + 0.7152 * c(g) + 0.0722 * c(b);
}

export function contrast(a, b) {
  const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}

/** Runs IN THE PAGE (serialised by page.evaluate); tested in node over a stub document. */
export function pageProbe() {
  const parse = s => { const m = String(s).match(/rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:[,\s/]+([\d.]+))?/); return m ? [+m[1], +m[2], +m[3], m[4] === undefined ? 1 : +m[4]] : null; };
  const lum = ([r, g, b]) => [r, g, b].map(v => { const s = v / 255; return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4; })
    .reduce((n, v, i) => n + v * [0.2126, 0.7152, 0.0722][i], 0);
  const bgOf = el => {
    for (let e = el; e; e = e.parentElement) { const c = parse(getComputedStyle(e).backgroundColor); if (c && c[3] >= 1) return c; }
    return [255, 255, 255, 1];
  };
  let fails = 0, checked = 0;
  for (const el of document.querySelectorAll('body *')) {
    const own = [...el.childNodes].some(n => n.nodeType === 3 && n.textContent.trim());
    const r = el.getBoundingClientRect();
    const st = getComputedStyle(el);
    if (!own || !r.width || !r.height || st.visibility === 'hidden' || st.display === 'none' || +st.opacity === 0) continue;
    const fg = parse(st.color);
    if (!fg) continue;
    checked += 1;
    const [x, y] = [lum(fg), lum(bgOf(el))].sort((p, q) => q - p);
    const px = parseFloat(st.fontSize);
    const large = px >= 24 || (px >= 18.66 && +st.fontWeight >= 700);
    if ((x + 0.05) / (y + 0.05) < (large ? 3 : 4.5)) fails += 1;
  }
  const p = performance.getEntriesByType('paint').find(e => e.name === 'first-contentful-paint');
  const csp = window.__csp || [];
  return { fails, checked, fcp: p ? Math.round(p.startTime) : null, csp: csp.length, cspDirectives: [...new Set(csp)].join(' ') };
}

/** Installed before any page script: counts CSP violations the page itself sees. */
export const CSP_HOOK = 'window.__csp=[];document.addEventListener("securitypolicyviolation",e=>window.__csp.push(e.violatedDirective))';

/** One surface at one width. `browser` is a Playwright Browser. */
export async function visit(browser, url, width, settleMs = SETTLE_MS) {
  const ctx = await browser.newContext({ viewport: { width, height: 800 }, serviceWorkers: 'block' });
  const page = await ctx.newPage();
  await page.addInitScript(CSP_HOOK);
  const seen = { console: 0, pageErrors: 0, requests: 0, bytes: 0, first: '' };
  page.on('console', m => { if (m.type() === 'error') { seen.console += 1; seen.first ||= m.text().slice(0, 120); } });
  page.on('pageerror', () => { seen.pageErrors += 1; });
  page.on('requestfinished', async rq => {
    seen.requests += 1;
    const s = await rq.sizes().catch(() => null);
    if (s) seen.bytes += s.responseBodySize + s.responseHeadersSize;
  });
  await page.goto(url, { waitUntil: 'load', timeout: 90_000 });
  await page.waitForTimeout(settleMs);
  const probe = await page.evaluate(pageProbe);
  await ctx.close();
  return { ...seen, ...probe };
}

export async function defaultLaunch() {
  return (await import('playwright')).chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage'] });
}

export async function collect(context) {
  const ctx = { launch: defaultLaunch, settleMs: SETTLE_MS, ...context };
  const browser = await ctx.launch();
  const out = [];
  try {
    for (const [name, path] of Object.entries(SURFACES)) {
      const runs = [];
      for (const w of WIDTHS) runs.push(await visit(browser, ctx.host + path, w, ctx.settleMs));
      const [narrow] = runs;
      const src = `headless Chromium, GET ${ctx.host}${path} at ${WIDTHS.join(' and ')} px, no sign-in`;
      const sum = k => runs.reduce((n, r) => n + r[k], 0);
      out.push(ind(`ux.${name}.fcp_ms`, narrow.fcp, 'ms', 'plus25', `${src} (360 px)`));
      out.push(ind(`ux.${name}.boot_requests`, narrow.requests, 'requests', 'ratchet', `${src} (360 px)`));
      out.push(ind(`ux.${name}.boot_wire_bytes`, narrow.bytes, 'bytes', 'trend', `${src} (360 px, headers + bodies: live headers jitter by bytes, so the ratchet is surfaces.*.gzip)`));
      out.push(ind(`ux.${name}.console_errors`, sum('console'), 'errors', 'ratchet', src, narrow.first || runs[1].first ? { note: narrow.first || runs[1].first } : {}));
      out.push(ind(`ux.${name}.page_errors`, sum('pageErrors'), 'errors', 'zero', src));
      const dirs = [...new Set(runs.flatMap(r => (r.cspDirectives || '').split(' ')).filter(Boolean))].join(' ');
      out.push(ind(`ux.${name}.csp_violations`, sum('csp'), 'violations', 'zero', src, dirs ? { note: `directives: ${dirs}` } : {}));
      out.push(ind(`ux.${name}.contrast_failures`, Math.max(...runs.map(r => r.fails)), 'elements', 'ratchet', src,
        { note: `approximate (not axe): of ${Math.max(...runs.map(r => r.checked))} text elements` }));
    }
  } finally {
    await browser.close();
  }
  return out;
}
