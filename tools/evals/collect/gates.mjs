// SUITE ci · GATES: every gate and its proof, run once through tools/gates/run-all.sh (no
// compiler: the gates job in ci.yml runs the --cargo half). A row is its exit code; the green
// count, the failures and the skips become indicators (§B.2 Correctness, "Gates green count").
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { ind } from '../rules.mjs';

/** run-all.sh's table: `ok|FAIL|skip <name> rc=<n> <last line>`. */
export function parseTable(text) {
  const rows = [];
  for (const line of String(text).split('\n')) {
    const m = line.match(/^(ok|FAIL|skip)\s+(\S+)(?:\s+rc=(\d+))?/);
    if (m) rows.push({ mark: m[1], name: m[2], rc: m[3] === undefined ? null : Number(m[3]) });
  }
  return rows;
}

export function run(root, cmd, args, timeout = 900_000) {
  const r = spawnSync(cmd, args, { cwd: root, encoding: 'utf8', timeout, maxBuffer: 64 << 20 });
  return { rc: r.status, out: (r.stdout || '') + (r.stderr || ''), err: r.error ? r.error.message : '' };
}

export async function collect(ctx) {
  const exec = ctx.exec || run;
  const r = exec(ctx.root, 'sh', ['tools/gates/run-all.sh']);
  const rows = parseTable(r.out);
  const where = 'sh tools/gates/run-all.sh (proofs first, then every gate)';
  if (!rows.length) {
    return [ind('gates.table_rows', 0, 'rows', 'min', where, { limit: 1, note: `rc=${r.rc} ${r.err} ${r.out.slice(-200)}` })];
  }
  const ran = rows.filter(x => x.mark !== 'skip');
  const failed = ran.filter(x => x.mark === 'FAIL');
  const baselines = fs.readdirSync(path.join(ctx.root, 'tools/gates')).filter(f => f.endsWith('.baseline')).length;
  return [
    ind('gates.ran', ran.length, 'gates', 'floor', where, { note: 'a gate deleted is a lower number: a dated note' }),
    ind('gates.green', ran.length - failed.length, 'gates', 'floor', where),
    ind('gates.failed', failed.length, 'gates', 'zero', where, failed.length ? { note: failed.map(x => `${x.name} rc=${x.rc}`).join(', ') } : {}),
    ind('gates.skipped', rows.length - ran.length, 'gates', 'trend', where, { note: rows.filter(x => x.mark === 'skip').map(x => x.name).join(', ') }),
    ind('gates.baselines', baselines, 'files', 'floor', 'ls tools/gates/*.baseline'),
  ];
}
