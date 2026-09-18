// Render gate — what a phone actually shows.
//
// WHY A REAL BROWSER. The jsdom gate that built these screens cannot see a
// content policy at all, and that blind spot cost this product a week: every
// dowiz surface rendered unstyled in production for a day while every
// server-side check said 200. This suite is the answer to "how would we know".
//
// It has already caught, in a real browser and nowhere else:
//   * the icon sprite carrying `style="display:none"` and `mask-type:alpha` as
//     INLINE STYLE ATTRIBUTES, blocked by `style-src 'self'` — three violations
//     per page and any masked icon rendering wrong;
//   * the four extracted stylesheets actually applying (238–793 rules), which
//     is the thing the CSP incident broke.
//
// ZERO NEW DEPENDENCIES. It uses the `playwright` library already in the tree,
// not `@playwright/test`: adding a runner would be a new supply-chain entry and
// the repo requires a decart comparison for one. The assertions are plain.

import { chromium } from 'playwright';

const HOST = process.env.HOST || 'https://dubin-sushi.dowiz.org';
const PLATFORM = process.env.PLATFORM_HOST || 'https://dowiz.org';

// Cloudflare serves a challenge page under rapid load from one address, and
// that page's own policy blocks our module scripts. It is not a defect in the
// page under test — the same route passes on its own every time — so a failure
// is retried before it is believed. Two retries, then it is real.
const RETRIES = Number(process.env.RETRIES ?? 2);

/** Every surface dowiz serves, plus every screen of the kit. */
export async function targets() {
  const res = await fetch(`${HOST}/kit/app.js`);
  const src = await res.text();
  const block = src.split('const SCREENS = {')[1].split('};')[0];
  const routes = [...new Set([...block.matchAll(/^\s*'?([a-z-]+)'?:/gm)].map(m => m[1]))];
  return [
    { name: 'storefront', url: `${HOST}/` },
    { name: 'owner console', url: `${HOST}/admin/` },
    { name: 'courier app', url: `${HOST}/courier/` },
    { name: 'platform', url: PLATFORM + '/' },
    ...routes.sort().map(r => ({ name: `kit:${r}`, url: `${HOST}/kit/#/${r}` })),
  ];
}

/** One page, measured. Returns a list of failures; empty means it passed. */
async function inspect(ctx, url) {
  const page = await ctx.newPage();
  const consoleErrors = [];
  page.on('console', m => {
    if (m.type() !== 'error') return;
    const t = m.text();
    // The edge's own blocked script logs here too; classified below.
    if (/Content Security Policy/.test(t)) return;
    consoleErrors.push(t.slice(0, 200));
  });
  page.on('pageerror', e => consoleErrors.push('PAGE ERROR: ' + String(e.message).slice(0, 200)));

  await page.addInitScript(() => {
    window.__csp = [];
    document.addEventListener('securitypolicyviolation', e => {
      window.__csp.push({ directive: e.violatedDirective, blocked: e.blockedURI || '' });
    });
  });

  try {
    await page.goto(url, { waitUntil: 'networkidle', timeout: 45_000 });
  } catch (e) {
    await page.close();
    return { failures: [`navigation: ${e.message.slice(0, 120)}`], probe: null };
  }
  await page.waitForTimeout(1200);

  const probe = await page.evaluate(() => {
    const cs = getComputedStyle(document.body);
    const doc = document.documentElement;
    const rules = [...document.styleSheets].reduce((n, s) => {
      try { return n + s.cssRules.length; } catch { return n; }
    }, 0);
    // `blockedURI` is the literal string "inline" for an inline script, not an
    // empty one — the first version of this predicate classified nothing.
    const isInline = b => !b || b === 'inline' || b === 'eval';
    const raw = window.__csp || [];
    const ours = raw.filter(v => !(v.directive.startsWith('script-src') && isInline(v.blocked)));
    const edge = raw.filter(v => v.directive.startsWith('script-src') && isInline(v.blocked));
    const wide = [...document.querySelectorAll('body *')]
      .filter(el => el.getBoundingClientRect().right > window.innerWidth + 1)
      .slice(0, 5)
      .map(el => `${el.tagName.toLowerCase()}.${String(el.className).split(' ')[0]}`);
    return {
      rules,
      bg: cs.backgroundColor,
      font: cs.fontFamily.split(',')[0].replace(/["']/g, ''),
      interLoaded: [...document.fonts].some(f => /Inter/i.test(f.family) && f.status === 'loaded'),
      csp: ours.map(v => `${v.directive} blocked ${v.blocked || 'inline'}`),
      cspEdge: edge.length,
      scrollW: doc.scrollWidth,
      clientW: doc.clientWidth,
      wide,
      painted: (document.body.innerText || '').trim().length,
      // The sprite must carry no inline style attribute. Regenerating it from
      // Figma exports without stripping them is the regression this guards.
      spriteInlineStyles: document.querySelectorAll('#kit-sprite [style]').length,
      brokenIcons: [...document.querySelectorAll('svg.ic use')]
        .map(u => u.getAttribute('href') || '')
        .filter(h => h.startsWith('#') && !document.getElementById(h.slice(1))).length,
    };
  });
  await page.close();

  const failures = [];
  if (consoleErrors.length) failures.push(`console: ${consoleErrors[0]}`);
  if (probe.csp.length) failures.push(`CSP (ours): ${probe.csp[0]}`);
  // The CSP incident's own falsifier: a page with almost no rules is a page
  // whose stylesheet did not apply.
  if (probe.rules < 50) failures.push(`only ${probe.rules} CSS rules applied`);
  if (probe.scrollW > probe.clientW + 1) failures.push(`scrolls sideways: ${probe.wide.join(', ')}`);
  if (!probe.painted) failures.push('nothing painted');
  if (probe.spriteInlineStyles) failures.push(`${probe.spriteInlineStyles} inline styles in the sprite`);
  if (probe.brokenIcons) failures.push(`${probe.brokenIcons} icon refs point at nothing`);
  return { failures, probe };
}

export async function run(only) {
  const all = await targets();
  const list = only ? all.filter(t => only.some(o => t.name.includes(o))) : all;

  const browser = await chromium.launch({
    args: ['--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu'],
  });
  const ctx = await browser.newContext({
    viewport: { width: 375, height: 812 },   // the design's own frame
    deviceScaleFactor: 2,
    userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 ' +
               '(KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
  });

  let failed = 0;
  for (const t of list) {
    let result = null;
    for (let attempt = 0; attempt <= RETRIES; attempt++) {
      result = await inspect(ctx, t.url);
      if (!result.failures.length) {
        if (attempt) console.log(`      (passed on retry ${attempt} — edge transient)`);
        break;
      }
      if (attempt < RETRIES) await new Promise(r => setTimeout(r, 1500));
    }
    const p = result.probe;
    const ok = !result.failures.length;
    if (!ok) failed++;
    console.log(
      `${ok ? 'PASS' : 'FAIL'}  ${t.name.padEnd(28)}` +
      (p ? ` rules=${String(p.rules).padStart(4)} font=${p.font} inter=${p.interLoaded}` +
           ` w=${p.scrollW}/${p.clientW}` : ''));
    for (const f of result.failures) console.log(`      ${f}`);
  }

  await browser.close();
  console.log(failed ? `\n${failed} of ${list.length} FAILED` : `\nall ${list.length} pages pass`);
  return failed;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const only = process.argv.slice(2).filter(Boolean);
  process.exit(await run(only.length ? only : null));
}
