/**
 * SMC — Dead-State Auditor (state-machine coverage guardrail)
 *
 * Detects OrderStatus enum values that are UNREACHABLE (dead states).
 * A state is ALIVE iff it is:
 *   (a) an insert-entry state — written by an `INSERT INTO orders` in apps/api/src
 *       (dev-only writers under routes/dev/ are counted separately), or
 *   (b) reachable from an alive state via the TRANSITIONS edges of the canonical
 *       order machine (packages/domain/src/order-machine.ts — every UPDATE of
 *       orders.status funnels through updateOrderStatus → assertTransition), or
 *   (c) written by a DB-side function or raw-SQL exception
 *       (`... orders SET status='X'` in packages/db/migrations or apps/api/src —
 *       the latter are the sibling gate's allowed exceptions, see
 *       verify-no-raw-status-update.ts).
 * Edges INTO a SCAFFOLD_STATUSES member are blocked (assertTransition throws
 * ScaffoldDisabledError), so a scaffold state can never be alive via an edge.
 *
 * DEAD = in the enum (source of truth: OrderStatusEnum, packages/shared-types)
 * but not alive. Gate: exit 1 if any dead state is NOT in KNOWN_DEAD; exit 0
 * otherwise. Test Integrity: any parse producing 0 states / 0 edges / 0 writers
 * is a LOUD failure (exit 1) — the gate never silently passes on a parse failure.
 *
 * Proof hook: SMC_KNOWN_DEAD env (comma-separated, empty string = empty
 * allowlist) overrides KNOWN_DEAD so the red arm stays reproducible:
 *   SMC_KNOWN_DEAD= tsx verify-state-machine-coverage.ts   → RED (SCHEDULED)
 *   tsx verify-state-machine-coverage.ts                   → GREEN
 *
 * Usage: tsx verify-state-machine-coverage.ts
 */

import { readFileSync, readdirSync } from 'fs';
import { join } from 'path';

const REPO_ROOT = join(import.meta.dirname, '../../..');
const ENUM_FILE = join(REPO_ROOT, 'packages/shared-types/src/legacy.ts');
const MACHINE_FILE = join(REPO_ROOT, 'packages/domain/src/order-machine.ts');
const API_SRC = join(REPO_ROOT, 'apps/api/src');
const MIGRATIONS_DIR = join(REPO_ROOT, 'packages/db/migrations');

const KNOWN_DEAD: string[] = ['SCHEDULED']; // council-pending: remove or implement

function fail(msg: string): never {
  console.error(`\n❌ SMC PARSE FAILURE — ${msg}`);
  console.error('   (Test Integrity: the gate never silently passes on a parse failure.)');
  process.exit(1);
}

function getAllFiles(dir: string, ext = '.ts'): string[] {
  const result: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory() && !entry.name.startsWith('node_modules')) result.push(...getAllFiles(full, ext));
    else if (entry.isFile() && entry.name.endsWith(ext)) result.push(full);
  }
  return result;
}

function rel(file: string): string {
  return file.replace(/\\/g, '/').replace(`${REPO_ROOT.replace(/\\/g, '/')}/`, '');
}

// ── 1. Enum states (source of truth: OrderStatusEnum, shared-types) ──────────
function parseEnumStates(): string[] {
  const src = readFileSync(ENUM_FILE, 'utf-8');
  const m = src.match(/OrderStatusEnum\s*=\s*z\.enum\(\[([\s\S]*?)\]\)/);
  if (!m) fail(`OrderStatusEnum z.enum([...]) not found in ${rel(ENUM_FILE)}`);
  const states = [...m[1].matchAll(/'([A-Z_]+)'/g)].map((x) => x[1]);
  if (states.length === 0) fail(`OrderStatusEnum parsed to 0 states in ${rel(ENUM_FILE)}`);
  return states;
}

