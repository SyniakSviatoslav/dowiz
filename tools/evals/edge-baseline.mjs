#!/usr/bin/env node
// THE edge.* BASELINES, FROM N REAL RUNS OF THE WATCHER'S PROBES (W-EVALSCF, 2026-10-08).
//
// The live.* baselines were taken on the box, through the box's uplink. From inside Cloudflare the
// same GETs are a different measurement, so they are a different group (edge.*) with its own file,
// written from N >= 7 runs of the DEPLOYED dowiz-watch, never from one and never by hand.
//
//   EVALS_READ_TOKEN=... node tools/evals/edge-baseline.mjs [--runs 7] [--url https://dowiz-watch...] [--replace]
//   (the token defaults to the contents of /root/.dowiz_evals_read_token)
//
// Each run: POST /evals/run, then GET /evals/latest every POLL_MS until a NEW finished run is there
// (bounded: RUN_TIMEOUT_MS per run, said loudly). Per indicator over the N runs: plus25 -> the
// median, ratchet -> the max, floor -> the min (the rule's own direction, so none of the runs it was
// taken from is a breach of it); min/max/zero/trend need no baseline. An id that was null in any run
// is NOT written and is named. The file's header names every run. An existing edge.baseline is
// refused without --replace: re-baselining is a dated decision, not a side effect.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { formatBaseline, judge, median } from './rules.mjs';
import { DEFAULT_URL } from './collect/watch.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
export const MIN_RUNS = 7;
export const POLL_MS = 20_000;
export const RUN_TIMEOUT_MS = 10 * 60_000;
export const TOKEN_FILE = '/root/.dowiz_evals_read_token';

export function parseArgs(argv) {
  const a = { runs: MIN_RUNS, url: DEFAULT_URL, replace: false, out: path.join(HERE, 'baselines/edge.baseline') };
  for (let i = 0; i < argv.length; i++) {
    const k = argv[i];
    if (k === '--runs') a.runs = Number(argv[++i]);
    else if (k === '--url') a.url = argv[++i].replace(/\/$/, '');
    else if (k === '--out') a.out = argv[++i];
    else if (k === '--replace') a.replace = true;
    else throw new Error(`unknown argument ${k}`);
  }
  if (!(a.runs >= MIN_RUNS)) throw new Error(`--runs must be >= ${MIN_RUNS} (got ${a.runs})`);
  return a;
}

/** The baseline per id over the runs' edge.* indicators, the ids left out and why, and a self-check. */
export function derive(runs) {
  const by = {};
  for (const doc of runs) for (const i of doc.public || []) if (i.id.startsWith('edge.')) (by[i.id] ||= []).push(i);
  const out = {};
  const skipped = {};
  const check = {};
  for (const [id, xs] of Object.entries(by)) {
    const rule = xs[0].rule;
    if (!['plus25', 'ratchet', 'floor'].includes(rule)) continue;
    const vals = xs.map(x => x.value);
    if (xs.length !== runs.length || vals.some(v => !Number.isFinite(v))) { skipped[id] = `${vals.filter(Number.isFinite).length} of ${runs.length} runs had a value`; continue; }
    out[id] = rule === 'plus25' ? median(vals) : rule === 'ratchet' ? Math.max(...vals) : Math.min(...vals);
    check[id] = { rule, values: vals, breaches: xs.filter(x => judge(x, out[id]).status === 'breach').length };
  }
  return { baseline: out, skipped, check };
}

export function header(runs, url) {
  return `edge — dowiz-watch's own GETs from inside Cloudflare (${url}), written by tools/evals/edge-baseline.mjs `
    + `${new Date().toISOString().slice(0, 10)} from ${runs.length} runs: `
    + runs.map(r => `${new Date(r.measuredAtMs).toISOString()}@${r.watcher?.commit}`).join(', ')
    + '; plus25 = median, ratchet = max, floor = min of the runs; lowered freely, raised only with a dated note';
}

/** One fresh finished run: trigger it, then wait (bounded) for /evals/latest to carry a newer run. */
export async function oneRun(a, d) {
  const auth = { authorization: `Bearer ${d.token}` };
  const before = await d.fetch(`${a.url}/evals/latest`, { headers: auth }).then(r => r.json()).catch(() => ({}));
  const t = await d.fetch(`${a.url}/evals/run`, { method: 'POST', headers: auth });
  if (t.status !== 202 && t.status !== 409) throw new Error(`POST /evals/run answered ${t.status}: ${(await t.text()).slice(0, 200)}`);
  const t0 = d.now();
  for (;;) {
    await d.sleep(POLL_MS);
    const l = await d.fetch(`${a.url}/evals/latest`, { headers: auth }).then(r => r.json()).catch(() => ({}));
    if (l.complete && l.runId !== before.runId && !l.running) return l;
    if (d.now() - t0 > RUN_TIMEOUT_MS) throw new Error(`no finished run within ${RUN_TIMEOUT_MS / 1000} s (last: ${JSON.stringify(l).slice(0, 200)})`);
  }
}

export async function main(argv, deps = {}) {
  const a = parseArgs(argv);
  const d = { fetch: globalThis.fetch, now: Date.now, sleep: ms => new Promise(r => setTimeout(r, ms)), log: console.log,
    token: process.env.EVALS_READ_TOKEN || (fs.existsSync(TOKEN_FILE) ? fs.readFileSync(TOKEN_FILE, 'utf8').trim() : ''), ...deps };
  if (!d.token) throw new Error(`no EVALS_READ_TOKEN (env or ${TOKEN_FILE}): POST /evals/run needs it`);
  if (fs.existsSync(a.out) && !a.replace) throw new Error(`${a.out} exists; pass --replace to re-baseline it (a dated decision)`);
  const runs = [];
  for (let i = 0; i < a.runs; i++) {
    const r = await oneRun(a, d);
    runs.push(r);
    d.log(`run ${i + 1}/${a.runs}: ${new Date(r.measuredAtMs).toISOString()} ${r.public.length} edge indicators`);
  }
  const { baseline, skipped, check } = derive(runs);
  for (const [id, c] of Object.entries(check)) d.log(`${id} ${c.rule} = ${baseline[id]}  runs ${c.values.join(' ')}  breaches-of-own-runs ${c.breaches}`);
  for (const [id, why] of Object.entries(skipped)) d.log(`NOT WRITTEN ${id}: ${why}`);
  if (!Object.keys(baseline).length) throw new Error('no edge.* indicator had a value in every run: nothing written');
  fs.writeFileSync(a.out, formatBaseline(baseline, header(runs, a.url)));
  d.log(`wrote ${Object.keys(baseline).length} baselines -> ${a.out}`);
  return { baseline, skipped, check };
}

/* node:coverage disable */
if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  main(process.argv.slice(2)).then(() => process.exit(0), e => { console.error(`edge-baseline: ${e.message}`); process.exit(1); });
}
/* node:coverage enable */
