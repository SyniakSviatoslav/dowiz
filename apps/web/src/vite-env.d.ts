/// <reference types="vite/client" />

interface ImportMetaEnv {
  // Geo-seams — TileSource (see src/lib/tileConfig.ts, docs/adr/ADR-GEO-SEAMS.md).
  readonly VITE_TILE_STYLE_URL?: string;
  readonly VITE_TILE_PROVIDER?: 'free' | 'self';
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

// Map side-entry (perf — lazy ~1MB maplibre chunk, see src/components/maps.tsx).
// At bundle time a dedicated vite alias serves packages/ui/src/maps.ts for this
// specifier; for tsc this ambient declaration types it off the barrel's existing
// declarations, so `pnpm typecheck` never depends on packages/ui/dist/maps.d.ts
// having been built yet (pre-commit typechecks BEFORE it builds).
declare module '@deliveryos/ui/dist/maps.js' {
  export { MapLibreBase, MapWithPin, CourierLiveMap, MapWithRadius } from '@deliveryos/ui';
  export type { LngLatLike, CourierOnMap } from '@deliveryos/ui';
}
