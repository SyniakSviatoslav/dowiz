#!/usr/bin/env node
/**
 * metric-core/run-checks.mjs
 *
 * Deterministic gate that decides "done" — not an LLM judgement.
 *
 * 1. Reads checks.config.mjs
 * 2. Runs every check in series, capturing stdout/stderr/duration/exit-code
 * 3. Aggregates into metric-core-result.json
 * 4. Optionally emits a Langfuse span (if LANGFUSE_* env vars are set)
 * 5. Exits 0 if all hard gates passed, 1 otherwise
 *
 * Usage:
 *   RUN_ID="$run" node metric-core/run-checks.mjs
 *
 * Env:
 *   RUN_ID          – same id as the agent run (Phase A), defaults to 'dev'
 *   LANGFUSE_*      – Langfuse credentials for OTLP export (optional)
 *   CI              – set to 'true' for CI output (no progress spinners)
 */
import { execSync } from 'child_process';
import { writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { platform } from 'os';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '..');

// ── Load config ────────────────────────────────────────────────
const configUrl = new URL('checks.config.mjs', import.meta.url).href;
let checksConfig;
try {
  const mod = await import(configUrl);
  checksConfig = mod.default;
} catch (err) {
  console.error(`[metric-core] FATAL: could not load checks.config.mjs — ${err.message}`);
  process.exit(1);
}

const RUN_ID = process.env.RUN_ID || 'dev';
const IS_CI = process.env.CI === 'true';

// ── Langfuse setup ────────────────────────────────────────────
let langfuseSpanProcessor = null;
let otelSdk = null;

async function initLangfuse() {
  const hasKeys =
    process.env.LANGFUSE_PUBLIC_KEY &&
    process.env.LANGFUSE_SECRET_KEY;

  if (!hasKeys) {
    if (!IS_CI) console.log('[metric-core] Langfuse env vars not set — skipping span export');
    return;
  }

  try {
    const { LangfuseSpanProcessor } = await import('@langfuse/otel');
    const { NodeSDK } = await import('@opentelemetry/sdk-node');
    const { BatchSpanProcessor } = await import('@opentelemetry/sdk-trace-base');
    const { OTLPTraceExporter } = await import('@opentelemetry/exporter-trace-otlp-proto');

    const traceExporter = new OTLPTraceExporter({
      url: `${process.env.LANGFUSE_HOST || 'https://us.cloud.langfuse.com'}/api/public/otel/v1/traces`,
      headers: {
        Authorization: `Basic ${Buffer.from(
          `${process.env.LANGFUSE_PUBLIC_KEY}:${process.env.LANGFUSE_SECRET_KEY}`
        ).toString('base64')}`,
      },
    });

    langfuseSpanProcessor = new LangfuseSpanProcessor();
    const batchProcessor = new BatchSpanProcessor(traceExporter);

    otelSdk = new NodeSDK({
      spanProcessors: [batchProcessor, langfuseSpanProcessor],
    });

    otelSdk.start();
    if (!IS_CI) console.log('[metric-core] Langfuse OTLP exporter initialised');
  } catch (err) {
    console.warn(`[metric-core] Langfuse init skipped: ${err.message}`);
  }
}

async function shutdownLangfuse() {
  if (otelSdk) {
    try {
      await otelSdk.shutdown();
    } catch (err) {
      console.warn(`[metric-core] otel shutdown error: ${err?.message}`);
    }
  }
}

// ── Run a single check ────────────────────────────────────────
function runCheck(check) {
  const start = performance.now();
  const result = {
    name: check.name,
    gate: check.gate,
    passed: false,
    durationMs: 0,
    exitCode: null,
    stdout: '',
    stderr: '',
    summary: '',
  };

  try {
    const stdout = execSync(check.cmd, {
      cwd: check.cwd || REPO_ROOT,
      timeout: check.timeoutMs || 60_000,
      encoding: 'utf-8',
      maxBuffer: 10 * 1024 * 1024,
      stdio: ['pipe', 'pipe', 'pipe'],
      shell: platform() === 'win32',
    });

    result.passed = true;
    result.exitCode = 0;
    result.stdout = stdout.trim();
    result.summary = extractSummary(stdout, check.name);

    if (check.preCheck && typeof check.preCheck === 'function') {
      try {
        const preCheckResult = check.preCheck(result);
        if (!preCheckResult) {
          result.passed = false;
          result.summary = 'pre-check validation failed';
        }
      } catch (preErr) {
        result.passed = false;
        result.summary = `pre-check threw: ${preErr.message}`;
      }
    }
  } catch (err) {
    result.exitCode = err.status || 1;
    result.stderr = (err.stderr || '').trim();
    result.stdout = (err.stdout || '').trim();
    result.summary = extractSummary(err.stdout || err.stderr || err.message, check.name) || err.message.slice(0, 200);
  }

  result.durationMs = Math.round(performance.now() - start);
  return result;
}

