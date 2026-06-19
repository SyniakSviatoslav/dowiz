import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@ui': path.resolve(__dirname, '../../packages/ui/src'),
      '@deliveryos/ui': path.resolve(__dirname, '../../packages/ui/src'),
      '@shared-types': path.resolve(__dirname, '../../packages/shared-types/src'),
      '@deliveryos/shared-types': path.resolve(__dirname, '../../packages/shared-types/src'),
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://localhost:3000',
      '/public': 'http://localhost:3000',
      '/auth': 'http://localhost:3000',
      '^/s/': {
        target: 'http://localhost:3000',
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
