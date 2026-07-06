#!/usr/bin/env node
// Guardrail — governance/security edit-hooks must cover ALL edit tools (Edit|Write|MultiEdit).
//
// Regression (2026-06-29): protect-paths.sh + post-edit-gates.sh matched only "Edit|Write", while the
// newer governance hooks (serious-gate / pre-edit-lessons / red-line-doubt-gate / loop-detector) matched
// "Edit|Write|MultiEdit". A MultiEdit to a protected path therefore BYPASSED protect-paths entirely (and
// skipped the post-edit red-line gate) — a hole in a security gate. This asserts every edit-governance
// hook covers Edit, Write, AND MultiEdit so the matchers can never drift out of sync again.
//
// Run: node scripts/guardrail-hook-matchers.mjs
import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = process.cwd();
const SETTINGS = join(ROOT, '.claude/settings.json');
// Hooks that gate edits and MUST see every edit tool (file → which event it lives under).
// guard-bash.sh (2026-07-02, P0 gate-rearm): the Bash lane bypassed EVERY edit gate — a heredoc/
// sed -i/node script via Bash could mutate protected paths ungoverned (the guard existed but was
// unregistered since 43a018c1). It must stay registered under a Bash matcher.
const EDIT_TOOLS = ['Edit', 'Write', 'MultiEdit'];
// serious-gate.sh (council gate) was DELIBERATELY UNREGISTERED from settings.json on 2026-07-05
// (operator-approved council-disable — council is now optional, not required; the route-request
// UserPromptSubmit nudge remains). It is intentionally absent from this list so this guardrail
// matches that decision instead of demanding a hook the operator removed. If the council gate is
// ever re-enabled, re-add it here so its matcher stays Edit|Write|MultiEdit (no MultiEdit bypass).
// agent-dispatch-gate.sh (STRUCTURE-UPGRADE Part B / B1, 2026-07-06): the MODEL ROUTING dispatch
// gate. Both dispatch tool names asserted (Agent = current, Task = future rename) so the matcher
// can't drift out from under the gate. Ships in warn-mode (never blocks) — but #47 warns the
// easiest "fix" for a noisy gate is to unregister it, so registration is pinned here to fail loudly.
const DISPATCH_TOOLS = ['Agent', 'Task'];
// token-reduction gates (STRUCTURE-UPGRADE Part B): registration pinned so the #47 "just
// unregister the noisy gate" anti-pattern fails loudly in pre-commit. context-budget-guard has no
// tool matcher (UserPromptSubmit) → tools:[] just asserts it stays registered.
const MUST_COVER = [
  { hook: 'protect-paths.sh', event: 'PreToolUse', tools: EDIT_TOOLS },
  { hook: 'pre-edit-lessons.sh', event: 'PreToolUse', tools: EDIT_TOOLS },
  { hook: 'red-line-doubt-gate.sh', event: 'PreToolUse', tools: EDIT_TOOLS },
  { hook: 'post-edit-gates.sh', event: 'PostToolUse', tools: EDIT_TOOLS },
  { hook: 'guard-bash.sh', event: 'PreToolUse', tools: ['Bash'] },
  { hook: 'agent-dispatch-gate.sh', event: 'PreToolUse', tools: DISPATCH_TOOLS },
  { hook: 'distill-nudge.sh', event: 'PostToolUse', tools: ['Bash'] },
  { hook: 'context-budget-guard.sh', event: 'UserPromptSubmit', tools: [] },
];

if (!existsSync(SETTINGS)) {
  console.error(`✗ guardrail-hook-matchers: ${SETTINGS} not found.`);
  process.exit(1);
}
const cfg = JSON.parse(readFileSync(SETTINGS, 'utf8'));
const errors = [];

for (const { hook, event, tools: required } of MUST_COVER) {
  const entries = (cfg.hooks?.[event] || []).filter((e) =>
    (e.hooks || []).some((h) => (h.command || '').includes(hook)),
  );
  if (entries.length === 0) {
    errors.push(`${hook}: not registered under ${event}.`);
    continue;
  }
  for (const e of entries) {
    const tools = String(e.matcher || '').split('|').map((s) => s.trim());
    const missing = required.filter((t) => !tools.includes(t));
    if (missing.length) errors.push(`${hook} (${event}): matcher "${e.matcher}" omits ${missing.join(', ')} — a tool lane would bypass this gate.`);
  }
}

if (errors.length) {
  console.error(`✗ guardrail-hook-matchers: ${errors.length} gate(s) missing or not covering their tool lane:`);
  for (const e of errors) console.error('  - ' + e);
  console.error('\nEdit-governance hooks must match Edit|Write|MultiEdit and guard-bash.sh must match Bash — no tool lane may bypass a gate.');
  process.exit(1);
}
console.log(`✓ guardrail-hook-matchers: all ${MUST_COVER.length} governance gates cover their required tool lanes.`);
