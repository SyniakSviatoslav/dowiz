#!/usr/bin/env node
// plane-report — the plane-maintainer's status digest (charter: docs/governance/plane-maintainer-agent.md).
// Runs the deterministic gates, assembles ONE markdown digest, writes it (versioned), and pushes the
// one-line verdict to every configured channel. Always emits the full report (rule-loop-report-always).
//
// Deterministic channels only (committed md + Telegram). The GitHub PR/issue and the research/OSS
// "net-new for the plane" section are produced by the scheduled AGENT (it has web tools); this script
// writes the skeleton section for the agent to fill and, with --github-issue-on-fail, files an issue.
//
// Run:  node scripts/plane-report.mjs [--github-issue-on-fail] [--stdout-only]
// Env:  TELEGRAM_BOT_TOKEN + PLANE_REPORT_CHAT_ID (optional — channel skipped cleanly if unset)
import { execSync, spawnSync } from 'node:child_process';
import { writeFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = process.cwd();
const today = new Date().toISOString().slice(0, 10);
const nowIso = new Date().toISOString();
const STDOUT_ONLY = process.argv.includes('--stdout-only');
const ISSUE_ON_FAIL = process.argv.includes('--github-issue-on-fail');

const capture = (cmd) => {
  try { return { ok: true, out: execSync(cmd, { cwd: ROOT, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] }) }; }
  catch (e) { return { ok: false, out: (e.stdout || '') + (e.stderr || e.message || '') }; }
};

// Telemetry seam (ADR-plane-telemetry-and-calibration): lifecycle events via the single egress
// choke-point CLI. Best-effort — a missing/failed plane-telemetry.mjs NEVER affects this report's
// behavior or exit code. spawnSync arg arrays, shell:false (R2-M3 — no string interpolation).
const RUN_ID = `plane-${nowIso.slice(0, 16).replace(/:/g, '-')}-00Z`; // firing-timestamp-derived (M2)
const telemetry = (args) => {
  try {
    const r = spawnSync('node', [join(ROOT, 'scripts/plane-telemetry.mjs'), ...args],
      { cwd: ROOT, encoding: 'utf8', shell: false });
    return { ok: r.status === 0, out: (r.stdout || '') + (r.stderr || '') };
  } catch (e) { return { ok: false, out: e.message || '' }; }
};
if (!STDOUT_ONLY) {
  telemetry(['emit', '--run-id', RUN_ID, '--emitter', 'plane-report', '--kind', 'report', '--step', 'REPORT',
    '--outcome', 'pass', '--target', 'report_start', '--detail', 'plane-report run started']);
}

// 1. Gates
const guard = capture('node scripts/plane-guard.mjs --json');
let guardJson = null;
try { guardJson = JSON.parse(guard.out); } catch {}
const verdict = guardJson?.verdict ?? (guard.ok ? 'PASS' : 'FAIL');
const hardFails = guardJson?.results?.filter((r) => r.level === 'hard' && !r.ok) ?? [];

// 2. Health pass (advisory)
const health = capture('node scripts/agent-health-pass.mjs --stdout');
const healthWarns = (health.out.match(/⚠️ \*\*(.+?)\*\*/g) || []).map((w) => w.replace(/⚠️ \*\*|\*\*/g, ''));

// 3. Assemble the digest
// Telemetry channel-status line (H3 fix): every digest states whether egress happened —
// silence must never be mistaken for success. Degrades to an explicit "unavailable".
const tstat = telemetry(['digest', '--status-line']);
const telemetryLine = tstat.ok ? (tstat.out.trim().split('\n').pop() || 'telemetry=unavailable') : 'telemetry=unavailable';
const emoji = verdict === 'PASS' ? '🟢' : '🔴';
const patternLines = (guardJson?.results ?? []).map(
  (r) => `| ${r.ok ? '✅' : r.level === 'hard' ? '❌' : '⚠️'} | ${r.pattern} | ${r.name} | ${r.detail} |`
).join('\n');

const md = `# Plane status — ${today}

${emoji} **${verdict}** · generated ${nowIso} by \`scripts/plane-report.mjs\`
Charter & autonomy envelope: [plane-maintainer-agent.md](./plane-maintainer-agent.md)
Telemetry: ${telemetryLine} · run_id=${RUN_ID}

## 11-pattern gate (\`plane-guard\`)
${guardJson ? `${guardJson.results.filter((r) => r.level === 'hard' && r.ok).length}/${guardJson.results.filter((r) => r.level === 'hard').length} hard checks pass · ${guardJson.softFails} soft warn(s)` : '⚠️ gate output unparseable — see raw below'}

| | pattern | check | detail |
|---|---|---|---|
${patternLines}
${hardFails.length ? `\n### ❌ Hard fails — carry-forward\n${hardFails.map((r) => `- **${r.pattern}**: ${r.detail}`).join('\n')}` : ''}

