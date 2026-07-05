#!/usr/bin/env node
// Dispatch composer — makes the AGENTS.md VSA rule #1 a one-liner: a task prompt with
// JSON attachments as frames (spec included ONCE), ready to paste into an Agent dispatch.
//
//   node tools/vsa/dispatch.mjs --task "Audit these payloads for X" a.json b.json
//
// Prints the composed prompt to stdout and a stderr summary of tokens saved vs attaching
// the raw JSON (the per-dispatch before/after line the usage ledger records).

import fs from 'node:fs';
import path from 'node:path';
import { encode, FRAME_SPEC } from './src/codec.mjs';
import { countTokens } from './src/tokens.mjs';

const args = process.argv.slice(2);
const taskIdx = args.indexOf('--task');
if (taskIdx === -1 || !args[taskIdx + 1]) {
  console.error(
    'usage: dispatch.mjs --task "<instructions>" [--expect exp.json --actual act.json [--fields a,b] [--corridor-ms N --age-ms M]] <file.json> [more.json…]',
  );
  process.exit(2);
}
const task = args[taskIdx + 1];

// ── Integrity Gate (pre-flight circuit breaker, $0) ─────────────────────────
// If the state this dispatch is premised on no longer matches the source of truth,
// the lane never launches — killing the corrupted flow before it spends its ~17K floor.
const flagIdx = {};
for (const f of ['--expect', '--actual', '--fields', '--corridor-ms', '--age-ms']) {
  const i = args.indexOf(f);
  if (i !== -1) flagIdx[f] = i;
}
const consumed = new Set([taskIdx, taskIdx + 1]);
for (const i of Object.values(flagIdx)) consumed.add(i).add(i + 1);
if (flagIdx['--expect'] !== undefined || flagIdx['--actual'] !== undefined) {
  if (flagIdx['--expect'] === undefined || flagIdx['--actual'] === undefined) {
    console.error('--expect and --actual must be given together');
    process.exit(2);
  }
  const { integrityGate } = await import('./src/integrity.mjs');
  const g = integrityGate(
    JSON.parse(fs.readFileSync(args[flagIdx['--expect'] + 1], 'utf8')),
    JSON.parse(fs.readFileSync(args[flagIdx['--actual'] + 1], 'utf8')),
    {
      fields: flagIdx['--fields'] !== undefined ? args[flagIdx['--fields'] + 1].split(',') : undefined,
      corridorMs: Number(flagIdx['--corridor-ms'] !== undefined ? args[flagIdx['--corridor-ms'] + 1] : 0),
      ageMs: Number(flagIdx['--age-ms'] !== undefined ? args[flagIdx['--age-ms'] + 1] : 0),
    },
  );
  if (!g.pass) {
    console.error(
      `[vsa dispatch] CIRCUIT-BREAK: state diverged (${g.mismatches.join(', ')}; drift ${g.drift.toFixed(3)}; ` +
        `shc ${g.shcExpected}≠${g.shcActual}) — lane NOT composed; re-snapshot state and retry. ~17K floor saved.`,
    );
    process.exit(3);
  }
  if (g.inCorridor) {
    console.error(
      `[vsa dispatch] WARN in-corridor drift (${g.mismatches.join(', ')}) — proceeding, verify on landing.`,
    );
  }
}

const files = args.filter((a, i) => !consumed.has(i));
if (files.length === 0) {
  console.error('at least one JSON attachment required (otherwise you need no frame)');
  process.exit(2);
}

// Crossover-aware (the VSA-VIZ finding applied): frame ONLY attachments that actually shrink, and
// include the once-per-prompt FRAME_SPEC only if the total framing win beats the spec's ~fixed cost.
// A payload that frames larger than its raw JSON is sent RAW — framing is never a net loss.
const specTok = await countTokens(FRAME_SPEC);
const items = [];
for (const f of files) {
  const value = JSON.parse(fs.readFileSync(f, 'utf8'));
  const raw = JSON.stringify(value);
  const framed = encode(value);
  const [rawTok, frameTok] = await Promise.all([countTokens(raw), countTokens(framed)]);
  items.push({ name: path.basename(f), raw, framed, rawTok, frameTok });
}
const winners = items.filter((it) => it.frameTok < it.rawTok);
const framingWin = winners.reduce((s, it) => s + (it.rawTok - it.frameTok), 0);
const useFrames = framingWin > specTok; // spec only pays off if the aggregate win covers it

let out = task.trimEnd() + '\n\n';
if (useFrames) out += `Attached data is VSA1-framed where marked (VSA1). ${FRAME_SPEC}\n`;
let rawTotal = 0;
for (const it of items) {
  rawTotal += it.rawTok;
  const frameThis = useFrames && it.frameTok < it.rawTok;
  out += `\n--- ${it.name}${frameThis ? ' (VSA1)' : ''} ---\n${frameThis ? it.framed : it.raw}\n`;
}
const outTok = await countTokens(out);
const taskTok = await countTokens(task);
process.stdout.write(out);
console.error(
  `[vsa dispatch] ${items.length} attach, raw≈${rawTotal} tok → prompt ${outTok} tok; ` +
    (useFrames
      ? `framed ${winners.length}/${items.length} (spec ${specTok}); saved ≈${rawTotal + taskTok - outTok} tok`
      : `all RAW — framing would not beat the ${specTok}-tok spec (below crossover)`),
);
