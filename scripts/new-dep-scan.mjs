#!/usr/bin/env node
// new-dep-scan — detect newly-added libs across the workspace so the plane-maintainer can
// reverse-engineer each one (charter: docs/governance/plane-maintainer-agent.md, SCOUT step).
// Collects every dependency from all workspace package.json files, diffs against the recorded
// baseline (loops/runs/dep-baseline.json), and reports newcomers. Deterministic, zero-dep.
//
// Run:  node scripts/new-dep-scan.mjs            (report newcomers vs baseline)
//       node scripts/new-dep-scan.mjs --json      (machine form)
//       node scripts/new-dep-scan.mjs --bump      (write current set as the new baseline)
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { join } from 'node:path';

const ROOT = process.cwd();
const BASELINE = join(ROOT, 'loops/runs/dep-baseline.json');
const BUMP = process.argv.includes('--bump');
const JSON_OUT = process.argv.includes('--json');

// Ignore first-party workspace packages and the vendored skill tree (not product/plane deps).
const IGNORE_PREFIX = ['@dowiz/', '@deliveryos/'];
const IGNORE_PKG_PATH = /^\.claude\/skills\//;

const pkgFiles = execSync('git ls-files "**/package.json" package.json', { cwd: ROOT, encoding: 'utf8' })
  .split('\n').filter(Boolean).filter((p) => !IGNORE_PKG_PATH.test(p));

const current = {}; // name -> { version, sites: [] }
for (const f of pkgFiles) {
  let json;
  try { json = JSON.parse(readFileSync(join(ROOT, f), 'utf8')); } catch { continue; }
  for (const field of ['dependencies', 'devDependencies', 'optionalDependencies', 'peerDependencies']) {
    for (const [name, version] of Object.entries(json[field] || {})) {
      if (IGNORE_PREFIX.some((p) => name.startsWith(p))) continue;
      if (!current[name]) current[name] = { version, sites: [] };
      current[name].sites.push(f);
    }
  }
}
const currentNames = Object.keys(current).sort();

const baseline = existsSync(BASELINE) ? JSON.parse(readFileSync(BASELINE, 'utf8')) : { deps: [] };
const baseSet = new Set(baseline.deps || []);
const newcomers = currentNames.filter((n) => !baseSet.has(n));
const removed = (baseline.deps || []).filter((n) => !current[n]);

if (BUMP) {
  writeFileSync(BASELINE, JSON.stringify({ generated: new Date().toISOString(), deps: currentNames }, null, 2));
  console.log(`[new-dep-scan] baseline bumped: ${currentNames.length} deps recorded.`);
  process.exit(0);
}

const payload = { total: currentNames.length, baselineExists: existsSync(BASELINE), newcomers: newcomers.map((n) => ({ name: n, version: current[n].version, sites: current[n].sites })), removed };
if (JSON_OUT) { console.log(JSON.stringify(payload, null, 2)); process.exit(0); }

if (!existsSync(BASELINE)) {
  console.log(`[new-dep-scan] no baseline yet — ${currentNames.length} deps in tree. Run --bump to record the first baseline (then future newcomers get reverse-engineered).`);
  process.exit(0);
}
console.log(`[new-dep-scan] ${currentNames.length} deps · ${newcomers.length} NEW · ${removed.length} removed`);
if (newcomers.length) {
  console.log('\n  NEW libs to reverse-engineer (per the 12-rule tooling grammar), then --bump:');
  for (const n of newcomers) console.log(`   → ${n}@${current[n].version}  [${current[n].sites.join(', ')}]`);
}
if (removed.length) console.log(`\n  removed since baseline: ${removed.join(', ')}`);
