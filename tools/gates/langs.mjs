// LANGS — ONE LANGUAGE SET, AND EVERY DICTIONARY SPEAKS ALL OF IT (lane W-RU, 2026-09-27).
//
// Research 2026-09-26 Part B found the set written out in 8 JS files, 20 Rust
// sites and 7 tools; adding Russian meant finding every copy by hand, and a
// missed one shows up only when a Russian reader meets an Albanian sentence.
// The set now lives in workers/api/public/lib/langs.js and
// crates/dowiz-hub/src/lang.rs. This gate is what keeps it there. Five rules,
// each line printed names a file:
//
//   set       the JS LANGS and the Rust LANGS are the same list, in order.
//   literal   no list of two or more language codes (['sq', 'en', ...],
//             "enum":["sq",...]) outside the two source files. Tests may pin
//             a list; code must import it.
//   parity    a JS file with N `uk:` object keys has N `ru:` keys (and the
//             same for sq/en); a Rust file with N `"uk" =>` arms has N `"ru" =>`
//             arms. This is what catches a map that is not exported (a
//             recogniser's tag table, an allergen's name) and a match arm.
//   files     a per-language file (notice/uk.rs, apple-words-uk.js,
//             DPA-...uk.md) has a sibling for every language.
//   keys      every dictionary a JS file exports, imported the way the
//             browser imports it, has every language, and every language has
//             every key of the union (qtyWords.* excepted, as in the eval).
//
// `node tools/gates/langs.mjs [ROOT]` prints `langs: <n> violation(s)` last and
// exits 1 when n > 0. The design prototype (public/kit/) is out of scope: it is
// not a product surface (research B1, last row).
import fs from 'node:fs';
import path from 'node:path';
import { register } from 'node:module';
import { pathToFileURL } from 'node:url';

const ROOT = path.resolve(process.argv[2] || path.join(path.dirname(new URL(import.meta.url).pathname), '../..'));
const PUB = path.join(ROOT, 'workers/api/public');
const SRC_JS = 'workers/api/public/lib/langs.js';
const SRC_RS = 'crates/dowiz-hub/src/lang.rs';
const out = [];
const bad = (rule, file, msg) => out.push(`  ${rule.padEnd(7)} ${file}: ${msg}`);

const { LANGS } = await import(pathToFileURL(path.join(ROOT, SRC_JS)).href);
const CODES = LANGS.join('|');

// ── set ────────────────────────────────────────────────────────────────────
const rsSrc = fs.readFileSync(path.join(ROOT, SRC_RS), 'utf8');
const rsSet = (rsSrc.match(/pub const LANGS: \[&str; \d+\] = \[([^\]]*)\]/) || [, ''])[1].match(/"[a-z]+"/g)?.map(s => s.slice(1, -1)) || [];
if (rsSet.join() !== LANGS.join()) bad('set', SRC_RS, `LANGS is [${rsSet}] but ${SRC_JS} says [${LANGS}]`);

// ── the files in scope ─────────────────────────────────────────────────────
const SCOPE = ['workers/api/public', 'workers/api/src', 'crates/dowiz-hub/src', 'tools/learn', 'tools/evals', 'docs/privacy'];
const isTest = f => /\.test\.m?js$|(^|\/)tests\.rs$|\/tests\/|\.prove\.|_test\.|\/test_/.test(f);
function walk(rel, acc = []){
  const abs = path.join(ROOT, rel);
  if (!fs.existsSync(abs)) return acc;
  for (const e of fs.readdirSync(abs, { withFileTypes: true })) {
    const r = path.posix.join(rel, e.name);
    if (e.isDirectory()) { if (!['node_modules', 'kit', 'target'].includes(e.name)) walk(r, acc); }
    else acc.push(r);
  }
  return acc;
}
const files = SCOPE.flatMap(d => walk(d)).filter(f => /\.(m?js|rs|html|md)$/.test(f));
const code = files.filter(f => !isTest(f) && !f.endsWith('.md'));

