#!/usr/bin/env node
// scripts/rebuild-map/extract-scripts-gates.mjs
//
// Namespace: scripts-gates
// Mirrors inventory/13-scripts-ops-guardrails.md §0 extraction commands:
//   root npm scripts:        Object.keys(require('package.json').scripts)          (doc: 70)
//   verify:all gate registry: grep -c "name:" scripts/verify-all.ts                (doc: 25)
//   eslint-plugin-local rules: grep -cE "^    '[a-z-]+': \{" tools/.../index.js     (doc: 26)
//   guardrail scripts:        ls scripts/guardrail-*.mjs | wc -l                   (doc: 11 +1 test)
// Four sub-kinds folded into one namespace per the task brief ("scripts+gates"); each gets
// a distinct id prefix so a script and a gate that happen to share a name never collide.

import { readRepoFile, idSafe, isMain, printRecords, stableSort, walkFiles } from './lib/common.mjs';

const PACKAGE_JSON = 'package.json';
const VERIFY_ALL = 'scripts/verify-all.ts';
const ESLINT_LOCAL = 'tools/eslint-plugin-local/src/index.js';

const GATE_NAME_RE = /\{\s*name:\s*'([^']+)'/;
const ESLINT_RULE_RE = /^\s{4}'([a-z-]+)':\s*\{/;

/** Pure/testable: root package.json scripts -> [{name, line}] (line = best-effort, 0 if not found). */
export function parsePackageScripts(content) {
  const pkg = JSON.parse(content);
  const scripts = pkg.scripts || {};
  const lines = content.split('\n');
  return Object.keys(scripts).map((name) => {
    const idx = lines.findIndex((l) => l.includes(`"${name}":`));
    return { name, line: idx >= 0 ? idx + 1 : 0 };
  });
}

/** Pure/testable: verify-all.ts text -> [{name, line}] gate entries. */
export function parseVerifyAllGates(content) {
  const out = [];
  const lines = content.split('\n');
  for (let i = 0; i < lines.length; i++) {
    const m = GATE_NAME_RE.exec(lines[i]);
    if (m) out.push({ name: m[1], line: i + 1 });
  }
  return out;
}

/** Pure/testable: eslint-plugin-local index.js text -> [{name, line}] rule entries. */
export function parseEslintLocalRules(content) {
  const out = [];
  const lines = content.split('\n');
  for (let i = 0; i < lines.length; i++) {
    const m = ESLINT_RULE_RE.exec(lines[i]);
    if (m) out.push({ name: m[1], line: i + 1 });
  }
  return out;
}

export async function extract() {
  const scripts = parsePackageScripts(readRepoFile(PACKAGE_JSON)).map(({ name, line }) => ({
    ns: 'scripts-gates',
    id: `GUARD-SCRIPT-${idSafe(name)}`,
    file: PACKAGE_JSON,
    line,
  }));

  const gates = parseVerifyAllGates(readRepoFile(VERIFY_ALL)).map(({ name, line }) => ({
    ns: 'scripts-gates',
    id: `GUARD-GATE-${idSafe(name)}`,
    file: VERIFY_ALL,
    line,
  }));

  const eslintRules = parseEslintLocalRules(readRepoFile(ESLINT_LOCAL)).map(({ name, line }) => ({
    ns: 'scripts-gates',
    id: `GUARD-ESLINT-${idSafe(name)}`,
    file: ESLINT_LOCAL,
    line,
  }));

  const guardrailFiles = walkFiles('scripts', ['.mjs']).filter((f) =>
    /\/guardrail-.*\.mjs$/.test(f) || /^scripts\/guardrail-.*\.mjs$/.test(f),
  );
  const guardrailScripts = guardrailFiles.map((f) => ({
    ns: 'scripts-gates',
    id: `GUARD-FILE-${idSafe(f.split('/').pop())}`,
    file: f,
    line: 1,
  }));

  return stableSort([...scripts, ...gates, ...eslintRules, ...guardrailScripts]);
}

if (isMain(import.meta.url)) {
  const records = await extract();
  printRecords(records);
}
