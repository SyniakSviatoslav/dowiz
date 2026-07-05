// NODE-KEEP generator — the per-route strangler gate (ADR-0022 follow-through).
//
// PROBLEM (staging oracle 2026-07-05): the front-door forwards by SURFACE, but each surface's
// Rust build covers its council-approved core, not every route the corrected 236-route map
// assigns to it. Flipping S3/S5 surface-wide 404'd ~45 mapped-but-unmounted routes (owner
// promotions/settings/brand/import; owner order actions/settlements/messages/plisio webhook).
//
// MECHANISM: this script diffs the Node route map (route-templates.generated.ts — SSOT of what
// Node serves) against the Rust tree's ACTUAL axum `.route()` mounts (the openapi document lists
// unmounted ops by design, so mounts are the only serving-truth), and emits
// apps/api/src/lib/cutover/node-keep.generated.ts: the set of mapped routes the front-door must
// KEEP ON NODE even when their surface is flipped. Re-run whenever the Rust surface grows —
// routes drop out of the keep set as they gain mounts (the strangler shrinks Node one route at
// a time, honestly).
//
// It is ALSO the phantom-path gate: a Rust mount whose path is NOT in the Node map (the cutover
// breaker's C1 class — e.g. `/orders` vs the real `/api/orders`) fails the run. A phantom mount
// is unreachable through the front-door and hides a parity break.
//
// Usage: node scripts/rebuild-cutover/gen-node-keep.mjs [--check]
//   --check: verify the committed generated file is current (CI-able); exit 1 on drift.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const RUST_SRC = path.join(ROOT, 'rebuild', 'crates', 'api', 'src');
const MAP_FILE = path.join(ROOT, 'apps', 'api', 'src', 'lib', 'cutover', 'route-templates.generated.ts');
const OUT_FILE = path.join(ROOT, 'apps', 'api', 'src', 'lib', 'cutover', 'node-keep.generated.ts');

// Deliberate Rust 404s — council-retired ops where the 404 IS the port, never a keep.
const RETIRED = new Set(['POST /api/auth/courier/activate']);
// Rust mounts that are infra/internal, never in the Node map.
const RUST_INFRA = new Set(['/healthz', '/livez', '/openapi.json', '/ws']);
// Map families served by Rust wildcard/segment-parse mounts (S1, live-proven) whose template
// shapes don't string-match the axum patterns.
const WILDCARD_OK = new Set(['/images/*', '/media/*', '/sitemap-locations-:shard.xml']);

function walk(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? walk(path.join(dir, e.name)) : e.name.endsWith('.rs') ? [path.join(dir, e.name)] : []);
}

const norm = (p) => p.replace(/\{\*?[a-zA-Z_]+\}/g, ':x').replace(/:[a-zA-Z_]+/g, ':x').replace(/\*/g, ':x');

// Normalized paths actually mounted in the Rust tree. PATH-level (not method-level) on
// purpose: axum method routers chain/alias in ways a regex can't attribute reliably (the
// method-level attempt false-kept live-proven routes like POST /api/auth/refresh), and
// method-level coverage is already enforced elsewhere (openapi-diff + live probes). A
// mounted path whose method set is short shows up as a 405/404 in the per-surface probe,
// not as silent traffic loss.
const rustPaths = new Map(); // normPath -> raw example (for phantom reporting)
for (const file of walk(RUST_SRC)) {
  const src = fs.readFileSync(file, 'utf8');
  const re = /\.route\(\s*"([^"]+)"/g;
  for (let m; (m = re.exec(src)); ) {
    const np = norm(m[1]);
    if (!rustPaths.has(np)) rustPaths.set(np, { rawPath: m[1], file: path.relative(ROOT, file) });
  }
}

const mapSrc = fs.readFileSync(MAP_FILE, 'utf8');
// Whitespace-tolerant: entries with long flags are pretty-printed across lines.
const entries = [...mapSrc.matchAll(/\{\s*method: '([A-Z]+)',\s*template: '([^']+)',\s*surface: '([^']+)'/g)]
  .map(([, method, template, surface]) => ({ method, template, surface }));
const mapPathSet = new Set(entries.map((e) => norm(e.template)));

// Direction 1 — phantom gate: Rust mounts not in the Node map.
const phantoms = [];
for (const [np, info] of rustPaths) {
  if (RUST_INFRA.has(np) || np.startsWith('/api/media/upload')) continue;
  if (np === '/:x') continue; // sitemap shard segment-parse mount (S1, live-proven)
  if (!mapPathSet.has(np)) phantoms.push(`${info.rawPath}   [${info.file}]`);
}
if (phantoms.length) {
  console.error('PHANTOM Rust mounts (path not in the Node route map — C1 class, unreachable via front-door):');
  for (const p of phantoms) console.error('  ' + p);
  process.exit(1);
}

// Direction 2 — the keep set: mapped flippable routes with no Rust mount.
const keep = [];
for (const { method, template, surface } of entries) {
  if (surface === 'UNMAPPED' || surface === 'INFRA_NEVER_FLIPS' || surface === 'S6') continue;
  if (WILDCARD_OK.has(template)) continue;
  const key = `${method} ${template}`;
  if (RETIRED.has(key)) continue;
  if (rustPaths.has(norm(template))) continue;
  keep.push({ key, surface });
}
keep.sort((a, b) => (a.surface + a.key).localeCompare(b.surface + b.key));

const bySurface = {};
for (const k of keep) bySurface[k.surface] = (bySurface[k.surface] || 0) + 1;

const body = `/**
 * GENERATED by scripts/rebuild-cutover/gen-node-keep.mjs — DO NOT EDIT.
 *
 * Mapped routes whose surface may be flipped to Rust while the route itself has NO Rust
 * mount yet: the front-door keeps these on Node regardless of the surface flag (per-route
 * strangler). Regenerate whenever the Rust surface grows; entries disappearing from this
 * file IS the strangler making progress. Deliberately-retired ops (courier/activate) are
 * excluded — their Rust 404 is the port.
 *
 * Per-surface unmounted counts: ${JSON.stringify(bySurface)}
 */
export const NODE_KEEP_ROUTES: ReadonlySet<string> = new Set([
${keep.map((k) => `  '${k.key}', // ${k.surface}`).join('\n')}
]);
`;

if (process.argv.includes('--check')) {
  const current = fs.existsSync(OUT_FILE) ? fs.readFileSync(OUT_FILE, 'utf8') : '';
  if (current !== body) {
    console.error('node-keep.generated.ts is STALE — run: node scripts/rebuild-cutover/gen-node-keep.mjs');
    process.exit(1);
  }
  console.log('node-keep.generated.ts is current.');
} else {
  fs.writeFileSync(OUT_FILE, body);
  console.log(`wrote ${path.relative(ROOT, OUT_FILE)}: ${keep.length} keep routes`, bySurface);
}
