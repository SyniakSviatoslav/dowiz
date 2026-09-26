// SUITE ci · STATIC: boot bytes per surface, the served tree, unreferenced assets, the offline
// shells, the poll cadences and the route table (§B.2 Performance/UX/Correctness rows).
// No network, no build: it reads workers/api/public and workers/api/src/lib.rs.
import fs from 'node:fs';
import path from 'node:path';
import { bootGraph, importsOf, resolveSpec, stripComments, walk, weigh } from './graph.mjs';
import { ind } from '../rules.mjs';

export const SURFACES = {
  store: 'store/index.html',
  admin: 'admin/index.html',
  room: 'room/index.html',
  courier: 'courier/index.html',
  landing: 'platform/index.html',
};

/** Poll cadences and where they are written; each must equal its documented value. */
export const POLLS = [
  ['poll.console_ms', 'admin/core.js', /export const POLL_MS = ([\d_]+)/],
  ['poll.console_idle_ms', 'admin/core.js', /export const POLL_IDLE_MS = ([\d_]+)/],
  ['poll.room_ms', 'room/app.js', /const POLL_MS = ([\d_]+)/],
  ['poll.track_ms', 'store/track.js', /const POLL_MS = ([\d_]+)/],
  ['poll.track_slow_ms', 'store/track.js', /const POLL_SLOW_MS = ([\d_]+)/],
  ['poll.courier_shift_ms', 'courier/app.js', /S\.onShift \? ([\d_]+) :/],
  ['poll.courier_off_ms', 'courier/app.js', /S\.onShift \? [\d_]+ : ([\d_]+)/],
];

export const SHELLS = ['sw.js', 'room/sw.js', 'courier/sw.js'];
/** The Worker serves the storefront page at `/` (lib.rs serve_root). */
export const PAGES = { '/': 'store/index.html' };

/** String literals inside every `const SHELL… = [ … ]` array of a service worker. */
export function shellList(src) {
  const out = [];
  for (const m of stripComments(src).matchAll(/const SHELL\w*\s*=\s*\[([\s\S]*?)\]/g)) {
    for (const s of m[1].matchAll(/['"]([^'"]+)['"]/g)) out.push(s[1]);
  }
  return out;
}

/** Files a shell must hold offline that it does not list (the sw-shell gate's count, per shell). */
export function shellMissing(pub, swRel) {
  const listed = shellList(fs.readFileSync(path.join(pub, swRel), 'utf8'));
  const page = p => PAGES[p] ?? (p.endsWith('/') ? p.slice(1) + 'index.html' : p.slice(1));
  const have = new Set(listed.map(p => path.join(pub, page(p))));
  const need = new Set();
  for (const p of listed) {
    if (p.endsWith('/') || p.endsWith('.html')) {
      const rel = page(p);
      if (!fs.existsSync(path.join(pub, rel))) continue;
      for (const f of bootGraph(pub, rel).files) if (/\.(m?js|css)$/.test(f)) need.add(f);
    } else if (/\.m?js$/.test(p)) {
      const f = path.join(pub, p);
      if (!fs.existsSync(f)) continue;
      for (const sp of importsOf(fs.readFileSync(f, 'utf8')).stat) {
        const r = resolveSpec(pub, f, sp);
        if (r) need.add(r);
      }
    }
  }
  return [...need].filter(f => !have.has(f)).map(f => '/' + path.relative(pub, f)).sort();
}

/** Served files whose path or name no text in the tree (public + the Worker's Rust) mentions. */
export function unreferenced(pub, srcDir, bootSets) {
  const all = walk(pub);
  const texts = all.filter(f => /\.(html|js|mjs|css|json|webmanifest|txt|svg)$|_headers$/.test(f.path))
    .map(f => fs.readFileSync(f.path, 'utf8'));
  for (const f of walk(srcDir)) if (f.path.endsWith('.rs')) texts.push(fs.readFileSync(f.path, 'utf8'));
  const corpus = texts.join('\n');
  const inBoot = new Set(bootSets.flat());
  return all.filter(f => {
    if (inBoot.has(f.path)) return false;
    const rel = '/' + path.relative(pub, f.path);
    const base = path.basename(f.path);
    if (base === 'index.html' || base === '_headers') return false;
    return !corpus.includes(rel) && !corpus.includes(base);
  });
}

