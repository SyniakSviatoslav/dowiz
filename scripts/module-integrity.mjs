#!/usr/bin/env node
// ─────────────────────────────────────────────────────────────────────────────
//  module-integrity.mjs — the modular-boundary gate (STRUCTURE-UPGRADE A0/A1).
//
//  Deterministic (NO LLM). Enforces the Sovereign Core module topology against
//  REALITY (cargo metadata + the real `use` graph), so `module.toml` manifests
//  can never rot into decoration (the #1 self-critique risk in STRUCTURE-UPGRADE.md).
//
//  Checks:
//    (1) every module.toml parses + matches the schema
//        (name/kind/depends/events_in/events_out/contract/red_line);
//    (2) crate-level manifests (kind=core|shell-adapter) declare `depends` EQUAL to
//        their workspace-internal deps from `cargo metadata` (declared≠actual = red);
//    (3) the core crate (kind=core) carries NONE of the banned heavy/impure
//        production deps (tokio,sqlx,axum,reqwest,rand,chrono,time) — a structural
//        mirror of the wasm32 sovereign gate so the two can't silently drift apart;
//    (4) hub-module manifests: any cross-module `use crate::modules::<other>`
//        import not declared in the importer's `depends` = red (modules talk via
//        events/ports, never each other's internals);
//    (+) every `contract` doc pointer resolves to a real file (dangling = rot).
//
//  Modes:
//    (default)     full gate over the real tree; exit 1 on any violation.
//                  Degrades to a SKIP-with-warning for the cargo-dependent checks
//                  (2)/(3) when `cargo` isn't installed (like sovereign-gate.sh's
//                  cargo-deny) — schema/contract/module checks still run.
//    --self-test   hermetic in-memory fixtures prove the gate DENIES each violation
//                  class AND stays SILENT on the legitimate neighbour (over-block
//                  guard), per the Part-B armament doctrine. No cargo, no fs writes.
//
//  Run:  node scripts/module-integrity.mjs            (real tree)
//        node scripts/module-integrity.mjs --self-test
// ─────────────────────────────────────────────────────────────────────────────
import { readFileSync, existsSync, readdirSync, statSync } from 'node:fs';
import { join, dirname, basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execSync } from 'node:child_process';

const REPO = join(dirname(fileURLToPath(import.meta.url)), '..');
const REBUILD = join(REPO, 'rebuild');

// The banned PRODUCTION deps for kind=core — the direct-dep analog of the wasm32
// sovereign build (0b-6 ban list). Dev-deps are exempt (proptest→rand etc.), exactly
// like the sovereign gate builds `--lib` only.
const CORE_BAN = ['tokio', 'sqlx', 'axum', 'reqwest', 'rand', 'chrono', 'time'];
const KINDS = ['core', 'shell-adapter', 'hub-module'];
const REQUIRED = ['name', 'kind', 'depends', 'events_in', 'events_out', 'contract', 'red_line'];

// ── Minimal TOML-subset parser (flat key=value; string/bool/string-array; #comments;
//    single- or multi-line arrays). Purpose-built for the controlled manifest schema so
//    the gate stays dependency-free (a new npm dep would trip guardrail-license.mjs). ──
function parseValue(raw, key) {
  const v = raw.trim();
  if (v === 'true') return true;
  if (v === 'false') return false;
  if (v.startsWith('"') && v.endsWith('"') && v.length >= 2) return v.slice(1, -1);
  if (v.startsWith('[') && v.endsWith(']')) {
    const inner = v.slice(1, -1).trim();
    if (inner === '') return [];
    return inner
      .split(',')
      .map((s) => s.trim())
      .filter((s) => s !== '')
      .map((s) => {
        if (s.startsWith('"') && s.endsWith('"') && s.length >= 2) return s.slice(1, -1);
        throw new Error(`array element not a quoted string in '${key}': ${s}`);
      });
  }
  throw new Error(`unsupported value for '${key}': ${raw}`);
}

// Strip an inline `#` comment that sits OUTSIDE a double-quoted string (quote-aware, so a `#`
// inside a value is preserved and a trailing `# note` is dropped). Values always precede comments
// in our schema, so the in-string toggle state at the first bare `#` is always correct.
function stripComment(s) {
  let inStr = false;
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (c === '"') inStr = !inStr;
    else if (c === '#' && !inStr) return s.slice(0, i);
  }
  return s;
}