## Harness health (advisory — \`agent-health-pass\`)
${healthWarns.length ? healthWarns.map((w) => `- ⚠️ ${w}`).join('\n') : '- ✅ no warnings'}

## Net-new for the plane (research / OSS scout)
<!-- The scheduled agent fills this each run: trigger-matched OSS candidates (TOOLING-REGISTRY.md),
     upstream releases of adopted deps, relevant research. Advisory — adoption is a separate decision. -->
_(populated by the scheduled agent)_

## Actions taken this run
<!-- The agent appends: staging fixes committed/deployed (with proof), PRs opened, escalations raised. -->
_(populated by the scheduled agent)_
`;

// 4. Emit + write + dispatch
console.log(md);
if (STDOUT_ONLY) process.exit(verdict === 'PASS' ? 0 : 1);

try { mkdirSync(join(ROOT, 'docs/governance'), { recursive: true }); } catch {}
const outPath = join(ROOT, `docs/governance/plane-status-${today}.md`);
writeFileSync(outPath, md);
console.error(`\n[plane-report] wrote ${outPath}`);

// Telegram (skips cleanly if unconfigured)
const tgToken = process.env.TELEGRAM_BOT_TOKEN;
const tgChat = process.env.PLANE_REPORT_CHAT_ID;
if (tgToken && tgChat) {
  const text = `${emoji} dowiz plane ${verdict} — ${today}\n${hardFails.length ? hardFails.map((r) => `❌ ${r.pattern}`).join('\n') : 'all 11 patterns enforced'}${healthWarns.length ? `\n⚠️ ${healthWarns.length} health warn(s)` : ''}`;
  try {
    const res = await fetch(`https://api.telegram.org/bot${tgToken}/sendMessage`, {
      method: 'POST', headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ chat_id: tgChat, text, disable_web_page_preview: true }),
    });
    console.error(`[plane-report] telegram: ${res.ok ? 'sent' : 'HTTP ' + res.status}`);
  } catch (e) { console.error(`[plane-report] telegram failed: ${e.message}`); }
} else {
  console.error('[plane-report] telegram: skipped (TELEGRAM_BOT_TOKEN / PLANE_REPORT_CHAT_ID unset)');
}

// GitHub issue on hard fail (opt-in). spawnSync ARG ARRAY, shell:false — gate details are
// untrusted free text; never interpolate them into a shell line (R2-M3 injection fix).
if (ISSUE_ON_FAIL && hardFails.length) {
  const title = `plane-guard FAIL ${today}: ${hardFails.map((r) => r.pattern).join(', ')}`;
  const body = hardFails.map((r) => `- **${r.pattern}** — ${r.name}: ${r.detail}`).join('\n');
  let r;
  try {
    const gr = spawnSync('gh', ['issue', 'create', '--title', title, '--body', body, '--label', 'plane-guard'],
      { cwd: ROOT, encoding: 'utf8', shell: false });
    r = { ok: gr.status === 0, out: (gr.stdout || '') + (gr.stderr || gr.error?.message || '') };
  } catch (e) { r = { ok: false, out: e.message || '' }; }
  console.error(`[plane-report] github issue: ${r.ok ? 'filed' : 'skipped/failed — ' + r.out.slice(0, 120)}`);
}

// Telemetry lifecycle close + per-run publish cadence (R3-1): report_done carries the verdict,
// then ONE batched plumbing push to telemetry/plane. Best-effort — never alters the exit code.
if (!STDOUT_ONLY) {
  telemetry(['emit', '--run-id', RUN_ID, '--emitter', 'plane-report', '--kind', 'report', '--step', 'REPORT',
    '--outcome', verdict === 'PASS' ? 'pass' : 'fail', '--target', 'report_done',
    '--detail', `verdict=${verdict} hard_fails=${hardFails.length} health_warns=${healthWarns.length}`]);
  const pub = telemetry(['publish', '--run-id', RUN_ID]);
  if (!pub.ok) console.error(`[plane-report] telemetry publish: failed (flagged in digest status line) — ${pub.out.trim().slice(0, 120)}`);
}

process.exit(verdict === 'PASS' ? 0 : 1);
