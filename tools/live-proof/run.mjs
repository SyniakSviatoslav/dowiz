// LIVE PROOF, PHASE B (docs/research/2026-10-03-live-proof-plan.md §1 F-E, §4).
//
// Operator, 2026-10-03: "не просто мокати дані, а перевіряти усе на живому
// сайті з реальними підключеннями, детальними контрактами та схемами і
// версіями й описом". An in-memory test is the fast layer; a row is
// LIVE-PROVEN only when this runner drove the deployed Worker on a qa-* venue,
// read the effect back at the consumer, and the live answer validated against
// the row's contract (contracts/NN.json).
//
//   node tools/live-proof/run.mjs                 every LIVE-NOW row
//   node tools/live-proof/run.mjs --rows 37,59    a subset (never reported as the whole)
//   node tools/live-proof/run.mjs --record        also append the lines to docs/measurements/live-proof/<date>.jsonl
//
// Host: LIVE_HOST (default https://qa-durres.dowiz.org). e2e/flows/lib.mjs
// refuses, with exit 2, any venue whose first label is not qa-*.
//
// ONE JSON LINE PER ROW: {row, name, status, evidence, at, build, contract}.
// `build` is the live /api/version commit, so a FAILED can be matched to the
// build it hit. Status is one of
//   LIVE-PROVEN  the probe held AND every answer it validated passed the schema
//   FAILED       a step failed on the live site (the reason is the evidence)
//   NEEDS-KEY    a secret the box does not hold; never a pass
//   NO-PROBE     the row is LIVE-NOW and nobody has written its probe yet
// The last line is `proven=N of M LIVE-NOW`; exit 0 only when N == M and the
// run covered every LIVE-NOW row (memory gates-that-count-skips-as-passes).
process.env.FLOWS_HOST ||= process.env.LIVE_HOST || 'https://qa-durres.dowiz.org';
const RUN = `LIVE-${Date.now().toString(36)}`;
process.env.FLOWS_RUN ||= RUN;

import fs from 'node:fs';
import path from 'node:path';
import { validate } from './lib/schema.mjs';
const lib = await import('../../e2e/flows/lib.mjs');

const HERE = path.dirname(new URL(import.meta.url).pathname);
const ROOT = path.resolve(HERE, '../..');
const arg = k => { const i = process.argv.indexOf(k); return i < 0 ? null : (process.argv[i + 1] ?? ''); };
const subset = arg('--rows')?.split(',').map(Number).filter(Boolean) || null;
const record = process.argv.includes('--record');

const contracts = fs.readdirSync(`${HERE}/contracts`).filter(f => /^\d+\.json$/.test(f))
  .map(f => JSON.parse(fs.readFileSync(`${HERE}/contracts/${f}`, 'utf8'))).sort((a, b) => a.id - b.id);
const liveNow = contracts.filter(c => c.class === 'LIVE-NOW');
const probes = Object.fromEntries(fs.readdirSync(`${HERE}/probes`).filter(f => /^\d+-.*\.mjs$/.test(f))
  .map(f => [Number(f.split('-')[0]), f]));
const rows = liveNow.filter(c => !subset || subset.includes(c.id));
if (subset) for (const id of subset) if (!rows.some(c => c.id === id)) {
  console.log(`live-proof: row ${id} is not a LIVE-NOW contract (${contracts.find(c => c.id === id)?.class || 'no such row'}); refusing`);
  process.exit(2);
}

// An earlier run that died half way left TEST orders open: close them first.
const { sweep } = await import('./probes/_order.mjs');
const stuck = await sweep(lib);
if (stuck.length) console.log(`live-proof: ${stuck.length} TEST order(s) from an earlier run could not be closed: ${stuck.join(', ')}`);

const version = await lib.api('/api/version');
const build = version.body?.commit || `unknown (${version.status})`;
console.log(`live-proof: host ${lib.HOST}, build ${build.slice(0, 12)}, run ${lib.RUN}, ${rows.length} of ${liveNow.length} LIVE-NOW rows`);

/// A probe's failure: its message is the row's evidence.
export class Fail extends Error {}
/// A probe that needs a secret the box does not hold.
export class NeedsKey extends Error {}

const out = [];
for (const c of rows) {
  const at = new Date().toISOString();
  const line = { row: c.id, name: c.name, status: 'NO-PROBE', evidence: [], at, build, contract: c.contract_version };
  if (!probes[c.id]) {
    line.evidence.push(`no tools/live-proof/probes/${String(c.id).padStart(2, '0')}-*.mjs yet`);
  } else {
    // The probe calls `check(name, value)` on every live answer it relies on:
    // name is a schema of the contract (response_schema, readback_schema, ...).
    // A row whose probe validated nothing cannot be LIVE-PROVEN.
    let checked = 0;
    const bad = [];
    const ctx = {
      c, lib, run: lib.RUN, Fail, NeedsKey,
      note: s => line.evidence.push(s),
      check(name, value) {
        const s = c[name];
        if (!s) throw new Fail(`contract ${c.id} has no ${name}`);
        checked++;
        const v = validate(value, s);
        if (v.length) bad.push(`${name}: ${v.slice(0, 4).join('; ')}`);
        return value;
      },
      must(ok, why) { if (!ok) throw new Fail(why); },
    };
    const t0 = Date.now();
    try {
      const mod = await import(`./probes/${probes[c.id]}`);
      await mod.default(ctx);
      line.status = bad.length ? 'FAILED' : checked ? 'LIVE-PROVEN' : 'FAILED';
      if (bad.length) line.evidence.push(`SCHEMA ${bad.join(' | ')}`);
      if (!checked) line.evidence.push('the probe validated no live answer against the contract');
    } catch (e) {
      line.status = e instanceof NeedsKey ? 'NEEDS-KEY' : 'FAILED';
      line.evidence.push(`${e instanceof Fail || e instanceof NeedsKey ? '' : 'CRASH '}${e.message}`);
      if (bad.length) line.evidence.push(`SCHEMA ${bad.join(' | ')}`);
    }
    line.ms = Date.now() - t0;
  }
  console.log(JSON.stringify(line));
  out.push(line);
}

const proven = out.filter(l => l.status === 'LIVE-PROVEN').length;
const by = s => out.filter(l => l.status === s).length;
console.log(`live-proof: proven=${proven} of ${liveNow.length} LIVE-NOW` +
  ` (ran ${out.length}; FAILED ${by('FAILED')}, NEEDS-KEY ${by('NEEDS-KEY')}, NO-PROBE ${by('NO-PROBE')})` +
  (subset ? ' -- SUBSET RUN, not a verdict on the whole' : ''));
if (record) {
  const dir = `${ROOT}/docs/measurements/live-proof`;
  fs.mkdirSync(dir, { recursive: true });
  fs.appendFileSync(`${dir}/${new Date().toISOString().slice(0, 10)}.jsonl`, out.map(l => JSON.stringify(l)).join('\n') + '\n');
}
process.exit(!subset && proven === liveNow.length ? 0 : 1);
