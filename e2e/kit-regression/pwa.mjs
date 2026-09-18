// Installability gate — the app is installable, and it opens with no network.
//
// "Installable" is not a feeling. A browser checks a specific list before it
// will offer an install, and every item on that list is checked here against
// the LIVE deploy: a manifest that parses and is served as a manifest, a
// start_url inside the scope, an icon at 192 and one at 512, one marked
// maskable, and a service worker that reaches `activated` with a fetch handler.
//
// Then the part that matters to a customer in a basement: the browser is taken
// offline and the page is reloaded. If the shell was not really cached, this is
// where it shows, and no amount of correct manifest JSON hides it.

import { chromium, devices } from 'playwright';

const HOST = process.env.HOST || 'https://dubin-sushi.dowiz.org';
const results = [];
function check(name, ok, detail = '') {
  results.push({ name, ok, detail });
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${ok || !detail ? '' : `\n      ${detail}`}`);
}

// A PNG's real dimensions, from the IHDR chunk. The manifest DECLARES a size;
// this reads the one the file actually has, because a 192 entry pointing at a
// 64px image is installable-looking and wrong.
function pngSize(buf) {
  if (buf.length < 24 || buf.readUInt32BE(0) !== 0x89504e47) return null;
  return { w: buf.readUInt32BE(16), h: buf.readUInt32BE(20) };
}

export async function run() {
  // ── The manifest, as served ────────────────────────────────────────────────
  const mres = await fetch(`${HOST}/kit/manifest.webmanifest`);
  const mtype = mres.headers.get('content-type') || '';
  const mtext = await mres.text();
  let m = null;
  try { m = JSON.parse(mtext); } catch { /* reported below */ }

  check('the manifest is served and parses', mres.status === 200 && !!m,
    `${mres.status} ${mtype} ${mtext.slice(0, 120)}`);
  check('it is served as a manifest, not as text',
    /application\/manifest\+json|application\/json/.test(mtype), mtype);
  if (!m) { return report(); }

  check('it has a name and a short_name that fits under an icon',
    !!m.name && !!m.short_name && m.short_name.length <= 12,
    `${m.name} / ${m.short_name}`);
  check('display is standalone, so it opens without browser chrome',
    m.display === 'standalone', String(m.display));
  check('start_url is inside scope',
    typeof m.start_url === 'string' && typeof m.scope === 'string'
    && m.start_url.startsWith(m.scope), `${m.start_url} in ${m.scope}`);
  check('a theme colour and a background colour are set — the splash needs both',
    /^#[0-9a-f]{3,8}$/i.test(m.theme_color || '') && /^#[0-9a-f]{3,8}$/i.test(m.background_color || ''),
    `${m.theme_color} / ${m.background_color}`);

  // ── The icons, as files ────────────────────────────────────────────────────
  const icons = Array.isArray(m.icons) ? m.icons : [];
  const fetched = [];
  for (const i of icons) {
    const r = await fetch(new URL(i.src, HOST).href);
    const buf = Buffer.from(await r.arrayBuffer());
    fetched.push({ ...i, status: r.status, real: pngSize(buf) });
  }
  const has = px => fetched.find(f => f.status === 200 && f.real
    && f.real.w === px && f.real.h === px && f.real.w === f.real.h);
  check('there is a real 192x192 icon', !!has(192),
    JSON.stringify(fetched.map(f => [f.src, f.status, f.real])));
  check('there is a real 512x512 icon', !!has(512),
    JSON.stringify(fetched.map(f => [f.src, f.status, f.real])));
  check('every declared size is the size the file really is',
    fetched.every(f => f.real && `${f.real.w}x${f.real.h}` === f.sizes),
    JSON.stringify(fetched.map(f => [f.sizes, f.real])));
  check('one icon is maskable, so Android does not put it in a white circle',
    fetched.some(f => /maskable/.test(f.purpose || '') && f.status === 200),
    JSON.stringify(icons.map(i => i.purpose)));

  // iOS reads none of the manifest. It reads these.
  const html = await (await fetch(`${HOST}/kit/`)).text();
  check('iOS has an apple-touch-icon and is told the app is capable',
    /rel="apple-touch-icon"/.test(html) && /name="apple-mobile-web-app-capable"/.test(html)
    && /rel="manifest"/.test(html),
    html.match(/<link rel="[^"]*"[^>]*>/g)?.join(' ')?.slice(0, 200));
  const touch = await fetch(`${HOST}/kit/img/apple-touch-icon.png`);
  const tbuf = Buffer.from(await touch.arrayBuffer());
  check('the apple-touch-icon is a real PNG of at least 180px',
    touch.status === 200 && pngSize(tbuf)?.w >= 180, `${touch.status} ${JSON.stringify(pngSize(tbuf))}`);

  // ── The worker, in a real browser ──────────────────────────────────────────
  const browser = await chromium.launch();
  const ctx = await browser.newContext({ ...devices['iPhone 13'] });
  const page = await ctx.newPage();
  // Cloudflare injects its own inline bot-management script at the edge, into a
  // page whose policy forbids inline scripts. It is the edge's, not ours — our
  // HTML ships no inline script at all, checked against the source — so it is
  // classified out here exactly as render.mjs does, by name rather than by
  // widening the filter to anything about a content policy.
  const ours = t => !(/Content Security Policy/.test(t) && /inline script/i.test(t));
  const errors = [];
  page.on('console', msg => {
    if (msg.type() === 'error' && ours(msg.text())) errors.push(msg.text());
  });

  await page.goto(`${HOST}/kit/`, { waitUntil: 'load' });
  const state = await page.evaluate(async () => {
    // `ready` resolves as soon as there IS an active worker, which can be while
    // it is still `activating` — the activate handler here clears old caches and
    // claims the clients, and until that finishes the worker is not yet the one
    // answering fetches. So the state is waited for, not sampled.
    const r = await navigator.serviceWorker.ready.catch(() => null);
    if (!r || !r.active) return null;
    if (r.active.state !== 'activated') {
      await new Promise(res => {
        const t = setTimeout(res, 8000);
        r.active.addEventListener('statechange', () => {
          if (r.active.state === 'activated') { clearTimeout(t); res(); }
        });
      });
    }
    return { scope: r.scope, active: !!r.active, st: r.active.state,
             controlling: !!navigator.serviceWorker.controller };
  });
  check('a service worker activates, scoped to the app',
    !!state && state.st === 'activated' && state.controlling && /\/kit\/$/.test(state.scope),
    JSON.stringify(state));
  check('registering it logs no error', errors.length === 0, errors.join(' | '));

  // Precache is the install step; wait for it rather than guessing.
  const cached = await page.evaluate(async () => {
    for (let i = 0; i < 40; i++) {
      const names = await caches.keys();
      let n = 0;
      for (const k of names) n += (await (await caches.open(k)).keys()).length;
      if (n >= 10) return { names, n };
      await new Promise(r => setTimeout(r, 250));
    }
    return { names: await caches.keys(), n: -1 };
  });
  check('the shell is precached', cached.n >= 10, JSON.stringify(cached));

  // ── The part a customer notices ────────────────────────────────────────────
  await ctx.setOffline(true);
  await page.reload({ waitUntil: 'load' }).catch(() => {});
  const offline = await page.evaluate(() => ({
    rules: [...document.styleSheets].reduce((n, s) => {
      try { return n + s.cssRules.length; } catch { return n; } }, 0),
    nodes: document.getElementById('app')?.childElementCount ?? 0,
    text: (document.body.innerText || '').trim().length,
  }));
  check('with the network off the app still opens, styled',
    offline.nodes > 0 && offline.rules > 50 && offline.text > 40, JSON.stringify(offline));

  // And the one thing that must NOT be answered from a cache.
  await ctx.setOffline(false);
  const apiCached = await page.evaluate(async () => {
    const keys = [];
    for (const k of await caches.keys())
      for (const req of await (await caches.open(k)).keys())
        if (new URL(req.url).pathname.startsWith('/api/')) keys.push(req.url);
    return keys;
  });
  check('no API response was ever cached — a price is never served stale',
    apiCached.length === 0, apiCached.join(' '));

  await browser.close();
  return report();
}

function report() {
  const failures = results.filter(r => !r.ok).length;
  console.log(failures
    ? `\n${failures} of ${results.length} FAILED`
    : `\nall ${results.length} installability checks hold`);
  return failures;
}

if (import.meta.url === `file://${process.argv[1]}`) process.exit(await run());