// ── Extract meaningful summary from output ─────────────────────
function extractSummary(output, checkName) {
  if (!output) return '';

  const lines = output.split('\n').filter(l => l.trim());

  // TSC: look for "Done" or error count
  if (checkName === 'tsc') {
    const errorLine = lines.find(l => /\d+ error/.test(l));
    if (errorLine) return errorLine.trim();
    const doneLine = lines.find(l => /typecheck/.test(l) || /Done/.test(l));
    if (doneLine) return doneLine.trim();
  }

  // Lint: look for error/warning summary
  if (checkName === 'lint') {
    const summaryLine = lines.find(l => /\d+ problems?/.test(l));
    if (summaryLine) return summaryLine.trim();
  }

  // Guardrail checkers: the JSON has a summary
  if (checkName === 'check-money' || checkName === 'check-rls') {
    try {
      const parsed = JSON.parse(output);
      if (parsed.summary) {
        const parts = Object.entries(parsed.summary.byRule || {})
          .map(([rule, count]) => `${rule}:${count}`);
        return `${parsed.passed ? 'PASS' : 'FAIL'} — ${parsed.violations.length} violations${parts.length ? ` (${parts.join(', ')})` : ''}`;
      }
    } catch {
      return output.slice(0, 200);
    }
  }

  // Playwright: extract summary
  if (checkName === 'playwright-smoke') {
    const passedLine = lines.find(l => /passed/.test(l) && /\d+/.test(l));
    if (passedLine) return passedLine.trim();
  }

  return output.slice(0, 200);
}

// ── Emit Langfuse span ────────────────────────────────────────
async function emitLangfuseSpan(results) {
  if (!otelSdk) return;

  try {
    const { trace, SpanStatusCode } = await import('@opentelemetry/api');

    const tracer = trace.getTracer('metric-core');
    const span = tracer.startSpan('metric-core', {
      attributes: {
        'dowiz.run_id': RUN_ID,
        'dowiz.core_passed': results.passed ? 'true' : 'false',
        'dowiz.core_score': String(results.score),
      },
    });

    for (const check of results.checks) {
      span.setAttribute(`dowiz.check.${check.name}.passed`, check.passed ? 'true' : 'false');
      span.setAttribute(`dowiz.check.${check.name}.duration_ms`, String(check.durationMs));
    }

    if (!results.passed) {
      span.setStatus({ code: SpanStatusCode.ERROR, message: `Gating failed: ${results.gating_failed.join(', ')}` });
    }

    span.end();
    await shutdownLangfuse();
    if (!IS_CI) console.log('[metric-core] Langfuse span emitted');
  } catch (err) {
    console.warn(`[metric-core] Langfuse span emission failed: ${err.message}`);
  }
}

// ── Main ──────────────────────────────────────────────────────
async function main() {
  if (!IS_CI) console.log(`\n[metric-core] run_id=${RUN_ID} — ${checksConfig.length} checks configured\n`);

  await initLangfuse();

  const results = [];

  for (const check of checksConfig) {
    if (!IS_CI) process.stdout.write(`  ${check.name} ... `);
    const result = runCheck(check);
    results.push(result);

    if (!IS_CI) {
      const icon = result.passed ? '✓' : '✗';
      console.log(`${icon} (${result.durationMs}ms)${result.summary ? ` — ${result.summary}` : ''}`);
    }
  }

  // Aggregate
  const hardFailures = results.filter(r => r.gate === 'hard' && !r.passed);
  const softFailures = results.filter(r => r.gate === 'soft' && !r.passed);
  const hardPassed = hardFailures.length === 0;
  const score = results.length > 0 ? results.filter(r => r.passed).length / results.length : 0;

  const report = {
    run_id: RUN_ID,
    passed: hardPassed,
    score: Math.round(score * 100) / 100,
    gating_failed: hardFailures.map(r => r.name),
    soft_failed: softFailures.map(r => r.name),
    checks: results,
  };

  // Write result JSON
  const outPath = resolve(__dirname, 'metric-core-result.json');
  writeFileSync(outPath, JSON.stringify(report, null, 2), 'utf-8');
  if (!IS_CI) console.log(`\n[metric-core] result written to ${outPath}`);

  // Emit Langfuse span
  await emitLangfuseSpan(report);

  // Print summary
  if (!IS_CI) {
    console.log(`\n[metric-core] score=${report.score} passed=${report.passed} hard=${results.filter(r => r.gate === 'hard').length} soft=${results.filter(r => r.gate === 'soft').length}`);
    if (hardFailures.length > 0) {
      console.log(`[metric-core] GATING FAILED: ${hardFailures.map(r => r.name).join(', ')}`);
    }
    console.log('');
  }

  process.exit(hardPassed ? 0 : 1);
}

main().catch(err => {
  console.error(`[metric-core] FATAL: ${err.message}`);
  process.exit(1);
});
