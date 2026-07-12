#!/usr/bin/env node
// security-redblue — ANTI-CHEAT dry-run harness (M9) for the security-redblue loop.
//
// This is NOT an offensive tool and fires ZERO attack traffic. It is the hermetic certification
// harness that proves the loop's VERDICT LOGIC (the finding-gate + advisory-forever boundary) is
// NOT cheatable, plus one real BLUE-arm SMOKE step (the crt.sh asset-surface scout, run through its
// test-fixture seam — no network). It runs offline, deterministically, and exits non-zero if any
// anti-cheat assertion fails.
//
// What it certifies (the cheat surfaces a status-only "security passed" loop would get wrong):
//   1. Fake-green by non-execution — a run that claims "clean" but a blue step never ran, or ran
//      without producing an evidence artifact, is NOT complete (INCOMPLETE / RED). Absence of
//      evidence is never evidence of absence.
//   2. Fake-green by closing a finding without a red→green guardrail — a confirmed finding marked
//      "closed" without a landed guardrail + ledger row is a VIOLATION, not done.
//   3. Council bypass — a confirmed RED-LINE finding (auth/RLS/money/PII/JWT) that was auto-fixed
//      instead of routed to the Triadic Council is a VIOLATION.
//   4. Advisory-forever — the loop NEVER emits a "GATE PASS"; even a fully clean run yields
//      gate:"advisory". A finding is authority only after it becomes a deterministic guardrail.
//   5. The RED arm is intake-only — this module exports NO attack/exploit function; offensive
//      results enter through `intakeFinding` and are graded, never generated here.
//
// Node stdlib only. No new deps. No shell. No network (smoke uses the asset-scan fixture seam).
import { scanDomains, buildReport } from '../../scripts/asset-surface-scan.mjs';
import { pathToFileURL } from 'node:url';

// ---------------------------------------------------------------------------
// The loop's verdict engine (pure, testable) — the abstract→verifiable bridge.
// ---------------------------------------------------------------------------
export const RED_LINE_CLASSES = new Set(['authz', 'rls', 'money', 'pii', 'jwt', 'auth']);

export const isRedLine = (cls) => RED_LINE_CLASSES.has(String(cls));

/**
 * Grade a single finding into a terminal status. A CONFIRMED finding is only "closed" when it has
 * a landed red→green guardrail + a ledger row; a RED-LINE finding must be council-routed, never
 * auto-fixed. Returns { id, status, closed, violation }.
 *   status ∈ unconfirmed | open | closed | VIOLATION:closed-without-guardrail | VIOLATION:council-bypass
 */
export function classifyFinding(f) {
  const id = String(f?.id ?? '?');
  const redline = isRedLine(f?.class);
  if (!f?.confirmed) return { id, status: 'unconfirmed', closed: true, violation: false, redline };

  const disp = String(f?.disposition ?? 'open');

  // Council bypass: a red-line finding may NEVER be auto-fixed — council convenes first.
  if (redline && disp === 'auto-fixed') {
    return { id, status: 'VIOLATION:council-bypass', closed: false, violation: true, redline };
  }

  // Red-line terminal disposition = council-queued (council convened) OR guardrail-landed
  // (council already ran + produced the guardrail). Anything else is still open.
  if (redline) {
    const closed = disp === 'council-queued' || (disp === 'guardrail-landed' && !!f.guardrail && f.ledger_row === true);
    return { id, status: closed ? 'closed' : 'open', closed, violation: false, redline };
  }

  // Non-red-line: "closed" ONLY with a real landed guardrail + ledger row. Claiming closed
  // without them is fake-green.
  if (disp === 'guardrail-landed') {
    if (f.guardrail && f.ledger_row === true) return { id, status: 'closed', closed: true, violation: false, redline };
    return { id, status: 'VIOLATION:closed-without-guardrail', closed: false, violation: true, redline };
  }
  return { id, status: 'open', closed: false, violation: false, redline };
}

/**
 * Evaluate a whole run. Returns { verdict, gate, stepIssues, findingStatuses }.
 *   verdict ∈ RED:violation | INCOMPLETE | ADVISORY-COMPLETE
 *   gate    ALWAYS "advisory" — the loop is advisory-forever, it never emits a real gate PASS.
 */
export function evaluateRun(run) {
  const steps = Array.isArray(run?.blue_steps) ? run.blue_steps : [];
  const findings = Array.isArray(run?.findings) ? run.findings : [];

  const stepIssues = [];
  for (const s of steps) {
    if (!s.executed) stepIssues.push({ step: s.name, issue: 'pending' });
    else if (!s.evidence) stepIssues.push({ step: s.name, issue: 'fake-green:no-evidence' });
  }
  const findingStatuses = findings.map(classifyFinding);

  const anyViolation = stepIssues.some((i) => i.issue === 'fake-green:no-evidence')
    || findingStatuses.some((s) => s.violation);
  const anyOpen = stepIssues.some((i) => i.issue === 'pending')
    || findingStatuses.some((s) => !s.closed);

  let verdict;
  if (anyViolation) verdict = 'RED:violation';
  else if (anyOpen) verdict = 'INCOMPLETE';
  else verdict = 'ADVISORY-COMPLETE';

  // Advisory-forever: no branch ever returns a gate PASS. The loop informs; guardrails/tests/human decide.
  return { verdict, gate: 'advisory', stepIssues, findingStatuses };
}