// ── 2. TRANSITIONS edges + ORDER_STATUSES cross-check + SCAFFOLD set ─────────
function parseMachine(enumStates: string[]): { edges: Map<string, string[]>; scaffold: Set<string> } {
  const src = readFileSync(MACHINE_FILE, 'utf-8');

  const listM = src.match(/ORDER_STATUSES\s*=\s*\[([\s\S]*?)\]/);
  if (!listM) fail(`ORDER_STATUSES not found in ${rel(MACHINE_FILE)}`);
  const machineStates = [...listM[1].matchAll(/'([A-Z_]+)'/g)].map((x) => x[1]);
  const a = [...enumStates].sort().join(',');
  const b = [...machineStates].sort().join(',');
  if (a !== b) fail(`enum drift: OrderStatusEnum (${a}) != ORDER_STATUSES (${b})`);

  const tM = src.match(/TRANSITIONS\s*:[^=]*=\s*\{([\s\S]*?)\n\};/);
  if (!tM) fail(`TRANSITIONS table not found in ${rel(MACHINE_FILE)}`);
  const edges = new Map<string, string[]>();
  for (const row of tM[1].matchAll(/^\s*([A-Z_]+)\s*:\s*\[([^\]]*)\]/gm)) {
    const from = row[1];
    const targets = [...row[2].matchAll(/'([A-Z_]+)'/g)].map((x) => x[1]);
    edges.set(from, targets);
  }
  if (edges.size === 0) fail(`TRANSITIONS parsed to 0 states in ${rel(MACHINE_FILE)}`);
  for (const [from, targets] of edges) {
    for (const s of [from, ...targets]) {
      if (!enumStates.includes(s)) fail(`TRANSITIONS references '${s}' which is not in OrderStatusEnum`);
    }
  }

  const scaffold = new Set<string>();
  const sM = src.match(/SCAFFOLD_STATUSES[^=]*=\s*new Set\(\[([\s\S]*?)\]\)/);
  if (sM) for (const x of sM[1].matchAll(/'([A-Z_]+)'/g)) scaffold.add(x[1]);
  return { edges, scaffold };
}

// ── 3. Writers: INSERT-entry (API), DB-fn / raw-update (`orders SET status='X'`) ──
interface Writer {
  state: string;
  kind: 'insert' | 'insert-dev' | 'db-fn' | 'raw-update';
  where: string;
}

function extractInsertLiterals(content: string, enumSet: Set<string>): string[] {
  const found: string[] = [];
  for (const m of content.matchAll(/INSERT\s+INTO\s+orders\b/gi)) {
    // Window: from the INSERT to the end of its SQL template literal (or 1500 chars).
    const start = m.index! + m[0].length;
    let window = content.slice(start, start + 1500);
    const tick = window.indexOf('`');
    if (tick !== -1) window = window.slice(0, tick);
    for (const lit of window.matchAll(/'([A-Z_]+)'/g)) {
      if (enumSet.has(lit[1])) found.push(lit[1]);
    }
  }
  return found;
}

function extractStatusSetLiterals(content: string, enumSet: Set<string>): string[] {
  const found: string[] = [];
  // Tempered gap ((?!WHERE)) so a parameterized `SET status=$1 ... WHERE status='X'`
  // can never mint a false writer from its WHERE clause. NB: do NOT wrap the lazy
  // quantifier in an extra optional group — `(?:[\s\S]{0,120}?)?` breaks lazy
  // backtracking in V8 and skips the SET literal (captured the WHERE literal instead).
  // (?<![A-Za-z_$])status → the bare column only, never payment_status / order_status.
  for (const m of content.matchAll(/orders\s+SET\s+(?:(?!WHERE)[\s\S]){0,120}?(?<![A-Za-z_$])status\s*=\s*'([A-Z_]+)'/gi)) {
    if (enumSet.has(m[1])) found.push(m[1]);
  }
  return found;
}

function collectWriters(enumSet: Set<string>): Writer[] {
  const writers: Writer[] = [];

  for (const file of getAllFiles(API_SRC)) {
    const content = readFileSync(file, 'utf-8');
    const relPath = rel(file);
    const isDev = /\/(dev|mock)[^/]*\//.test(relPath) || /\/dev\//.test(relPath);
    for (const s of extractInsertLiterals(content, enumSet)) {
      writers.push({ state: s, kind: isDev ? 'insert-dev' : 'insert', where: relPath });
    }
    for (const s of extractStatusSetLiterals(content, enumSet)) {
      writers.push({ state: s, kind: 'raw-update', where: relPath });
    }
  }

  for (const file of getAllFiles(MIGRATIONS_DIR)) {
    const content = readFileSync(file, 'utf-8');
    for (const s of extractStatusSetLiterals(content, enumSet)) {
      writers.push({ state: s, kind: 'db-fn', where: rel(file) });
    }
    for (const s of extractInsertLiterals(content, enumSet)) {
      writers.push({ state: s, kind: 'db-fn', where: rel(file) });
    }
  }

  if (!writers.some((w) => w.kind === 'insert')) {
    fail('0 production INSERT-entry states found in apps/api/src — orders must be born somewhere');
  }
  if (!writers.some((w) => w.kind === 'db-fn')) {
    fail("0 DB-side status writers found in packages/db/migrations — app_sweep_timeout_orders (mig ...078) writes CANCELLED; the scan regex has rotted");
  }
  return writers;
}

