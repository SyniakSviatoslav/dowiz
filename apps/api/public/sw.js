<<<<<<< Updated upstream
"use strict";(()=>{var i="dowiz-shell-v",n=i+"1";self.addEventListener("install",t=>{t.waitUntil(self.skipWaiting())});self.addEventListener("activate",t=>{t.waitUntil(caches.keys().then(a=>Promise.all(a.map(e=>{if(e.startsWith(i)&&e!==n)return caches.delete(e)}))).then(()=>self.clients.claim()))});self.addEventListener("fetch",t=>{let a=new URL(t.request.url);a.pathname.startsWith("/api/")||a.pathname.startsWith("/ws/")||a.pathname.endsWith("sw.js")||a.pathname.endsWith(".webmanifest")||t.request.method==="GET"&&t.respondWith(caches.match(t.request).then(e=>e||fetch(t.request).then(s=>{if(!s||s.status!==200||s.type!=="basic")return s;let r=s.clone();return caches.open(n).then(h=>{h.put(t.request,r)}),s})))});self.addEventListener("message",t=>{if(t.data&&t.data.type==="UPDATE_CACHE_VERSION"){let a=t.data.version;a&&(n=i+a,self.clients.claim())}});})();
=======
"use strict";
(() => {
  // src/client/pwa/sw.ts
  var CACHE_PREFIX = "dowiz-shell-v";
  var currentCacheName = CACHE_PREFIX + "1";
  function cachePut(cacheName, request, response) {
    caches.open(cacheName).then((cache) => cache.put(request, response)).catch(() => {
    });
  }
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
            cachePut(currentCacheName, event.request, responseToCache);
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
>>>>>>> Stashed changes
