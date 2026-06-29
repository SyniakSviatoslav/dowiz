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
const MUST_COVER = [
  { hook: 'protect-paths.sh', event: 'PreToolUse' },
  { hook: 'serious-gate.sh', event: 'PreToolUse' },
  { hook: 'pre-edit-lessons.sh', event: 'PreToolUse' },
  { hook: 'red-line-doubt-gate.sh', event: 'PreToolUse' },
  { hook: 'post-edit-gates.sh', event: 'PostToolUse' },
];
const REQUIRED_TOOLS = ['Edit', 'Write', 'MultiEdit'];

if (!existsSync(SETTINGS)) {
  console.error(`✗ guardrail-hook-matchers: ${SETTINGS} not found.`);
  process.exit(1);
}
const cfg = JSON.parse(readFileSync(SETTINGS, 'utf8'));
const errors = [];

for (const { hook, event } of MUST_COVER) {
  const entries = (cfg.hooks?.[event] || []).filter((e) =>
    (e.hooks || []).some((h) => (h.command || '').includes(hook)),
  );
  if (entries.length === 0) {
    errors.push(`${hook}: not registered under ${event}.`);
    continue;
  }
  for (const e of entries) {
    const tools = String(e.matcher || '').split('|').map((s) => s.trim());
    const missing = REQUIRED_TOOLS.filter((t) => !tools.includes(t));
    if (missing.length) errors.push(`${hook} (${event}): matcher "${e.matcher}" omits ${missing.join(', ')} — an edit tool would bypass this gate.`);
  }
}

if (errors.length) {
  console.error(`✗ guardrail-hook-matchers: ${errors.length} gate(s) do not cover every edit tool:`);
  for (const e of errors) console.error('  - ' + e);
  console.error('\nEvery edit-governance hook must match Edit|Write|MultiEdit so no edit tool bypasses it.');
  process.exit(1);
}
console.log(`✓ guardrail-hook-matchers: all ${MUST_COVER.length} edit-governance gates cover Edit|Write|MultiEdit.`);
