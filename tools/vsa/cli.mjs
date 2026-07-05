#!/usr/bin/env node
// VSA CLI — the project's Transmitter + local vector math + token telemetry.
//
//   encode  <file.json|->            JSON → frame (stdout)
//   decode  <frame.txt|->            frame → JSON (stdout)
//   tokens  <file|->                 count tokens (BPE)
//   bench   [dir]                    before/after table over bench/payloads → BENCH.md
//   match   <query> <corpus.jsonl>   rank {id,text} lines by hypervector similarity
//   pe      <predFile> <actualFile>  prediction-error (1−cos) between two texts
//   spec                             print the one-time frame decode spec
//
// Ledger: every encode/bench appends {ts,bytesIn,bytesOut,tokIn,tokOut} to
// telemetry/usage.jsonl — the before/after trail the operator asked for.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { encode, decode, FRAME_SPEC } from './src/codec.mjs';
import { match, predictionError } from './src/hv.mjs';
import { shc, integrityGate } from './src/integrity.mjs';
import { countTokens, tokenMethod } from './src/tokens.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const TELEMETRY = path.join(HERE, 'telemetry');

function readInput(arg) {
  return arg === '-' || arg === undefined
    ? fs.readFileSync(0, 'utf8')
    : fs.readFileSync(arg, 'utf8');
}

function ledger(row) {
  fs.mkdirSync(TELEMETRY, { recursive: true });
  fs.appendFileSync(
    path.join(TELEMETRY, 'usage.jsonl'),
    JSON.stringify({ ts: new Date().toISOString(), ...row }) + '\n',
  );
}

const [, , cmd, ...args] = process.argv;

