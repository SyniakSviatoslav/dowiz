// The storefront's service worker: enough to be an app, never in the way.
//
// NETWORK FIRST, ALWAYS. The menu, the prices and the order's state are the
// hub's and must never come from a cache; a stale price is the failure this
// product exists to prevent. What this worker keeps is the SHELL -- the
// document, its scripts and stylesheets -- so an installed app opens with
// the venue's own page even when the connection has gone, and shows the
// page's own offline line rather than the browser's error.
//
// The cache is named by the deploy's own version line below; a new deploy
// with a new line drops the old shell on activation.

const SHELL_CACHE = 'dowiz-shell-2026-09-18';
const SHELL = ['/', '/app.js', '/store/store.css', '/lib/tokens.css', '/lib/components.css', '/lib/icons.css'];
/// Paths that are never cached: the hub speaks, the media is immutable already.
const NEVER = [/^\/api\//, /^\/media\//];

self.addEventListener('install', ev => {
  ev.waitUntil(caches.open(SHELL_CACHE).then(c => c.addAll(SHELL)).catch(() => {}));
  self.skipWaiting();
});
self.addEventListener('activate', ev => {
  ev.waitUntil(caches.keys().then(keys => Promise.all(keys.filter(k => k !== SHELL_CACHE).map(k => caches.delete(k)))));
  self.clients.claim();
});
self.addEventListener('fetch', ev => {
  const url = new URL(ev.request.url);
  if (ev.request.method !== 'GET' || url.origin !== self.location.origin) return;
  if (NEVER.some(re => re.test(url.pathname))) return;
  ev.respondWith(
    fetch(ev.request).then(res => {
      if (res.ok && SHELL.includes(url.pathname)) caches.open(SHELL_CACHE).then(c => c.put(ev.request, res.clone())).catch(() => {});
      return res;
    }).catch(() => caches.match(ev.request).then(hit => hit || (ev.request.mode === 'navigate' ? caches.match('/') : undefined)))
  );
});
