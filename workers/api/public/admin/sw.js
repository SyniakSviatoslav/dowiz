// The owner console's service worker (W-PUSH). It exists for ONE reason: a
// browser delivers Web Push only to a service worker, and the console had
// none. It caches nothing and answers no request -- the console is always the
// network's -- it only shows the venue's "new order" and opens the console
// when the notification is tapped (`/lib/push-sw.js`). Scope `/admin/`.
importScripts('/lib/push-sw.js');
self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', ev => ev.waitUntil(self.clients.claim()));
