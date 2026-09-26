// The courier app's own service worker — the phone that reopens underground.
//
// WHY IT EXISTS. The courier app had none: `/sw.js` is the storefront's, it is
// registered only by the storefront, and its shell lists the storefront's
// modules. So a courier who killed the app in a basement and opened it again
// there got the browser's own "no internet" page — not the address they were
// driving to, not the queued "delivered" tap, not even the words "no
// connection". The outbox (`/lib/outbox.js`) kept the tap safe in IndexedDB;
// nothing could run to show it.
//
// SCOPE IS `/courier/`, the default for a worker served from here, so it
// never answers a storefront or console request.
//
// NETWORK FIRST, like the storefront's. The run itself comes from the hub; what
// this keeps is the SHELL — the document and its whole static module graph —
// and `app.js` shows the last answer the hub gave, labelled with its time.
//
// The list is the module graph of `/courier/app.js`, derived rather than
// remembered (every static `import … from '/…'` reachable from it), and
// `tools/gates/sw-shell.sh` refuses a list that has fallen behind. The two
// dynamic imports (`/lib/live.js`, `/lib/particle-cloud.js`) and the map need
// the network to be useful and are deliberately not here.

// Bumped 2026-09-24 (b): courier.css changed (the --st-* status colours were
// restored) and the lessons engine (/lib/learn.js + its sheet) joined the graph;
// a cache under the old name would keep serving the old sheet offline.
const SHELL_CACHE = 'dowiz-courier-shell-2026-09-25-mcp';
const SHELL = [
  '/courier/',
  '/courier/app.js',
  '/courier/i18n.js',
  '/courier/screens.js',
  '/courier/mcp.js',
  '/lib/mcp.js',
  '/lib/mcp-words.js',
  '/lib/mcp.css',
  '/courier/courier.css',
  '/lib/guide.js',
  // The lessons (C1..C6). learn.js is a static import of app.js; the two sheets
  // are <link>s the modules add at run time, listed so a lesson opened in a
  // tunnel is still drawn. /learn/lessons.json is NOT here: it is fetched when
  // the list is opened, and the list says "lessons need a connection" without it.
  '/lib/learn.js',
  '/lib/learn.css',
  '/lib/guide.css',
  '/lib/money.js',
  // `money.js` imports it. It was NOT here until 2026-09-24 (audit row G2), and
  // one missing import rejects the whole graph: the courier app was a blank
  // page offline. `tools/gates/sw-shell.sh` now refuses that shape.
  '/lib/vocab.js',
  '/lib/outbox.js',
  '/lib/voice.js',
  '/store/storage.js',
  '/lib/icons.css',
  '/lib/tokens.css',
  '/lib/components.css',
  // The design system: `index.js` re-exports every component module, so the
  // whole folder is in the graph, not only what the courier calls.
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
  // One missing file must not empty the whole shell (see `/sw.js`).
  ev.waitUntil(
    caches.open(SHELL_CACHE).then(async c => {
      const missing = [];
      await Promise.all(SHELL.map(p => c.add(p).catch(() => missing.push(p))));
      if (missing.length) console.error('courier sw: shell incomplete, could not cache', missing);
    }).catch(e => console.error('courier sw: no shell cache at all', e))
  );
  self.skipWaiting();
});

self.addEventListener('activate', ev => {
  ev.waitUntil(caches.keys().then(keys => Promise.all(
    keys.filter(k => k.startsWith('dowiz-courier-shell-') && k !== SHELL_CACHE).map(k => caches.delete(k))
  )));
  self.clients.claim();
});

self.addEventListener('fetch', ev => {
  const url = new URL(ev.request.url);
  if (ev.request.method !== 'GET' || url.origin !== self.location.origin) return;
  if (NEVER.some(re => re.test(url.pathname))) return;
  const key = url.pathname === '/courier/index.html' ? '/courier/' : url.pathname;
  ev.respondWith(
    fetch(ev.request).then(res => {
      if (res.ok && SHELL.includes(key)) caches.open(SHELL_CACHE).then(c => c.put(key, res.clone())).catch(() => {});
      return res;
    }).catch(() => caches.match(key).then(hit =>
      hit || (ev.request.mode === 'navigate' ? caches.match('/courier/') : undefined)))
  );
});
