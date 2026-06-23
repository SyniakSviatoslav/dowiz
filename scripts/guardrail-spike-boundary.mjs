#!/usr/bin/env node
// Guardrail (Agent Operating Model 🔴 "two speeds, one boundary"): recon-speed code in `spikes/`
// must NEVER cross into execution-speed code. Concretely: no file under apps/ or packages/ may
// import from `spikes/`. A spike's *evidence* informs a build; its *code* is thrown away — an
// import is the boundary breaking. Deterministic backstop so the sacred boundary can't erode.
//
// Run: node scripts/guardrail-spike-boundary.mjs
import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const ROOTS = ['apps', 'packages'];
// import ... from 'spikes...' | "../spikes/..." | require('…/spikes/…') | dynamic import('…spikes…')
const SPIKE_IMPORT = /(from\s+|import\s*\(|require\s*\()\s*['"][^'"]*(^|\/|\b)spikes\//;
const violations = [];

function walk(dir) {
  if (!existsSync(dir)) return;
  for (const name of readdirSync(dir)) {
    if (name === 'node_modules' || name === 'dist') continue;
    const p = join(dir, name);
    const s = statSync(p);
    if (s.isDirectory()) walk(p);
    else if (/\.(ts|tsx|js|jsx|mjs)$/.test(p)) scan(p);
  }
}

function scan(file) {
  readFileSync(file, 'utf8').split('\n').forEach((line, i) => {
    if (/['"][^'"]*spikes\//.test(line) && /(from\s|import\s*\(|require\s*\()/.test(line)) {
      violations.push(`${file}:${i + 1}: ${line.trim().slice(0, 120)}`);
    }
  });
}

ROOTS.forEach(walk);

if (violations.length) {
  console.error(`✗ guardrail-spike-boundary: ${violations.length} import(s) from spikes/ into execution code (boundary violation):`);
  for (const v of violations) console.error('  ' + v);
  console.error('\nRecon output never crosses into apps/packages as code. Re-implement under a `build` prompt; throw the spike away.');
  process.exit(1);
}
console.log('✓ guardrail-spike-boundary: no apps/packages import from spikes/ (two speeds, one boundary).');