switch (cmd) {
  case 'encode': {
    const raw = readInput(args[0]);
    const value = JSON.parse(raw);
    const frame = encode(value);
    const [tokIn, tokOut] = [await countTokens(JSON.stringify(value)), await countTokens(frame)];
    ledger({ op: 'encode', src: args[0] || 'stdin', bytesIn: raw.length, bytesOut: frame.length, tokIn, tokOut });
    process.stdout.write(frame + '\n');
    break;
  }
  case 'decode': {
    process.stdout.write(JSON.stringify(decode(readInput(args[0]))) + '\n');
    break;
  }
  case 'tokens': {
    const text = readInput(args[0]);
    console.log(await countTokens(text), `(${await tokenMethod()})`);
    break;
  }
  case 'spec': {
    console.log(FRAME_SPEC);
    break;
  }
  case 'match': {
    const [query, corpusFile] = args;
    const items = fs
      .readFileSync(corpusFile, 'utf8')
      .split('\n')
      .filter(Boolean)
      .map((l) => JSON.parse(l));
    for (const r of match(query, items).slice(0, 5)) {
      console.log(r.score.toFixed(3), r.id, '—', String(r.text).slice(0, 90));
    }
    break;
  }
  case 'pe': {
    const pe = predictionError(readInput(args[0]), readInput(args[1]));
    console.log(pe.toFixed(4));
    process.exitCode = pe > Number(process.env.VSA_PE_THRESHOLD || 0.5) ? 1 : 0;
    break;
  }
  case 'shc': {
    // State Hash Checksum: shc <state.json|-> [fields,comma,separated]
    const fields = args[1] ? args[1].split(',') : undefined;
    console.log(shc(JSON.parse(readInput(args[0])), fields));
    break;
  }
  case 'integrity': {
    // Pre-flight gate: integrity <expected.json> <actual.json> [--fields a,b] [--age-ms N --corridor-ms M]
    const flag = (name) => {
      const i = args.indexOf(name);
      return i === -1 ? undefined : args[i + 1];
    };
    const g = integrityGate(JSON.parse(readInput(args[0])), JSON.parse(readInput(args[1])), {
      fields: flag('--fields')?.split(','),
      ageMs: Number(flag('--age-ms') || 0),
      corridorMs: Number(flag('--corridor-ms') || 0),
    });
    ledger({ op: 'integrity', pass: g.pass, inCorridor: g.inCorridor, drift: Number(g.drift.toFixed(4)), mismatches: g.mismatches.length });
    console.log(JSON.stringify(g));
    process.exitCode = g.pass ? 0 : 1;
    break;
  }
  case 'lane': {
    // FCE telemetry: lane <ok|fail> <tokens> [label] — one row per finished agent lane.
    const [outcome, tok, label] = args;
    if (!['ok', 'fail'].includes(outcome) || !Number(tok)) {
      console.error('usage: vsa lane <ok|fail> <tokens> [label]');
      process.exit(2);
    }
    ledger({ op: 'lane', ok: outcome === 'ok', tok: Number(tok), label: label || '' });
    break;
  }
  case 'report': {
    // Cumulative before/after from the usage ledger — the operator's running savings counter.
    const file = path.join(TELEMETRY, 'usage.jsonl');
    if (!fs.existsSync(file)) {
      console.log('no usage recorded yet');
      break;
    }
    const rows = fs.readFileSync(file, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l));
    const enc = rows.filter((r) => r.tokIn !== undefined);
    const tokIn = enc.reduce((a, r) => a + r.tokIn, 0);
    const tokOut = enc.reduce((a, r) => a + r.tokOut, 0);
    console.log(`encodes: ${enc.length}`);
    console.log(`tokens in (raw/min JSON): ${tokIn}`);
    console.log(`tokens out (frames):      ${tokOut}`);
    console.log(`tokens saved:             ${tokIn - tokOut} (${((1 - tokOut / Math.max(tokIn, 1)) * 100).toFixed(1)}%)`);
    const lanes = rows.filter((r) => r.op === 'lane');
    if (lanes.length) {
      const okLanes = lanes.filter((r) => r.ok);
      const laneTok = lanes.reduce((a, r) => a + r.tok, 0);
      // FCE = successful lanes per 100K tokens spent across all lanes (higher = healthier fan-out).
      console.log(`lanes ok/total:           ${okLanes.length}/${lanes.length}`);
      console.log(`FCE (ok lanes / 100K tok): ${((okLanes.length / Math.max(laneTok, 1)) * 100000).toFixed(2)}`);
    }
    const gates = rows.filter((r) => r.op === 'integrity');
    if (gates.length) {
      const broken = gates.filter((r) => !r.pass);
      console.log(`integrity gates:          ${gates.length} (${broken.length} circuit-broke ≈ ${broken.length * 17000} lane-floor tok saved)`);
    }
    break;
  }
  case 'bench': {
    const dir = args[0] || path.join(HERE, 'bench', 'payloads');
    const rows = [];
    for (const f of fs.readdirSync(dir).filter((f) => f.endsWith('.json')).sort()) {
      const raw = fs.readFileSync(path.join(dir, f), 'utf8');
      const value = JSON.parse(raw);
      const minified = JSON.stringify(value);
      const frame = encode(value);
      const ok = JSON.stringify(decode(frame)) === minified;
      const [tokRaw, tokMin, tokFrame] = [
        await countTokens(raw),
        await countTokens(minified),
        await countTokens(frame),
      ];
      rows.push({ f, tokRaw, tokMin, tokFrame, ok, saveVsMin: 1 - tokFrame / tokMin });
      ledger({ op: 'bench', src: f, tokIn: tokMin, tokOut: tokFrame, roundTrip: ok });
    }
    const method = await tokenMethod();
    let md = `# VSA codec bench — ${new Date().toISOString()}\n\nTokenizer: ${method}. `;
    md += `"min" = minified JSON (the honest baseline — pretty-printing is free to remove); "frame" = VSA1.\n\n`;
    md += `| payload | raw tok | min tok | frame tok | save vs min | lossless |\n|---|---|---|---|---|---|\n`;
    for (const r of rows) {
      md += `| ${r.f} | ${r.tokRaw} | ${r.tokMin} | ${r.tokFrame} | ${(r.saveVsMin * 100).toFixed(1)}% | ${r.ok ? '✅' : '❌ FAIL'} |\n`;
    }
    const tot = rows.reduce((a, r) => ({ min: a.min + r.tokMin, fr: a.fr + r.tokFrame }), { min: 0, fr: 0 });
    md += `| **TOTAL** | | **${tot.min}** | **${tot.fr}** | **${((1 - tot.fr / tot.min) * 100).toFixed(1)}%** | |\n`;
    fs.writeFileSync(path.join(HERE, 'BENCH.md'), md);
    console.log(md);
    if (rows.some((r) => !r.ok)) process.exit(1);
    break;
  }
  default:
    console.error('usage: vsa encode|decode|tokens|bench|match|pe|spec|shc|integrity|lane|report');
    process.exit(2);
}
