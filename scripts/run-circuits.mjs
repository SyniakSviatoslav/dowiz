#!/usr/bin/env node
// run-circuits.mjs — the mechanical circuit runner (KNOWLEDGE-AS-CIRCUITS).
//
// Turns the circuit registry (docs/operating-model/circuits/registry.json) — the machine-readable
// form of our error-patterns / lessons / design-rules / library-best-practices — into a deterministic
// gate. Given file paths (or, with --staged, the git-staged set), it checks each file against every
// circuit whose glob matches and prints violations. Exit 2 if any RED-LINE circuit trips, exit 1 for
// warn-level, exit 0 clean. No reasoning, no skills — a pattern either matches or it does not.
//
// Circuit shapes (registry.json .circuits[]):
//   { id, source, severity: "red-line"|"warn", glob, type, message, ... }
//   type "forbid":          `pattern` (regex) must NOT appear.
//   type "require_together": if `pattern` appears, `required` (regex) must ALSO appear in the file.
import { readFileSync, existsSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { join } from 'node:path';

const ROOT = (() => { try { return execSync('git rev-parse --show-toplevel', { stdio: ['ignore', 'pipe', 'ignore'] }).toString().trim(); } catch { return process.cwd(); } })();
const REG = join(ROOT, 'docs/operating-model/circuits/registry.json');

// glob -> regex: escape literal parts, `**` -> `.*`, `*` -> `[^/]*`.
const escLit = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const globToRe = (g) => new RegExp('^' + g.split('**').map((seg) => seg.split('*').map(escLit).join('[^/]*')).join('.*') + '$');

function files() {
  const args = process.argv.slice(2).filter((a) => a !== '--staged');
  if (process.argv.includes('--staged')) {
    try { return execSync('git diff --cached --name-only --diff-filter=ACM', { cwd: ROOT }).toString().split('\n').filter(Boolean); } catch { return []; }
  }
  return args;
}

// Strip line/doc comments so a circuit never trips on prose that merely MENTIONS a forbidden token
// (e.g. money.rs doc-comments explaining why it avoids f64). Heuristic, per-language line comments.
function stripComments(text, rel) {
  if (/\.(rs|ts|tsx|js|jsx|mjs|cjs)$/.test(rel)) return text.replace(/\/\/.*$/gm, '');
  if (/\.sql$/.test(rel)) return text.replace(/--.*$/gm, '');
  return text;
}

if (!existsSync(REG)) { console.log('OK run-circuits: no registry yet (docs/operating-model/circuits/registry.json) — nothing to enforce.'); process.exit(0); }
const circuits = JSON.parse(readFileSync(REG, 'utf8')).circuits || [];
const targets = files();
const violations = [];

for (const rel of targets) {
  const abs = join(ROOT, rel);
  if (!existsSync(abs)) continue;
  let text; try { text = stripComments(readFileSync(abs, 'utf8'), rel); } catch { continue; }
  for (const c of circuits) {
    if (!globToRe(c.glob).test(rel)) continue;
    const pat = new RegExp(c.pattern, c.flags || 'm');
    if (c.type === 'forbid') {
      if (pat.test(text)) violations.push({ c, rel });
    } else if (c.type === 'require_together') {
      if (pat.test(text) && !new RegExp(c.required, c.flags || 'm').test(text)) violations.push({ c, rel });
    }
  }
}

if (violations.length === 0) { console.log(`OK run-circuits: ${targets.length} file(s) clean against ${circuits.length} circuit(s).`); process.exit(0); }

let red = 0;
console.error(`FAIL run-circuits: ${violations.length} circuit violation(s):`);
for (const { c, rel } of violations) {
  const tag = c.severity === 'red-line' ? 'RED-LINE' : 'warn';
  if (c.severity === 'red-line') red++;
  console.error(`  [${tag}] [${c.id}] ${rel}: ${c.message}  (source: ${c.source})`);
}
console.error('\nThese are mechanical circuits from docs/operating-model/circuits/registry.json — fix the code, do not weaken the circuit.');
process.exit(red > 0 ? 2 : 1);
