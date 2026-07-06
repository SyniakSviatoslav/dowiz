#!/usr/bin/env node
// ─────────────────────────────────────────────────────────────────────────────
//  guardrail-legacy-freeze.mjs — forward-only freeze on the strangled Node shell
//  (STRUCTURE-UPGRADE A4). The Rust rebuild is strangling `apps/api`; new HTTP
//  surface must land in the rebuild, NOT grow the legacy Node shell. This counts
//  route registrations in `apps/api/src` and REDS when the count exceeds the
//  committed baseline. Removing routes is fine (the shell shrinking is the whole
//  point) — ratchet the floor down with `--update`. Deterministic, no LLM.
//
//  A route registration = a Fastify method call whose first arg is a string PATH
//  (leads with `/`) — `fastify.get('/api/…', …)`. The leading-`/` discriminator
//  excludes `map.get(key)` / `params.get('id')` (non-route `.get`s).
//
//  Modes:
//    (default)     count apps/api/src, compare to scripts/legacy-api-baseline.json;
//                  exit 1 if it GREW. Never writes.
//    --update      recompute + rewrite the baseline (ratchet the floor down after a
//                  removal, or record an approved increase). The only writer.
//    --self-test   hermetic: proves the regex counts routes (not Map/params .get)
//                  and the compare logic (grew→red / shrank / frozen). No product code.
//
//  Run:  node scripts/guardrail-legacy-freeze.mjs
// ─────────────────────────────────────────────────────────────────────────────
import { readFileSync, writeFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO = join(dirname(fileURLToPath(import.meta.url)), '..');
const SRC = join(REPO, 'apps/api/src');
const BASELINE = join(REPO, 'scripts/legacy-api-baseline.json');

const ROUTE_RE = /\.(?:get|post|put|patch|delete)\s*\(\s*['"`]\//g;
const EXCLUDE = (rel) => /\.test\.ts$/.test(rel) || /(^|\/)(__tests__|tests|scripts)\//.test(rel);

export function countMatches(text) {
  const m = text.match(ROUTE_RE);
  return m ? m.length : 0;
}

export function evaluate(count, base) {
  if (count > base) return { status: 'red', delta: count - base };
  if (count < base) return { status: 'shrank', delta: base - count };
  return { status: 'frozen', delta: 0 };
}

function tsFiles(dir, rel = '') {
  const out = [];
  for (const e of readdirSync(dir)) {
    const abs = join(dir, e);
    const r = rel ? `${rel}/${e}` : e;
    const st = statSync(abs);
    if (st.isDirectory()) out.push(...tsFiles(abs, r));
    else if (e.endsWith('.ts') && !EXCLUDE(r)) out.push({ abs, rel: r });
  }
  return out;
}

function scan() {
  let count = 0;
  const perFile = {};
  for (const f of tsFiles(SRC)) {
    const n = countMatches(readFileSync(f.abs, 'utf8'));
    if (n) {
      count += n;
      perFile[f.rel] = n;
    }
  }
  return { count, perFile };
}

function writeBaseline({ count, perFile }) {
  const body = {
    count,
    note: 'Forward-only freeze on the strangled Node shell (apps/api). Route registrations may not GROW — new HTTP surface belongs in the Rust rebuild. Removals ratchet this down via --update. STRUCTURE-UPGRADE A4; guardrail scripts/guardrail-legacy-freeze.mjs.',
    pattern: ROUTE_RE.source,
    scope: 'apps/api/src/**/*.ts (excluding *.test.ts, __tests__/, tests/, scripts/)',
    perFile,
  };
  writeFileSync(BASELINE, JSON.stringify(body, null, 2) + '\n');
}

function selfTest() {
  let fail = 0;
  const ok = (name, cond) => {
    console.log(`  ${cond ? '✓' : '✗'} ${name}`);
    if (!cond) fail++;
  };
  console.log('legacy-freeze self-test:');
  ok("counts fastify.get('/x')", countMatches("fastify.get('/x', h)") === 1);
  ok('counts double-quote + template paths', countMatches('app.post("/y", h)\nserver.put(`/z`, h)') === 2);
  ok('counts all five methods', countMatches("a.get('/1');b.post('/2');c.put('/3');d.patch('/4');e.delete('/5')") === 5);
  ok("does NOT count map.get('key') / params.get('id') (no leading slash)", countMatches("map.get('key'); params.get('id'); headers.get('x')") === 0);
  ok('does NOT count a method with a non-path string', countMatches("redis.get('user:1')") === 0);
  ok('grew → red', evaluate(177, 176).status === 'red' && evaluate(177, 176).delta === 1);
  ok('shrank → shrank', evaluate(175, 176).status === 'shrank');
  ok('equal → frozen', evaluate(176, 176).status === 'frozen');
  if (fail) {
    console.error(`✗ legacy-freeze self-test: ${fail} assertion(s) failed.`);
    process.exit(1);
  }
  console.log('✓ legacy-freeze self-test: regex + compare logic proven.');
}

// ── entry ──
if (process.argv.includes('--self-test')) {
  selfTest();
} else if (process.argv.includes('--update')) {
  const scanned = scan();
  writeBaseline(scanned);
  console.log(`✓ legacy-freeze: baseline set to ${scanned.count} route registration(s) across ${Object.keys(scanned.perFile).length} file(s).`);
} else {
  if (!existsSync(BASELINE)) {
    console.error('✗ legacy-freeze: scripts/legacy-api-baseline.json missing — create it with: node scripts/guardrail-legacy-freeze.mjs --update');
    process.exit(1);
  }
  const base = JSON.parse(readFileSync(BASELINE, 'utf8'));
  const { count, perFile } = scan();
  const { status, delta } = evaluate(count, base.count);
  if (status === 'red') {
    console.error(`✗ legacy-freeze: apps/api route registrations ${base.count} → ${count} (+${delta}). The Node shell is FROZEN forward-only (strangler) — new HTTP surface belongs in the Rust rebuild, not apps/api.`);
    const grown = Object.entries(perFile).filter(([f, n]) => n > (base.perFile?.[f] || 0));
    if (grown.length) {
      console.error('  files that grew:');
      for (const [f, n] of grown) console.error(`    ${f}: ${base.perFile?.[f] || 0} → ${n}`);
    }
    console.error('  If this addition is genuinely intentional and approved, rebaseline: node scripts/guardrail-legacy-freeze.mjs --update');
    process.exit(1);
  }
  if (status === 'shrank') {
    console.log(`✓ legacy-freeze: shell SHRANK ${base.count} → ${count} (−${delta}) — ratchet the floor down: node scripts/guardrail-legacy-freeze.mjs --update`);
  } else {
    console.log(`✓ legacy-freeze: ${count} route registration(s) == baseline (frozen).`);
  }
}
