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
      '/courier': {
        target: 'http://localhost:3000',
        bypass: (req) => req.url || '',
      },
      '^/s/': {
        target: 'http://localhost:3000',
        bypass: (req) => req.url || '',
      },
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
});
