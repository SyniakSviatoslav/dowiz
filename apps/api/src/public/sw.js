const CACHE_NAME = 'deliveryos-v1';

self.addEventListener('install', () => {
  self.skipWaiting();
});

self.addEventListener('activate', (event) => {
  event.waitUntil(clients.claim());
});

self.addEventListener('push', (event) => {
  let data = { title: 'DeliveryOS', body: '', tag: 'deliveryos', data: {} };
  if (event.data) {
    try {
      data = event.data.json();
    } catch { /* ignore malformed */ }
  }

  const icon = '/favicon.ico';
  const badge = '/favicon.ico';

  event.waitUntil(
    self.registration.showNotification(data.title || 'DeliveryOS', {
      body: data.body || '',
      icon,
      badge,
      tag: data.tag || 'deliveryos',
      data: data.data || {},
      vibrate: [200, 100, 200],
      requireInteraction: true,
      silent: false,
    })
  );
});

self.addEventListener('notificationclick', (event) => {
  event.notification.close();

  const targetUrl = event.notification.data?.url || '/';
  const orderId = event.notification.data?.orderId;

  event.waitUntil(
    clients.matchAll({ type: 'window', includeUncontrolled: true }).then((clientList) => {
      for (const client of clientList) {
        if (client.url.includes(targetUrl) && 'focus' in client) {
          return client.focus();
        }
      }
      if (clients.openWindow) {
        return clients.openWindow(targetUrl);
      }
    })
  );
});

// pushsubscriptionchange: rely on client re-subscribe on next page load
// (old API route removed for security — anonymous endpoint update not safe)