// ── 4. Reachability (BFS over TRANSITIONS from all writer-seeded states) ─────
function reach(
  seeds: Set<string>,
  edges: Map<string, string[]>,
  scaffold: Set<string>,
  via: Map<string, string>,
): Set<string> {
  const alive = new Set<string>(seeds);
  const queue = [...seeds];
  while (queue.length > 0) {
    const cur = queue.shift()!;
    if (scaffold.has(cur)) continue; // assertTransition blocks FROM scaffold
    for (const next of edges.get(cur) ?? []) {
      if (scaffold.has(next)) continue; // assertTransition blocks INTO scaffold
      if (!alive.has(next)) {
        alive.add(next);
        if (!via.has(next)) via.set(next, `transition ${cur} → ${next}`);
        queue.push(next);
      }
    }
  }
  return alive;
}

function main() {
  const enumStates = parseEnumStates();
  const enumSet = new Set(enumStates);
  const { edges, scaffold } = parseMachine(enumStates);
  const writers = collectWriters(enumSet);

  const knownDead = process.env.SMC_KNOWN_DEAD !== undefined
    ? process.env.SMC_KNOWN_DEAD.split(',').map((s) => s.trim()).filter(Boolean)
    : KNOWN_DEAD;

  console.log('=== SMC: Dead-State Auditor (state-machine coverage) ===\n');

  const via = new Map<string, string>();
  const prodSeeds = new Set<string>();
  const allSeeds = new Set<string>();
  for (const w of writers) {
    allSeeds.add(w.state);
    if (w.kind !== 'insert-dev') prodSeeds.add(w.state);
  }
  // via: prefer a production writer as the displayed reason, fall back to dev.
  for (const w of writers) if (w.kind !== 'insert-dev' && !via.has(w.state)) via.set(w.state, `${w.kind}: ${w.where}`);
  for (const w of writers) if (!via.has(w.state)) via.set(w.state, `${w.kind}: ${w.where}`);

  const aliveProd = reach(prodSeeds, edges, scaffold, new Map(via));
  const aliveAll = reach(allSeeds, edges, scaffold, via);

  const dead = enumStates.filter((s) => !aliveAll.has(s));
  const devOnly = enumStates.filter((s) => aliveAll.has(s) && !aliveProd.has(s));

  for (const s of enumStates) {
    if (aliveAll.has(s)) {
      const devMark = devOnly.includes(s) ? ' [DEV-ONLY entry]' : '';
      console.log(`  ✅ ${s} — alive (${via.get(s)})${devMark}`);
    } else {
      const allow = knownDead.includes(s) ? ' [in KNOWN_DEAD allowlist]' : '';
      console.log(`  ❌ ${s} — DEAD (in enum; no writer, no reachable transition)${allow}`);
    }
  }

  const stale = knownDead.filter((s) => aliveAll.has(s) || !enumSet.has(s));
  if (stale.length > 0) {
    console.log(`\n  ⚠️ STALE KNOWN_DEAD entries (alive or not in enum — prune the allowlist): ${stale.join(', ')}`);
  }

  const pct = ((aliveAll.size / enumStates.length) * 100).toFixed(1);
  console.log(`\nSMC coverage: ${aliveAll.size}/${enumStates.length} states alive (${pct}%)`);
  const unexpected = dead.filter((s) => !knownDead.includes(s));
  console.log(
    `SMC_JSON ${JSON.stringify({ total: enumStates.length, alive: aliveAll.size, coveragePct: Number(pct), dead, knownDead, unexpectedDead: unexpected, devOnlyEntry: devOnly, scaffold: [...scaffold] })}`,
  );

  if (unexpected.length > 0) {
    console.log(`\n❌ ${unexpected.length} dead state(s) NOT in KNOWN_DEAD: ${unexpected.join(', ')}`);
    console.log('   Either implement a writer/transition for it, or (council decision) add it to KNOWN_DEAD with a comment.');
    process.exit(1);
  }
  console.log(`\n✅ No unexpected dead states (${dead.length} known-dead allowlisted: ${dead.join(', ') || 'none'})`);
  process.exit(0);
}

main();