export function parseToml(text) {
  const out = {};
  const lines = text.split(/\r?\n/);
  let i = 0;
  while (i < lines.length) {
    const line = stripComment(lines[i]);
    const trimmed = line.trim();
    i++;
    if (trimmed === '') continue; // blank or comment-only line
    if (trimmed.startsWith('[')) throw new Error(`unsupported section header (manifests are flat): ${trimmed}`);
    const eq = line.indexOf('=');
    if (eq === -1) throw new Error(`malformed line (no '='): ${trimmed}`);
    const key = line.slice(0, eq).trim();
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(key)) throw new Error(`malformed key: ${key}`);
    let rest = line.slice(eq + 1).trim();
    // accumulate a multi-line array until the closing bracket (each line comment-stripped)
    if (rest.startsWith('[') && !rest.includes(']')) {
      while (i < lines.length && !rest.includes(']')) {
        rest += ' ' + stripComment(lines[i]).trim();
        i++;
      }
    }
    out[key] = parseValue(rest, key);
  }
  return out;
}

function validateSchema(m, path) {
  const errs = [];
  for (const k of REQUIRED) if (!(k in m)) errs.push(`${path}: missing required key '${k}'`);
  if ('kind' in m && !KINDS.includes(m.kind)) errs.push(`${path}: kind '${m.kind}' not one of ${KINDS.join('|')}`);
  for (const arrk of ['depends', 'events_in', 'events_out']) {
    if (arrk in m && !Array.isArray(m[arrk])) errs.push(`${path}: '${arrk}' must be an array`);
  }
  if ('red_line' in m && typeof m.red_line !== 'boolean') errs.push(`${path}: 'red_line' must be a boolean`);
  if ('contract' in m && typeof m.contract !== 'string') errs.push(`${path}: 'contract' must be a string`);
  if ('name' in m && typeof m.name !== 'string') errs.push(`${path}: 'name' must be a string`);
  return errs;
}

// ── The pure gate. All inputs are plain data so --self-test can feed synthetic
//    fixtures with no cargo and no filesystem. ──
//   manifests : [{ path, moduleName?, data }]
//   workspace : { packages: [{ name, internalDeps:Set, normalDeps:Set }] } | null (null=cargo skipped)
//   moduleUses: [{ module, imports:[moduleName...] }]
//   fileExists: (repoRelPath) => bool
export function checkIntegrity({ manifests, workspace, moduleUses = [], fileExists }) {
  const errors = [];

  for (const man of manifests) errors.push(...validateSchema(man.data, man.path));

  // (+) contract pointer must resolve
  for (const man of manifests) {
    const c = man.data.contract;
    if (typeof c === 'string' && c && !fileExists(c)) {
      errors.push(`${man.path}: contract '${c}' does not resolve to a file (dangling pointer — manifest rot)`);
    }
  }

  // (2)/(3) crate-level manifests vs cargo metadata
  if (workspace) {
    const byName = new Map(workspace.packages.map((p) => [p.name, p]));
    for (const man of manifests) {
      const { kind, name, depends } = man.data;
      if (kind !== 'core' && kind !== 'shell-adapter') continue;
      const pkg = byName.get(name);
      if (!pkg) {
        errors.push(`${man.path}: name '${name}' is not a workspace package (cargo metadata)`);
        continue;
      }
      const declared = new Set(Array.isArray(depends) ? depends : []);
      const actual = pkg.internalDeps;
      const missing = [...actual].filter((d) => !declared.has(d));
      const extra = [...declared].filter((d) => !actual.has(d));
      if (missing.length) errors.push(`${man.path}: depends is missing ${missing.join(', ')} (present in cargo metadata) — declared≠actual`);
      if (extra.length) errors.push(`${man.path}: depends declares ${extra.join(', ')} absent from cargo metadata — declared≠actual`);
      if (kind === 'core') {
        const banned = [...pkg.normalDeps].filter((d) => CORE_BAN.includes(d));
        if (banned.length) {
          errors.push(`${man.path}: core crate has BANNED production dep(s) ${banned.join(', ')} — impurity in dowiz-core (mirror of the wasm32 sovereign gate)`);
        }
      }
    }
  }

  // (4) hub-module cross-import boundary
  const moduleManifests = new Map(
    manifests.filter((m) => m.data.kind === 'hub-module' && m.moduleName).map((m) => [m.moduleName, m]),
  );
  for (const u of moduleUses) {
    const man = moduleManifests.get(u.module);
    const declared = new Set(man && Array.isArray(man.data.depends) ? man.data.depends : []);
    for (const imp of u.imports) {
      if (imp === u.module) continue; // self-reference is fine
      if (!declared.has(imp)) {
        errors.push(
          `module '${u.module}' imports crate::modules::${imp} internals but does not declare '${imp}' in depends — modules communicate via events/ports, not each other's internals`,
        );
      }
    }
  }

  return errors;
}

