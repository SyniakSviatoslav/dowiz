import { z } from 'zod';

// TileSource seam config (frontend). The map style URL and provider come from env,
// never hardcoded — see docs/adr/ADR-GEO-SEAMS.md. Swapping the free vector provider
// for a self-hosted tileserver-gl / Protomaps in `fra` is one env change
// (VITE_TILE_STYLE_URL), zero code. MapLibreBase consumes getTileConfig() in G3.
//
// The default equals MapLibreBase's current hardcoded style, so wiring this in later
// is behavior-neutral. Absent vars fall back to the default; an INVALID value (bad
// URL, unknown provider) fails fast at module load.

const DEFAULT_STYLE_URL = 'https://tiles.openfreemap.org/styles/liberty';

const TileConfigSchema = z.object({
  styleUrl: z.string().url().default(DEFAULT_STYLE_URL),
  provider: z.enum(['free', 'self']).default('free'),
});

export type TileConfig = z.infer<typeof TileConfigSchema>;

export function getTileConfig(): TileConfig {
  // import.meta.env is Vite-injected at build time; guard so the module is also
  // importable in a plain Node test context.
  const env: Record<string, string | undefined> =
    (typeof import.meta !== 'undefined' && (import.meta as any).env) || {};
  return TileConfigSchema.parse({
    styleUrl: env.VITE_TILE_STYLE_URL || undefined,
    provider: env.VITE_TILE_PROVIDER || undefined,
  });
}

export const TILE_CONFIG: TileConfig = getTileConfig();
