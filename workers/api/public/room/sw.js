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

const SHELL_CACHE = 'dowiz-room-shell-2026-09-25-mcp-voice';
const SHELL = [
  '/room/',
  '/room/app.js',
  '/room/logic.js',
  '/room/net.js',
  '/room/i18n.js',
  '/room/sheet.js',
  '/room/menu.js',
  '/room/pay.js',
  '/room/open.js',
  '/room/transfer.js',
  '/room/till.js',
  '/room/till-view.js',
  '/room/floor.js',
  '/room/guest.js',
  '/room/screens.js',
  '/room/parts.js',
  // Voice (2026-09-25): the mic in the header and the browser half it drives.
  '/room/voice.js',
  '/lib/voice.js',
  '/room/mcp.js',
  '/lib/mcp.js',
  '/lib/mcp-words.js',
  '/lib/mcp.css',
  '/room/room.css',
  '/lib/money.js',
  '/lib/vocab.js',
  '/lib/outbox.js',
  // The lessons (W1..W10): learn.js and guide.js are static imports of app.js;
  // their sheets are added by the modules at run time. /learn/lessons.json is
  // fetched when a lesson list opens and needs the network, so it is not here.
  '/lib/guide.js',
  '/lib/guide.css',
  '/lib/learn.js',
  '/lib/learn.css',
  '/store/storage.js',
  '/admin/i18n.js',
  '/lib/icons.css',
  '/lib/tokens.css',
  '/lib/components.css',
  // The design system: `index.js` re-exports every component module, so the
  // whole folder is in the graph, not only what the room calls.
  '/lib/ui/ui.css',
  '/lib/ui/index.js',
  '/lib/ui/core.js',
  '/lib/ui/button.js',
  '/lib/ui/badge.js',
  '/lib/ui/chip.js',
  '/lib/ui/field.js',
  '/lib/ui/segmented.js',
  '/lib/ui/tabs.js',
  '/lib/ui/list.js',
  '/lib/ui/empty.js',
  '/lib/ui/skeleton.js',
  '/lib/ui/toast.js',
  '/lib/ui/sheet.js',
  '/lib/ui/money.js',
  '/lib/ui/card.js',
  '/lib/ui/time.js',
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
