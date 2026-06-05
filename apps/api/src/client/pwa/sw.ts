/// <reference lib="webworker" />

const CACHE_PREFIX = 'dowiz-shell-v';
let currentCacheName = CACHE_PREFIX + '1'; // Default, will be updated dynamically

self.addEventListener('install', (event: any) => {
  event.waitUntil(self.skipWaiting());
});

self.addEventListener('activate', (event: any) => {
  event.waitUntil(
    caches.keys().then((keys) => {
      return Promise.all(
        keys.map((key) => {
          if (key.startsWith(CACHE_PREFIX) && key !== currentCacheName) {
            return caches.delete(key);
          }
        })
      );
    }).then(() => (self as any).clients.claim())
  );
});

self.addEventListener('fetch', (event: any) => {
  const url = new URL(event.request.url);

  // Network Only for APIs and WS
  if (url.pathname.startsWith('/api/') || url.pathname.startsWith('/ws/')) {
    return; // Pass through to browser
  }

  // Do not cache the SW or manifest
  if (url.pathname.endsWith('sw.js') || url.pathname.endsWith('.webmanifest')) {
    return;
  }

  // Cache First for shell HTML and static assets (JS/CSS)
  if (event.request.method === 'GET') {
    event.respondWith(
      caches.match(event.request).then((cachedResponse) => {
        if (cachedResponse) {
          return cachedResponse;
        }

        return fetch(event.request).then((networkResponse) => {
          // Check if valid response
          if (!networkResponse || networkResponse.status !== 200 || networkResponse.type !== 'basic') {
            return networkResponse;
          }

          // Clone before caching
          const responseToCache = networkResponse.clone();
          caches.open(currentCacheName).then((cache) => {
            cache.put(event.request, responseToCache);
          });

          return networkResponse;
        });
      })
    );
  }
});

self.addEventListener('message', (event: any) => {
  if (event.data && event.data.type === 'UPDATE_CACHE_VERSION') {
    const newVersion = event.data.version;
    if (newVersion) {
      currentCacheName = CACHE_PREFIX + newVersion;
      // Triggers activate logic which will prune old caches
      (self as any).clients.claim(); 
    }
  }
});
