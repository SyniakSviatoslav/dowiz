#!/usr/bin/env node
// BODY-FIELDS (W-VERIFY, 2026-09-30): every key a browser sender may put in a
// POST/PUT body, against the struct the Worker parses that body into.
//
// Since 620e2464 (strict bodies) a key the struct does not declare is a live
// 400 when the struct carries `deny_unknown_fields` -- "REFUSED" below -- and
// was a silent drop before. A key sent to a struct WITHOUT the attribute is
// still dropped -- "DROPPED": the owner's input goes nowhere with an ok:true.
//
//   node tools/gates/body-fields.mjs [ROOT]        report + exit 1 on any REFUSED
//   BODY_FIELDS_ALL=1 ...                          also list DROPPED / unresolved
//
// Exit 2 when fewer than BODY_FIELDS_MIN senders were matched to a struct: a
// gate that matched nothing has measured nothing.
import fs from 'node:fs';
import path from 'node:path';
import { structsOf, routesOf, fileOf, bodyTypes, fnBody, walk } from './body-fields/rust.mjs';
import { sendersOf, jsFiles } from './body-fields/js.mjs';

const ROOT = process.argv[2] || path.resolve(path.dirname(new URL(import.meta.url).pathname), '../..');
const SRC = path.join(ROOT, 'workers/api/src'), PUB = path.join(ROOT, 'workers/api/public');
process.env.BODY_FIELDS_PUB = PUB;
const MIN = Number(process.env.BODY_FIELDS_MIN || 40);

// Struct index: name -> [{file, st}].
const index = {};
for (const f of walk(SRC)) for (const [n, st] of Object.entries(structsOf(fs.readFileSync(f, 'utf8'))))
  (index[n] ||= []).push({ file: f, st });
function structFor(name, fromFile, fnSrc){
  const c = index[name] || [];
  // A struct declared inside the handler itself wins (owner.rs has five `In`s).
  if (fnSrc){ const local = structsOf(fnSrc)[name]; if (local) return { file: fromFile, st: local }; }
  return (c.find(x => x.file === fromFile) || c.find(x => path.dirname(x.file) === path.dirname(fromFile)
    || x.file.startsWith(fromFile.replace(/\.rs$/, '/'))) || (c.length === 1 ? c[0] : null));
}
/// Accepted keys of a struct, flattened members included; null when open.
function accepted(ent){
  if (!ent || ent.st.enum) return null;
  const keys = new Set();
  for (const f of ent.st.fields){
    if (f.flatten){
      const inner = structFor(f.ty.replace(/^Option<|>$/g, '').split('::').pop(), ent.file);
      const a = accepted(inner); if (!a) return null; a.forEach(k => keys.add(k));
    } else f.keys.forEach(k => keys.add(k));
  }
  return keys;
}

const routes = routesOf(fs.readFileSync(path.join(SRC, 'lib.rs'), 'utf8')).map(r => {
  const { file, fn } = fileOf(SRC, r.handler);
  const types = file ? bodyTypes(fs.readFileSync(file, 'utf8'), fn) : [];
  const fsrc = file ? fnBody(fs.readFileSync(file, 'utf8'), fn) : null;
  const ents = types.map(t => structFor(t, file, fsrc)).filter(Boolean);
  return { ...r, file, types, ents, re: new RegExp('^' + r.path.replace(/:\w+/g, '[^/]+').replace(/\*\w+/, '.*') + '$') };
});
// The router prefers a static segment over a `:param` (matchit): fewest params first.
const byParams = [...routes].sort((a, b) => (a.path.match(/:/g) || []).length - (b.path.match(/:/g) || []).length);
const routeOf = (m, p) => byParams.find(r => r.method === m && r.re.test(p))
  || byParams.find(r => r.re.test(p));

