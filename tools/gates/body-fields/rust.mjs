// THE RUST SIDE of body-fields: every POST/PUT route of the Worker router, the
// handler behind it, the struct its body is parsed into, and the JSON keys that
// struct accepts (rename_all / rename / alias / flatten / default applied).
// Regex over source, on purpose: no build, no toolchain, runs in a second.
import fs from 'node:fs';
import path from 'node:path';

/// Comments blanked (offsets kept), so a doc comment never looks like code.
export function stripComments(src){
  let out = '', i = 0;
  while (i < src.length){
    if (src.startsWith('//', i)){ const j = src.indexOf('\n', i); const e = j < 0 ? src.length : j; out += ' '.repeat(e - i); i = e; }
    else if (src[i] === '"'){ let j = i + 1; while (j < src.length && src[j] !== '"') j += src[j] === '\\' ? 2 : 1; out += src.slice(i, j + 1); i = j + 1; }
    else { out += src[i]; i++; }
  }
  return out;
}

function matchBrace(s, b){ let d = 0; for (let i = b; i < s.length; i++){ if (s[i] === '{') d++; else if (s[i] === '}'){ d--; if (!d) return i; } } return s.length; }

const CASE = {
  camelCase: n => n.replace(/_([a-z0-9])/g, (_, c) => c.toUpperCase()),
  lowercase: n => n.toLowerCase(),
  snake_case: n => n,
  kebab: n => n.replace(/_/g, '-'),
};

