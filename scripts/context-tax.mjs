#!/usr/bin/env node
// context-tax — named-bucket breakdown of what's guaranteed-loaded into context every session
// (agentfiles/skillkit's `always_loaded` pattern: named sources, not one opaque total). Heuristic:
// est. tokens = chars / 4 (validated independently by skillkit's own `estimateTokens`, per
// docs/research/2026-07-04-agentfiles-obsidian-teardown.md §4c; real cost on Cyrillic-heavy
// strings is likely 1.3-1.8x higher — see docs/research/2026-07-04-harness-token-audit.md).
// Also tracks docs/regressions/REGRESSION-LEDGER.md against a hard token budget (warns >= 80%),
// since it's the one doc-store that grows unboundedly by design (ratchet rule forbids shrinking
// it). Read-only. Node stdlib only. No new deps.
//
// Usage: node scripts/context-tax.mjs [--json] [--ledger-budget 20000] [--log <path>]
//   --json           emit machine-readable output instead of the text report
//   --ledger-budget  token budget for REGRESSION-LEDGER.md (default 20000; tune as it grows)
//   --log <path>     append a dated JSON row to this trend-log file (opt-in; off by default —
//                     this script does not write anywhere unless --log is passed explicitly)

import { readFileSync, appendFileSync, existsSync } from 'node:fs';
import { homedir } from 'node:os';
import path from 'node:path';

const REPO_ROOT = path.resolve(new URL('.', import.meta.url).pathname, '..');
const CHARS_PER_TOKEN = 4;

const SOURCES = [
  { name: 'claude_md', label: 'CLAUDE.md', path: path.join(REPO_ROOT, '.claude/CLAUDE.md'), always: true },
  { name: 'memory_md', label: 'MEMORY.md', path: path.join(homedir(), '.claude/projects/-root-dowiz/memory/MEMORY.md'), always: true },
  { name: 'agents_md', label: 'AGENTS.md (NOT auto-loaded — on-demand only, per CLAUDE.md)', path: path.join(REPO_ROOT, 'AGENTS.md'), always: false },
];

const LEDGER_PATH = path.join(REPO_ROOT, 'docs/regressions/REGRESSION-LEDGER.md');
const DEFAULT_LEDGER_BUDGET_TOKENS = 20000;

function parseArgs(argv) {
  const args = { _: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith('--')) {
      const key = a.slice(2);
      const next = argv[i + 1];
      if (next === undefined || next.startsWith('--')) args[key] = true;
      else { args[key] = next; i++; }
    } else args._.push(a);
  }
  return args;
}

function sizeOf(p) {
  try {
    return Buffer.byteLength(readFileSync(p));
  } catch {
    return null; // file missing — don't fabricate a number
  }
}

function tokensOf(bytes) {
  return bytes === null ? null : Math.ceil(bytes / CHARS_PER_TOKEN);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const ledgerBudget = args['ledger-budget'] ? Number(args['ledger-budget']) : DEFAULT_LEDGER_BUDGET_TOKENS;

  const sources = SOURCES.map((s) => {
    const bytes = sizeOf(s.path);
    return { ...s, bytes, tokens: tokensOf(bytes) };
  });

  const guaranteedFloorTokens = sources
    .filter((s) => s.always && s.tokens !== null)
    .reduce((sum, s) => sum + s.tokens, 0);

  const ledgerBytes = sizeOf(LEDGER_PATH);
  const ledgerTokens = tokensOf(ledgerBytes);
  const ledgerPct = ledgerTokens === null ? null : Math.round((ledgerTokens / ledgerBudget) * 1000) / 10;
  const ledgerWarn = ledgerPct !== null && ledgerPct >= 80;

  const report = {
    heuristic: 'chars / 4 (bytes / 4); Cyrillic-heavy strings run 1.3-1.8x higher in practice',
    always_loaded: Object.fromEntries(sources.map((s) => [s.name, { label: s.label, bytes: s.bytes, tokens: s.tokens, always_loaded: s.always }])),
    guaranteed_floor_tokens: guaranteedFloorTokens,
    regression_ledger: {
      path: 'docs/regressions/REGRESSION-LEDGER.md',
      bytes: ledgerBytes,
      tokens: ledgerTokens,
      budget_tokens: ledgerBudget,
      pct_of_budget: ledgerPct,
      warn_over_80pct: ledgerWarn,
      note: 'append-only by ratchet rule — never shrinks; raise --ledger-budget as it grows, never delete rows to fit',
    },
  };

  if (args.json) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    console.log('== context-tax (advisory, chars/4 heuristic) ==');
    console.log('-- always-loaded sources --');
    for (const s of sources) {
      const flag = s.always ? '' : '  [NOT guaranteed — on-demand only]';
      const val = s.tokens === null ? 'MISSING' : `${s.tokens} tok (${s.bytes} B)`;
      console.log(` ${s.label}: ${val}${flag}`);
    }
    console.log(`-- guaranteed per-session floor: ${guaranteedFloorTokens} tok --`);
    console.log('-- docs/regressions/REGRESSION-LEDGER.md (unboundedly-growing doc-store) --');
    console.log(` ${ledgerTokens ?? 'MISSING'} tok / ${ledgerBudget} tok budget = ${ledgerPct ?? '?'}%${ledgerWarn ? '  *** OVER 80% WARN ***' : ''}`);
  }

  if (args.log) {
    const row = { ts: new Date().toISOString(), ...report };
    appendFileSync(args.log, JSON.stringify(row) + '\n');
    if (!args.json) console.log(`(appended trend row to ${args.log})`);
  }

  if (ledgerWarn && !args.json) {
    console.log('\nWARN: REGRESSION-LEDGER.md is >= 80% of its token budget. Consider adding a');
    console.log('compact INDEX table at the top so librarian/pattern-critic can grep instead of');
    console.log('reading the whole file (see docs/research/2026-07-04-harness-token-audit.md, Finding row 7).');
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
