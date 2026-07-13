import path from 'path';
import fs from 'fs';

// Build the deliverable artifact for the decentralized app shell.
//
// Root-cause note (2026-07-13, MANIFESTO/DECISIONS D1): the legacy
// centralized server (apps/api + apps/worker + packages/db migrations, deployed
// via attic/fly.toml → dist/api/server.cjs, dist/worker, dist/migrate) was
// DROPPED. There is no server process, no central DB, no Supabase, no Fly in
// the decentralized protocol (bebop2 peer nodes own their local SQLite). The
// only thing this repo ships is the static SPA (apps/web/dist), which the
// thin client / reference alt-client loads. We do NOT emit a fake "built
// dist/api" success over a dead pipeline — that was the false-green the
// red-team flagged. If a backend entrypoint ever returns (un-quarantine),
// that is a deliberate operator decision and the bundling can be re-added then.
async function build() {
  const distPublic = path.resolve('dist/public');
  fs.mkdirSync(distPublic, { recursive: true });

  const webDist = path.resolve('apps/web/dist');
  if (!fs.existsSync(path.join(webDist, 'index.html'))) {
    throw new Error(
      'build-apps: apps/web/dist/index.html missing — run "pnpm -r build" first',
    );
  }
  fs.cpSync(webDist, distPublic, { recursive: true });

  // Any stale backend artifacts from before the D1 quarantine must NOT survive
  // (fail-closed: an empty dist/api must mean "no server", not "last build's
  // server"). Purge them so a static-only deploy can never ship old server code.
  for (const stale of ['dist/api', 'dist/worker', 'dist/migrate']) {
    const p = path.resolve(stale);
    if (fs.existsSync(p)) {
      fs.rmSync(p, { recursive: true, force: true });
      console.warn(`⚠️  build-apps: purged stale ${stale} (centralized server dropped per D1).`);
    }
  }

  console.log('✅ Static SPA assembled at dist/public (apps/web/dist). No backend bundle — centralized server dropped per MANIFESTO D1.');
}

build().catch((err) => {
  console.error(err);
  process.exit(1);
});
