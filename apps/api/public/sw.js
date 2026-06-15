"use strict";
(() => {
  // src/client/pwa/sw.ts
  var CACHE_PREFIX = "dowiz-shell-v";
  var currentCacheName = CACHE_PREFIX + "1";
  self.addEventListener("install", (event) => {
    event.waitUntil(self.skipWaiting());
  });
  self.addEventListener("activate", (event) => {
    event.waitUntil(
      caches.keys().then((keys) => {
        return Promise.all(
          keys.map((key) => {
            if (key.startsWith(CACHE_PREFIX) && key !== currentCacheName) {
              return caches.delete(key);
            }
          })
        );
      }).then(() => self.clients.claim())
    );
  });
  self.addEventListener("fetch", (event) => {
    const url = new URL(event.request.url);
    if (url.pathname.startsWith("/api/") || url.pathname.startsWith("/ws/")) {
      return;
    }
    if (url.pathname.endsWith("sw.js") || url.pathname.endsWith(".webmanifest")) {
      return;
    }
    if (event.request.method === "GET") {
      event.respondWith(
        caches.match(event.request).then((cachedResponse) => {
          if (cachedResponse) {
            return cachedResponse;
          }
          return fetch(event.request).then((networkResponse) => {
            if (!networkResponse || networkResponse.status !== 200 || networkResponse.type !== "basic") {
              return networkResponse;
            }
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
  self.addEventListener("message", (event) => {
    if (event.data && event.data.type === "UPDATE_CACHE_VERSION") {
      const newVersion = event.data.version;
      if (newVersion) {
        currentCacheName = CACHE_PREFIX + newVersion;
        self.clients.claim();
      }
    }
  });
})();
