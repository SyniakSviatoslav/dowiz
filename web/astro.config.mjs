import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';

// Astro + Svelte 5 integration. No TS, no legacy Node/TS wiring.
export default defineConfig({
  integrations: [svelte()],
});
