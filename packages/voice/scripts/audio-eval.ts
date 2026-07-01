// Real-audio eval harness — the H1 "harness-deterministic" gate artifact (ADR-0015 §4.2).
//
// Feeds a FIXED corpus of pre-recorded WAVs through the REAL WhisperProvider (TransformersTranscriber
// → the same deterministic matcher the storefront uses) and scores it. Given a pinned model + greedy
// decode, a fixed WAV set produces the same numbers every run — so this report IS the gate evidence,
// distinct from cloud CI (which only proves matcher/gate wiring on text, never transcription).
//
// It measures, per shipped locale:
//   - IRA (intent-recognition accuracy): resolved kind == expected kind, over clips with an expected intent.
//   - dangerous-misfire: a clip resolving to a WRONG *STATEFUL* intent, or any resolution a user could
//     read as a dietary/safety assertion (a dietary-named category auto-apply). Bounded by the gate.
//   - fail-quiet correctness: a non-command clip (expected=null) must resolve to NO intent.
//
// NOTE ON CORPUS: the launch-grade corpus is the C2-consented, ≥300-per-locale, ≥15-speaker dataset
// (recruited adult speakers — NOT the platform's workforce; see resolution.md C-4). This harness runs
// on whatever manifest it is given; a tiny smoke manifest proves the PIPELINE, not a launch gate.
//
// Usage:  node --import tsx packages/voice/scripts/audio-eval.ts <manifest.json> [--out report.json] [--dtype fp32]
// Manifest: [{ "wav": "<path|url>", "locale": "sq|en|uk", "expected_kind": "SET_SORT"|null,
//             "expected_args"?: {..}, "transcript_contains"?: "substr", "menu"?: MenuContext }]

import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname, isAbsolute } from 'node:path';
import { classify } from '../src/capability-table.js';
import { isDietaryCategory } from '../src/dietary-denylist.js';
import { WhisperProvider } from '../src/whisper-provider.js';
import { TransformersTranscriber } from '../src/transformers-transcriber.js';
import type { Locale, MenuContext } from '../src/matcher.js';
import type { PcmAudio } from '../src/transcriber.js';

interface Clip {
  wav: string;
  locale: Locale;
  expected_kind: string | null;
  expected_args?: Record<string, unknown>;
  transcript_contains?: string;
  menu?: MenuContext;
}

const DEFAULT_MENU: MenuContext = { products: [], categories: [] };

/** Minimal WAV reader: 16-bit PCM, mono, 16 kHz → Float32 in [-1,1]. Whisper's expected input format. */
function decodeWav16kMono(buf: Buffer): PcmAudio {
  if (buf.toString('ascii', 0, 4) !== 'RIFF' || buf.toString('ascii', 8, 12) !== 'WAVE') {
    throw new Error('not a RIFF/WAVE file');
  }
  let off = 12;
  let fmt: { channels: number; sampleRate: number; bits: number } | null = null;
  let dataOff = -1;
  let dataLen = 0;
  while (off + 8 <= buf.length) {
    const id = buf.toString('ascii', off, off + 4);
    const size = buf.readUInt32LE(off + 4);
    const body = off + 8;
    if (id === 'fmt ') {
      fmt = { channels: buf.readUInt16LE(body + 2), sampleRate: buf.readUInt32LE(body + 4), bits: buf.readUInt16LE(body + 14) };
    } else if (id === 'data') {
      dataOff = body;
      dataLen = size;
    }
    off = body + size + (size % 2); // chunks are word-aligned
  }
  if (!fmt || dataOff < 0) throw new Error('missing fmt/data chunk');
  if (fmt.bits !== 16) throw new Error(`expected 16-bit PCM, got ${fmt.bits}-bit`);
  if (fmt.channels !== 1) throw new Error(`expected mono, got ${fmt.channels} channels — pre-convert to mono 16kHz`);
  if (fmt.sampleRate !== 16000) throw new Error(`expected 16 kHz, got ${fmt.sampleRate} Hz — pre-resample to 16kHz`);
  const n = Math.floor(dataLen / 2);
  const out = new Float32Array(n);
  for (let i = 0; i < n; i++) out[i] = buf.readInt16LE(dataOff + i * 2) / 32768;
  return out;
}

async function loadWav(ref: string, manifestDir: string): Promise<PcmAudio> {
  if (/^https?:\/\//.test(ref)) {
    const res = await fetch(ref, { signal: AbortSignal.timeout(60000) });
    if (!res.ok) throw new Error(`fetch ${ref}: ${res.status}`);
    return decodeWav16kMono(Buffer.from(await res.arrayBuffer()));
  }
  return decodeWav16kMono(readFileSync(isAbsolute(ref) ? ref : resolve(manifestDir, ref)));
}

function argsMatch(expected: Record<string, unknown> | undefined, got: Readonly<Record<string, unknown>>): boolean {
  if (!expected) return true;
  return Object.entries(expected).every(([k, v]) => got[k] === v);
}

