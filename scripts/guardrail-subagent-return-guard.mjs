#!/usr/bin/env node
// Guardrail — subagent-return-guard.sh must RED the degenerate 0-tool-use return and stay SILENT
// on real work (fable-audit-findings-2026-07-07 ROOT-CAUSE).
//
// Hermetic: runs the hook with SUBAGENT_TRANSCRIPT pinned at committed fixtures (the 2 real
// degenerate transcript signatures + a good control + a legit no-tool reply), fixture
// CLAUDE_PROJECT_DIR outside the repo. Proves the DETECTION+DECISION logic independent of the
// runtime location heuristic. Lesson #47: a gate that cannot actually deny is worthless — simulate
// the block, and prove it stays silent on the legitimate neighbour (over-block guard).
//
// Run: node scripts/guardrail-subagent-return-guard.mjs
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, rmSync, existsSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const ROOT = process.cwd();
const HOOKS = process.env.GATE_HOOKS_DIR || join(ROOT, '.claude/hooks');
const HOOK = join(HOOKS, 'subagent-return-guard.sh');
const FIX = join(ROOT, 'scripts/fixtures/subagent-return-guard');
const TMP = mkdtempSync(join(tmpdir(), 'subagent-guard-'));
mkdirSync(join(TMP, '.claude/logs'), { recursive: true });
const EVENTS = join(TMP, '.claude/logs/harness-events.jsonl');
const eventsHas = (frag) => existsSync(EVENTS) && readFileSync(EVENTS, 'utf8').includes(frag);

const baseEnv = { ...process.env, CLAUDE_PROJECT_DIR: TMP };
delete baseEnv.GIT_DIR; delete baseEnv.GIT_WORK_TREE;

function run(payload, transcript, extraEnv = {}) {
  const env = { ...baseEnv, ...extraEnv };
  if (transcript) env.SUBAGENT_TRANSCRIPT = join(FIX, transcript);
  return spawnSync('bash', [HOOK], {
    input: JSON.stringify(payload), cwd: TMP, env, encoding: 'utf8', timeout: 15000,
  });
}
const blocks = (r) => /"decision":\s*"block"/.test(r.stdout || '');
const nudges = (r) => /additionalContext/.test(r.stdout || '');

const failures = [];
const check = (name, ok, detail) => {
  if (ok) console.log(`  ✓ ${name}`);
  else { console.error(`  ✗ ${name}${detail ? ` — ${detail}` : ''}`); failures.push(name); }
};

console.log('subagent-return-guard.sh — SubagentStop:');
const stop = (t, extra) => run({ hook_event_name: 'SubagentStop', transcript_path: '/nope/x.jsonl', stop_hook_active: false, ...extra }, t);

const r1 = stop('degenerate-context-relevance.jsonl');
check('degenerate "_context_relevance:" echo → BLOCK', blocks(r1) && r1.status === 0);
const r2 = stop('degenerate-system-echo.jsonl');
check('degenerate "_id:/The system is Claude Code" echo → BLOCK', blocks(r2));
check('a block leaves a _hev block line', eventsHas('"event":"block"'));

const g = stop('good-control.jsonl');
check('good control (35-tool-use analog: has tool_use) → NO block, silent', !blocks(g) && (g.stdout || '') === '' && g.status === 0);

const w = stop('warn-legit-no-tool.jsonl');
check('legit no-tool reply (0 tool_use, no signature) → NO block', !blocks(w) && w.status === 0);
check('legit no-tool reply → _hev warn line', eventsHas('"event":"warn"'));

console.log('subagent-return-guard.sh — loop guard + belt + degraded:');
const rLoop = run({ hook_event_name: 'SubagentStop', transcript_path: '/nope/x.jsonl', stop_hook_active: true }, 'degenerate-context-relevance.jsonl');
check('degenerate + stop_hook_active=true → NO re-block (loop guard)', !blocks(rLoop) && rLoop.status === 0);

const rBelt = run({ hook_event_name: 'PostToolUse', tool_name: 'Agent', transcript_path: '/nope/x.jsonl' }, 'degenerate-system-echo.jsonl');
check('belt PostToolUse Agent on degenerate → non-blocking additionalContext nudge', nudges(rBelt) && !blocks(rBelt) && rBelt.status === 0);

const rMiss = run({ hook_event_name: 'SubagentStop', transcript_path: '/does/not/exist.jsonl' }, null);
check('no locatable transcript → fail OPEN (exit 0, no block)', !blocks(rMiss) && rMiss.status === 0);
check('fail-open is logged as degraded', eventsHas('"event":"degraded"'));

const rEmpty = spawnSync('bash', [HOOK], { input: '', cwd: TMP, env: baseEnv, encoding: 'utf8', timeout: 15000 });
check('empty stdin → exit 0, no block', !blocks(rEmpty) && rEmpty.status === 0);

rmSync(TMP, { recursive: true, force: true });

if (failures.length) {
  console.error(`\n✗ guardrail-subagent-return-guard: ${failures.length} case(s) failed — the 0-tool-use checker is disarmed or over-blocking.`);
  process.exit(1);
}
console.log('\n✓ guardrail-subagent-return-guard: degenerate returns block, real work passes, loop guard + belt + fail-open hold.');
