#!/usr/bin/env node
// audit-model-spend.mjs — deterministic ($0, no LLM) per-MODEL dollar-spend auditor over
// Claude Code session transcripts. Answers the operator's audit question (2026-07-07):
// "Which model takes the biggest share of the token-budget in MONEY terms — reasoning-heavy
// Opus, or huge-context Haiku/Sonnet?"  VSA rule 0: measure, don't assume.
//
// HONEST SCOPE (same visibility gap as scripts/audit-token-router.mjs): sub-agent sidechains do
// NOT appear in the lead transcript (isSidechain False on all lines, verified 2026-07-06), so this
// measures the LEAD (main-loop) spend only. That is the largest single visible cost line and the
// one the operator's "which model dominates" question is really about (the lead loop is pinned to
// Opus). Sub-agent lane $ is not measurable here — reported as a known blind spot, no fabricated number.
//
// Pricing (per-MTok) is loaded from the authoritative claude-api skill catalog (2026-06-24), NOT
// from memory — Opus 4.8 is $5/$25 with 1M context at STANDARD pricing (no long-context premium).
// Cache economics from shared/prompt-caching.md: cache WRITE = 1.25x input (5-min ephemeral, Claude
// Code default), cache READ = 0.1x input.
//
// USAGE:
//   node scripts/audit-model-spend.mjs [--dir <transcriptDir>] [--last N] [file.jsonl ...]
//   node scripts/audit-model-spend.mjs --self-test     # hermetic red/green proof (no real data)

import { readdirSync, statSync, createReadStream } from 'node:fs';
import { join } from 'node:path';
import { homedir } from 'node:os';
import { createInterface } from 'node:readline';

// ── pricing per-MTok ($), from claude-api skill catalog (cached 2026-06-24) ──
// [input, output, cacheWrite(1.25x in), cacheRead(0.1x in)]
const RATE = {
  opus:   { in: 5.00, out: 25.00 },   // claude-opus-4-8 — 1M ctx at standard price, no premium
  sonnet: { in: 3.00, out: 15.00 },   // claude-sonnet-5 / 4-6 (sticker; intro $2/$10 not applied — conservative/high)
  haiku:  { in: 1.00, out: 5.00 },    // claude-haiku-4-5
  fable:  { in: 10.00, out: 50.00 },  // claude-fable-5
  other:  { in: 5.00, out: 25.00 },   // unknown model → price as Opus (conservative)
};
const perTok = (r) => ({ in: r.in / 1e6, out: r.out / 1e6, cw: (r.in * 1.25) / 1e6, cr: (r.in * 0.1) / 1e6 });

// model string → bucket. Claude Code stamps e.g. "claude-opus-4-8[1m]"; match by family substring.
export function bucket(model) {
  const m = String(model || '').toLowerCase();
  if (!m) return null; // no model on this line → not an assistant usage row we can price
  if (m.includes('fable') || m.includes('mythos')) return 'fable';
  if (m.includes('opus')) return 'opus';
  if (m.includes('sonnet')) return 'sonnet';
  if (m.includes('haiku')) return 'haiku';
  return 'other';
}

// one usage row → dollars for its bucket
export function costOf(b, u) {
  const p = perTok(RATE[b]);
  const inp = u.input_tokens || 0;
  const out = u.output_tokens || 0;
  const cw = u.cache_creation_input_tokens || 0;
  const cr = u.cache_read_input_tokens || 0;
  return {
    dollars: inp * p.in + out * p.out + cw * p.cw + cr * p.cr,
    in: inp, out, cw, cr,
  };
}

function emptyTally() {
  const t = {};
  for (const b of ['opus', 'sonnet', 'haiku', 'fable', 'other']) t[b] = { dollars: 0, in: 0, out: 0, cw: 0, cr: 0, rows: 0 };
  return t;
}

async function auditFile(path, tally) {
  const rl = createInterface({ input: createReadStream(path, 'utf8'), crlfDelay: Infinity });
  for await (const line of rl) {
    if (line.indexOf('"usage"') < 0) continue;
    let d; try { d = JSON.parse(line); } catch { continue; }
    const msg = d.message || {};
    const u = msg.usage;
    if (!u) continue;
    const b = bucket(msg.model);
    if (!b) continue;
    const c = costOf(b, u);
    const bt = tally[b];
    bt.dollars += c.dollars; bt.in += c.in; bt.out += c.out; bt.cw += c.cw; bt.cr += c.cr; bt.rows += 1;
  }
}

