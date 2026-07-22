// dowiz service worker — P39 app-shell installability
// Cache-first for static assets, network-first for API calls, offline fallback for everything else.

const CACHE = 'dowiz-v1';
const PRECACHE = [
  '/index.html',
  '/manifest.json',
  '/public/icon-192.svg',
  '/src/styles/tokens.css',
  '/src/styles/base.css',
  '/src/styles/animations.css',
  '/src/app.js',
  '/src/lib/compose/compose.mjs',
  '/src/lib/compose/fragments.mjs',
  '/src/lib/compose/journey.mjs',
  '/src/lib/vendor/dubin_sushi_menu.mjs',
];

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE).then((cache) => cache.addAll(PRECACHE)).then(() => self.skipWaiting())
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((keys) => Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k))))
  );
});

self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);
  if (url.pathname.startsWith('/api/')) {
    event.respondWith(networkFirst(event.request));
  } else {
    event.respondWith(cacheFirst(event.request));
  }
});

async function cacheFirst(request) {
  const cached = await caches.match(request);
  return cached || fetchAndCache(request);
}

async function networkFirst(request) {
  try {
    return await fetchAndCache(request);
  } catch {
    const cached = await caches.match(request);
    return cached || new Response('{"ok":false,"error":"offline"}', { status: 503, headers: { 'Content-Type': 'application/json' } });
  }
}

async function fetchAndCache(request) {
  const response = await fetch(request);
  if (response.ok) {
    const clone = response.clone();
    caches.open(CACHE).then((cache) => cache.put(request, clone));
  }
  return response;
}