// ── Real-tree gathering ──
function deriveWorkspace(meta) {
  const wsNames = new Set(meta.packages.map((p) => p.name));
  const packages = meta.packages.map((p) => {
    const internalDeps = new Set();
    const normalDeps = new Set();
    for (const d of p.dependencies) {
      if (d.path || wsNames.has(d.name)) internalDeps.add(d.name);
      if (d.kind === null || d.kind === undefined) normalDeps.add(d.name); // normal (non-dev/non-build)
    }
    return { name: p.name, internalDeps, normalDeps };
  });
  return { packages };
}

function hasCargo() {
  try {
    execSync('cargo --version', { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}

function gatherManifests() {
  const manifests = [];
  const rel = (abs) => abs.slice(REPO.length + 1);
  for (const p of ['rebuild/crates/domain/module.toml', 'rebuild/crates/api/module.toml']) {
    const abs = join(REPO, p);
    if (existsSync(abs)) manifests.push({ path: p, data: parseToml(readFileSync(abs, 'utf8')) });
  }
  const modulesDir = join(REBUILD, 'crates/api/src/modules');
  if (existsSync(modulesDir)) {
    for (const entry of readdirSync(modulesDir)) {
      const manPath = join(modulesDir, entry, 'module.toml');
      if (existsSync(manPath) && statSync(join(modulesDir, entry)).isDirectory()) {
        manifests.push({ path: rel(manPath), moduleName: entry, data: parseToml(readFileSync(manPath, 'utf8')) });
      }
    }
  }
  return manifests;
}

function scanModuleUses() {
  const modulesDir = join(REBUILD, 'crates/api/src/modules');
  if (!existsSync(modulesDir)) return [];
  const uses = [];
  for (const mod of readdirSync(modulesDir)) {
    const dir = join(modulesDir, mod);
    if (!statSync(dir).isDirectory()) continue;
    const imports = new Set();
    const walk = (d) => {
      for (const e of readdirSync(d)) {
        const abs = join(d, e);
        const st = statSync(abs);
        if (st.isDirectory()) walk(abs);
        else if (e.endsWith('.rs')) {
          const src = readFileSync(abs, 'utf8');
          for (const m of src.matchAll(/crate\s*::\s*modules\s*::\s*(\w+)/g)) imports.add(m[1]);
        }
      }
    };
    walk(dir);
    uses.push({ module: mod, imports: [...imports] });
  }
  return uses;
}

function runRealTree() {
  const manifests = gatherManifests();
  if (manifests.length === 0) {
    console.error('✗ module-integrity: no module.toml manifests found (expected at least the crate-level manifests).');
    process.exit(1);
  }
  let workspace = null;
  let degraded = null;
  if (!hasCargo()) {
    degraded = 'cargo not installed';
  } else {
    try {
      const raw = execSync('cargo metadata --no-deps --format-version 1', {
        cwd: REBUILD,
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'ignore'],
        maxBuffer: 64 * 1024 * 1024,
      });
      workspace = deriveWorkspace(JSON.parse(raw));
    } catch (e) {
      console.error('✗ module-integrity: `cargo metadata` failed (broken Cargo.toml?):', e.message.split('\n')[0]);
      process.exit(1);
    }
  }
  const moduleUses = scanModuleUses();
  const fileExists = (rel) => existsSync(join(REPO, rel));
  const errors = checkIntegrity({ manifests, workspace, moduleUses, fileExists });

  if (errors.length) {
    console.error(`✗ module-integrity: ${errors.length} boundary violation(s):`);
    for (const e of errors) console.error('  - ' + e);
    console.error('\nManifests must match the real dependency graph — fix the manifest or the code, never delete the gate.');
    process.exit(1);
  }
  const modCount = manifests.filter((m) => m.data.kind === 'hub-module').length;
  console.log(
    `✓ module-integrity: ${manifests.length} manifest(s) match reality` +
      (modCount ? ` (${modCount} hub-module(s), cross-import boundary clean)` : '') +
      (degraded ? ` — ⚠ crate-dep checks SKIPPED (${degraded}); CI must run with cargo installed` : ''),
  );
}

// ── Hermetic self-test (armament): each violation DENIES, each neighbour is SILENT ──
function selfTest() {
  let fail = 0;
  const ok = (name, cond) => {
    console.log(`  ${cond ? '✓' : '✗'} ${name}`);
    if (!cond) fail++;
  };
  const denies = (name, input) => ok(name, checkIntegrity(input).length > 0);
  const allows = (name, input) => {
    const errs = checkIntegrity(input);
    ok(name, errs.length === 0);
    if (errs.length) errs.forEach((e) => console.log(`      unexpected: ${e}`));
  };

  const CORE = { name: 'dowiz-core', kind: 'core', depends: [], events_in: [], events_out: [], contract: 'rebuild/README.md', red_line: true };
  const API = { name: 'api', kind: 'shell-adapter', depends: ['dowiz-core'], events_in: [], events_out: [], contract: 'x.md', red_line: false };
  const ws = { packages: [
    { name: 'dowiz-core', internalDeps: new Set(), normalDeps: new Set(['serde', 'sha2', 'uuid']) },
    { name: 'api', internalDeps: new Set(['dowiz-core']), normalDeps: new Set(['axum', 'sqlx']) },
  ] };
  const allExist = () => true;

  console.log('module-integrity self-test:');

  // parser round-trip
  try {
    const p = parseToml(
      ['# c', 'name = "m"', 'kind = "hub-module"', 'depends = [', '  "a",', '  "b",  # trailing', ']', 'red_line = false', 'events_in = []', 'events_out = []', 'contract = "d.md"'].join('\n'),
    );
    ok('parser: multi-line array + comments + bool', p.name === 'm' && Array.isArray(p.depends) && p.depends.length === 2 && p.depends[1] === 'b' && p.red_line === false);
  } catch (e) {
    ok('parser: multi-line array + comments + bool', false);
    console.log('      ' + e.message);
  }
  ok('parser: rejects section headers', (() => { try { parseToml('[bad]\nx=1'); return false; } catch { return true; } })());

  // DENY cases
  denies('missing required key (no kind)', { manifests: [{ path: 'm', data: { name: 'x', depends: [], events_in: [], events_out: [], contract: 'a', red_line: true } }], workspace: null, fileExists: allExist });
  denies('bad kind value', { manifests: [{ path: 'm', data: { ...CORE, kind: 'widget' } }], workspace: ws, fileExists: allExist });
  denies('crate depends missing (api declares [] but deps on dowiz-core)', { manifests: [{ path: 'm', data: { ...API, depends: [] } }], workspace: ws, fileExists: allExist });
  denies('crate depends extra (declares a ghost dep)', { manifests: [{ path: 'm', data: { ...API, depends: ['dowiz-core', 'ghost'] } }], workspace: ws, fileExists: allExist });
  denies('core carries a banned production dep (chrono)', { manifests: [{ path: 'm', data: CORE }], workspace: { packages: [{ name: 'dowiz-core', internalDeps: new Set(), normalDeps: new Set(['serde', 'chrono']) }] }, fileExists: allExist });
  denies('dangling contract pointer', { manifests: [{ path: 'm', data: CORE }], workspace: ws, fileExists: () => false });
  denies('name is not a workspace package', { manifests: [{ path: 'm', data: { ...CORE, name: 'nope' } }], workspace: ws, fileExists: allExist });
  denies('hub-module imports another module undeclared', { manifests: [{ path: 'm', moduleName: 'a', data: { name: 'a', kind: 'hub-module', depends: [], events_in: [], events_out: [], contract: 'a', red_line: false } }], workspace: null, moduleUses: [{ module: 'a', imports: ['b'] }], fileExists: allExist });

  // ALLOW cases (over-block guards)
  allows('valid core + shell-adapter matching metadata', { manifests: [{ path: 'd', data: CORE }, { path: 'a', data: API }], workspace: ws, fileExists: allExist });
  allows('hub-module imports a DECLARED module', { manifests: [{ path: 'm', moduleName: 'a', data: { name: 'a', kind: 'hub-module', depends: ['b'], events_in: [], events_out: [], contract: 'a', red_line: false } }], workspace: null, moduleUses: [{ module: 'a', imports: ['b'] }], fileExists: allExist });
  allows('hub-module self-import is fine', { manifests: [{ path: 'm', moduleName: 'a', data: { name: 'a', kind: 'hub-module', depends: [], events_in: [], events_out: [], contract: 'a', red_line: false } }], workspace: null, moduleUses: [{ module: 'a', imports: ['a'] }], fileExists: allExist });
  allows('core with dev-only rand (rand NOT in normalDeps) is clean', { manifests: [{ path: 'm', data: CORE }], workspace: { packages: [{ name: 'dowiz-core', internalDeps: new Set(), normalDeps: new Set(['serde', 'sha2']) }] }, fileExists: allExist });
  allows('cargo skipped (workspace=null) → schema/contract still pass', { manifests: [{ path: 'd', data: CORE }], workspace: null, fileExists: allExist });

  if (fail) {
    console.error(`✗ module-integrity self-test: ${fail} assertion(s) failed.`);
    process.exit(1);
  }
  console.log('✓ module-integrity self-test: all DENY + ALLOW (over-block) assertions pass.');
}

if (process.argv.includes('--self-test')) selfTest();
else runRealTree();
