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

const SHELL_CACHE = 'dowiz-shell-2026-09-24d';
/// THE WHOLE MODULE GRAPH, not just its entry.
///
/// This list used to hold the document, `/app.js` and the three stylesheets,
/// while `/app.js` statically imports fourteen more modules. Offline the graph
/// failed at LINK time, so `app.js` never ran -- not even the line that shows
/// the venue's phone number and the words "no connection". An installed app
/// opened on a blank page under a header that said "...", and the install
/// sheet that persuaded the customer to add it promises "the menu opens with
/// one tap, even offline".
///
/// Derived from the graph rather than remembered: every `from '/...'` reachable
/// from `/app.js`. The dynamic imports (checkout, track, address, live, the
/// map, the sea) are deliberately NOT here -- each one needs the network to be
/// useful, and the point of the shell is that the menu and the venue's own
/// offline line come up without it.
const SHELL_CACHE_MODULES = [
  '/app.js',
  // Booking: nav.js imports /store/booking.js statically, so the offline
  // shell could not load without it (FIX-A, 2026-09-24).
  '/lib/booking-time.js',
  '/lib/booking-guest.js',
  '/store/booking.js',
  '/store/booking-mine.js',
  '/store/booking-words.js',
  '/lib/money.js',
  // `/store/sea.js` imports the generated vocabulary at parse time, and a
  // static import that is not in the shell is the exact failure the list above
  // describes: the shell caches, the module fails at link time, the page is
  // blank offline.
  '/lib/vocab.js',
  '/store/cart.js',
  // The privacy-notice link (P8): menu.js and booking.js import it statically.
  '/store/consent.js',
  '/store/dish.js',
  '/store/eta.js',
  '/store/i18n.js',
  '/store/install.js',
  '/store/menu.js',
  '/store/motion.js',
  '/store/nav.js',
  // The storefront's pieces on the design system (store/parts.js, 2026-09-24).
  '/store/parts.js',
  '/store/sea.js',
  '/store/state.js',
  '/store/storage.js',
  '/store/table.js',
  '/store/ui.js',
  '/store/venue.js',
  // The design system: `index.js` re-exports every component module, so the
  // whole folder is in the graph, not only what the storefront calls.
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
const SHELL = [
  '/',
  ...SHELL_CACHE_MODULES,
  '/store/store.css',
  '/lib/tokens.css',
  '/lib/components.css',
  '/lib/ui/ui.css',
  '/lib/icons.css',
];
/// Paths that are never cached: the hub speaks, the media is immutable already.
const NEVER = [/^\/api\//, /^\/media\//];

self.addEventListener('install', ev => {
  // ONE MISSING FILE MUST NOT EMPTY THE WHOLE SHELL. `addAll` is
  // all-or-nothing, and the `.catch(() => {})` around it turned any single 404
  // into "nothing is cached at all" -- the precise state this worker exists to
  // prevent, reached silently and indistinguishable from success.
  ev.waitUntil(
    caches.open(SHELL_CACHE).then(async c => {
      const missing = [];
      await Promise.all(SHELL.map(p => c.add(p).catch(() => missing.push(p))));
      if (missing.length) console.error('sw: shell incomplete, could not cache', missing);
    }).catch(e => console.error('sw: no shell cache at all', e))
  );
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
