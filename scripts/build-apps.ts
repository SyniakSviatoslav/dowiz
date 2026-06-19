import * as esbuild from 'esbuild';
import path from 'path';
import fs from 'fs';

async function build() {
  const commonOptions: esbuild.BuildOptions = {
    bundle: true,
    platform: 'node',
    target: 'node22',
    format: 'cjs',
    minify: false,
    sourcemap: true,
    // We want all workspaces (which are essentially local packages) bundled 
    // into the single file. We mark external generic node_modules that should
    // not be bundled or are native modules.
    external: [
      'pg-native', 
      'fsevents',
      'mock-aws-s3',
      'aws-sdk',
      'nock',
      'argon2',
      'sharp',
      '@aws-sdk/*',
      '@smithy/*'
    ],
  };

  await Promise.all([
    esbuild.build({
      ...commonOptions,
      entryPoints: ['apps/api/src/server.ts'],
      outfile: 'dist/api/server.cjs',
    }),
    esbuild.build({
      ...commonOptions,
      entryPoints: ['apps/worker/src/index.ts'],
      outfile: 'dist/worker/index.cjs',
    })
  ]);

  console.log('✅ Apps built and bundled to dist/api and dist/worker');

  // Assemble the static web root the server serves from (dist/public — the
  // bundled server.cjs lives in dist/api, so its static root resolves to dist/public).
  // Mirrors the Dockerfile COPY steps so `pnpm bundle` alone yields a runnable artifact
  // (and local `node dist/api/server.cjs` serves the SPA, matching prod). (TI-5)
  const distPublic = path.resolve('dist/public');
  fs.mkdirSync(distPublic, { recursive: true });
  const apiPublic = path.resolve('apps/api/public');
  const webDist = path.resolve('apps/web/dist');
  if (fs.existsSync(apiPublic)) fs.cpSync(apiPublic, distPublic, { recursive: true });
  if (!fs.existsSync(path.join(webDist, 'index.html'))) {
    throw new Error(`build-apps: apps/web/dist/index.html missing — run "pnpm -r build" first`);
  }
  fs.cpSync(webDist, distPublic, { recursive: true }); // web overlays api/public
  console.log('✅ Static web root assembled at dist/public (api public + web dist)');
}

build().catch((err) => {
  console.error(err);
  process.exit(1);
});
