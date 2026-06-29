#!/usr/bin/env node
// G5 — License / forbidden-dep / env-classification boundary (tooling-integration-eval, ledger #30).
//
// Cross-cutting fence so the council DEFER/REJECT triage and the AGPL/PII boundary are MECHANICAL,
// not reviewer memory (fixes Breaker H2/M3 + RE-ATTACK RA-1/RA-2). Three checks, all `process.exit(1)`:
//
//   (a) LICENSE — scan installed THIRD-PARTY packages; deny AGPL-*/GPL-2.0*/GPL-3.0* (LGPL is NOT
//       denied — exact SPDX IDs, no substring matching, OR/AND aware). First-party `private` workspace
//       packages are exempt (no license field by construction). A missing/unparseable third-party
//       license is cleared ONLY by a reviewed row in compliance/license-exceptions.md.
//   (b) FORBIDDEN-DEP — every ADR DEFER/REJECT (langgraph/@workos-inc/skyvern/recall/moneyprinter/
//       glossopetrae) blocks if present in the lockfile or as a source import (AGPL sidecars like
//       Skyvern live OUT of tree, reached over HTTP — never a dependency).
//   (c) ENV-CLASSIFICATION — every `*_(URL|KEY|TOKEN|SECRET|DSN|ENDPOINT)` env in packages/config must
//       be classified internal|external-subprocessor in compliance/env-classification.md; external →
//       its subprocessor must be in compliance/subprocessors.md. Fail-closed on any unclassified env.
//
// Run: node scripts/guardrail-license.mjs
import { readFileSync, readdirSync, existsSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';

const ROOT = process.cwd();
const errors = [];
const info = [];

// ───────────────────────── (a) LICENSE ─────────────────────────
const DENY = (id) => {
  const s = id.trim().toUpperCase().replace(/\+$/, '');
  if (s.startsWith('LGPL')) return false;                 // LGPL is permissive-enough — NOT denied (RA-2)
  return /^AGPL(-|$)/.test(s) || /^GPL-2/.test(s) || /^GPL-3/.test(s) || s === 'GPL';
};
// Evaluate an SPDX expression: deny only if the WHOLE expression is unavoidably copyleft.
// OR → ok if any disjunct ok; AND → deny if any conjunct denied; single → deny if denied.
function spdxDenied(expr) {
  if (!expr) return null;                                  // null = missing/unparseable
  const e = expr.replace(/[()]/g, ' ').trim();
  if (/\bOR\b/i.test(e)) return e.split(/\bOR\b/i).every((d) => spdxDenied(d) === true);
  if (/\bAND\b/i.test(e)) return e.split(/\bAND\b/i).some((c) => spdxDenied(c) === true);
  const id = e.split(/\bWITH\b/i)[0].trim();              // honour `WITH <exception>` — base ID decides
  if (!id) return null;
  return DENY(id);
}
function licenseOf(pkgJsonPath) {
  try {
    const j = JSON.parse(readFileSync(pkgJsonPath, 'utf8'));
    if (typeof j.license === 'string') return j.license;
    if (j.license && typeof j.license.type === 'string') return j.license.type;
    if (Array.isArray(j.licenses) && j.licenses[0]?.type) return j.licenses.map((l) => l.type).join(' OR ');
    return null;
  } catch { return null; }
}
function parseExceptions() {
  const f = join(ROOT, 'compliance/license-exceptions.md');
  const set = new Set();
  if (!existsSync(f)) return set;
  for (const line of readFileSync(f, 'utf8').split('\n')) {
    const m = line.match(/^\|\s*([^|\s][^|]*@[^|\s]+)\s*\|/);
    if (m) set.add(m[1].trim());
  }
  return set;
}
const exceptions = parseExceptions();

// Installed third-party closure = node_modules/.pnpm/<pkg@ver>/node_modules/<pkg>/package.json
function scanPnpm() {
  const base = join(ROOT, 'node_modules/.pnpm');
  if (!existsSync(base)) { info.push('license: node_modules/.pnpm absent — run pnpm install (skipped).'); return; }
  const seen = new Set();                                   // dedupe pkg@ver across .pnpm peers
  for (const entry of readdirSync(base)) {
    if (entry === 'node_modules' || entry.startsWith('.')) continue;
    const inner = join(base, entry, 'node_modules');
    if (!existsSync(inner)) continue;
    for (const name of readdirSync(inner)) {
      const dirs = name.startsWith('@')
        ? readdirSync(join(inner, name)).map((s) => `${name}/${s}`)
        : [name];
      for (const pkgName of dirs) {
        const pj = join(inner, pkgName, 'package.json');
        if (!existsSync(pj)) continue;
        let j; try { j = JSON.parse(readFileSync(pj, 'utf8')); } catch { continue; }
        if (j.private === true) continue;                  // first-party-style/private — exempt
        const ver = j.version || entry.split('@').pop();
        const id = `${pkgName}@${ver}`;
        if (seen.has(id)) continue;
        seen.add(id);
        const lic = licenseOf(pj);
        const denied = spdxDenied(lic);
        if (denied === true) errors.push(`LICENSE: ${id} is copyleft (${lic}) — denied. Remove it or fence as an out-of-tree HTTP sidecar.`);
        else if (denied === null && !exceptions.has(id)) errors.push(`LICENSE: ${id} has no parseable license — add a reviewed row to compliance/license-exceptions.md or remove it.`);
      }
    }
  }
}
scanPnpm();

// ───────────────────────── (b) FORBIDDEN-DEP ─────────────────────────
const FORBIDDEN = [
  'langgraph', '@langchain/langgraph', '@langchain/', 'langchain', '@workos-inc/',
  'skyvern', 'recall', 'moneyprinter', 'glossopetrae',
  // Out-of-tree harness pilots (2026-06-29): DeerFlow (LangChain/LangGraph-based — already DEFERRED),
  // Decepticon (offensive red-team), EvoMap (external agent-capability network). All reached only
  // out-of-band; NEVER a product dependency or source import. See docs/research/{deerflow,decepticon,evomap}-pilot.md.
  'deer-flow', 'deerflow', 'decepticon', 'evomap', '@evomap/',
  // STORM (research synthesis, PyPI `knowledge-storm`) + Scrapling (scraper) + dspy (STORM's framework):
  // out-of-tree harness pilots only (docs/research/storm-scrapling-pilot.md). Specific tokens (not bare
  // "storm") to avoid false-positives on unrelated packages.
  'knowledge-storm', 'scrapling', 'dspy',
];
const lockPath = join(ROOT, 'pnpm-lock.yaml');
if (existsSync(lockPath)) {
  const lock = readFileSync(lockPath, 'utf8');
  for (const f of FORBIDDEN) {
    // match a package key in the lock (e.g. "  skyvern@" / "  @workos-inc/...")
    const re = new RegExp(`(^|/|\\s)${f.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}[@/]`, 'm');
    if (re.test(lock)) errors.push(`FORBIDDEN-DEP: '${f}' resolved in pnpm-lock.yaml — an ADR DEFER/REJECT dependency. Remove it (Skyvern is an out-of-tree sidecar, never a dep).`);
  }
}
// source imports
const SRC_ROOTS = ['apps', 'packages'];
const SKIP = new Set(['node_modules', 'dist', '.git', 'coverage']);
function scanImports(dir) {
  if (!existsSync(dir)) return;
  for (const name of readdirSync(dir)) {
    if (SKIP.has(name)) continue;
    const p = join(dir, name);
    let s; try { s = statSync(p); } catch { continue; }
    if (s.isDirectory()) { scanImports(p); continue; }
    if (!/\.(ts|tsx|js|jsx|mjs|cjs)$/.test(p)) continue;
    const txt = readFileSync(p, 'utf8');
    for (const f of FORBIDDEN) {
      const re = new RegExp(`(?:from|import|require)\\s*\\(?\\s*['"]${f.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`);
      if (re.test(txt)) errors.push(`FORBIDDEN-DEP: ${p.replace(ROOT + '/', '')} imports '${f}' (ADR DEFER/REJECT).`);
    }
  }
}
SRC_ROOTS.forEach((r) => scanImports(resolve(ROOT, r)));

// ───────────────────────── (c) ENV-CLASSIFICATION ─────────────────────────
const ENV_RE = /\b([A-Z][A-Z0-9_]*_(?:URL|KEY|TOKEN|SECRET|DSN|ENDPOINT))\b/g;
const configFile = join(ROOT, 'packages/config/src/index.ts');
const matchedEnvs = new Set();
if (existsSync(configFile)) {
  const txt = readFileSync(configFile, 'utf8');
  for (const m of txt.matchAll(ENV_RE)) matchedEnvs.add(m[1]);
}
// parse the classification manifest
const manifest = new Map();
const manFile = join(ROOT, 'compliance/env-classification.md');
if (existsSync(manFile)) {
  for (const line of readFileSync(manFile, 'utf8').split('\n')) {
    const m = line.match(/^\|\s*([A-Z][A-Z0-9_]+)\s*\|\s*(internal|external-subprocessor)\s*\|\s*([^|]*)\|/);
    if (m) manifest.set(m[1], { cls: m[2], sub: m[3].trim() });
  }
} else {
  errors.push('ENV: compliance/env-classification.md missing.');
}
const subprocessors = existsSync(join(ROOT, 'compliance/subprocessors.md'))
  ? readFileSync(join(ROOT, 'compliance/subprocessors.md'), 'utf8') : '';
for (const env of [...matchedEnvs].sort()) {
  const row = manifest.get(env);
  if (!row) { errors.push(`ENV: '${env}' is unclassified — add a row to compliance/env-classification.md (internal | external-subprocessor).`); continue; }
  if (row.cls === 'external-subprocessor') {
    if (!row.sub || row.sub === '—') errors.push(`ENV: '${env}' is external-subprocessor but names no subprocessor.`);
    else if (!subprocessors.includes(row.sub)) errors.push(`ENV: '${env}' → subprocessor '${row.sub}' is not registered in compliance/subprocessors.md.`);
  }
}

// ───────────────────────── verdict ─────────────────────────
for (const i of info) console.log('  · ' + i);
if (errors.length) {
  console.error(`✗ guardrail-license: ${errors.length} violation(s):`);
  for (const e of errors) console.error('  - ' + e);
  process.exit(1);
}
console.log(`✓ guardrail-license: no copyleft/forbidden third-party deps; all ${matchedEnvs.size} service envs classified + registered.`);
