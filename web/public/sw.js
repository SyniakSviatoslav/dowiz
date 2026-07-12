/*
 * sw.js — courier out-of-app dispatch push handler.
 *
 * Closes the gap: the backend (apps/api notifications/workers) sends web-push
 * with the order payload, but the legacy flow had no Service Worker push
 * handler, so couriers received NOTHING when the app/phone was locked or
 * backgrounded. This SW runs even when the page is closed, shows the
 * notification, and routes the tap back into the order.
 *
 * No deps. Plain Service Worker scope.
 *
 * Payload contract (mirrors apps/api WebPushAdapter / customer-status push):
 *   { title, body, tag, data: { orderId, locationId, url, pickup, dropoff } }
 * `pickup` / `dropoff` are courier-dispatch extras this SW renders when present.
 *
 * LIMITATION (documented, not fixed here): iOS/iPadOS Safari only surfaces
 * Web Push for PWAs *added to the Home Screen* and routed through Apple's
 * APNs — there is no direct Web Push on iOS without the installed PWA. Android
 * + desktop Chrome/Edge/Firefox deliver normally. See README.
 */
/* eslint-disable local/no-hardcoded-string -- Service Worker has no i18n runtime; notification copy is the contract payload */
/* eslint-disable local/no-empty-catch -- push payload parse fallback is intentional; nothing to log when a push body is non-JSON */
self.addEventListener('install', function () {
  self.skipWaiting();
});

self.addEventListener('activate', function (event) {
  event.waitUntil(self.clients.claim());
});

// ---- push: render a notification even when the app is locked/closed -------
self.addEventListener('push', function (event) {
  var payload = { title: 'Delivery update', body: '', data: {} };
  try {
    if (event.data) payload = Object.assign(payload, event.data.json());
  } catch {
    // Non-JSON push body: surface the raw text.
    payload.body = event.data ? event.data.text() : 'New dispatch';
  }

  var data = payload.data || {};
  var lines = [];
  if (payload.body) lines.push(payload.body);
  if (data.orderId) lines.push('Order #' + String(data.orderId).slice(0, 8).toUpperCase());
  if (data.pickup) lines.push('Pickup: ' + data.pickup);
  if (data.dropoff) lines.push('Dropoff: ' + data.dropoff);

  var title = payload.title || 'New dispatch';
  var body = lines.join(' · ') || 'New courier dispatch';

  var actions = [{ action: 'open', title: 'Open' }];
  if (data.orderId) actions.push({ action: 'navigate', title: 'View order' });

  var opts = {
    body,
    tag: data.orderId ? 'courier-' + data.orderId : (payload.tag || 'courier-dispatch'),
    renotify: true,
    data,
    // ponytail: icons live in web/public/icons (committed). If a deployment
    // drops them, the notification still renders without them.
    badge: '/icons/badge.png',
    icon: '/icons/icon-192.png',
    actions,
    // Highest priority so a locked-screen courier still feels the dispatch.
    requireInteraction: false,
    silent: false
  };

  event.waitUntil(self.registration.showNotification(title, opts));
});

// ---- notificationclick: focus an existing window or open the order --------
self.addEventListener('notificationclick', function (event) {
  event.notification.close();
  var d = (event.notification && event.notification.data) || {};
  var targetUrl = d.url || (d.orderId ? '/order/' + d.orderId : '/');

  // Action buttons: 'navigate' deep-links, 'open' just focuses.
  if (event.action === 'open' && !d.url) {
    targetUrl = '/';
  }

  event.waitUntil(
    self.clients.matchAll({ type: 'window', includeUncontrolled: true }).then(function (cls) {
      for (var i = 0; i < cls.length; i++) {
        var c = cls[i];
        if ('focus' in c) {
          c.postMessage({ type: 'courier_dispatch', data });
          if (targetUrl && 'navigate' in c) {
            try { c.navigate(targetUrl); } catch { /* cross-origin navigate may throw; focus still applies */ }
          }
          return c.focus();
        }
      }
      if (self.clients.openWindow) return self.clients.openWindow(targetUrl);
    })
  );
});

// ---- pushsubscriptionchange: keep the subscription alive ------------------
self.addEventListener('pushsubscriptionchange', function (event) {
  event.waitUntil(
    self.registration.pushManager.subscribe(event.oldSubscription.options)
      .then(function (sub) {
        // Tier-0 B fix: the canonical server exposes this route (was 404'ing
        // under the legacy `/api/push/resubscribe`).
        return fetch('/api/courier/push/resubscribe', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            courier_id: sub.courier_id || 'unknown',
            endpoint: sub.endpoint,
            auth: (sub.keys && sub.keys.auth) || '',
            p256dh: (sub.keys && sub.keys.p256dh) || ''
          })
        });
      })
      .catch(function () { /* best-effort resubscribe; ignore failure */ })
  );
});
