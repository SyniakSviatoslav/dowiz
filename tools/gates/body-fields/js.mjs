// THE BROWSER SIDE of body-fields: every POST/PUT the console, store, courier,
// room, kit and platform code sends, with the set of top-level keys its JSON
// body can carry. Parsed with the TypeScript compiler (a real AST), so spreads,
// withLoc(), conditional spreads and `x.foo = ...` after the literal are seen.
// A body the walker cannot resolve is REPORTED, never assumed empty.
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
// TS_PATH, else the repo root's node_modules (CI installs it there), else the box's promo-30s copy.
const ts = (() => {
  const tries = [process.env.TS_PATH, path.resolve(path.dirname(new URL(import.meta.url).pathname), '../../../node_modules/typescript'),
    '/root/dowiz/marketing/promo-30s/node_modules/typescript'].filter(Boolean);
  for (const t of tries) { try { return require(t); } catch {} }
  throw new Error(`body-fields: typescript not found (tried ${tries.join(', ')}); set TS_PATH or npm install typescript`);
})();

const K = ts.SyntaxKind;
const POSTERS = new Set(['post', 'write', 'tapped', 'send', 'postJson', 'postJSON']);

/// A path expression as text: literal parts kept, holes become ':p'.
function pathText(n, sf){
  if (!n) return null;
  if (ts.isStringLiteralLike(n)) return n.text;
  if (ts.isTemplateExpression(n)) return n.head.text + n.templateSpans.map(s => (holeText(s.expression, sf)) + s.literal.text).join('');
  if (ts.isBinaryExpression(n) && n.operatorToken.kind === K.PlusToken){
    const a = pathText(n.left, sf), b = pathText(n.right, sf);
    return (a ?? ':p') + (b ?? ':p');
  }
  if (ts.isIdentifier(n) && /^API$/.test(n.text)) return '/api';
  if (ts.isParenthesizedExpression(n)) return pathText(n.expression, sf);
  return null;
}
const holeText = (e) => (ts.isIdentifier(e) && e.text === 'API') ? '/api' : ':p';

/// The declaration a name refers to, walking outward through the scopes.
function resolveIdent(id){
  const name = id.text;
  for (let p = id.parent; p; p = p.parent){
    const stmts = p.statements || (ts.isSourceFile(p) ? p.statements : null);
    if (stmts) for (const st of stmts){
      if (ts.isVariableStatement(st)) for (const d of st.declarationList.declarations)
        if (ts.isIdentifier(d.name) && d.name.text === name && d.initializer) return { decl: d, scope: p };
    }
    if ((ts.isFunctionLike(p)) && p.parameters?.some(q => ts.isIdentifier(q.name) && q.name.text === name)) return { param: true, fn: p };
  }
  return null;
}

