// The YAML the lesson files use, and nothing more -- so the build needs no
// package (CI has node and no node_modules for tools/).
//
// SUPPORTED: block mappings (`key: value`, `key:` + an indented block), block
// sequences (`- item`, `- key: value` opening a mapping), double-quoted scalars
// (JSON escapes), single-quoted scalars ('' is a quote), flow sequences of
// scalars (`[a, b]`, plain items only), plain scalars (a ` #` starts a comment), full-line
// comments and blank lines. Every plain scalar stays a STRING: `yes`, `8` and
// `true` are typed by the reader that knows what the field means, not here.
//
// REFUSED, loudly, with the line number: tabs in indentation, a key twice in
// one mapping, a line indented where nothing can be, anything else.
const KEY = /^([A-Za-z0-9_-]+):(?:\s+(.*))?$/;

class YamlError extends Error {}
const fail = (ln, msg) => { throw new YamlError(`line ${ln}: ${msg}`); };

function scalar(raw, ln){
  const s = raw.trim();
  if (s.startsWith('"')) {
    try { const v = JSON.parse(s); if (typeof v === 'string') return v; } catch {}
    return fail(ln, `bad double-quoted string ${s}`);
  }
  if (s.startsWith("'")) {
    const m = /^'((?:[^']|'')*)'$/.exec(s);
    return m ? m[1].replace(/''/g, "'") : fail(ln, `bad single-quoted string ${s}`);
  }
  if (s.startsWith('[')) {
    if (!s.endsWith(']')) fail(ln, `unclosed flow sequence ${s}`);
    const body = s.slice(1, -1).trim();
    if (/["'\[\]{}]/.test(body)) fail(ln, 'a flow sequence holds plain scalars only');
    return body ? body.split(',').map(x => scalar(x, ln)) : [];
  }
  if (s.startsWith('{')) fail(ln, 'flow mappings are not supported');
  return s.replace(/\s+#.*$/, '');
}

function lines(text){
  const out = [];
  String(text).split(/\r?\n/).forEach((l, i) => {
    if (!l.trim() || /^\s*#/.test(l)) return;
    const ind = /^( *)/.exec(l)[1].length;
    if (l[ind] === '\t') fail(i + 1, 'tab in indentation');
    out.push({ ind, text: l.slice(ind).replace(/\s+$/, ''), ln: i + 1 });
  });
  return out;
}

const isItem = t => t === '-' || t.startsWith('- ');

function block(L, i, ind){
  return isItem(L[i].text) ? seq(L, i, ind) : map(L, i, ind);
}

function map(L, i, ind){
  const obj = {};
  while (i < L.length && L[i].ind === ind && !isItem(L[i].text)) {
    const { text, ln } = L[i];
    const m = KEY.exec(text);
    if (!m) fail(ln, `expected "key: value", got ${text}`);
    if (Object.prototype.hasOwnProperty.call(obj, m[1])) fail(ln, `duplicate key ${m[1]}`);
    i++;
    if (m[2] !== undefined && m[2].trim() !== '') { obj[m[1]] = scalar(m[2], ln); continue; }
    const next = L[i];
    if (next && next.ind > ind) { [obj[m[1]], i] = block(L, i, next.ind); continue; }
    if (next && next.ind === ind && isItem(next.text)) { [obj[m[1]], i] = seq(L, i, ind); continue; }
    obj[m[1]] = null;
  }
  if (i < L.length && L[i].ind > ind) fail(L[i].ln, 'unexpected indentation');
  return [obj, i];
}

function seq(L, i, ind){
  const arr = [];
  while (i < L.length && L[i].ind === ind && isItem(L[i].text)) {
    const { text, ln } = L[i];
    const rest = text === '-' ? '' : text.slice(2);
    const off = ind + text.length - rest.trimStart().length;
    if (!rest.trim()) {
      i++;
      if (!L[i] || L[i].ind <= ind) { arr.push(null); continue; }
      let v; [v, i] = block(L, i, L[i].ind); arr.push(v); continue;
    }
    if (KEY.test(rest.trim())) {
      // `- key: v` opens a mapping whose keys sit where `key` starts.
      L[i] = { ind: off, text: rest.trim(), ln };
      let v; [v, i] = map(L, i, off); arr.push(v); continue;
    }
    arr.push(scalar(rest, ln)); i++;
  }
  return [arr, i];
}

/// Parse one document. Throws YamlError naming the line.
export function parseYaml(text){
  const L = lines(text);
  if (!L.length) return null;
  const [v, i] = block(L, 0, L[0].ind);
  if (i < L.length) fail(L[i].ln, `unexpected content ${L[i].text}`);
  return v;
}
export { YamlError };