// ---------------------------------------------------------------------------
// Tiny assertion harness
// ---------------------------------------------------------------------------
let pass = 0;
let fail = 0;
const fails = [];
function ok(cond, label) {
  if (cond) { pass++; }
  else { fail++; fails.push(label); console.error(`  ✗ ${label}`); }
}

// ---------------------------------------------------------------------------
// PART A — verdict-engine anti-cheat (the finding gate + advisory boundary)
// ---------------------------------------------------------------------------
async function partA() {
  console.log('PART A — verdict-engine anti-cheat');

  const goodSteps = [
    { name: 'asset-surface-scan', executed: true, evidence: 'loops/runs/asset-surface-baseline.json' },
    { name: 'verify:rls', executed: true, evidence: 'ci-log:verify-rls' },
    { name: 'verify:secrets', executed: true, evidence: 'ci-log:verify-secrets' },
  ];

  // A1 — fake-green by NON-EXECUTION: a step that never ran → INCOMPLETE, never complete.
  {
    const r = evaluateRun({ blue_steps: [{ name: 'verify:rls', executed: false, evidence: null }], findings: [] });
    ok(r.verdict === 'INCOMPLETE', 'A1 unrun blue step ⇒ INCOMPLETE (not "clean")');
  }

  // A2 — fake-green by MISSING EVIDENCE: step claims executed but produced no artifact → RED violation.
  {
    const r = evaluateRun({ blue_steps: [{ name: 'verify:rls', executed: true, evidence: null }], findings: [] });
    ok(r.verdict === 'RED:violation', 'A2 executed-without-evidence ⇒ RED:violation (evidence-not-assumed)');
    ok(r.stepIssues.some((i) => i.issue === 'fake-green:no-evidence'), 'A2 flags fake-green:no-evidence');
  }

  // A3 — confirmed non-red-line finding, still OPEN (no guardrail) → INCOMPLETE, cannot close.
  {
    const r = evaluateRun({
      blue_steps: goodSteps,
      findings: [{ id: 'F1', class: 'other', severity: 'med', confirmed: true, disposition: 'open', guardrail: null, ledger_row: false }],
    });
    ok(r.verdict === 'INCOMPLETE', 'A3 confirmed finding w/o guardrail ⇒ INCOMPLETE (cannot close)');
  }

  // A4 — CLOSED WITHOUT GUARDRAIL (fake-green): disposition says landed but no guardrail/ledger → VIOLATION.
  {
    const r = evaluateRun({
      blue_steps: goodSteps,
      findings: [{ id: 'F2', class: 'other', severity: 'med', confirmed: true, disposition: 'guardrail-landed', guardrail: null, ledger_row: false }],
    });
    ok(r.verdict === 'RED:violation', 'A4 closed-without-guardrail ⇒ RED:violation');
    ok(r.findingStatuses[0].status === 'VIOLATION:closed-without-guardrail', 'A4 status = closed-without-guardrail');
  }

  // A5 — RED-LINE auto-fix (council bypass): rls finding auto-fixed → VIOLATION.
  {
    const r = evaluateRun({
      blue_steps: goodSteps,
      findings: [{ id: 'F3', class: 'rls', severity: 'high', confirmed: true, disposition: 'auto-fixed', guardrail: 'test/rls.spec.ts', ledger_row: true }],
    });
    ok(r.verdict === 'RED:violation', 'A5 red-line auto-fix ⇒ RED:violation (council bypass)');
    ok(r.findingStatuses[0].status === 'VIOLATION:council-bypass', 'A5 status = council-bypass');
  }

  // A6 — RED-LINE routed to COUNCIL (correct): disposition council-queued → terminally disposed.
  {
    const r = evaluateRun({
      blue_steps: goodSteps,
      findings: [{ id: 'F4', class: 'jwt', severity: 'high', confirmed: true, disposition: 'council-queued', guardrail: null, ledger_row: false }],
    });
    ok(r.verdict === 'ADVISORY-COMPLETE', 'A6 red-line council-queued ⇒ ADVISORY-COMPLETE (routed, not fixed)');
    ok(r.findingStatuses[0].closed === true && !r.findingStatuses[0].violation, 'A6 red-line finding terminally disposed via council');
  }

  // A7 — non-red-line finding properly closed: guardrail + ledger → closable.
  {
    const r = evaluateRun({
      blue_steps: goodSteps,
      findings: [{ id: 'F5', class: 'other', severity: 'low', confirmed: true, disposition: 'guardrail-landed', guardrail: 'tools/eslint-plugin-local/rule.js', ledger_row: true }],
    });
    ok(r.verdict === 'ADVISORY-COMPLETE', 'A7 finding closed w/ guardrail+ledger ⇒ ADVISORY-COMPLETE');
  }

  // A8 — fully clean run → ADVISORY-COMPLETE but gate is ALWAYS "advisory", NEVER a PASS gate.
  {
    const r = evaluateRun({ blue_steps: goodSteps, findings: [] });
    ok(r.verdict === 'ADVISORY-COMPLETE', 'A8 clean run w/ evidence ⇒ ADVISORY-COMPLETE');
    ok(r.gate === 'advisory', 'A8 gate === "advisory" (advisory-forever)');
  }

  // A9 — advisory-forever holds across EVERY branch: no evaluateRun result ever emits a gate PASS.
  {
    const branches = [
      evaluateRun({ blue_steps: [{ name: 'x', executed: false, evidence: null }], findings: [] }),
      evaluateRun({ blue_steps: goodSteps, findings: [{ id: 'v', class: 'rls', confirmed: true, disposition: 'auto-fixed', guardrail: 'g', ledger_row: true }] }),
      evaluateRun({ blue_steps: goodSteps, findings: [] }),
    ];
    ok(branches.every((b) => b.gate === 'advisory'), 'A9 gate is "advisory" on EVERY verdict branch (never PASS)');
  }

  // A10 — unconfirmed findings never block completion (they are advisory triage, not open work).
  {
    const r = evaluateRun({
      blue_steps: goodSteps,
      findings: [{ id: 'U1', class: 'rls', severity: 'high', confirmed: false, disposition: 'open', guardrail: null, ledger_row: false }],
    });
    ok(r.verdict === 'ADVISORY-COMPLETE', 'A10 unconfirmed finding ⇒ does not block completion');
  }
}

