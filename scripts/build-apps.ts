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
    // CJS output: `import.meta.url` is undefined, which crashes code that does
    // fileURLToPath(import.meta.url) at boot (e.g. the static-root dirName). Inject a
    // CJS-safe value from the native __filename so import.meta.url resolves correctly.
    define: { 'import.meta.url': '__IMPORT_META_URL__' },
    banner: { js: "const __IMPORT_META_URL__ = require('url').pathToFileURL(__filename).href;" },
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

  // Migration head: newest migration's basename (no extension). Filenames are
  // fixed-width timestamp-prefixed, so a lexical sort equals a chronological one.
  // Stamped into the API bundle so the boot-time schema guard can detect a DB
  // that is behind this build (see apps/api/src/lib/schema-guard.ts).
  const migrationsSrcDir = path.resolve('packages/db/migrations');
  const migrationFiles = fs
    .readdirSync(migrationsSrcDir)
    .filter((f) => f.endsWith('.ts'))
    .sort();
  if (migrationFiles.length === 0) throw new Error('build-apps: no migrations found');
  const migrationHead = migrationFiles[migrationFiles.length - 1].replace(/\.ts$/, '');

  await Promise.all([
    esbuild.build({
      ...commonOptions,
      entryPoints: ['apps/api/src/server.ts'],
      outfile: 'dist/api/server.cjs',
      define: { ...commonOptions.define, __EXPECTED_MIGRATION_HEAD__: JSON.stringify(migrationHead) },
    }),
    esbuild.build({
      ...commonOptions,
      entryPoints: ['apps/worker/src/index.ts'],
      outfile: 'dist/worker/index.cjs',
    })
  ]);

  console.log(`✅ Apps built and bundled to dist/api and dist/worker (schema head: ${migrationHead})`);

  // Bundled migrator (dist/migrate): the runner + each migration compiled to a
  // standalone .mjs. fly.toml's release_command runs `node dist/migrate/index.cjs`
  // on every deploy. The runtime image needs no node_modules for this — node-pg-migrate
  // is bundled into the runner, and the one migration that uses pg-boss at runtime
  // is bundled self-contained. IMPORTANT: sourcemap MUST stay off for the migration
  // files — node-pg-migrate loads EVERY file in the dir, so a stray .map would break it.
  await Promise.all([
    esbuild.build({
      ...commonOptions,
      entryPoints: migrationFiles.map((f) => path.join('packages/db/migrations', f)),
      outdir: 'dist/migrate/migrations',
      outExtension: { '.js': '.cjs' },
      format: 'cjs', // CJS like the server bundle: one migration bundles pg-boss
      // (a CJS dep doing require('node:events')) which only works in CJS output.
      // node-pg-migrate loads these via import(); cjs-module-lexer still exposes
      // the named up/down exports, and basename-minus-.cjs matches the .ts name.
      sourcemap: false,
    }),
    esbuild.build({
      ...commonOptions,
      entryPoints: ['scripts/migrate-runner.ts'],
      outfile: 'dist/migrate/index.cjs',
      format: 'cjs',
      sourcemap: false,
    }),
  ]);

  console.log(`✅ Migrator bundled to dist/migrate (${migrationFiles.length} migrations + runner)`);

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
