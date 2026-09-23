// The room app's own service worker -- the waiter who reopens it at the bar
// where the signal drops.
//
// NETWORK FIRST, like the courier's (`courier/sw.js`, commit 1b978569). The
// room itself comes from the hub; what this keeps is the SHELL -- the document
// and its whole static module graph, so the app can open, draw the last room
// with its age, and show the taps the outbox still holds.
//
// SCOPE IS `/room/`, so it never answers a storefront, console or courier
// request. `/api/` and `/media/` are never touched: a cached price or a
// cached room presented as now is worse than none.
//
// The list is the module graph of `/room/app.js`, every static import
// reachable from it (relative imports resolve to these same URLs).

const SHELL_CACHE = 'dowiz-room-shell-2026-09-23c';
const SHELL = [
  '/room/',
  '/room/app.js',
  '/room/logic.js',
  '/room/net.js',
  '/room/i18n.js',
  '/room/sheet.js',
  '/room/menu.js',
  '/room/pay.js',
  '/room/transfer.js',
  '/room/till.js',
  '/room/till-view.js',
  '/room/room.css',
  '/lib/money.js',
  '/lib/vocab.js',
  '/lib/outbox.js',
  '/store/storage.js',
  '/admin/i18n.js',
  '/lib/tokens.css',
  '/lib/components.css',
];
const NEVER = [/^\/api\//, /^\/media\//];

self.addEventListener('install', ev => {
  ev.waitUntil(
    caches.open(SHELL_CACHE).then(async c => {
      const missing = [];
      await Promise.all(SHELL.map(p => c.add(p).catch(() => missing.push(p))));
      if (missing.length) console.error('room sw: shell incomplete, could not cache', missing);
    }).catch(e => console.error('room sw: no shell cache at all', e))
  );
  self.skipWaiting();
});

self.addEventListener('activate', ev => {
  ev.waitUntil(caches.keys().then(keys => Promise.all(
    keys.filter(k => k.startsWith('dowiz-room-shell-') && k !== SHELL_CACHE).map(k => caches.delete(k))
  )));
  self.clients.claim();
});

self.addEventListener('fetch', ev => {
  const url = new URL(ev.request.url);
  if (ev.request.method !== 'GET' || url.origin !== self.location.origin) return;
  if (NEVER.some(re => re.test(url.pathname))) return;
  const key = url.pathname === '/room/index.html' ? '/room/' : url.pathname;
  ev.respondWith(
    fetch(ev.request).then(res => {
      if (res.ok && SHELL.includes(key)) caches.open(SHELL_CACHE).then(c => c.put(key, res.clone())).catch(() => {});
      return res;
    }).catch(() => caches.match(key).then(hit =>
      hit || (ev.request.mode === 'navigate' ? caches.match('/room/') : undefined)))
  );
});
