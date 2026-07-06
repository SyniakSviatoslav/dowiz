#!/usr/bin/env node
// Guardrail — the token/MODEL-ROUTING dispatch gate must be ARMED, not just registered.
//
// STRUCTURE-UPGRADE.md Part B, B1 armament (the mandatory red→green proof, same hermetic pattern
// as scripts/guardrail-gate-armament.mjs: fixture CLAUDE_PROJECT_DIR outside the repo, JSON piped
// in). Rollout mode (A) warn-then-ratchet: the gate ships in WARN mode (never blocks), but the DENY
// path is proven here (TOKEN_GATE_MODE=deny) so promoting a check to teeth is a config flip, not a
// rewrite. Lesson docs/lessons/2026-07-02-gate-state-file-expiry.md #47: a registered gate that
// cannot actually deny is worthless — simulate the DENY, and prove it stays SILENT on the
// legitimate neighbor (over-block guard).
//
// Run: node scripts/guardrail-token-gates.mjs
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, rmSync, existsSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const ROOT = process.cwd();
const HOOKS = process.env.GATE_HOOKS_DIR || join(ROOT, '.claude/hooks');
const HOOK = 'agent-dispatch-gate.sh';
const FIX = mkdtempSync(join(tmpdir(), 'token-gates-'));
mkdirSync(join(FIX, '.claude/logs'), { recursive: true });
const EVENTS = join(FIX, '.claude/logs/harness-events.jsonl');
const eventsHas = (frag) => existsSync(EVENTS) && readFileSync(EVENTS, 'utf8').includes(frag);

const baseEnv = { ...process.env, CLAUDE_PROJECT_DIR: FIX };
delete baseEnv.GIT_DIR; delete baseEnv.GIT_WORK_TREE;

function run(toolName, toolInput, extraEnv = {}) {
  return spawnSync('bash', [join(HOOKS, HOOK)], {
    input: JSON.stringify({ tool_name: toolName, tool_input: toolInput }),
    cwd: FIX, env: { ...baseEnv, ...extraEnv }, encoding: 'utf8', timeout: 15000,
  });
}
const denies = (r) => /"permissionDecision":\s*"deny"/.test(r.stdout || '');

const failures = [];
const check = (name, ok, detail) => {
  if (ok) console.log(`  ✓ ${name}`);
  else { console.error(`  ✗ ${name}${detail ? ` — ${detail}` : ''}`); failures.push(name); }
};

const A = (input) => input; // readability alias for an Agent tool_input

// ── WARN mode (production default — never blocks) ────────────────────────────
console.log('agent-dispatch-gate.sh — WARN mode (default):');
const w1 = run('Agent', A({ description: 'read-only sweep', subagent_type: 'general-purpose', prompt: 'find X' }));
check('model-less dispatch does NOT block (warn, exit 0)', w1.status === 0 && !denies(w1));
check('model-less dispatch leaves a _hev warn line', eventsHas('"hook":"agent-dispatch-gate","event":"warn"'));

const w2 = run('Agent', A({ description: 'reason', subagent_type: 'general-purpose', model: 'opus', prompt: 'design Y' }));
check('compliant dispatch (model set) is silent (no deny, no stdout nudge)', w2.status === 0 && !denies(w2) && (w2.stdout || '') === '');

const w3 = run('TaskCreate', { subject: 'a task' });
check('TaskCreate is NOT a dispatch — untouched (exact-name guard)', w3.status === 0 && !denies(w3) && (w3.stdout || '') === '');

const w4 = run('Bash', { command: 'ls' });
check('non-dispatch tool (Bash) untouched', w4.status === 0 && !denies(w4));

// ── DENY mode (the ratchet — proven armed, shipped OFF) ──────────────────────
console.log('agent-dispatch-gate.sh — DENY mode (ratchet armed via TOKEN_GATE_MODE=deny):');
const d1 = run('Agent', A({ description: 'read-only sweep', subagent_type: 'general-purpose', prompt: 'find X' }), { TOKEN_GATE_MODE: 'deny' });
check('model-less dispatch BLOCKS when promoted (permissionDecision deny)', denies(d1));
check('deny leaves a _hev deny line', eventsHas('"hook":"agent-dispatch-gate","event":"deny"'));

const d2 = run('Agent', A({ description: 'reason', subagent_type: 'Explore', model: 'haiku', prompt: 'find X' }), { TOKEN_GATE_MODE: 'deny' });
check('compliant dispatch still ALLOWED in deny mode (over-block guard)', d2.status === 0 && !denies(d2));

const d3 = run('TaskCreate', { subject: 'x' }, { TOKEN_GATE_MODE: 'deny' });
check('TaskCreate never blocked even in deny mode (exact-name guard)', d3.status === 0 && !denies(d3));

// ── degraded (fail-open) ─────────────────────────────────────────────────────
console.log('agent-dispatch-gate.sh — degraded (fail-open on unparseable input):');
const g = spawnSync('bash', [join(HOOKS, HOOK)], { input: 'not json at all', cwd: FIX, env: baseEnv, encoding: 'utf8', timeout: 15000 });
check('unparseable input fails OPEN (exit 0, no deny)', g.status === 0 && !denies(g));
check('degraded decision is logged', eventsHas('"event":"degraded"'));

rmSync(FIX, { recursive: true, force: true });

if (failures.length) {
  console.error(`\n✗ guardrail-token-gates: ${failures.length} case(s) failed — the dispatch gate is disarmed or over-blocking.`);
  process.exit(1);
}
console.log('\n✓ guardrail-token-gates: warn mode never blocks, deny path is armed, over-block guards hold, decisions logged.');
