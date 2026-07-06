#!/usr/bin/env node
// audit-token-router.mjs — deterministic ($0, no LLM) post-hoc auditor of the TOKEN ROUTER /
// MODEL ROUTING discipline over Claude Code session transcripts.
//
// WHY (STRUCTURE-UPGRADE.md Part B, step B4 + B0 baseline): the token-reduction stack
// (tools/vsa, TOKEN ROUTER, MODEL ROUTING) is BUILT and MEASURED but was never ENFORCED — error
// class 7 ("discipline-triggered steps die; only hook-enforced artifacts survive"). Before B1's
// live deny-gate can be tuned without over-blocking (the #47 failure mode), we must MEASURE the
// real violation rate — VSA rule 0: measure, don't assume. This script is that measurement, and
// it doubles as the drift-check B4 (B1 stays armed) once B1 ships.
//
// It counts, per session transcript, the rules that are ROBUSTLY measurable from the LEAD
// transcript, and is HONEST about what it cannot see:
//   (a) Agent dispatches with NO explicit `model:`   — MODEL ROUTING violation   [EXIT-1 signal]
//   (e) Agent dispatches with `model: fable`         — Fable-off-for-lanes viol. [EXIT-1 signal]
//   (d) Agent prompts carrying a >1KB raw JSON blob with no `[route:*]` stamp     [advisory]
//   (c) runs of >=3 consecutive single-tool assistant turns (batching-miss cand.) [advisory]
//   peak lead-session context (input+cache_read+cache_creation)                   [datapoint]
//   (b) per-LANE 80K crossings are NOT emitted: sub-agent sidechains do not appear in the lead
//       transcript (verified 2026-07-06: isSidechain never True in lead files), so a lead-only
//       auditor CANNOT see them. Reported as visibility:"lead-only" — no fabricated number.
//
// EXIT: non-zero if any (a) or (e) hit across all inputs (the enforcement contract B4/EYE wire in).
// The BASELINE run reads the numbers regardless of exit code.
//
// USAGE:
//   node scripts/audit-token-router.mjs [--dir <transcriptDir>] [--last N] [--out report.jsonl] [file.jsonl ...]
//   node scripts/audit-token-router.mjs --self-test     # hermetic red/green proof (no real data)
// Transcript dir defaults to $AUDIT_TRANSCRIPT_DIR, else ~/.claude/projects/<cwd-slug> (Claude
// Code's per-project store; slug = absolute cwd with every '/' replaced by '-').

import { readFileSync, writeFileSync, readdirSync, statSync, mkdtempSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir, homedir } from 'node:os';

const JSON_BLOB_MIN = 1024; // bytes — the B1 check-6 threshold; below the crossover raw is CORRECT
const CTX_PEAK_HINT = 300_000; // the 300K session recycle directive (for the summary flag only)

// ── one transcript line → parsed, or null ───────────────────────────────────
function parseLine(line) {
  if (line.indexOf('"tool_use"') < 0 && line.indexOf('"usage"') < 0) return null;
  try { return JSON.parse(line); } catch { return null; }
}