/// Keys the object can carry: {keys:Set, open:[reasons]}.
function keysOf(n, depth = 0){
  const r = { keys: new Set(), open: [] };
  if (!n || depth > 6){ r.open.push('depth'); return r; }
  const add = o => { o.keys.forEach(k => r.keys.add(k)); r.open.push(...o.open); };
  if (ts.isParenthesizedExpression(n) || ts.isAsExpression?.(n)) return keysOf(n.expression, depth + 1);
  if (ts.isObjectLiteralExpression(n)){
    for (const p of n.properties){
      if (ts.isSpreadAssignment(p)) add(keysOf(p.expression, depth + 1));
      else if (p.name && (ts.isIdentifier(p.name) || ts.isStringLiteralLike(p.name))){
        // `key: undefined` is dropped by JSON.stringify: not sent.
        if (!(p.initializer && ts.isIdentifier(p.initializer) && p.initializer.text === 'undefined')) r.keys.add(p.name.text);
      }
      else if (p.name && ts.isComputedPropertyName(p.name)){
        const e = p.name.expression;
        if (ts.isConditionalExpression(e) && ts.isStringLiteralLike(e.whenTrue) && ts.isStringLiteralLike(e.whenFalse)){ r.keys.add(e.whenTrue.text); r.keys.add(e.whenFalse.text); }
        else r.open.push('computed key ' + e.getText());
      }
    }
    return r;
  }
  if (ts.isConditionalExpression(n)){ add(keysOf(n.whenTrue, depth + 1)); add(keysOf(n.whenFalse, depth + 1)); return r; }
  if (ts.isBinaryExpression(n) && [K.AmpersandAmpersandToken, K.BarBarToken, K.QuestionQuestionToken].includes(n.operatorToken.kind)){
    if (n.operatorToken.kind !== K.AmpersandAmpersandToken) add(keysOf(n.left, depth + 1));
    add(keysOf(n.right, depth + 1)); return r;
  }
  if (ts.isCallExpression(n)){
    const c = n.expression.getText();
    if (c === 'withLoc'){ r.keys.add('location_id'); if (n.arguments[0]) add(keysOf(n.arguments[0], depth + 1)); return r; }
    if (c === 'JSON.stringify' || c === 'json') return keysOf(n.arguments[0], depth + 1);
    if (c === 'Object.assign'){ n.arguments.forEach(a => add(keysOf(a, depth + 1))); return r; }
    const fnNode = functionFor(n);
    if (fnNode){ returnsOf(fnNode).forEach(e => add(keysOf(e, depth + 1))); return r; }
    r.open.push('call ' + c + '()'); return r;
  }
  if (ts.isIdentifier(n)){
    if (n.text === 'undefined' || n.text === 'null') return r;
    const d = resolveIdent(n);
    if (!d){ r.open.push('unresolved ' + n.text); return r; }
    if (d.param){ r.open.push('parameter ' + n.text + ' of ' + (d.fn.name?.getText() || 'fn')); return r; }
    add(keysOf(d.decl.initializer, depth + 1));
    // `x.foo = ...` / `x['foo'] = ...` anywhere in the declaring scope.
    const walkAssign = m => {
      if (ts.isBinaryExpression(m) && m.operatorToken.kind === K.EqualsToken){
        const l = m.left;
        if (ts.isPropertyAccessExpression(l) && l.expression.getText() === n.text) r.keys.add(l.name.text);
        if (ts.isElementAccessExpression(l) && l.expression.getText() === n.text){
          if (ts.isStringLiteralLike(l.argumentExpression)) r.keys.add(l.argumentExpression.text); else r.open.push('dynamic key on ' + n.text);
        }
      }
      ts.forEachChild(m, walkAssign);
    };
    walkAssign(d.scope);
    return r;
  }
  if (n.kind === K.NullKeyword || (ts.isIdentifier(n) && n.text === 'undefined')) return r;
  r.open.push('expr ' + n.getText().slice(0, 40));
  return r;
}

// ── functions a body is built by: same file, or an imported module ─────────

const sfCache = new Map();
function sourceFile(file){
  if (!sfCache.has(file)){
    if (!fs.existsSync(file)) return null;
    sfCache.set(file, ts.createSourceFile(file, fs.readFileSync(file, 'utf8'), ts.ScriptTarget.Latest, true, ts.ScriptKind.JS));
  }
  return sfCache.get(file);
}
function declIn(sf, name){
  let hit = null;
  const v = n => {
    if (hit) return;
    if (ts.isFunctionDeclaration(n) && n.name?.text === name) hit = n;
    else if (ts.isVariableDeclaration(n) && ts.isIdentifier(n.name) && n.name.text === name && n.initializer
      && (ts.isArrowFunction(n.initializer) || ts.isFunctionExpression(n.initializer))) hit = n.initializer;
    else ts.forEachChild(n, v);
  };
  v(sf); return hit;
}
function importFile(sf, local){
  for (const st of sf.statements){
    if (!ts.isImportDeclaration(st) || !st.importClause) continue;
    const spec = st.moduleSpecifier.text, b = st.importClause.namedBindings;
    const file = spec.startsWith('/') ? path.join(process.env.BODY_FIELDS_PUB || '', spec) : path.resolve(path.dirname(sf.fileName), spec);
    if (b && ts.isNamespaceImport(b) && b.name.text === local) return { file, ns: true };
    if (b && ts.isNamedImports(b)) for (const e of b.elements) if (e.name.text === local) return { file, name: (e.propertyName || e.name).text };
  }
  return null;
}
/// The function a call names, or null.
function functionFor(call){
  const sf = call.getSourceFile(), e = call.expression;
  if (ts.isIdentifier(e)){
    const local = declIn(sf, e.text); if (local) return local;
    const im = importFile(sf, e.text); if (im && !im.ns){ const o = sourceFile(im.file); return o && declIn(o, im.name); }
  }
  if (ts.isPropertyAccessExpression(e) && ts.isIdentifier(e.expression)){
    const im = importFile(sf, e.expression.text); if (im && im.ns){ const o = sourceFile(im.file); return o && declIn(o, e.name.text); }
  }
  return null;
}
function returnsOf(fn){
  if (fn.body && !ts.isBlock(fn.body)) return [fn.body];
  const out = [];
  const v = n => { if (n !== fn && ts.isFunctionLike(n)) return; if (ts.isReturnStatement(n) && n.expression) out.push(n.expression); ts.forEachChild(n, v); };
  if (fn.body) ts.forEachChild(fn.body, v);
  return out;
}

