#!/usr/bin/env node
// G1 — Injection-corpus REACHABILITY guardrail (tooling-integration-eval, ledger #29).
//
// The AI menu-parser ingests UNTRUSTED scraped/OCR text and concatenates it into a Claude prompt
// (apps/api/src/lib/ai-ocr-parser.ts sink ~:544). The adversarial corpus at tests/injection-corpus/
// must NEVER be reachable from a prompt-assembly path or shipped in the runtime image — a corpus file
// reaching the parser as *source* would turn an inert test into a live injection.
//
// This is the exit-1 FLOOR (not a warn-level lint). It enforces FOUR layers; primary is STRUCTURAL:
//   1. Structural: the corpus lives at repo-root tests/injection-corpus/ — outside every Dockerfile
//      COPY target and every build-apps.ts cpSync source. Verified against the REAL build paths, so a
//      move under apps/packages/scripts (or any other build-reachable root) fails CI.
//   2. Sentinel/content scan: no source file under apps/**, packages/**, scripts/** references the
//      corpus (by the rename-proof sentinel or the dir-name substring).
//   3. Import-edge: no import resolves under the corpus dir.
//   4. Fixture-PII (defense-in-depth, NOT the proof): no fixture carries STRUCTURED PII
//      (email / ≥8-digit card·phone / iban / url-with-query / role-triggered name). Bare names are
//      NOT caught here — held by the authoring ritual + human review (see corpus README).
//
// Run: node scripts/guardrail-corpus-reachability.mjs
import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { join, resolve, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = process.cwd();
const SELF = fileURLToPath(import.meta.url);                 // RA-8: self-skip (one constant, no list)
const CANONICAL = 'tests/injection-corpus';                 // RA-8: the one canonical corpus dir
const CANONICAL_ABS = resolve(ROOT, CANONICAL);
const SENTINEL = 'DOWIZ-INJECTION-CORPUS-SENTINEL';         // L1: rename-proof, no digit run
const DIR_TOKEN = 'injection-corpus';

const errors = [];

// ---- helpers ---------------------------------------------------------------
const isUnder = (childAbs, parentAbs) => {
  const rel = relative(parentAbs, childAbs);
  return rel === '' || (!rel.startsWith('..') && !resolve(parentAbs, rel).includes('..') && !rel.startsWith(sep));
};

function findDirNamed(dirAbs, token, hits) {
  if (!existsSync(dirAbs)) return;
  let st;
  try { st = statSync(dirAbs); } catch { return; }
  if (!st.isDirectory()) return;
  for (const name of readdirSync(dirAbs)) {
    if (name === 'node_modules' || name === '.git') continue;
    const p = join(dirAbs, name);
    let s; try { s = statSync(p); } catch { continue; }
    if (s.isDirectory()) {
      if (name === token) hits.push(p);
      findDirNamed(p, token, hits);
    }
  }
}

// ---- layer 1: STRUCTURAL — corpus must be outside every build-reachable root ----
function buildReachableRoots() {
  const roots = new Set();
  // Dockerfile: builder-stage `COPY <src...> <dest>` — every src arg is build-reachable.
  const df = join(ROOT, 'Dockerfile');
  if (existsSync(df)) {
    for (const line of readFileSync(df, 'utf8').split('\n')) {
      const m = line.match(/^\s*COPY\s+(.+)$/);
      if (!m) continue;
      const args = m[1].trim().split(/\s+/).filter((a) => !a.startsWith('--'));
      if (args.length < 2) continue;            // need at least src + dest
      for (const src of args.slice(0, -1)) {    // all but the dest
        if (src.startsWith('/')) continue;      // absolute (--from stage paths) — not host context
        roots.add(src.replace(/\/$/, ''));
      }
    }
  }
  // build-apps.ts cpSync sources (the REAL asset pipeline — RA-3).
  const ba = join(ROOT, 'scripts/build-apps.ts');
  if (existsSync(ba)) {
    const txt = readFileSync(ba, 'utf8');
    for (const m of txt.matchAll(/path\.resolve\(\s*['"]([^'"]+)['"]\s*\)/g)) roots.add(m[1].replace(/\/$/, ''));
  }
  return [...roots];
}

for (const root of buildReachableRoots()) {
  const rootAbs = resolve(ROOT, root);
  // (a) canonical corpus must not sit inside a build-reachable root
  if (isUnder(CANONICAL_ABS, rootAbs)) {
    errors.push(`STRUCTURAL: corpus '${CANONICAL}' is inside build-reachable path '${root}' (would ship in the image / reach a prompt path).`);
  }
  // (b) no directory named `injection-corpus` may exist anywhere under a build-reachable root
  const hits = [];
  findDirNamed(rootAbs, DIR_TOKEN, hits);
  for (const h of hits) errors.push(`STRUCTURAL: a '${DIR_TOKEN}' directory exists under build-reachable '${root}': ${relative(ROOT, h)} (move it to repo-root ${CANONICAL}).`);
}

if (!existsSync(CANONICAL_ABS)) {
  errors.push(`STRUCTURAL: canonical corpus dir '${CANONICAL}' is missing — the guard cannot verify a relocated corpus.`);
}

// ---- layers 2 & 3: sentinel/content scan + import-edge over source roots ----
const SCAN_ROOTS = ['apps', 'packages', 'scripts'];
const SKIP_DIR = new Set(['node_modules', 'dist', '.git', 'coverage', '__fixtures__']);
const isTestFile = (p) => /\.(test|spec)\.[tj]sx?$/.test(p) || /(^|\/)tests?(\/|$)/.test(relative(ROOT, p).replace(/\\/g, '/'));
const IMPORT_RE = /(?:from\s+|import\s*\(|require\s*\(|import\s+)['"]([^'"]+)['"]/g;

function scanSource(dirAbs) {
  if (!existsSync(dirAbs)) return;
  for (const name of readdirSync(dirAbs)) {
    if (SKIP_DIR.has(name)) continue;
    const p = join(dirAbs, name);
    let s; try { s = statSync(p); } catch { continue; }
    if (s.isDirectory()) { scanSource(p); continue; }
    if (resolve(p) === resolve(SELF)) continue;       // RA-8 self-skip
    if (!/\.(ts|tsx|js|jsx|mjs|cjs|json)$/.test(p)) continue;
    if (isTestFile(p)) continue;                      // tests legitimately reach the corpus
    const txt = readFileSync(p, 'utf8');
    // layer 2: sentinel + dir-name substring
    if (txt.includes(SENTINEL)) errors.push(`SENTINEL: ${relative(ROOT, p)} references the corpus sentinel — a fixture leaked into non-test source.`);
    if (txt.includes(DIR_TOKEN)) errors.push(`CONTENT: ${relative(ROOT, p)} references '${DIR_TOKEN}' from non-test source (only tests may load the corpus).`);
    // layer 3: import-edge resolving under the corpus dir
    for (const m of txt.matchAll(IMPORT_RE)) {
      const spec = m[1];
      if (!spec.startsWith('.')) continue;
      const target = resolve(dirAbs, spec);
      if (isUnder(target, CANONICAL_ABS)) errors.push(`IMPORT: ${relative(ROOT, p)} imports from the corpus dir (${spec}).`);
    }
  }
}
SCAN_ROOTS.forEach((r) => scanSource(resolve(ROOT, r)));

// ---- layer 4: fixture-content STRUCTURED-PII gate (defense-in-depth) ----
// Mirrors apps/api/src/lib/pii-redactor.ts structured patterns (NOT bare names — see README).
const PII = [
  { kind: 'email', re: /[\w.+-]+@[\w-]+\.[\w.-]+/ },
  { kind: 'url-with-query', re: /https?:\/\/[^\s]+\?[^\s]+/ },
  { kind: 'iban', re: /[A-Z]{2}\d{2}[A-Z0-9]{10,30}/ },
  { kind: 'card', re: /(?:\d[ -]*?){13,19}/ },
  { kind: 'phone', re: /(?:\+|00)?(?:[0-9]{1,3})?[-\s()]*[0-9][-\s()0-9]{6,}[0-9]/ },
  { kind: 'role-name', re: /\b(?:[Cc]hef|[Oo]wner|[Mm]anager|[Ff]ounder|[Dd]irector|[Pp]roprietor|[Hh]ost|[Hh]ostess|[Ss]erved by|[Pp]repared by)\b[:.,]?\s+(?:[Oo]ur\s+)?[A-Z][a-z]+(?:\s+[A-Z][a-z.]+){1,2}/ },
];
function scanFixturesPii(dirAbs) {
  if (!existsSync(dirAbs)) return;
  for (const name of readdirSync(dirAbs)) {
    const p = join(dirAbs, name);
    let s; try { s = statSync(p); } catch { continue; }
    if (s.isDirectory()) { scanFixturesPii(p); continue; }
    if (!/\.txt$/.test(p)) continue;
    const txt = readFileSync(p, 'utf8');
    for (const { kind, re } of PII) {
      const m = txt.match(re);
      if (m) errors.push(`FIXTURE-PII (${kind}): ${relative(ROOT, p)} contains structured PII-like data: "${m[0].slice(0, 40)}" — fixtures must be synthetic/PII-free.`);
    }
  }
}
scanFixturesPii(CANONICAL_ABS);

// ---- verdict ----
if (errors.length) {
  console.error(`✗ guardrail-corpus-reachability: ${errors.length} violation(s):`);
  for (const e of errors) console.error('  - ' + e);
  console.error(`\nThe injection corpus must stay at repo-root ${CANONICAL} (out of every build path) and be referenced ONLY from test files.`);
  process.exit(1);
}
console.log('✓ guardrail-corpus-reachability: injection corpus structurally non-reachable; no source reference; fixtures PII-clean.');
