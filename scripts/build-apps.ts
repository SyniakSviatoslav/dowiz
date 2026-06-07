import * as esbuild from 'esbuild';
import path from 'path';

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
}

build().catch((err) => {
  console.error(err);
  process.exit(1);
});
