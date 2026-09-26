#!/usr/bin/env node
// THE EVALS ORCHESTRATOR (BLUEPRINT-OPTIMIZATION-AND-EVALS-2026-09-24 §B.3).
//
//   node tools/evals/run.mjs --suite ci|nightly|live [--host URL]... [--out DIR] [--no-baseline-write]
//
// Runs every collector of the suite, judges each indicator by its rule against
// tools/evals/baselines/<group>.baseline, writes docs/measurements/evals/<date>-<commit>-<suite>.{json,md}
// (the live suite appends one line to docs/measurements/live/<date>.jsonl instead), and EXITS 1
// ON ANY BREACH. A collector that throws is a breach with its message, never a skipped group.
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { judge, parseBaseline, formatBaseline, groupOf, ind } from './rules.mjs';
import { readCreds } from './collect/net.mjs';
import { render } from './report.mjs';

export const HERE = path.dirname(fileURLToPath(import.meta.url));
export const ROOT = path.resolve(HERE, '../..');
export const SUITES = {
  ci: ['static', 'i18n', 'wasm', 'gates', 'tests'],
  nightly: ['live', 'order', 'health', 'product', 'ux', 'cost'],
  live: ['health'],
};
/** GET-only venues; the order suite never runs by itself (see collect/order.mjs). */
export const HOSTS = ['https://sushi-durres.dowiz.org', 'https://dubin-sushi.dowiz.org'];

export function parseArgs(argv) {
  const a = { suite: 'ci', hosts: [], out: null, writeBaselines: true, only: null };
  for (let i = 0; i < argv.length; i++) {
    const k = argv[i];
    if (k === '--suite') a.suite = argv[++i];
    else if (k === '--host') a.hosts.push(argv[++i].replace(/\/$/, ''));
    else if (k === '--out') a.out = argv[++i];
    else if (k === '--only') a.only = argv[++i].split(',');
    else if (k === '--no-baseline-write') a.writeBaselines = false;
    else throw new Error(`unknown argument ${k}`);
  }
  if (!SUITES[a.suite]) throw new Error(`unknown suite ${a.suite} (ci, nightly, live)`);
  if (!a.hosts.length) a.hosts = [...HOSTS];
  return a;
}

/** The python collector: its stdout is a JSON list of indicators. */
export function wasmCollector(exec = spawnSync) {
  return {
    async collect(ctx) {
      const r = exec('python3', [path.join(HERE, 'collect/wasm.py')], { cwd: ctx.root, encoding: 'utf8' });
      if (r.status !== 0) throw new Error(`wasm.py rc=${r.status}: ${String(r.stderr).slice(-300)}`);
      return JSON.parse(r.stdout);
    },
  };
}

export async function loadCollector(name) {
  if (name === 'wasm') return wasmCollector();
  return import(pathToFileURL(path.join(HERE, 'collect', `${name}.mjs`)).href);
}

export function git(root, args, exec = spawnSync) {
  const r = exec('git', args, { cwd: root, encoding: 'utf8' });
  return r.status === 0 ? String(r.stdout).trim() : '';
}

export function commitOf(root, exec = spawnSync) {
  const c = git(root, ['rev-parse', '--short', 'HEAD'], exec) || 'nocommit';
  return git(root, ['status', '--porcelain', '--untracked-files=no'], exec) ? `${c}-dirty` : c;
}

/** The newest earlier run of the same suite, for growth-per-day (health) — or {}. */
export function previousRun(dir, suite) {
  if (!fs.existsSync(dir)) return {};
  const f = fs.readdirSync(dir).filter(x => x.endsWith(`-${suite}.json`)).sort().pop();
  return f ? JSON.parse(fs.readFileSync(path.join(dir, f), 'utf8')) : {};
}

export function readBaselines(dir) {
  const out = {};
  if (!fs.existsSync(dir)) return out;
  for (const f of fs.readdirSync(dir).filter(x => x.endsWith('.baseline'))) {
    out[f.slice(0, -9)] = parseBaseline(fs.readFileSync(path.join(dir, f), 'utf8'));
  }
  return out;
}