/// Every struct in one file: name -> {deny, fields: [{keys, ty, flatten}], open}.
export function structsOf(src){
  const s = stripComments(src), out = {};
  const re = /((?:#\[[^\]]*\]\s*)*)(?:pub(?:\([^)]*\))?\s+)?(struct|enum)\s+(\w+)(?:<[^>{]*>)?\s*\{/g;
  let m;
  while ((m = re.exec(s))){
    const attrs = m[1], kind = m[2], name = m[3];
    const b = m.index + m[0].length - 1, e = matchBrace(s, b), body = s.slice(b + 1, e);
    const serde = [...attrs.matchAll(/#\[serde\(([^\]]*)\)\]/g)].map(x => x[1]).join(',');
    const derives = /Deserialize/.test(attrs);
    const ra = (serde.match(/rename_all\s*=\s*"(\w+)"/) || [])[1];
    const conv = ra ? (CASE[ra] || (n => n)) : (n => n);
    const deny = /deny_unknown_fields/.test(serde);
    if (kind === 'enum'){ out[name] = { name, kind, deny, derives, enum: true, tag: (serde.match(/tag\s*=\s*"(\w+)"/) || [])[1], fields: [] }; continue; }
    const fields = [];
    // Fields at depth 0 of the body, each with the attributes above it.
    let depth = 0, cur = '';
    for (const ch of body){
      if ('<([{'.includes(ch)) depth++;
      if ('>)]}'.includes(ch)) depth--;
      if (ch === ',' && depth === 0){ fields.push(cur); cur = ''; } else cur += ch;
    }
    if (cur.trim()) fields.push(cur);
    const parsed = [];
    for (const f of fields){
      const fa = [...f.matchAll(/#\[serde\(([^\]]*)\)\]/g)].map(x => x[1]).join(',');
      const decl = f.replace(/#\[[^\]]*\]/g, '').trim().match(/^(?:pub(?:\([^)]*\))?\s+)?(?:r#)?(\w+)\s*:\s*([\s\S]+)$/);
      if (!decl) continue;
      if (/\bskip\b|skip_deserializing/.test(fa)) continue;
      const ren = (fa.match(/rename\s*=\s*"([^"]+)"/) || [])[1];
      const keys = [ren || conv(decl[1]), ...[...fa.matchAll(/alias\s*=\s*"([^"]+)"/g)].map(x => x[1])];
      parsed.push({ keys, ty: decl[2].trim(), flatten: /\bflatten\b/.test(fa), optional: /default/.test(fa) || /^Option</.test(decl[2].trim()) });
    }
    out[name] = { name, kind, deny, derives, fields: parsed };
  }
  return out;
}

/// The router: [{method, path, handler}] from lib.rs.
export function routesOf(libSrc){
  const out = [];
  // W-COV (2026-09-30): a route is also `|r, c| edge::run(r, c, handler)` -- the route seam's
  // adapter (`workers/api/src/edge.rs`); the handler is the third argument.
  for (const m of stripComments(libSrc).matchAll(/\.(post|put|patch)_async\(\s*"([^"]+)"\s*,\s*(?:\|\s*\w+\s*,\s*\w+\s*\|\s*edge::run\(\s*\w+\s*,\s*\w+\s*,\s*)?([\w:]+)/g))
    out.push({ method: m[1].toUpperCase(), path: m[2], handler: m[3] });
  return out;
}

/// `services::operations::preps::set_prep` -> the file that holds `set_prep`.
export function fileOf(src, handler){
  const parts = handler.split('::'), fn = parts.pop();
  const cands = [];
  for (let k = parts.length; k >= 0; k--){
    const p = parts.slice(0, k).join('/');
    if (p) cands.push(path.join(src, p + '.rs'), path.join(src, p, 'mod.rs'));
  }
  for (const c of cands){ if (fs.existsSync(c) && new RegExp(`fn\\s+${fn}\\b`).test(fs.readFileSync(c, 'utf8'))) return { file: c, fn }; }
  // `pub use` re-exports: search the module directory.
  const dir = path.join(src, parts.join('/'));
  if (fs.existsSync(dir)) for (const f of walk(dir)) if (new RegExp(`fn\\s+${fn}\\b`).test(fs.readFileSync(f, 'utf8'))) return { file: f, fn };
  return { file: null, fn };
}

export function* walk(d){
  for (const e of fs.readdirSync(d, { withFileTypes: true })){
    const p = path.join(d, e.name);
    if (e.isDirectory()) yield* walk(p); else if (p.endsWith('.rs') && !p.endsWith('tests.rs') && !p.includes('/tests/')) yield p;
  }
}

/// The body of `fn name` in `src` (comment-stripped), or null.
export function fnBody(src, name){
  const s = stripComments(src), m = new RegExp(`fn\\s+${name}\\b[^{;]*\\{`).exec(s);
  if (!m) return null;
  const b = m.index + m[0].length - 1;
  return s.slice(b, matchBrace(s, b) + 1);
}

const PARSE = /let\s+(?:mut\s+)?\w+\s*:\s*([\w:]+)\s*=\s*(?:match\s+)?[^;]{0,60}?(?:body::(?:parse|strict)|serde_json::from_(?:str|value|slice)\(\s*&?\s*(\w+))|body::(?:parse|strict)::<([\w:]+)>/g;

/// The typed bodies a handler parses (following one level of local helper calls).
/// A `serde_json::from_*` counts only when its argument is the request's own
/// text or the Value parsed from it -- not a stored row read back.
export function bodyTypes(fileSrc, fn, depth = 0){
  const body = fnBody(fileSrc, fn);
  if (!body) return [];
  const raw = new Set([...body.matchAll(/let\s+(?:mut\s+)?(\w+)\s*(?::\s*[\w:<>]+)?\s*=\s*(?:match\s+)?(?:req\.text\(|crate::body::parse|body::parse)/g)].map(m => m[1]));
  const types = [...body.matchAll(PARSE)].filter(m => !m[2] || raw.has(m[2]))
    .map(m => (m[1] || m[3]).split('::').pop()).filter(t => t !== 'Value' && t !== 'String');
  if (types.length || depth > 1) return types;
  const out = [];
  for (const c of body.matchAll(/\b(\w+)\(\s*(?:&mut\s+)?req\b/g)) if (c[1] !== fn) out.push(...bodyTypes(fileSrc, c[1], depth + 1));
  return out;
}
