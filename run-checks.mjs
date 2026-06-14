#!/usr/bin/env node
// metric-core/run-checks.mjs
// Runs every deterministic check, aggregates to ONE structured pass/fail score for the run,
// writes it to metric-core-result.json, emits a `metric-core` span to Langfuse (Phase A pipeline),
// and exits non-zero if any HARD gate failed. This exit code is what the verifier/convergence
// loop uses to decide "done" — it is your executable truth, not an LLM judgement.
//
// Usage:  RUN_ID=<run> node metric-core/run-checks.mjs
//   RUN_ID correlates this score with the agent run's trace (set the same id in Phase A's withRun).
//   Requires LANGFUSE_* env (from Phase A) and E2E_BASE_URL (for the Playwright check).

import { spawn } from "node:child_process";
import { writeFileSync } from "node:fs";
import { randomUUID } from "node:crypto";
import { NodeSDK } from "@opentelemetry/sdk-node";
import { LangfuseSpanProcessor } from "@langfuse/otel";
import { trace, SpanStatusCode } from "@opentelemetry/api";
import { CHECKS } from "./checks.config.mjs";

const RUN_ID = process.env.RUN_ID || randomUUID();

function runCheck(check) {
  return new Promise((resolve) => {
    const start = Date.now();
    const child = spawn(check.cmd, { shell: true, env: { ...process.env, ...(check.env || {}) } });
    let out = "";
    child.stdout.on("data", (d) => (out += d));
    child.stderr.on("data", (d) => (out += d));
    const timer = setTimeout(() => child.kill("SIGKILL"), check.timeoutMs ?? 120_000);
    child.on("close", (code) => {
      clearTimeout(timer);
      resolve({
        name: check.name,
        gate: check.gate,
        passed: code === 0,
        exitCode: code,
        durationMs: Date.now() - start,
        // Keep only the tail — never dump full logs into telemetry (may carry paths/secrets).
        summary: out.trim().split("\n").slice(-5).join("\n"),
      });
    });
  });
}

// Serial on purpose: deterministic, and avoids Playwright contending with tsc/etc. (anti-flake).
const results = [];
for (const c of CHECKS) results.push(await runCheck(c));

const hardFailed = results.filter((r) => r.gate === "hard" && !r.passed).map((r) => r.name);
const passed = hardFailed.length === 0;
const score = results.filter((r) => r.passed).length / results.length; // granular, for analytics

const result = {
  run_id: RUN_ID,
  ts: new Date().toISOString(),
  passed,
  score,
  gating_failed: hardFailed,
  checks: results,
};

writeFileSync("metric-core-result.json", JSON.stringify(result, null, 2));
console.log(JSON.stringify(result, null, 2));

// Emit as a span on the run, via the same OTLP→Langfuse path as Phase A (@langfuse/otel).
try {
  // NOTE: option is `spanProcessors` (array) in current @opentelemetry/sdk-node;
  // older versions use `spanProcessor` (singular) — confirm for your version.
  const sdk = new NodeSDK({ spanProcessors: [new LangfuseSpanProcessor()] });
  sdk.start();
  const span = trace.getTracer("metric-core").startSpan("metric-core");
  span.setAttribute("dowiz.run_id", RUN_ID);
  span.setAttribute("dowiz.core_passed", passed);
  span.setAttribute("dowiz.core_score", score);
  for (const r of results) {
    span.setAttribute(`dowiz.check.${r.name}.passed`, r.passed);
    span.setAttribute(`dowiz.check.${r.name}.duration_ms`, r.durationMs);
  }
  span.setStatus({ code: passed ? SpanStatusCode.OK : SpanStatusCode.ERROR });
  span.end();
  await sdk.shutdown(); // flush before exit
} catch (e) {
  // Telemetry must never gate the run — log and continue to the exit code.
  console.error("[metric-core] Langfuse emit failed (non-gating):", e?.message);
}

process.exit(passed ? 0 : 1);
