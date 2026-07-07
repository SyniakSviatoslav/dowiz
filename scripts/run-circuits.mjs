#!/usr/bin/env node
// run-circuits.mjs — the mechanical circuit runner (KNOWLEDGE-AS-CIRCUITS).
//
// Turns the circuit registry (docs/operating-model/circuits/registry.json) — the machine-readable
// form of our error-patterns / lessons / design-rules / library-best-practices — into a deterministic
// gate. Given file paths (or, with --staged, the git-staged set), it checks each file against every
// circuit whose glob matches and prints violations. No reasoning, no skills — a pattern either
// matches or it does not.
//
// EXIT CODES:
//   2  a RED-LINE circuit tripped (blocks the commit).
//   1  only warn-level circuits tripped   (default CLI: surfaces as a soft fail).
//   0  clean  — OR, with --warn-ok, warn-level violations are printed but NOT failed (only red-line
//      blocks). run-armaments.sh uses --warn-ok so a warn circuit never over-blocks a harness commit
//      (the whole point of the anti-over-block discipline: red-line has teeth, warn advises).
//
// MODES:
//   node scripts/run-circuits.mjs <file>...        check the given files
//   node scripts/run-circuits.mjs --staged         check the git-staged (ACM) set
//   node scripts/run-circuits.mjs --self-test       hermetic proof of the engine (no product files)
//
// Circuit shapes (registry.json .circuits[]):
//   { id, source, severity: "red-line"|"warn", glob, type, message, ... }
//   type "forbid":          `pattern` (regex) must NOT appear.
//   type "require_together": if `pattern` appears, `required` (regex) must ALSO appear in the file.
import { readFileSync, existsSync, mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { join, dirname } from 'node:path';
import { tmpdir } from 'node:os';

const ROOT = (() => { try { return execSync('git rev-parse --show-toplevel', { stdio: ['ignore', 'pipe', 'ignore'] }).toString().trim(); } catch { return process.cwd(); } })();
const REG = process.env.CIRCUITS_REGISTRY || join(ROOT, 'docs/operating-model/circuits/registry.json');

// glob -> regex. Proper globstar: `**/` matches ZERO or more path segments (so `loops/**/*.yaml`
// matches BOTH `loops/x.yaml` and `loops/sub/x.yaml`), trailing `**` matches anything, `*` matches
// within a segment. (The old form compiled `**/` to `.*/` which REQUIRED an intermediate dir, so a
// glob like `loops/**/*.yaml` silently skipped top-level files — a circuit that could never fire.)
const escLit = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const globToRe = (g) => {
  let re = '';
  for (let i = 0; i < g.length; i++) {
    if (g[i] === '*' && g[i + 1] === '*') {
      if (g[i + 2] === '/') { re += '(?:.*/)?'; i += 2; } else { re += '.*'; i += 1; }
    } else if (g[i] === '*') { re += '[^/]*'; }
    else { re += escLit(g[i]); }
  }
  return new RegExp('^' + re + '$');
};

// Strip line/doc comments so a circuit never trips on prose that merely MENTIONS a forbidden token
// (e.g. money.rs doc-comments explaining why it avoids f64). Heuristic, per-language line comments.
function stripComments(text, rel) {
  if (/\.(rs|ts|tsx|js|jsx|mjs|cjs)$/.test(rel)) return text.replace(/\/\/.*$/gm, '');
  if (/\.sql$/.test(rel)) return text.replace(/--.*$/gm, '');
  return text;
}

// Pure check: given circuits + a list of {rel, text}, return violations. No I/O.
function checkContents(circuits, contents) {
  const violations = [];
  for (const { rel, text: raw } of contents) {
    const text = stripComments(raw, rel);
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
  return violations;
}

function stagedFiles() {
  try { return execSync('git diff --cached --name-only --diff-filter=ACM', { cwd: ROOT }).toString().split('\n').filter(Boolean); } catch { return []; }
}

function report(violations, { warnOk }) {
  if (violations.length === 0) { console.log('OK run-circuits: clean against the registry.'); return 0; }
  let red = 0;
  console.error(`FAIL run-circuits: ${violations.length} circuit violation(s):`);
  for (const { c, rel } of violations) {
    const tag = c.severity === 'red-line' ? 'RED-LINE' : 'warn';
    if (c.severity === 'red-line') red++;
    console.error(`  [${tag}] [${c.id}] ${rel}: ${c.message}  (source: ${c.source})`);
  }
  console.error('\nThese are mechanical circuits from docs/operating-model/circuits/registry.json — fix the code, do not weaken the circuit.');
  if (red > 0) return 2;
  return warnOk ? 0 : 1;
}

// ── --self-test: hermetic proof the engine detects red-line/warn/require_together and stays clean ──
function selfTest() {
  const circuits = [
    { id: 't-forbid-red', source: 'self-test', severity: 'red-line', glob: 'x/**/*.rs', type: 'forbid', pattern: '\\bf64\\b', message: 'no float' },
    { id: 't-forbid-warn', source: 'self-test', severity: 'warn', glob: 'x/**/*.ts', type: 'forbid', pattern: '\\bas any\\b', message: 'no any' },
    { id: 't-require', source: 'self-test', severity: 'red-line', glob: 'x/**/*.sql', type: 'require_together', pattern: 'ENABLE ROW LEVEL SECURITY', required: 'FORCE ROW LEVEL SECURITY', flags: 'mi', message: 'force rls' },
  ];
  const failures = [];
  const ck = (name, ok) => { if (ok) console.log(`  ✓ ${name}`); else { console.error(`  ✗ ${name}`); failures.push(name); } };

  // globstar: `x/**/*.rs` matches both a top-level `x/a.rs` and a nested `x/d/a.rs`.
  ck('red-line forbid in a TOP-LEVEL file (globstar zero-segment) → exit 2', report(checkContents(circuits, [{ rel: 'x/a.rs', text: 'let v: f64 = 1.0;' }]), { warnOk: true }) === 2);
  // red-line forbid trips; comment-only mention does NOT.
  ck('red-line forbid pattern in .rs (nested) → exit 2', report(checkContents(circuits, [{ rel: 'x/d/a.rs', text: 'let v: f64 = 1.0;' }]), { warnOk: true }) === 2);
  ck('same token only in a // comment → clean (exit 0)', report(checkContents(circuits, [{ rel: 'x/d/a.rs', text: '// never use f64 here' }]), { warnOk: true }) === 0);
  // warn forbid: exit 1 by default, exit 0 with --warn-ok (advisory), never red.
  ck('warn forbid pattern → exit 1 default', report(checkContents(circuits, [{ rel: 'x/d/a.ts', text: 'const x = y as any;' }]), { warnOk: false }) === 1);
  ck('warn forbid pattern → exit 0 with --warn-ok (advisory, no over-block)', report(checkContents(circuits, [{ rel: 'x/d/a.ts', text: 'const x = y as any;' }]), { warnOk: true }) === 0);
  // require_together: ENABLE without FORCE trips; both present is clean.
  ck('ENABLE RLS without FORCE → exit 2', report(checkContents(circuits, [{ rel: 'x/d/m.sql', text: 'ALTER TABLE t ENABLE ROW LEVEL SECURITY;' }]), { warnOk: true }) === 2);
  ck('ENABLE + FORCE RLS together → clean', report(checkContents(circuits, [{ rel: 'x/d/m.sql', text: 'ENABLE ROW LEVEL SECURITY; FORCE ROW LEVEL SECURITY;' }]), { warnOk: true }) === 0);
  // glob scoping: a non-matching path is untouched.
  ck('non-matching glob → clean', report(checkContents(circuits, [{ rel: 'other/d/a.rs', text: 'let v: f64 = 1.0;' }]), { warnOk: true }) === 0);

  // and prove it end-to-end against a temp registry file + temp target (exercises file I/O path).
  const tmp = mkdtempSync(join(tmpdir(), 'circuits-selftest-'));
  try {
    const regPath = join(tmp, 'registry.json');
    writeFileSync(regPath, JSON.stringify({ circuits }));
    mkdirSync(join(tmp, 'x/d'), { recursive: true });
    writeFileSync(join(tmp, 'x/d/bad.rs'), 'let v: f64 = 1.0;');
    const r = execSync(`CIRCUITS_REGISTRY='${regPath}' node '${process.argv[1]}' x/d/bad.rs --warn-ok 2>&1; echo "EXIT:$?"`,
      { cwd: tmp, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
    ck('end-to-end: temp registry + temp file → red-line reported', /RED-LINE/.test(r) && /EXIT:2/.test(r));
  } finally { rmSync(tmp, { recursive: true, force: true }); }

  if (failures.length) { console.error(`\n✗ run-circuits --self-test: ${failures.length} case(s) failed.`); process.exit(1); }
  console.log('\n✓ run-circuits --self-test: engine detects red-line/warn/require_together; warn advisory under --warn-ok.');
  process.exit(0);
}

// ── main ──
const argv = process.argv.slice(2);
if (argv.includes('--self-test')) selfTest();

const warnOk = argv.includes('--warn-ok');
if (!existsSync(REG)) { console.log(`OK run-circuits: no registry yet (${REG}) — nothing to enforce.`); process.exit(0); }
const circuits = JSON.parse(readFileSync(REG, 'utf8')).circuits || [];
const targets = argv.includes('--staged') ? stagedFiles() : argv.filter((a) => !a.startsWith('--'));
const contents = [];
for (const rel of targets) {
  const abs = join(ROOT, rel);
  if (!existsSync(abs)) continue;
  try { contents.push({ rel, text: readFileSync(abs, 'utf8') }); } catch { /* unreadable → skip */ }
}
const violations = checkContents(circuits, contents);
if (violations.length === 0) { console.log(`OK run-circuits: ${contents.length} file(s) clean against ${circuits.length} circuit(s).`); process.exit(0); }
process.exit(report(violations, { warnOk }));
