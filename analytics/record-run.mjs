#!/usr/bin/env node
/**
 * analytics/record-run.mjs
 *
 * End-of-run: merge core (Phase B) + eval (Phase C) + meta into one line
 * in run-history.jsonl.
 *
 * Env:
 *   RUN_ID              — required, same as Phase A/B
 *   CONFIG_VERSION      — bump on prompt/model changes (default: "0")
 *   CATEGORY            — "frontend" | "backend" | "testing" | "infra"
 *   SUBAGENT            — subagent type (default: "general")
 *   MODEL               — model name (default: "unknown")
 *   PROVIDER            — "anthropic" | "openai" | etc (default: "unknown")
 *   TASK                — short task description (default: "")
 *
 * Reads (optional, silently skips missing):
 *   metric-core/metric-core-result.json
 *   eval-layer/deepeval-result.json
 *
 * Appends:
 *   run-history.jsonl   — one JSON object per line
 */

import { readFileSync, writeFileSync, existsSync, appendFileSync } from 'fs';
import { resolve } from 'path';
import { fileURLToPath } from 'url';

const __dirname = resolve(fileURLToPath(import.meta.url), '..');
const REPO_ROOT = resolve(__dirname, '..');

const RUN_ID = process.env.RUN_ID;
if (!RUN_ID) {
  console.error('[record-run] FATAL: RUN_ID env var required');
  process.exit(1);
}

// ── Load core (Phase B) ────────────────────────────────────────
let core = null;
const corePath = resolve(REPO_ROOT, 'metric-core', 'metric-core-result.json');
if (existsSync(corePath)) {
  try {
    core = JSON.parse(readFileSync(corePath, 'utf-8'));
    // Strip verbose stdout/stderr from checks to keep history lean
    if (core.checks) {
      core.checks = core.checks.map(c => ({
        name: c.name, gate: c.gate, passed: c.passed,
        durationMs: c.durationMs, exitCode: c.exitCode,
        summary: (c.summary || '').slice(0, 200),
      }));
    }
  } catch { /* skip */ }
}

// ── Load eval (Phase C) ────────────────────────────────────────
let evalResults = null;
const evalPath = resolve(REPO_ROOT, 'eval-layer', 'deepeval-result.json');
if (existsSync(evalPath)) {
  try {
    evalResults = JSON.parse(readFileSync(evalPath, 'utf-8'));
  } catch { /* skip */ }
}

// ── History file path ──────────────────────────────────────────
const historyPath = resolve(REPO_ROOT, 'run-history.jsonl');

// ── Build record ───────────────────────────────────────────────
const record = {
  run_id: RUN_ID,
  timestamp: new Date().toISOString(),
  config_version: process.env.CONFIG_VERSION || '0',
  meta: {
    category: process.env.CATEGORY || '',
    subagent: process.env.SUBAGENT || 'general',
    model: process.env.MODEL || 'unknown',
    provider: process.env.PROVIDER || 'unknown',
    task: process.env.TASK || '',
  },
  core: core ? {
    passed: core.passed,
    score: core.score,
    gating_failed: core.gating_failed || [],
    soft_failed: core.soft_failed || [],
    checks: core.checks || [],
  } : null,
  eval: evalResults,
};

// ── Append to history ──────────────────────────────────────────
const line = JSON.stringify(record) + '\n';
appendFileSync(historyPath, line, 'utf-8');

console.log(`[record-run] RUN_ID=${RUN_ID} config=${record.config_version} core=${record.core ? (record.core.passed ? 'PASS' : 'FAIL') : 'N/A'} eval=${evalResults ? evalResults.length + ' runs' : 'N/A'}`);
console.log(`[record-run] appended to ${historyPath}`);
