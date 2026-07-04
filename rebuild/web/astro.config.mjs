// Astro 5 + Svelte 5 islands config for the DeliveryOS Phase-A storefront spike.
// Self-contained: this project is NOT part of the root pnpm workspace (pnpm-workspace.yaml
// globs apps/*, packages/*, tools/*, spikes/* only — rebuild/web matches none of them).
import { defineConfig, envField } from 'astro/config';
import svelte from '@astrojs/svelte';

export default defineConfig({
  // SSR (server output) so /s/[slug] fetches the menu per-request from the S1 read API —
  // no live fetches happen at BUILD time (see src/pages/s/[slug].astro: fetch only in
  // the per-request `get`/frontmatter path, never at module scope).
  output: 'server',
  integrations: [svelte()],
  security: {
    // storefront embeds itself in iframes for the branding-preview flow (parity with
    // ClientLayout.tsx `?embed=true` / activation iframe) — do not lock frame-ancestors here;
    // that stays a per-tenant CSP concern owned by the API edge (parity note in README).
    checkOrigin: false,
  },
  env: {
    schema: {
      // Base URL of the Rust/Node S1 read API. No hardcoded host — every environment
      // (dev/staging/prod) supplies this at build or run time.
      PUBLIC_API_BASE_URL: envField.string({ context: 'client', access: 'public', default: '/api' }),
    },
  },
  vite: {
    build: {
      // Keep the storefront route's islands measurable in isolation — one that a
      // `du -h`/gzip pass on dist/ can attribute per-chunk (see README bundle-size table).
      rollupOptions: {
        output: {
          manualChunks: undefined,
        },
      },
    },
  },
});