async function main() {
  const manifestPath = process.argv[2];
  if (!manifestPath) {
    console.error('usage: audio-eval.ts <manifest.json> [--out report.json] [--dtype fp32|q8] [--device cpu|webgpu]');
    process.exit(1);
  }
  const outArg = process.argv.indexOf('--out');
  const outPath = outArg > 0 ? process.argv[outArg + 1] : null;
  const dtypeArg = process.argv.indexOf('--dtype');
  const dtype = (dtypeArg > 0 ? process.argv[dtypeArg + 1] : 'q8') as 'fp32' | 'q8';
  const deviceArg = process.argv.indexOf('--device');
  const device = (deviceArg > 0 ? process.argv[deviceArg + 1] : 'cpu') as 'cpu' | 'webgpu';

  const manifestDir = dirname(resolve(manifestPath));
  const clips: Clip[] = JSON.parse(readFileSync(resolve(manifestPath), 'utf8'));

  // one transcriber per locale (each pins its whisper language); reused across that locale's clips.
  const transcribers = new Map<Locale, TransformersTranscriber>();
  const getEngine = (locale: Locale, menu: MenuContext) => {
    let t = transcribers.get(locale);
    if (!t) { t = new TransformersTranscriber(locale, { dtype, device }); transcribers.set(locale, t); }
    return new WhisperProvider(t, locale, menu);
  };

  const rows: Array<Record<string, unknown>> = [];
  const perLocale = new Map<Locale, { withIntent: number; correct: number; dangerous: number; failquietOk: number; failquietTotal: number }>();
  const bump = (l: Locale) => perLocale.get(l) ?? perLocale.set(l, { withIntent: 0, correct: 0, dangerous: 0, failquietOk: 0, failquietTotal: 0 }).get(l)!;

  for (const clip of clips) {
    const menu = clip.menu ?? DEFAULT_MENU;
    const engine = getEngine(clip.locale, menu);
    const pcm = await loadWav(clip.wav, manifestDir);
    const t0 = process.hrtime.bigint();
    const { transcript, proposal } = await engine.once(pcm);
    const ms = Number(process.hrtime.bigint() - t0) / 1e6;
    const got = proposal?.kind ?? null;
    const stat = bump(clip.locale);

    // fail-quiet clip: expected no intent
    if (clip.expected_kind === null) {
      stat.failquietTotal += 1;
      const ok = got === null;
      if (ok) stat.failquietOk += 1;
      rows.push({ wav: clip.wav, locale: clip.locale, transcript, got, expected: null, verdict: ok ? 'ok' : 'FALSE-FIRE', ms: Math.round(ms) });
      continue;
    }

    stat.withIntent += 1;
    const correctKind = got === clip.expected_kind;
    const correctArgs = correctKind && argsMatch(clip.expected_args, proposal!.args);
    const correct = correctKind && correctArgs;
    if (correct) stat.correct += 1;

    // dangerous-misfire: resolved to a WRONG stateful intent, or a dietary-named category auto-apply
    let dangerous = false;
    if (got && got !== clip.expected_kind && classify(got) === 'STATEFUL') dangerous = true;
    if (got === 'SELECT_CATEGORY' && typeof proposal!.args.categoryName === 'string' && isDietaryCategory(proposal!.args.categoryName)) dangerous = true;
    if (dangerous) stat.dangerous += 1;

    const transcriptOk = !clip.transcript_contains || transcript.toLowerCase().includes(clip.transcript_contains.toLowerCase());
    rows.push({
      wav: clip.wav, locale: clip.locale, transcript, got, expected: clip.expected_kind,
      verdict: correct ? 'correct' : dangerous ? 'DANGEROUS' : 'miss',
      transcript_ok: transcriptOk, ms: Math.round(ms),
    });
  }

  // one-sided Wilson upper bound for the safety metric (dangerous-misfire), 95%.
  const wilsonUpper = (k: number, n: number): number => {
    if (n === 0) return 1;
    const z = 1.96, p = k / n, z2 = z * z;
    const centre = p + z2 / (2 * n);
    const margin = z * Math.sqrt((p * (1 - p) + z2 / (4 * n)) / n);
    return (centre + margin) / (1 + z2 / n);
  };

  const locales = [...perLocale.entries()].map(([locale, s]) => ({
    locale,
    n_with_intent: s.withIntent,
    ira: s.withIntent ? +(s.correct / s.withIntent).toFixed(4) : null,
    dangerous_misfires: s.dangerous,
    dangerous_upper95: s.withIntent ? +wilsonUpper(s.dangerous, s.withIntent).toFixed(4) : null,
    failquiet: `${s.failquietOk}/${s.failquietTotal}`,
  }));

  const report = {
    model: 'Xenova/whisper-base', dtype, device,
    generated_note: 'deterministic given pinned weights + greedy decode; NOT a launch gate unless corpus >=300/locale, >=15 speakers (resolution.md M1/C-4)',
    total_clips: clips.length,
    locales,
    clips: rows,
  };

  console.error('\n=== VOICE REAL-AUDIO EVAL ===');
  for (const l of locales) {
    console.error(`  ${l.locale}: IRA=${l.ira ?? '-'} (n=${l.n_with_intent})  dangerous=${l.dangerous_misfires} (upper95=${l.dangerous_upper95 ?? '-'})  fail-quiet=${l.failquiet}`);
  }
  for (const r of rows) console.error(`  [${r.verdict}] ${r.locale} "${r.transcript}" → ${r.got ?? 'null'} (exp ${r.expected ?? 'null'}) ${r.ms}ms`);
  if (outPath) { writeFileSync(resolve(outPath), JSON.stringify(report, null, 2)); console.error(`\nreport → ${outPath}`); }
  console.error('');
}

main().catch((e) => { console.error('FATAL', e?.stack || e?.message || e); process.exit(1); });