export function pollValues(pub) {
  return POLLS.map(([id, rel, re]) => {
    const m = fs.readFileSync(path.join(pub, rel), 'utf8').match(re);
    return [id, rel, m ? Number(m[1].replace(/_/g, '')) : null];
  });
}

export function routeTable(libRs) {
  return [...fs.readFileSync(libRs, 'utf8').matchAll(/_async\("(\/api\/[^"]*)"/g)].map(m => m[1]);
}

export async function collect(ctx) {
  const pub = path.join(ctx.root, 'workers/api/public');
  const src = path.join(ctx.root, 'workers/api/src');
  const out = [];
  const boots = [];
  for (const [name, rel] of Object.entries(SURFACES)) {
    const g = bootGraph(pub, rel);
    boots.push(g.files);
    const w = weigh(g.files);
    const where = `graph.mjs over public/${rel}`;
    out.push(ind(`surfaces.${name}.files`, g.files.length, 'files', 'ratchet', where));
    out.push(ind(`surfaces.${name}.raw`, w.raw, 'bytes', 'ratchet', where));
    out.push(ind(`surfaces.${name}.gzip`, w.gzip, 'bytes', 'ratchet', where));
    out.push(ind(`surfaces.${name}.brotli`, w.br, 'bytes', 'ratchet', where));
    out.push(ind(`surfaces.${name}.missing`, g.missing.length, 'files', 'zero', where,
      g.missing.length ? { note: g.missing.map(f => path.relative(pub, f)).join(', ') } : {}));
  }
  const tree = walk(pub);
  out.push(ind('served.files', tree.length, 'files', 'ratchet', 'find workers/api/public -type f'));
  out.push(ind('served.bytes', tree.reduce((n, f) => n + f.bytes, 0), 'bytes', 'ratchet', 'find workers/api/public -type f'));
  const dead = unreferenced(pub, src, boots);
  const deadBytes = dead.reduce((n, f) => n + f.bytes, 0);
  const tops = [...new Set(dead.map(f => path.relative(pub, f.path).split('/')[0]))].slice(0, 8).join(', ');
  out.push(ind('served.unreferenced_bytes', deadBytes, 'bytes', 'ratchet',
    'served files no html/js/css/rs text names (by path or basename)', { note: `${dead.length} files; top dirs: ${tops}` }));
  let missing = 0;
  const notes = [];
  for (const sw of SHELLS) {
    const m = shellMissing(pub, sw);
    missing += m.length;
    if (m.length) notes.push(`${sw}: ${m.slice(0, 5).join(' ')}`);
  }
  out.push(ind('ux.shell_missing', missing, 'files', 'zero', 'SHELL lists vs static import graph', notes.length ? { note: notes.join('; ') } : {}));
  for (const [id, rel, v] of pollValues(pub)) out.push(ind(id, v, 'ms', 'exact', `public/${rel}`));
  const tabs = fs.readdirSync(path.join(pub, 'admin')).filter(f => f.endsWith('.js'))
    .map(f => path.join(pub, 'admin', f)).filter(f => !boots[1].includes(f));
  out.push(ind('ux.console_tab_bytes', weigh(tabs).raw, 'bytes', 'ratchet', 'admin/*.js outside the console boot graph',
    { note: `${tabs.length} lazy chunks` }));
  out.push(ind('routes.count', routeTable(path.join(src, 'lib.rs')).length, 'routes', 'exact', 'lib.rs _async("/api/…")'));
  return out;
}
