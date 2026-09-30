#!/usr/bin/env node
// ICONS (W-VERIFY, 2026-09-30): every icon name the browser code asks for is
// one /lib/icons.css draws. A name it does not have renders as a solid grey
// square -- no error, no 404, nothing in any log. Third time in this repo:
// `apron` and `shield-check` (more.js), then `chef-hat` and `scale` (the ПФ
// screens, seen at 390 px on qa-durres).
//
//   node tools/gates/icons.mjs [ROOT]    exit 1 naming each missing icon and where
//
// Names are read from `icon('x')`, `icon: 'x'`, `ic: 'x'`, `leading: 'x'` and the
// `*_ICON` tables; a name built at runtime is not seen (say so in review).
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.argv[2] || path.resolve(path.dirname(new URL(import.meta.url).pathname), '../..');
const PUB = path.join(ROOT, 'workers/api/public');
const css = fs.readFileSync(path.join(PUB, 'lib/icons.css'), 'utf8');
const have = new Set([...css.matchAll(/^\.ti-([a-z0-9-]+)\s*\{/gm)].map(m => m[1]));
if (have.size < 20) { console.log(`icons: read ${have.size} icons from lib/icons.css -- the parse broke`); process.exit(2); }

const PATS = [/\bicon\(\s*'([a-z0-9-]+)'/g, /\bicon:\s*'([a-z0-9-]+)'/g, /\bic:\s*'([a-z0-9-]+)'/g, /_ICON\s*=\s*'([a-z0-9-]+)'/g];
const TABLE = /_ICON\s*=\s*\{([^}]*)\}/g;
const missing = [];
let seen = 0;
// The surfaces that draw `ti ti-<name>` masks from lib/icons.css. The kit
// (kit/) has its own sprite and its own names, and is not this gate's.
const SCOPE = ['admin', 'store', 'room', 'courier', 'platform', 'lib'];
const walk = d => {
  for (const e of fs.readdirSync(d, { withFileTypes: true })) {
    const p = path.join(d, e.name);
    if (e.isDirectory()) { if (!/vendor|node_modules|\/lib\/map$/.test(p)) walk(p); continue; }
    if (!/\.js$/.test(e.name)) continue;
    if (!SCOPE.includes(path.relative(PUB, p).split(path.sep)[0])) continue;
    const src = fs.readFileSync(p, 'utf8');
    const names = [];
    for (const re of PATS) for (const m of src.matchAll(re)) names.push([m[1], m.index]);
    for (const t of src.matchAll(TABLE)) for (const m of t[1].matchAll(/:\s*'([a-z0-9-]+)'/g)) names.push([m[1], t.index]);
    for (const [n, at] of names) {
      seen++;
      if (!have.has(n)) missing.push(`${path.relative(PUB, p)}:${src.slice(0, at).split('\n').length} ${n}`);
    }
  }
};
walk(PUB);
if (seen < 50) { console.log(`icons: found only ${seen} icon uses -- the parse broke`); process.exit(2); }
if (missing.length) { console.log(`icons: ${missing.length} icon(s) lib/icons.css does not draw (a grey square):`); missing.forEach(m => console.log('  ' + m)); process.exit(1); }
console.log(`icons: ${seen} icon uses, all ${have.size} drawable names cover them`);
