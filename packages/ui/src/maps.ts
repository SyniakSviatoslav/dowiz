// Map entry point — the ONLY module apps should load map components through.
//
// maplibre-gl (~1MB) is dynamically imported inside MapLibreBase, but the map
// COMPONENT modules themselves used to reach route chunks statically via the
// main barrel (src/index.ts). This side-entry lets consumers `React.lazy` the
// components so neither the component code nor the maplibre chunk is fetched
// until a map actually renders (see docs/security/product-media-OPERATOR-ENABLEMENT.md
// "Bundle: lazy-load the 1MB map").
//
// Built by the package's tsc build to dist/maps.js — no package.json change
// needed (no `exports` field; consumers resolve dist/maps.js directly).
export { MapLibreBase } from './components/molecules/MapLibreBase.js';
export type { LngLatLike } from './components/molecules/MapLibreBase.js';
export { MapWithPin } from './components/molecules/MapWithPin.js';
export { CourierLiveMap } from './components/molecules/CourierLiveMap.js';
export type { CourierOnMap } from './components/molecules/CourierLiveMap.js';
export { MapWithRadius } from './components/molecules/MapWithRadius.js';