function prop(obj, name){
  if (!obj || !ts.isObjectLiteralExpression(obj)) return undefined;
  for (const p of obj.properties){
    if (p.name?.getText() === name) return ts.isShorthandPropertyAssignment(p) ? p.name : p.initializer;
  }
}

/// Every write call in one file.
export function sendersOf(file, rel){
  const src = fs.readFileSync(file, 'utf8');
  const sf = ts.createSourceFile(file, src, ts.ScriptTarget.Latest, true, ts.ScriptKind.JS);
  const out = [];
  const visit = n => {
    if (ts.isCallExpression(n) && n.arguments.length >= 1){
      const callee = n.expression.getText();
      let p = pathText(n.arguments[0], sf);
      if (p && (p.startsWith('/') || p.startsWith('http'))){
        let opts = n.arguments[1], method = null, body;
        if (opts && ts.isIdentifier(opts)){ const d = resolveIdent(opts); if (d && d.decl) opts = d.decl.initializer; }
        if (opts && ts.isCallExpression(opts) && opts.expression.getText() === 'json'){ method = 'POST'; body = opts.arguments[0]; }
        else if (opts && ts.isObjectLiteralExpression(opts)){
          const m = prop(opts, 'method'); method = m && ts.isStringLiteralLike(m) ? m.text.toUpperCase() : (m ? '?' : null);
          body = prop(opts, 'body');
        }
        const short = callee.split('.').pop();
        if (!method && POSTERS.has(short) && n.arguments[1]){ method = 'POST'; body = n.arguments[1]; }
        if (method && method !== 'GET' && method !== 'DELETE' && (body || method)){
          // A hole glued to the end of a segment is a query string (`'/x' + q()`).
          p = p.replace(/([^/]):p$/, '$1');
          if (short === 'call' && rel.startsWith('kit/')) p = '/api/public/locations/:p' + p;
          else if (!p.startsWith('/api') && !p.startsWith('http')) p = '/api' + p;
          const k = body ? keysOf(body) : { keys: new Set(), open: [] };
          const { line } = sf.getLineAndCharacterOfPosition(n.getStart());
          out.push({ at: `${rel}:${line + 1}`, method, path: p.split('?')[0], keys: [...k.keys], open: k.open, callee });
        }
      }
    }
    ts.forEachChild(n, visit);
  };
  visit(sf);
  return out;
}

export function* jsFiles(d, root = d){
  for (const e of fs.readdirSync(d, { withFileTypes: true })){
    const p = path.join(d, e.name);
    if (e.isDirectory()){ if (!/vendor|node_modules|\/map$/.test(p)) yield* jsFiles(p, root); }
    else if (/\.m?js$/.test(e.name) && !/\.min\.js$|\.test\.m?js$/.test(e.name)) yield [p, path.relative(root, p)];
  }
}
