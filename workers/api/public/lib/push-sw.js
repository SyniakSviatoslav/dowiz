// The service workers' half of Web Push (W-PUSH), loaded by `importScripts`
// in `/sw.js`, `/courier/sw.js`, `/room/sw.js` and `/admin/sw.js`.
//
// THE MESSAGE IS ALREADY DECRYPTED by the browser (RFC 8291) when it gets
// here: `{title, body, url, tag}`, written by the hub (`notify/push/plan.rs`),
// carrying the short order number and a state, nothing personal.
//
// A TAP OPENS THE RIGHT PAGE: a window of ours already on that page is
// focused; otherwise one is opened at `url` (same origin only -- a payload can
// never send the phone somewhere else). Another of our windows is never
// navigated away: it may be the console in the middle of something.

/* eslint-env serviceworker */
self.dowizPush = {
  /// The notification to show for a push's text. PURE (node-tested).
  shape(text, origin){
    let m = {};
    try { m = JSON.parse(text || '{}') || {}; } catch { m = { body: String(text || '') }; }
    const title = String(m.title || 'dowiz').slice(0, 120);
    const body = String(m.body || '').slice(0, 240);
    let url = '/';
    try { const u = new URL(String(m.url || '/'), origin); if (u.origin === origin) url = u.pathname + u.search + u.hash; } catch {}
    return { title, options: { body, tag: String(m.tag || ''), renotify: !!m.tag, data: { url }, icon: '/platform/icon', badge: '/platform/icon' } };
  },
};

self.addEventListener('push', ev => {
  const n = self.dowizPush.shape(ev.data ? ev.data.text() : '', self.location.origin);
  ev.waitUntil(self.registration.showNotification(n.title, n.options));
});

self.addEventListener('notificationclick', ev => {
  ev.notification.close();
  const url = (ev.notification.data && ev.notification.data.url) || '/';
  ev.waitUntil((async () => {
    const wins = await self.clients.matchAll({ type: 'window', includeUncontrolled: true });
    const target = new URL(url, self.location.origin).href;
    for (const w of wins) {
      if (w.url === target && 'focus' in w) return w.focus();
    }
    return self.clients.openWindow(target);
  })());
});
