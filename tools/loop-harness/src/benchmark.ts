// §2.3 — the fixed benchmark-replay speed check. Runs a deterministic scenario
// and returns ONE number (lower = better) the oracle compares before→after.
// Two metric modes:
//   - 'wall_ms': median wall-clock over `repeats` runs (real perf; some variance).
//   - { parse }: a number parsed from stdout (deterministic — e.g. a query-row
//     count, a token count, an op count). Prefer this for repeatable gates.

import { execFileSync } from 'node:child_process';

export interface BenchmarkSpec {
  cmd: string;
  args: string[];
  cwd: string;
  /** 'wall_ms' = median wall-clock; {parse} = capture group 1 of the regex from stdout. */
  metric: 'wall_ms' | { parse: RegExp };
  repeats?: number;
}

function median(xs: number[]): number {
  const s = [...xs].sort((a, b) => a - b);
  return s[Math.floor(s.length / 2)] ?? NaN;
}

export function runBenchmark(spec: BenchmarkSpec): number {
  const repeats = spec.repeats ?? (spec.metric === 'wall_ms' ? 5 : 1);
  const samples: number[] = [];
  for (let i = 0; i < repeats; i++) {
    const t0 = process.hrtime.bigint();
    let out = '';
    try { out = execFileSync(spec.cmd, spec.args, { cwd: spec.cwd, encoding: 'utf8', timeout: 120_000 }); }
    catch (e: any) { out = (e?.stdout ?? '').toString(); } // a benchmark that exits non-zero still timed
    if (spec.metric === 'wall_ms') {
      samples.push(Number(process.hrtime.bigint() - t0) / 1e6);
    } else {
      const m = out.match(spec.metric.parse);
      samples.push(m ? Number(m[1]) : NaN);
    }
  }
  return median(samples);
}
