// W-STOCK's live probes, shared: the contract a probe proves, a validator for
// the subset of JSON Schema draft 2020-12 the contracts use, and the step
// reporter. qa-* venues only -- the host guard is e2e/flows/lib.mjs's, which
// exits 2 on any other venue before a single request.
//
// A probe's verdict is its exit code: 0 every step held on the live site;
// 1 a step failed (the reason is printed); 3 NEEDS-KEY (a step needs a secret
// the box does not hold -- never counted as a pass).
import fs from 'node:fs';
import path from 'node:path';

import { HOST, UA, owner } from '../../../e2e/flows/lib.mjs';
export { HOST, LOC, RUN, creds, api, own, owner, menu, dishes, courierCreds, sleep } from '../../../e2e/flows/lib.mjs';

const HERE = path.dirname(new URL(import.meta.url).pathname);

/// The contract `feature-<name>.json`, parsed.
export function contract(name) {
  return JSON.parse(fs.readFileSync(path.join(HERE, '..', 'contracts', `feature-${name}.json`), 'utf8'));
}

/// Validate `v` against `s` (type, const, enum, required, properties,
/// additionalProperties:false, items, minItems, minimum, maximum, $defs/$ref
/// within the schema). Answers the list of violations, each with its path.
export function validate(v, s, root = s, at = '$') {
  const out = [];
  if (s.$ref) {
    const ref = s.$ref.replace(/^#\/\$defs\//, '');
    return validate(v, root.$defs?.[ref] ?? {}, root, at);
  }
  const type = Array.isArray(v) ? 'array' : v === null ? 'null' : Number.isInteger(v) ? 'integer' : typeof v;
  if (s.type) {
    const want = [].concat(s.type);
    const ok = want.includes(type) || (type === 'integer' && want.includes('number'));
    if (!ok) return [`${at}: ${type}, wanted ${want.join('|')}`];
  }
  if ('const' in s && JSON.stringify(v) !== JSON.stringify(s.const)) out.push(`${at}: ${JSON.stringify(v)} is not ${JSON.stringify(s.const)}`);
  if (s.enum && !s.enum.some(e => JSON.stringify(e) === JSON.stringify(v))) out.push(`${at}: ${JSON.stringify(v)} not in ${JSON.stringify(s.enum)}`);
  if (typeof v === 'number') {
    if (s.minimum != null && v < s.minimum) out.push(`${at}: ${v} < ${s.minimum}`);
    if (s.maximum != null && v > s.maximum) out.push(`${at}: ${v} > ${s.maximum}`);
  }
  if (type === 'object') {
    for (const k of s.required || []) if (!(k in v)) out.push(`${at}: missing ${k}`);
    for (const [k, sub] of Object.entries(s.properties || {})) if (k in v) out.push(...validate(v[k], sub, root, `${at}.${k}`));
    if (s.additionalProperties === false) for (const k of Object.keys(v)) if (!(k in (s.properties || {}))) out.push(`${at}: unexpected ${k}`);
  }
  if (type === 'array') {
    if (s.minItems != null && v.length < s.minItems) out.push(`${at}: ${v.length} items < ${s.minItems}`);
    if (s.items) v.forEach((x, i) => out.push(...validate(x, s.items, root, `${at}[${i}]`)));
  }
  return out;
}

/// The step reporter: `step(name, ok, detail)`; `verdict()` prints and exits.
export function reporter(probe) {
  const fails = [], needs = [];
  const step = (name, ok, detail = '') => {
    if (ok === 'NEEDS-KEY') needs.push(name); else if (!ok) fails.push(name);
    console.log(`${ok === 'NEEDS-KEY' ? 'NEEDS-KEY' : ok ? 'ok  ' : 'FAIL'} ${name}${detail ? ' :: ' + String(detail).slice(0, 400) : ''}`);
    return ok === true;
  };
  /// A live answer against the contract's schema: one step, the violations named.
  const schema = (name, body, s) => { const bad = validate(body, s); return step(name, bad.length === 0, bad.slice(0, 6).join('; ')); };
  const verdict = () => {
    const rc = fails.length ? 1 : needs.length ? 3 : 0;
    console.log(`\n${probe}: ${fails.length ? `FAIL (${fails.length}): ${fails.join(' | ')}` : needs.length ? `NEEDS-KEY: ${needs.join(' | ')}` : 'LIVE OK'} rc=${rc}`);
    process.exit(rc);
  };
  return { step, schema, verdict, fails };
}

/// A read that the platform's "exceeded resource limits" 503 may refuse, asked at most six times.
export async function read(own, p) {
  let r;
  for (let i = 0; i < 6; i++) { r = await own(p); if (r.status !== 503) return r; }
  return r;
}

/// POST a CSV to an owner route with the owner's token (lib.mjs `api` sends JSON only).
export async function csvPost(p, text) {
  const r = await fetch(`${HOST}${p}`, { method: 'POST', body: text,
    headers: { 'content-type': 'text/csv', 'user-agent': UA, authorization: `Bearer ${await owner()}` } });
  const t = await r.text();
  let b; try { b = JSON.parse(t); } catch { b = t; }
  return { status: r.status, body: b, text: t.slice(0, 300) };
}

/// Every Stock screen row (`GET /api/owner/stock`).
export async function shelf(own, loc) {
  const r = await read(own, `/api/owner/stock?location_id=${loc}`);
  return { status: r.status, body: r.body, supplies: r.body?.supplies || [] };
}
