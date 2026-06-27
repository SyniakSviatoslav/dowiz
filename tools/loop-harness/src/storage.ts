// §7 — permanent, never-cleaned, lossless-only storage. Nothing here deletes,
// prunes, or overwrites: nextRunIndex assigns max+1 (never reuses), iteration
// traces and the metrics index are append-only, and run-records are gzipped at
// rest (lossless) with a gunzip round-trip. Total recall, light footprint.

import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import type { RunRecord, MetricsLine, IterationTelemetry } from './types.js';

const RUN_FILE_RE = /^(\d+)\.json(?:\.gz)?$/;

function ensureDir(dir: string): void {
  fs.mkdirSync(dir, { recursive: true });
}

function loopDir(baseDir: string, loop: string): string {
  return path.join(baseDir, loop);
}

/** Assign the next run index for a loop. Append-only: returns max(existing)+1,
 *  never reuses or overwrites a prior run. */
export function nextRunIndex(baseDir: string, loop: string): number {
  const dir = loopDir(baseDir, loop);
  if (!fs.existsSync(dir)) return 1;
  let max = 0;
  for (const name of fs.readdirSync(dir)) {
    const m = name.match(RUN_FILE_RE);
    if (m) max = Math.max(max, Number(m[1]));
  }
  return max + 1;
}

export function iterTracePath(baseDir: string, loop: string, runIndex: number): string {
  return path.join(loopDir(baseDir, loop), `${runIndex}.iters.jsonl`);
}

export function runRecordPath(baseDir: string, loop: string, runIndex: number): string {
  return path.join(loopDir(baseDir, loop), `${runIndex}.json.gz`);
}

/** Append one iteration's telemetry as a JSONL line. Append-only. */
export function appendIter(baseDir: string, loop: string, runIndex: number, iter: IterationTelemetry): void {
  ensureDir(loopDir(baseDir, loop));
  fs.appendFileSync(iterTracePath(baseDir, loop, runIndex), JSON.stringify(iter) + '\n');
}

/** Persist the canonical run-record, gzip-compressed (lossless). */
export function writeRunRecord(baseDir: string, loop: string, runIndex: number, record: RunRecord): void {
  ensureDir(loopDir(baseDir, loop));
  const gz = zlib.gzipSync(Buffer.from(JSON.stringify(record), 'utf8'));
  fs.writeFileSync(runRecordPath(baseDir, loop, runIndex), gz);
}

/** Read a run-record back (gunzip → parse). Round-trips writeRunRecord exactly. */
export function readRunRecord(baseDir: string, loop: string, runIndex: number): RunRecord {
  const raw = fs.readFileSync(runRecordPath(baseDir, loop, runIndex));
  return JSON.parse(zlib.gunzipSync(raw).toString('utf8')) as RunRecord;
}

/** Append the compact per-run line to the global trend/recall index. Append-only. */
export function appendMetricsLine(baseDir: string, line: MetricsLine): void {
  ensureDir(baseDir);
  fs.appendFileSync(path.join(baseDir, 'metrics.jsonl'), JSON.stringify(line) + '\n');
}

/** Read the metrics index, optionally filtered by loop (for §5 VS-HISTORY). */
export function readMetrics(baseDir: string, loop?: string): MetricsLine[] {
  const p = path.join(baseDir, 'metrics.jsonl');
  if (!fs.existsSync(p)) return [];
  const lines = fs.readFileSync(p, 'utf8').split('\n').filter(Boolean);
  const out = lines.map((l) => JSON.parse(l) as MetricsLine);
  return loop ? out.filter((m) => m.loop === loop) : out;
}