/** Judge every indicator; returns the flat result map and the baseline changes. */
export function judgeAll(inds, baselines, at) {
  const results = {};
  const changes = {};
  for (const i of inds) {
    const g = groupOf(i.id);
    const base = baselines[g]?.[i.id];
    const j = judge(i, base);
    results[i.id] = { ...i, baseline: base === undefined ? null : Number(base), status: j.status, why: j.why || '', collected_at: at };
    // A null the collector did not explain (an empty window) still says why it is not a number.
    if (j.status === 'unverified' && !i.unverified) results[i.id].unverified = `no value${i.note ? `: ${i.note}` : ''}`;
    delete results[i.id].id;
    if (j.next !== undefined) (changes[g] ||= {})[i.id] = j.next;
  }
  return { results, changes };
}

export function writeBaselines(dir, baselines, changes, stamp) {
  fs.mkdirSync(dir, { recursive: true });
  for (const [g, kv] of Object.entries(changes)) {
    const merged = { ...(baselines[g] || {}), ...kv };
    fs.writeFileSync(path.join(dir, `${g}.baseline`), formatBaseline(merged,
      `${g} — written by tools/evals/run.mjs ${stamp}; lowered freely, raised only with a dated note`));
  }
}

export async function main(argv, deps = {}) {
  const a = parseArgs(argv);
  const d = { root: ROOT, now: Date.now, load: loadCollector, log: console.log, baselines: path.join(HERE, 'baselines'),
    exec: spawnSync, env: process.env, creds: null, fetch: undefined, ...deps };
  const { root, now, load, log } = d;
  const outDir = a.out || path.join(root, 'docs/measurements/evals');
  const baseDir = d.baselines;
  const started = now();
  const date = new Date(started).toISOString().slice(0, 10);
  const commit = commitOf(root, d.exec);
  const ctx = {
    root, suite: a.suite, host: a.hosts[0], hosts: a.hosts, env: d.env,
    creds: d.creds ?? readCreds(), fetch: d.fetch, now, results: {},
    previous: previousRun(outDir, a.suite),
  };
  const inds = [];
  for (const name of SUITES[a.suite].filter(n => !a.only || a.only.includes(n))) {
    let got;
    try {
      got = await (await load(name)).collect(ctx);
    } catch (e) {
      got = [ind(`${name}.collector_ok`, 0, 'bool', 'min', `collect/${name}`, { limit: 1, note: String(e.message).slice(0, 300) })];
    }
    for (const i of got) ctx.results[i.id] = i;
    inds.push(...got);
    log(`${name}: ${got.length} indicators`);
  }
  const baselines = readBaselines(baseDir);
  const { results, changes } = judgeAll(inds, baselines, started);
  if (a.writeBaselines) writeBaselines(baseDir, baselines, changes, `${date} ${commit}`);
  const meta = {
    suite: a.suite, commit, date, hosts: a.suite === 'ci' ? [] : a.hosts,
    deployed_version: ctx.env.EVALS_DEPLOYED_VERSION || 'UNVERIFIED (no route or token reports the deployed version id)',
    started_at: started, finished_at: now(), node: process.version,
  };
  const doc = { _run: meta, ...results };
  const breaches = Object.entries(results).filter(([, r]) => r.status === 'breach');
  let file;
  if (a.suite === 'live') {
    const dir = path.join(root, 'docs/measurements/live');
    fs.mkdirSync(dir, { recursive: true });
    file = path.join(dir, `${date}.jsonl`);
    fs.appendFileSync(file, JSON.stringify(doc) + '\n');
  } else {
    fs.mkdirSync(outDir, { recursive: true });
    file = path.join(outDir, `${date}-${commit}-${a.suite}`);
    fs.writeFileSync(`${file}.json`, JSON.stringify(doc, null, 1));
    fs.writeFileSync(`${file}.md`, render(doc));
    file += '.md';
  }
  log(`${Object.keys(results).length} indicators, ${breaches.length} breaches -> ${path.relative(root, file)}`);
  for (const [id, r] of breaches) log(`BREACH ${id} = ${r.value} (${r.why})`);
  return { code: breaches.length ? 1 : 0, file, doc };
}

/* node:coverage disable */
if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  main(process.argv.slice(2)).then(r => process.exit(r.code), e => { console.error(e); process.exit(2); });
}
/* node:coverage enable */
