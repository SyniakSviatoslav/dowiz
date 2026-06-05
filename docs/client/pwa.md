# PWA Architecture

- **Manifest**: Dynamically served per-location at `/s/:slug/manifest.webmanifest`.
- **Service Worker**: Served at `/sw.js`. Caches shell HTML/CSS/JS. Ignores `/api/*` and `/ws/*`.
- **Cache Rotation**: Triggered dynamically via `menu_version` updates (using `postMessage` 'UPDATE_CACHE_VERSION' from the client).
