/// <reference types="vite/client" />

interface ImportMetaEnv {
  // Geo-seams — TileSource (see src/lib/tileConfig.ts, docs/adr/ADR-GEO-SEAMS.md).
  readonly VITE_TILE_STYLE_URL?: string;
  readonly VITE_TILE_PROVIDER?: 'free' | 'self';
  // Voice control (ADR-0015) — storefront MicFab render flag, baked at vite build time.
  // Defaults OFF so prod stays DARK; 'true' mounts the read-only voice UI. Launch is separate.
  readonly VITE_VOICE_ENABLED?: string;
  // Telegram Mini App (TMA) detection + theme mapping (see src/lib/tma.ts,
  // docs/design/channel-hub/TMA-VALIDATION.md). Default OFF — dark until validated.
  readonly VITE_TMA_ENABLED?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
