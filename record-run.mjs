#!/usr/bin/env node
// analytics/record-run.mjs
// Called at the END of each agent run: merges the metric-core result (Phase B) + eval result
// (Phase C) + run metadata into ONE consolidated line appended to run-history.jsonl.
// That JSONL is the substrate the analytics scripts read. (At scale you can source the same rows
// from Langfuse instead — the computations are identical.)
//
// Usage (env-driven so it slots into any runtime):
//   RUN_ID=... TASK_ID=... CONFIG_VERSION=agents@v3 CATEGORY=implementer-backend \
//   SUBAGENT=implementer MODEL=anthropic/claude-sonnet-4.6 PROVIDER=openrouter \
//   TOKENS=12345 COST_USD=0.04 TOUCHED_FILES="pricing/engine.ts,migrations/0NN.sql" \
//   node analytics/record-run.mjs

import { readFileSync, appendFileSync, existsSync } from "node:fs";

const readJson = (p) => (existsSync(p) ? JSON.parse(readFileSync(p, "utf8")) : null);

const core = readJson("metric-core-result.json"); // from Phase B
const evalAll = readJson("deepeval-result.json"); // from Phase C (array keyed by run_id)
const RUN_ID = process.env.RUN_ID || core?.run_id;
if (!RUN_ID) throw new Error("[record-run] RUN_ID required (or a metric-core-result.json with run_id)");

const evalForRun = Array.isArray(evalAll) ? evalAll.find((e) => e.run_id === RUN_ID)?.metrics ?? [] : [];

const record = {
  run_id: RUN_ID,
  task_id: process.env.TASK_ID || RUN_ID,
  ts: new Date().toISOString(),
  config_version: process.env.CONFIG_VERSION || "unset", // bump when you change a prompt/model mapping
  category: process.env.CATEGORY || null,
  subagent: process.env.SUBAGENT || null,
  model: process.env.MODEL || null,
  provider: process.env.PROVIDER || null,
  core: core ? { passed: core.passed, score: core.score, gating_failed: core.gating_failed, checks: core.checks?.map((c) => ({ name: c.name, passed: c.passed })) } : null,
  eval: evalForRun,
  usage: { tokens: numOrNull(process.env.TOKENS), cost_usd: numOrNull(process.env.COST_USD) },
  touched_files: (process.env.TOUCHED_FILES || "").split(",").map((s) => s.trim()).filter(Boolean),
};

function numOrNull(v) {
  return v == null || v === "" ? null : Number(v);
}

appendFileSync("run-history.jsonl", JSON.stringify(record) + "\n");
console.log(`[record-run] appended ${RUN_ID} (core.passed=${record.core?.passed})`);
