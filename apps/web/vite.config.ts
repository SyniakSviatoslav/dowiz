import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@ui': path.resolve(__dirname, '../../packages/ui/src'),
      // MUST stay ABOVE the broader '@deliveryos/ui' key (aliases match in insertion
      // order): the map side-entry is imported as '@deliveryos/ui/dist/maps.js' so
      // tsc resolves it against the package's built d.ts, while the bundler serves
      // the live source. Without this entry the broad alias would rewrite it to the
      // nonexistent packages/ui/src/dist/maps.js.
      '@deliveryos/ui/dist/maps.js': path.resolve(__dirname, '../../packages/ui/src/maps.ts'),
      '@deliveryos/ui': path.resolve(__dirname, '../../packages/ui/src'),
      '@shared-types': path.resolve(__dirname, '../../packages/shared-types/src'),
      '@deliveryos/shared-types': path.resolve(__dirname, '../../packages/shared-types/src'),
    },
  },
  server: {
    port: 5173,
    proxy: {
      // PROXY_TARGET lets E2E run the local FE against a remote backend (e.g. prod
      // for real data) without a deploy: `VITE_PROXY_TARGET=https://dowiz.fly.dev`.
      '/api': process.env.VITE_PROXY_TARGET || 'http://localhost:3000',
      '/public': process.env.VITE_PROXY_TARGET || 'http://localhost:3000',
      '/auth': process.env.VITE_PROXY_TARGET || 'http://localhost:3000',
      '^/s/': {
        target: process.env.VITE_PROXY_TARGET || 'http://localhost:3000',
        bypass: (req) => req.url || '',
      },
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    rollupOptions: {
      output: {
        // Function form so maplibre's own transitive deps land in the same chunk
        // as maplibre-gl, isolating the ~1MB map library from the main bundle.
        // NOTE: react/react-dom/framer-motion stay grouped in a single `vendor`
        // chunk on purpose — splitting React into its own chunk reorders the
        // module graph and triggers load-order/circular-init issues, so we do
        // NOT split them apart here.
        manualChunks(id) {
          if (!id.includes('node_modules')) return undefined;
          if (id.includes('maplibre-gl') || id.includes('@maplibre') || id.includes('maplibre')) {
            return 'map';
          }
          if (id.includes('/react/') || id.includes('/react-dom/') || id.includes('/framer-motion/')) {
            return 'vendor';
          }
          return undefined;
        },
      },
    },
  },
});