// Does a prompt carry a raw JSON blob >= JSON_BLOB_MIN with no [route:*] stamp?
// Conservative: require a real object/array region (>=2 `":` key markers) so prose that merely
// mentions "json" or long narrative prompts (the fable grand-plan prompt was 3.3KB of prose) do
// NOT false-positive. A [route: ...] stamp anywhere exempts (route.mjs already measured it).
function hasUnstampedJsonBlob(prompt) {
  if (typeof prompt !== 'string' || prompt.length < JSON_BLOB_MIN) return false;
  if (/\[route:\s*(raw|frame|viz)\b/i.test(prompt)) return false;
  // fenced ```json / ``` blocks whose body looks like JSON
  const fences = prompt.match(/```[\w]*\n([\s\S]*?)```/g) || [];
  for (const f of fences) {
    const body = f.replace(/```[\w]*\n?/, '').replace(/```$/, '');
    if (body.length >= JSON_BLOB_MIN && /^[\s]*[[{]/.test(body) && (body.match(/"\s*:/g) || []).length >= 2) return true;
  }
  // bare brace/bracket run: first opener to a plausible balanced close, length-gated
  const start = prompt.search(/[[{]/);
  if (start >= 0) {
    const region = prompt.slice(start);
    if (region.length >= JSON_BLOB_MIN && (region.match(/"\s*:/g) || []).length >= 3) return true;
  }
  return false;
}

// ── audit one transcript file → tally object ────────────────────────────────
export function auditFile(path) {
  const t = {
    session: path.split('/').pop().replace(/\.jsonl$/, ''),
    agents_total: 0, no_model: 0, fable: 0, big_json: 0,
    batching_candidates: 0, peak_context: 0, visibility: 'lead-only',
    fable_slugs: [], no_model_slugs: [],
  };
  let data;
  try { data = readFileSync(path, 'utf8'); } catch { return { ...t, error: 'unreadable' }; }

  let singleToolStreak = 0;
  for (const line of data.split('\n')) {
    if (!line) continue;
    const d = parseLine(line);
    if (!d) continue;
    const msg = d.message || {};
    const usage = msg.usage;
    if (usage) {
      const ctx = (usage.input_tokens || 0) + (usage.cache_read_input_tokens || 0) + (usage.cache_creation_input_tokens || 0);
      if (ctx > t.peak_context) t.peak_context = ctx;
    }
    const content = msg.content;
    if (!Array.isArray(content)) continue;

    const toolUses = content.filter((b) => b && b.type === 'tool_use');
    // (c) batching-miss candidate: a run of >=3 assistant turns each with exactly ONE tool_use.
    if (msg.role === 'assistant' && toolUses.length > 0) {
      if (toolUses.length === 1) {
        singleToolStreak += 1;
        if (singleToolStreak === 3) t.batching_candidates += 1; // count the run once, when it reaches 3
      } else {
        singleToolStreak = 0;
      }
    }

    for (const b of toolUses) {
      if (b.name !== 'Agent' && b.name !== 'Task') continue; // exact — excludes TaskCreate/Update/Output/etc.
      const inp = b.input || {};
      t.agents_total += 1;
      const model = String(inp.model || '').toLowerCase();
      const slug = String(inp.description || inp.subagent_type || '?').slice(0, 48);
      if (!model) { t.no_model += 1; t.no_model_slugs.push(slug); }
      else if (model.includes('fable')) { t.fable += 1; t.fable_slugs.push(slug); }
      if (hasUnstampedJsonBlob(inp.prompt)) t.big_json += 1;
    }
  }
  return t;
}

// ── file selection ──────────────────────────────────────────────────────────
function defaultDir() {
  if (process.env.AUDIT_TRANSCRIPT_DIR) return process.env.AUDIT_TRANSCRIPT_DIR;
  const slug = process.cwd().replace(/\//g, '-');
  return join(homedir(), '.claude', 'projects', slug);
}
function gatherFiles(argv) {
  // flag VALUES (the token after --dir/--last/--out) are not input files, even if they end .jsonl
  const consumed = new Set();
  ['--dir', '--last', '--out'].forEach((f) => { const i = argv.indexOf(f); if (i >= 0) consumed.add(i + 1); });
  const explicit = argv.filter((a, i) => a.endsWith('.jsonl') && !consumed.has(i) && !a.startsWith('--'));
  if (explicit.length) return explicit;
  const dirIdx = argv.indexOf('--dir');
  const dir = dirIdx >= 0 ? argv[dirIdx + 1] : defaultDir();
  let files;
  try {
    files = readdirSync(dir).filter((f) => f.endsWith('.jsonl')).map((f) => join(dir, f));
  } catch { return []; }
  files.sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs);
  const lastIdx = argv.indexOf('--last');
  if (lastIdx >= 0) files = files.slice(0, Number(argv[lastIdx + 1]) || files.length);
  return files;
}

// ── hermetic self-test: the mandatory red→green proof ───────────────────────
function selfTest() {
  const fix = mkdtempSync(join(tmpdir(), 'audit-token-'));
  const mk = (name, lines) => { const p = join(fix, name); writeFileSync(p, lines.map((l) => JSON.stringify(l)).join('\n')); return p; };
  const agentLine = (input) => ({ type: 'assistant', message: { role: 'assistant', content: [{ type: 'tool_use', name: 'Agent', input }] } });

  const dirty = mk('dirty.jsonl', [
    agentLine({ description: 'read-only sweep', subagent_type: 'general-purpose', prompt: 'find X' }), // no model → (a)
    agentLine({ description: 'author plan', subagent_type: 'general-purpose', model: 'fable', prompt: 'write it' }), // fable → (e)
  ]);
  const clean = mk('clean.jsonl', [
    agentLine({ description: 'read-only sweep', subagent_type: 'Explore', model: 'haiku', prompt: 'find X' }),
    agentLine({ description: 'reason', subagent_type: 'general-purpose', model: 'opus', prompt: 'design Y' }),
    { type: 'assistant', message: { role: 'assistant', content: [{ type: 'tool_use', name: 'TaskCreate', input: {} }] } }, // NOT an Agent dispatch
  ]);

  const results = [];
  let ok = true;
  const assert = (n, cond) => { console.log(`  ${cond ? '✓' : '✗'} ${n}`); if (!cond) ok = false; };

  const rd = auditFile(dirty);
  assert('dirty: 2 agent dispatches counted', rd.agents_total === 2);
  assert('dirty: model-less dispatch flagged (a)', rd.no_model === 1);
  assert('dirty: fable dispatch flagged (e)', rd.fable === 1);
  results.push(rd);

  const rc = auditFile(clean);
  assert('clean: 2 agent dispatches counted (TaskCreate excluded)', rc.agents_total === 2);
  assert('clean: 0 model-less', rc.no_model === 0);
  assert('clean: 0 fable', rc.fable === 0);
  results.push(rc);

  // JSON-blob heuristic over/under-block guards
  const bigJson = '{' + '"k":"v",'.repeat(200) + '"end":1}'; // >1KB, many "key": markers
  assert('big unstamped JSON blob in prompt → flagged (d)', hasUnstampedJsonBlob('here: ' + bigJson) === true);
  assert('same blob WITH [route: raw 512] stamp → NOT flagged (over-block guard)', hasUnstampedJsonBlob('[route: raw 512] ' + bigJson) === false);
  assert('long prose prompt (no JSON) → NOT flagged (over-block guard)', hasUnstampedJsonBlob('x'.repeat(4000) + ' mentions json but no object') === false);

  const exitCode = results.reduce((s, r) => s + r.no_model + r.fable, 0) > 0 ? 1 : 0;
  assert('enforcement contract: dirty set forces exit 1', exitCode === 1);
  assert('enforcement contract: clean-only would exit 0', (rc.no_model + rc.fable) === 0);

  rmSync(fix, { recursive: true, force: true });
  if (!ok) { console.error('\n✗ audit-token-router self-test: FAILED'); process.exit(1); }
  console.log('\n✓ audit-token-router self-test: red (a/e→exit1) + green + over-block guards all pass.');
}

// ── main ────────────────────────────────────────────────────────────────────
function main() {
  const argv = process.argv.slice(2);
  if (argv.includes('--self-test')) return selfTest();

  const files = gatherFiles(argv);
  if (!files.length) {
    console.error('audit-token-router: no transcript files found (pass files, --dir, or set AUDIT_TRANSCRIPT_DIR).');
    process.exit(2);
  }
  const outIdx = argv.indexOf('--out');
  const outPath = outIdx >= 0 ? argv[outIdx + 1] : null;

  const rows = files.map(auditFile);
  const jsonl = rows.map((r) => JSON.stringify(r)).join('\n');
  if (outPath) writeFileSync(outPath, jsonl + '\n');

  // aggregate
  const agg = rows.reduce((a, r) => ({
    sessions: a.sessions + 1, agents: a.agents + r.agents_total,
    no_model: a.no_model + r.no_model, fable: a.fable + r.fable,
    big_json: a.big_json + r.big_json, batching: a.batching + r.batching_candidates,
    peak: Math.max(a.peak, r.peak_context),
  }), { sessions: 0, agents: 0, no_model: 0, fable: 0, big_json: 0, batching: 0, peak: 0 });

  console.log(jsonl);
  console.error('\n── TOKEN-ROUTER AUDIT (baseline) ─────────────────────────────');
  console.error(`sessions audited      : ${agg.sessions}`);
  console.error(`Agent dispatches      : ${agg.agents}`);
  console.error(`(a) no explicit model : ${agg.no_model}   ${agg.agents ? '(' + Math.round(100 * agg.no_model / agg.agents) + '% of dispatches)' : ''}   [EXIT-1]`);
  console.error(`(e) model: fable      : ${agg.fable}   [EXIT-1]`);
  console.error(`(d) unstamped >1KB JSON: ${agg.big_json}   [advisory]`);
  console.error(`(c) batching-miss cand.: ${agg.batching}   [advisory, human-judged]`);
  console.error(`peak lead-session ctx : ${agg.peak.toLocaleString()} ${agg.peak >= CTX_PEAK_HINT ? '⚠ ≥300K recycle threshold' : ''}`);
  console.error('(b) per-lane 80K       : not measurable from lead transcript (sub-agent sidechains absent)');
  console.error('──────────────────────────────────────────────────────────────');

  const exitCode = agg.no_model + agg.fable > 0 ? 1 : 0;
  process.exit(exitCode);
}

main();