function defaultDir() {
  if (process.env.AUDIT_TRANSCRIPT_DIR) return process.env.AUDIT_TRANSCRIPT_DIR;
  return join(homedir(), '.claude', 'projects', process.cwd().replace(/\//g, '-'));
}
function gatherFiles(argv) {
  const explicit = argv.filter((a) => a.endsWith('.jsonl') && !a.startsWith('--'));
  const dirIdx = argv.indexOf('--dir');
  const consumed = dirIdx >= 0 ? argv[dirIdx + 1] : null;
  const files = explicit.filter((f) => f !== consumed);
  if (files.length) return files;
  const dir = dirIdx >= 0 ? argv[dirIdx + 1] : defaultDir();
  let list;
  try { list = readdirSync(dir).filter((f) => f.endsWith('.jsonl')).map((f) => join(dir, f)); } catch { return []; }
  list.sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs);
  const lastIdx = argv.indexOf('--last');
  if (lastIdx >= 0) list = list.slice(0, Number(argv[lastIdx + 1]) || list.length);
  return list;
}

function selfTest() {
  let ok = true;
  const assert = (n, cond) => { console.log(`  ${cond ? '✓' : '✗'} ${n}`); if (!cond) ok = false; };
  // bucketing
  assert('opus[1m] → opus', bucket('claude-opus-4-8[1m]') === 'opus');
  assert('sonnet-5 → sonnet', bucket('claude-sonnet-5') === 'sonnet');
  assert('haiku → haiku', bucket('claude-haiku-4-5') === 'haiku');
  assert('fable → fable', bucket('claude-fable-5') === 'fable');
  assert('empty model → null (skip)', bucket('') === null);
  // cost math — 1M input tokens on Opus = $5.00 exactly
  assert('1M opus input = $5.00', Math.abs(costOf('opus', { input_tokens: 1e6 }).dollars - 5.0) < 1e-9);
  assert('1M opus output = $25.00', Math.abs(costOf('opus', { output_tokens: 1e6 }).dollars - 25.0) < 1e-9);
  assert('1M opus cache-read = $0.50 (0.1x)', Math.abs(costOf('opus', { cache_read_input_tokens: 1e6 }).dollars - 0.5) < 1e-9);
  assert('1M opus cache-write = $6.25 (1.25x)', Math.abs(costOf('opus', { cache_creation_input_tokens: 1e6 }).dollars - 6.25) < 1e-9);
  assert('1M haiku input = $1.00', Math.abs(costOf('haiku', { input_tokens: 1e6 }).dollars - 1.0) < 1e-9);
  assert('1M fable output = $50.00', Math.abs(costOf('fable', { output_tokens: 1e6 }).dollars - 50.0) < 1e-9);
  // FALSIFIABLE: a wrong rate must fail — opus input priced at haiku's $1 would be caught
  assert('RED: opus input is NOT $1/M (would catch a haiku-rate bug)', Math.abs(costOf('opus', { input_tokens: 1e6 }).dollars - 1.0) > 1e-6);
  if (!ok) { console.error('\n✗ audit-model-spend self-test FAILED'); process.exit(1); }
  console.log('\n✓ audit-model-spend self-test: bucketing + $-math + RED case all pass.');
}

async function main() {
  const argv = process.argv.slice(2);
  if (argv.includes('--self-test')) return selfTest();
  const files = gatherFiles(argv);
  if (!files.length) { console.error('audit-model-spend: no transcripts found.'); process.exit(2); }
  const tally = emptyTally();
  for (const f of files) await auditFile(f, tally);

  const buckets = Object.entries(tally).filter(([, v]) => v.rows > 0).sort((a, b) => b[1].dollars - a[1].dollars);
  const total = buckets.reduce((s, [, v]) => s + v.dollars, 0);
  const totTok = buckets.reduce((s, [, v]) => s + v.in + v.out + v.cw + v.cr, 0);
  const fmt = (n) => '$' + n.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  const M = (n) => (n / 1e6).toFixed(1) + 'M';

  console.log(`\n── LEAD-LOOP $ SPEND BY MODEL (${files.length} transcripts) ─────────────`);
  console.log('model    share$   dollars       in     out   cache-wr cache-rd   billed-tok');
  for (const [b, v] of buckets) {
    const share = total ? (100 * v.dollars / total).toFixed(1).padStart(5) : '  0.0';
    console.log(
      `${b.padEnd(7)} ${share}%  ${fmt(v.dollars).padStart(11)}  ${M(v.in).padStart(6)} ${M(v.out).padStart(6)} ${M(v.cw).padStart(7)} ${M(v.cr).padStart(8)}   ${M(v.in + v.out + v.cw + v.cr).padStart(7)}`,
    );
  }
  console.log('─────────────────────────────────────────────────────────────────────');
  console.log(`TOTAL (lead-loop, visible)  ${fmt(total).padStart(11)}   over ${M(totTok)} billed tokens`);
  // Where does the $ go: cache-read vs fresh input vs output — the reduction levers
  const allIn = buckets.reduce((s, [, v]) => s + v.in, 0), allOut = buckets.reduce((s, [, v]) => s + v.out, 0);
  const allCw = buckets.reduce((s, [, v]) => s + v.cw, 0), allCr = buckets.reduce((s, [, v]) => s + v.cr, 0);
  const dIn = allIn * (5 / 1e6), dOut = allOut * (25 / 1e6), dCw = allCw * (6.25 / 1e6), dCr = allCr * (0.5 / 1e6);
  console.log(`\n$ by lever (Opus-priced approximation of the mix):`);
  console.log(`  fresh input : ${fmt(dIn).padStart(11)}  (${(100 * dIn / total).toFixed(1)}%)  ← distill/graph/frame cut this`);
  console.log(`  output      : ${fmt(dOut).padStart(11)}  (${(100 * dOut / total).toFixed(1)}%)  ← output-constraining/effort cut this`);
  console.log(`  cache-write : ${fmt(dCw).padStart(11)}  (${(100 * dCw / total).toFixed(1)}%)  ← stable-prefix caching amortizes this`);
  console.log(`  cache-read  : ${fmt(dCr).padStart(11)}  (${(100 * dCr / total).toFixed(1)}%)  ← already 0.1x; big volume, cheap`);
  console.log('\n(Scope: lead-loop only; sub-agent sidechain $ is NOT visible in lead transcripts.)');
}

main();
