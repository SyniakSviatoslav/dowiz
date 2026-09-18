// The kit's service worker — what makes the app installable, and what makes it
// survive a lift, a basement and a tram.
//
// THE RULES, AND WHY EACH ONE IS THIS WAY ROUND
//
// 1. `/api/` IS NEVER TOUCHED. A menu price, a wallet balance and an order's
//    status are facts with an owner; serving yesterday's copy of one is worse
//    than saying nothing. The worker does not even intercept those requests, so
//    there is no cache to go stale and no code path that could answer from one.
//
// 2. A DOCUMENT IS NETWORK-FIRST. If the network answers, the customer gets the
//    deploy that is live right now — a cache-first shell is how a PWA ends up
//    running last week's build for a month. The cache is the fallback, and the
//    fallback is what makes the app open at all when offline.
//
// 3. AN ASSET IS STALE-WHILE-REVALIDATE. The stylesheet, the modules, the
//    sprite and the font are served from cache instantly and refreshed in the
//    background, so the second visit is fast and the third visit is current.
//
// 4. NOTHING CROSS-ORIGIN IS CACHED. Tiles and Stripe's script are theirs, and
//    an opaque response tells us nothing about whether it succeeded.
//
// The cache name carries a version. Bumping it drops every previous cache on
// activate, which is the only reliable way to retire an asset that was removed.

const VERSION = 'kit-v2';
const SHELL = `${VERSION}-shell`;
const RUNTIME = `${VERSION}-runtime`;

// The smallest set that renders SOMETHING useful with no network at all: the
// document, both stylesheets, the four shell modules, the icon sprite's source
// and the Latin font. Screens are not precached — 34 modules is a download a
// phone on a restaurant's wifi did not ask for — they land in RUNTIME as they
// are first opened, which is exactly the set that customer actually uses.
const PRECACHE = [
  '/kit/',
  '/kit/index.html',
  '/kit/kit.css',
  '/lib/figma.css',
  '/kit/app.js',
  '/kit/parts.js',
  '/kit/icons.js',
  '/kit/data.js',
  // WHAT THE SHELL IMPORTS IS PART OF THE SHELL. `data.js` is precached and it
  // imports these three at parse time, so a phone with no network loaded the
  // shell and then failed on the first import -- the app opened to nothing,
  // which is the exact failure this list exists to prevent. `basket.js` and
  // `me.js` are also what a customer carries between visits: the cart and the
  // address, which is the part of checkout that must survive a dead signal.
  '/lib/money.js',
  '/kit/basket.js',
  '/kit/me.js',
  '/kit/orders.js',
  '/kit/install.js',
  '/lib/font/inter-latin.woff2',
  '/lib/font/inter-cyrillic.woff2',
  '/kit/img/icon-192.png',
  '/kit/manifest.webmanifest',
];

self.addEventListener('install', e => {
  // `addAll` rejects the whole install if ONE entry 404s, which is the right
  // behaviour: a shell with a missing stylesheet is the unstyled-app bug again,
  // and an install that fails loudly leaves the previous worker serving.
  e.waitUntil(caches.open(SHELL).then(c => c.addAll(PRECACHE)));
  self.skipWaiting();
});

self.addEventListener('activate', e => {
  e.waitUntil((async () => {
    const keep = new Set([SHELL, RUNTIME]);
    for (const name of await caches.keys()) if (!keep.has(name)) await caches.delete(name);
    await self.clients.claim();
  })());
});

const cacheable = res => res && res.ok && res.type !== 'opaque';

async function networkFirst(req) {
  try {
    const res = await fetch(req);
    if (cacheable(res)) (await caches.open(SHELL)).put(req, res.clone());
    return res;
  } catch {
    // Offline. The customer's own last copy of this page, then the shell.
    return (await caches.match(req))
        || (await caches.match('/kit/index.html'))
        || new Response('Немає зʼєднання, і немає збереженої копії цієї сторінки.',
             { status: 503, headers: { 'content-type': 'text/plain; charset=utf-8' } });
  }
}

async function staleWhileRevalidate(req) {
  const cache = await caches.open(RUNTIME);
  const hit = await cache.match(req);
  const fresh = fetch(req).then(res => {
    if (cacheable(res)) cache.put(req, res.clone());
    return res;
  }).catch(() => null);
  // A hit answers now; the refresh keeps running in the background either way.
  if (hit) { fresh.catch(() => {}); return hit; }
  return (await fresh)
      || (await caches.match(req))
      || Response.error();
}

self.addEventListener('fetch', e => {
  const req = e.request;
  if (req.method !== 'GET') return;                      // POSTs are not cacheable
  const url = new URL(req.url);
  if (url.origin !== location.origin) return;            // rule 4
  if (url.pathname.startsWith('/api/')) return;          // rule 1
  if (req.mode === 'navigate') { e.respondWith(networkFirst(req)); return; }  // rule 2
  if (url.pathname.startsWith('/kit/') || url.pathname.startsWith('/lib/')) {
    e.respondWith(staleWhileRevalidate(req));            // rule 3
  }
});