/// Comments out: a code in a comment is prose, not a list.
const strip = (src, f) => f.endsWith('.html') ? src.replace(/<!--[\s\S]*?-->/g, '')
  : src.replace(/\/\*[\s\S]*?\*\//g, '').replace(/(^|[^:'"\\])\/\/.*$/gm, '$1');

// ── literal ────────────────────────────────────────────────────────────────
const LIST = new RegExp(`(["'])(${CODES})\\1\\s*,\\s*(["'])(${CODES})\\3`, 'g');
for (const f of code) {
  if (f === SRC_JS || f === SRC_RS) continue;
  const src = strip(fs.readFileSync(path.join(ROOT, f), 'utf8'), f);
  src.split('\n').forEach((line, i) => {
    if (LIST.test(line)) bad('literal', `${f}:${i + 1}`, `a list of language codes: ${line.trim().slice(0, 90)}`);
    LIST.lastIndex = 0;
  });
}

// ── parity ─────────────────────────────────────────────────────────────────
const count = (src, re) => (src.match(re) || []).length;
const jsKey = l => new RegExp(`(^|[\\s{,(])${l}\\s*:(?!:)`, 'g');
const rsArm = l => new RegExp(`"${l}"\\s*(=>|\\|)|\\("${l}",|starts_with\\("${l}"\\)`, 'g');
const dictFiles = [];
for (const f of code) {
  const src = strip(fs.readFileSync(path.join(ROOT, f), 'utf8'), f);
  const key = f.endsWith('.rs') ? rsArm : f.endsWith('.html') ? (l => new RegExp(`data-lang="${l}"`, 'g')) : jsKey;
  const n = Object.fromEntries(LANGS.map(l => [l, count(src, key(l))]));
  const most = Math.max(...Object.values(n));
  // Rust may leave ONE language to its `_ =>` arm (the default); every other
  // language is written out as often as the most-written one.
  const absent = f.endsWith('.rs') ? LANGS.filter(l => n[l] === 0) : [];
  const short = LANGS.filter(l => n[l] !== most && !(absent.length === 1 && absent[0] === l));
  if (most > 0 && short.length) {
    bad('parity', f, LANGS.map(l => `${l}=${n[l]}`).join(' ') + (f.endsWith('.rs') ? ' (one language may be the `_` arm; the rest as often as the most)' : ' (every language as often as the most)'));
  }
  if (/\.m?js$/.test(f) && f.startsWith('workers/api/public/') && n.uk > 0) dictFiles.push(f);
}

// ── files ──────────────────────────────────────────────────────────────────
const PER = new RegExp(`(^|[-_.])(${CODES})(\\.[a-z]+)$`);
const seenSets = new Set();
for (const f of files) {
  const m = path.basename(f).match(PER);
  if (!m) continue;
  const dir = path.dirname(f), stem = path.basename(f).slice(0, m.index + m[1].length);
  const id = `${dir}/${stem}*${m[3]}`;
  if (seenSets.has(id)) continue;
  seenSets.add(id);
  // A SET is two or more languages in files of one stem; one file alone is a
  // language kept apart from a shared table (admin/i18n-ru.js), not a set.
  if (LANGS.filter(l => files.includes(`${dir}/${stem}${l}${m[3]}`)).length < 2) continue;
  for (const l of LANGS) {
    const sib = `${dir}/${stem}${l}${m[3]}`;
    if (!files.includes(sib)) bad('files', sib, `missing, while ${stem}${m[2]}${m[3]} exists`);
  }
}

// ── keys ───────────────────────────────────────────────────────────────────
const EXCLUDE = /^qtyWords(\.|$)/;
function flatten(obj, prefix = ''){
  const acc = [];
  for (const [k, v] of Object.entries(obj)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === 'object' && !Array.isArray(v)) acc.push(...flatten(v, key)); else acc.push(key);
  }
  return acc;
}
/// Objects keyed by language whose values are word tables (objects), found in
/// an export or one level inside it.
function dicts(mod){
  const found = [];
  const look = (v, name, depth) => {
    if (!v || typeof v !== 'object' || Array.isArray(v) || depth > 1) return;
    const langKeys = Object.keys(v).filter(k => LANGS.includes(k));
    if (langKeys.length >= 2 && langKeys.every(k => v[k] && typeof v[k] === 'object' && !Array.isArray(v[k]))) found.push([name, v]);
    else for (const [k, w] of Object.entries(v)) look(w, `${name}.${k}`, depth + 1);
  };
  for (const [k, v] of Object.entries(mod)) look(v, k, 0);
  return found;
}
function stubBrowser(g = globalThis){
  const mem = new Map();
  const store = { getItem: k => (mem.has(k) ? mem.get(k) : null), setItem: (k, v) => mem.set(k, String(v)), removeItem: k => mem.delete(k) };
  const el = { setAttribute(){}, addEventListener(){}, classList: { add(){}, remove(){}, toggle(){} }, style: {}, dataset: {} };
  const defs = { localStorage: store, sessionStorage: store, navigator: { language: 'en', languages: ['en'] },
    location: { href: 'https://x.dowiz.org/', search: '', hash: '', hostname: 'x.dowiz.org', pathname: '/' },
    document: { documentElement: el, body: el, querySelector: () => null, querySelectorAll: () => [], getElementById: () => ({ ...el, querySelectorAll: () => [], querySelector: () => null, appendChild(){}, append(){} }), addEventListener(){}, dispatchEvent(){}, createElement: () => el, title: '', readyState: 'complete' },
    MutationObserver: class { observe(){} disconnect(){} }, CustomEvent: class { constructor(t, o){ this.type = t; this.detail = o?.detail; } },
    addEventListener(){}, matchMedia: () => ({ matches: false, addEventListener(){} }) };
  for (const [k, v] of Object.entries(defs)) if (g[k] === undefined) Object.defineProperty(g, k, { value: v, configurable: true, writable: true });
  if (g.window === undefined) Object.defineProperty(g, 'window', { value: g, configurable: true, writable: true });
}
stubBrowser();
const pubUrl = pathToFileURL(PUB + '/').href;
register('data:text/javascript,' + encodeURIComponent(`const P = ${JSON.stringify(pubUrl)};
export async function resolve(spec, c, next){
  if (spec.startsWith('/') && c.parentURL && c.parentURL.startsWith(P)) return next(P + spec.slice(1), c);
  return next(spec, c);
}`));
// Two passes: every module first, because a sub-dictionary merges its words
// into the console's table at import, and the table is checked whole.
const mods = [];
for (const f of dictFiles) {
  try { mods.push([f, await import(pathToFileURL(path.join(ROOT, f)).href)]); }
  catch (e) { bad('keys', f, `does not import in node: ${String(e.message).split('\n')[0].slice(0, 80)}`); }
}
for (const [f, mod] of mods) {
  for (const [name, d] of dicts(mod)) {
    const keys = Object.fromEntries(LANGS.map(l => [l, new Set(d[l] ? flatten(d[l]).filter(k => !EXCLUDE.test(k)) : [])]));
    const union = new Set(LANGS.flatMap(l => [...keys[l]]));
    for (const l of LANGS) {
      if (!d[l]) { bad('keys', f, `${name} has no ${l}`); continue; }
      const miss = [...union].filter(k => !keys[l].has(k));
      if (miss.length) bad('keys', f, `${name}.${l} lacks ${miss.length}: ${miss.slice(0, 5).join(', ')}`);
    }
  }
}

for (const l of out) console.log(l);
console.log(`langs: ${out.length} violation(s); set [${LANGS}], ${code.length} code files, ${dictFiles.length} dictionaries imported`);
process.exit(out.length ? 1 : 0);