const report = { refused: [], dropped: [], unresolved: [], noRoute: [], noStruct: [], ok: 0 };
for (const [f, rel] of jsFiles(PUB)){
  for (const s of sendersOf(f, rel)){
    if (s.path.startsWith('http')) continue;
    const r = routeOf(s.method, s.path);
    if (!r){ report.noRoute.push(`${s.at} ${s.method} ${s.path}`); continue; }
    if (!r.ents.length){ report.noStruct.push(`${s.at} ${s.path} -> ${r.handler} (${r.types.join(',') || 'untyped/Value'})`); continue; }
    if (s.open.length) report.unresolved.push(`${s.at} ${s.path} keys{${s.keys}} OPEN: ${s.open.join('; ')}`);
    for (const e of r.ents){
      const acc = accepted(e);
      if (!acc){ report.unresolved.push(`${s.at} ${s.path} -> ${e.st.name} (enum/flatten, checked by hand)`); continue; }
      const extra = s.keys.filter(k => !acc.has(k));
      const where = `${s.at} ${s.method} ${s.path} -> ${r.handler} :: ${e.st.name} (${path.relative(ROOT, e.file)})`;
      if (!extra.length){ report.ok++; continue; }
      (e.st.deny ? report.refused : report.dropped).push(`${where}\n      sends {${extra.join(', ')}} not in {${[...acc].join(', ')}}`);
    }
  }
}

// ── MCP tools: an agent's arguments become the route's body (mcp/tools.rs plan()) ──
const toolsSrc = fs.readFileSync(path.join(SRC, 'mcp/tools.rs'), 'utf8');
let mcpTools = 0;
for (const m of toolsSrc.matchAll(/post\(\s*"(\w+)"[\s\S]*?(NO_ARGS|r#"([\s\S]*?)"#)\s*,\s*"([^"]+)"\s*,\s*(true|false)/g)){
  mcpTools++;
  const [, name, , schema, p, loc] = m;
  const holes = [...p.matchAll(/\{(\w+)\}/g)].map(x => x[1]);
  const props = schema ? Object.keys(JSON.parse(schema).properties || {}) : [];
  const keys = props.filter(k => !holes.includes(k)).concat(loc === 'true' ? ['location_id'] : []);
  const r = routeOf('POST', p.replace(/\{(\w+)\}/g, 'x'));
  const at = `mcp/tools.rs ${name}`;
  if (!r){ report.noRoute.push(`${at} POST ${p}`); continue; }
  if (!r.ents.length){ report.noStruct.push(`${at} ${p} -> ${r.handler}`); continue; }
  for (const e of r.ents){
    const acc = accepted(e); if (!acc){ report.unresolved.push(`${at} -> ${e.st.name}`); continue; }
    const extra = keys.filter(k => !acc.has(k));
    if (!extra.length){ report.ok++; continue; }
    (e.st.deny ? report.refused : report.dropped).push(`${at} POST ${p} -> ${r.handler} :: ${e.st.name}\n      sends {${extra.join(', ')}} not in {${[...acc].join(', ')}}`);
  }
}
if (!mcpTools){ console.log('body-fields: parsed no MCP tool out of mcp/tools.rs -- the parse broke'); process.exit(2); }

const all = process.env.BODY_FIELDS_ALL === '1';
const show = (t, xs, force) => { if (xs.length && (force || all)) { console.log(`\n${t} (${xs.length})`); xs.forEach(x => console.log('  ' + x)); } };
show('REFUSED -- a live 400 since strict bodies', report.refused, true);
show('DROPPED -- sent, not declared, silently ignored', report.dropped);
show('UNRESOLVED sender keys (checked by hand)', report.unresolved);
show('NO ROUTE for a sender path', report.noRoute);
show('NO TYPED STRUCT behind the route', report.noStruct);
console.log(`\nbody-fields: ${report.ok} sender/struct pairs clean, ${report.refused.length} REFUSED, ${report.dropped.length} dropped, ${report.unresolved.length} unresolved, ${report.noRoute.length} no-route, ${report.noStruct.length} untyped`);
if (report.ok + report.refused.length + report.dropped.length < MIN){ console.log(`body-fields: matched fewer than ${MIN} pairs -- the parse broke, not the code`); process.exit(2); }
process.exit(report.refused.length ? 1 : 0);
