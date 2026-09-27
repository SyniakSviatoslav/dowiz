// SUITE ci · I18N: every surface's dictionary, imported the way the browser imports it, keys
// flattened per language, `qtyWords.*` excluded (number words are MEANT to differ per language).
// missing = keys in the union a language does not have. Rule: 0 for every language of
// workers/api/public/lib/langs.js (§B.2 UX row; ru added by lane W-RU 2026-09-27).
import { register } from 'node:module';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { ind } from '../rules.mjs';
import { LANGS } from '../../../workers/api/public/lib/langs.js';

export { LANGS };
export const DICTS = {
  store: 'store/i18n.js',
  admin: 'admin/i18n.js',
  room: 'room/i18n.js',
  courier: 'courier/i18n.js',
};
export const EXCLUDE = /^qtyWords(\.|$)/;

/** Dotted leaf keys of a nested object. Arrays and functions are leaves. */
export function flatten(obj, prefix = '') {
  const out = [];
  for (const [k, v] of Object.entries(obj)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === 'object' && !Array.isArray(v)) out.push(...flatten(v, key));
    else out.push(key);
  }
  return out;
}

/** The export that is a dictionary: an object with every language as a key. */
export function pickDict(mod) {
  const cands = [mod.T, ...Object.values(mod)];
  return cands.find(v => v && typeof v === 'object' && LANGS.every(l => v[l] && typeof v[l] === 'object')) || null;
}

/** Per language: key count and the keys the union has and it lacks. */
export function completeness(dict) {
  const keys = Object.fromEntries(LANGS.map(l => [l, new Set(flatten(dict[l]).filter(k => !EXCLUDE.test(k)))]));
  const union = new Set(LANGS.flatMap(l => [...keys[l]]));
  return {
    union: union.size,
    per: Object.fromEntries(LANGS.map(l => [l, { keys: keys[l].size, missing: [...union].filter(k => !keys[l].has(k)) }])),
  };
}

/** The browser globals a dictionary module may touch at import time, stubbed and inert. */
export function stubBrowser(g = globalThis) {
  const mem = new Map();
  const store = { getItem: k => (mem.has(k) ? mem.get(k) : null), setItem: (k, v) => mem.set(k, String(v)), removeItem: k => mem.delete(k) };
  const el = { setAttribute() {}, addEventListener() {}, classList: { add() {}, remove() {}, toggle() {} }, style: {}, dataset: {} };
  const defs = {
    localStorage: store,
    sessionStorage: store,
    navigator: { language: 'en', languages: ['en'] },
    location: { href: 'https://x.dowiz.org/', search: '', hostname: 'x.dowiz.org', pathname: '/' },
    document: { documentElement: el, body: el, querySelector: () => null, querySelectorAll: () => [], addEventListener() {}, createElement: () => el, title: '' },
  };
  for (const [k, v] of Object.entries(defs)) {
    if (g[k] === undefined) Object.defineProperty(g, k, { value: v, configurable: true, writable: true });
  }
  if (g.window === undefined) Object.defineProperty(g, 'window', { value: g, configurable: true, writable: true });
}

/**
 * The page imports `/store/storage.js`: absolute from the SERVED root, which node reads as the
 * filesystem root. This resolve hook maps a `/…` specifier from a file under `pub` onto `pub`.
 */
export function rootHook(pubUrl) {
  return 'data:text/javascript,' + encodeURIComponent(`
const P = ${JSON.stringify(pubUrl)};
export async function resolve(spec, c, next) {
  if (spec.startsWith('/') && c.parentURL && c.parentURL.startsWith(P)) return next(P + spec.slice(1), c);
  return next(spec, c);
}`);
}

const hooked = new Set();
export function serveFrom(pub) {
  const url = pathToFileURL(pub + '/').href;
  if (!hooked.has(url)) { register(rootHook(url)); hooked.add(url); }
}

export async function collect(ctx) {
  stubBrowser();
  const pub = path.join(ctx.root, 'workers/api/public');
  serveFrom(pub);
  const out = [];
  for (const [name, rel] of Object.entries(DICTS)) {
    const where = `import public/${rel}, flatten each language (qtyWords.* excluded)`;
    let dict = null;
    let err = '';
    try {
      dict = pickDict(await import(pathToFileURL(path.join(pub, rel)).href));
      if (!dict) err = `no export holds ${LANGS.join(', ')}`;
    } catch (e) {
      err = e.message;
    }
    if (!dict) {
      out.push(ind(`i18n.${name}.importable`, 0, 'bool', 'min', where, { limit: 1, note: err }));
      continue;
    }
    const c = completeness(dict);
    out.push(ind(`i18n.${name}.keys`, c.union, 'keys', 'trend', where));
    for (const l of LANGS) {
      const m = c.per[l].missing;
      out.push(ind(`i18n.${name}.missing_${l}`, m.length, 'keys', 'zero', where, m.length ? { note: m.slice(0, 6).join(', ') } : {}));
    }
  }
  return out;
}