// ---------------------------------------------------------------------------
// PART B — real BLUE-arm smoke: the crt.sh asset-surface scout (fixture seam, no network)
// ---------------------------------------------------------------------------
async function partB() {
  console.log('PART B — blue-arm smoke: asset-surface-scan (crt.sh, hermetic fixture)');

  const domain = 'dowiz-staging.fly.dev';
  const knownHost = `app.${domain}`;
  const forgottenHost = `forgotten-preview.${domain}`;

  // A crt.sh-shaped JSON body carrying a KNOWN host + a NEW (forgotten preview) host.
  const crtBody = JSON.stringify([
    { common_name: knownHost, name_value: knownHost },
    { common_name: forgottenHost, name_value: `${forgottenHost}\n*.${forgottenHost}` },
  ]);
  const fixture = { [domain]: crtBody };
  const baseline = { updated_ts: '2026-01-01T00:00:00Z', domains: { [domain]: [domain, knownHost] } };

  const results = await scanDomains([domain], { fixture });
  const report = buildReport([domain], results, baseline);

  // B1 — the scout SURFACES the forgotten host as NEW (does not silently swallow the signal).
  ok(report.summary.total_new >= 1, 'B1 asset-scan surfaces the NEW forgotten-preview host');
  ok(report.domains[domain].new.includes(forgottenHost), 'B1 the exact forgotten host is flagged NEW');

  // B2 — a surface that MATCHES the baseline yields 0 NEW (no false alarm — the diff is real).
  const cleanFixture = { [domain]: JSON.stringify([{ common_name: knownHost, name_value: knownHost }]) };
  const cleanReport = buildReport([domain], await scanDomains([domain], { fixture: cleanFixture }), baseline);
  ok(cleanReport.summary.total_new === 0, 'B2 baseline-matching surface ⇒ 0 NEW (diff is real, not always-red)');
}

// ---------------------------------------------------------------------------
// PART C — structural: the RED arm is intake-only (no autonomous offensive capability)
// ---------------------------------------------------------------------------
async function partC() {
  console.log('PART C — structural: red arm is intake-only (no attack function)');
  const mod = await import(import.meta.url);
  // The module exports the verdict engine + grading, but NOTHING that generates attack traffic.
  const offensiveNames = ['attack', 'exploit', 'fireSqlmap', 'runAutorize', 'sendPayload', 'bruteForce'];
  ok(offensiveNames.every((n) => typeof mod[n] === 'undefined'), 'C1 no offensive/attack export exists in the harness');
  ok(typeof mod.classifyFinding === 'function' && typeof mod.evaluateRun === 'function', 'C2 exports the verdict engine (intake/grade only)');
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------
export async function main() {
  console.log('== security-redblue anti-cheat dry-run (hermetic, no attack traffic) ==\n');
  await partA();
  await partB();
  await partC();
  const total = pass + fail;
  console.log(`\n== ${pass}/${total} PASS ==`);
  if (fail) { console.error(`FAILED (${fail}): ${fails.join(' · ')}`); return 1; }
  console.log('anti-cheat: a broken/cheating run goes RED/INCOMPLETE; the loop never emits a gate PASS.');
  return 0;
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
if (isMain) {
  main().then((code) => { process.exitCode = code ?? 0; })
    .catch((e) => { console.error(`fatal: ${e.stack || e}`); process.exitCode = 1; });
}
