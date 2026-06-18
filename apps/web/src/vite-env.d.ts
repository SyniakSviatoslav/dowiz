/// <reference types="vite/client" />

interface ImportMetaEnv {
  // Geo-seams — TileSource (see src/lib/tileConfig.ts, docs/adr/ADR-GEO-SEAMS.md).
  readonly VITE_TILE_STYLE_URL?: string;
  readonly VITE_TILE_PROVIDER?: 'free' | 'self';
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
