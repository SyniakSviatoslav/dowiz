// THE BOOT GRAPH OF A PAGE: what a phone downloads before the page can run.
//
// From an index.html: every stylesheet, preload and modulepreload it links, every script it
// names, and the closure of STATIC `import`/`export … from` over those scripts. Dynamic
// `import()` is reported apart — it is fetched later, on a tab or a screen, not at boot.
// Absolute http(s) URLs are ignored; a specifier that names a missing file is COUNTED as
// missing, never silently dropped (it is broken online too).
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';

const STATIC_RE = /(?:^|[;\s}])(?:import|export)\s+(?:[\w*{}\s,$]+\s+from\s+)?['"]([^'"]+)['"]/g;
const DYNAMIC_RE = /import\(\s*['"]([^'"]+)['"]\s*\)/g;

/** Strip `//` line and block comments so a commented-out import is not an edge. */
export function stripComments(src) {
  return src.replace(/\/\*[\s\S]*?\*\//g, '').replace(/(^|[^:'"\\])\/\/.*$/gm, '$1');
}

export function importsOf(src) {
  const s = stripComments(src);
  const stat = [...s.matchAll(STATIC_RE)].map(m => m[1]);
  const dyn = [...s.matchAll(DYNAMIC_RE)].map(m => m[1]);
  return { stat, dyn };
}

/** The boot references of an HTML page, in document order. */
export function htmlEntries(html) {
  const out = [];
  const s = html.replace(/<!--[\s\S]*?-->/g, '');
  for (const m of s.matchAll(/<link\b[^>]*>/gi)) {
    const tag = m[0];
    const rel = (tag.match(/rel=["']?([\w-]+)/i) || [])[1];
    const href = (tag.match(/href=["']([^"']+)["']/i) || [])[1];
    if (href && ['stylesheet', 'preload', 'modulepreload'].includes(rel)) out.push(href);
  }
  for (const m of s.matchAll(/<script\b[^>]*\bsrc=["']([^"']+)["'][^>]*>/gi)) out.push(m[1]);
  return out;
}

/** Resolve a specifier against the file that names it; null for URLs and data:. */
export function resolveSpec(pub, fromFile, spec) {
  if (/^(https?:|data:|\/\/)/.test(spec)) return null;
  const clean = spec.split(/[?#]/)[0];
  if (clean.startsWith('/')) return path.join(pub, clean);
  return path.resolve(path.dirname(fromFile), clean);
}

/** The whole boot set of one page. */
export function bootGraph(pub, htmlRel) {
  const html = path.join(pub, htmlRel);
  const files = new Set([html]);
  const missing = new Set();
  const dynamic = new Set();
  const queue = [];
  for (const ref of htmlEntries(fs.readFileSync(html, 'utf8'))) {
    const f = resolveSpec(pub, html, ref);
    if (f) queue.push(f);
  }
  while (queue.length) {
    const f = queue.shift();
    if (files.has(f)) continue;
    if (!fs.existsSync(f)) { missing.add(f); continue; }
    files.add(f);
    if (!f.endsWith('.js') && !f.endsWith('.mjs')) continue;
    const { stat, dyn } = importsOf(fs.readFileSync(f, 'utf8'));
    for (const sp of stat) { const r = resolveSpec(pub, f, sp); if (r) queue.push(r); }
    for (const sp of dyn) { const r = resolveSpec(pub, f, sp); if (r) dynamic.add(r); }
  }
  return { files: [...files].sort(), missing: [...missing].sort(), dynamic: [...dynamic].sort() };
}

/** Raw, gzip -9 and brotli q11 bytes of a list of files, summed. */
export function weigh(files) {
  let raw = 0, gzip = 0, br = 0;
  for (const f of files) {
    const b = fs.readFileSync(f);
    raw += b.length;
    gzip += zlib.gzipSync(b, { level: 9 }).length;
    br += zlib.brotliCompressSync(b, { params: { [zlib.constants.BROTLI_PARAM_QUALITY]: 11 } }).length;
  }
  return { raw, gzip, br };
}

/** Every file under a directory, recursively, with its size; directories whose NAME matches
 * `skip` are not entered (a build tree is not source, and cargo rewrites it while we walk). */
export function walk(dir, skip = null) {
  const out = [];
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) { if (!skip || !skip.test(e.name)) out.push(...walk(p, skip)); }
    else out.push({ path: p, bytes: fs.statSync(p).size });
  }
  return out;
}
