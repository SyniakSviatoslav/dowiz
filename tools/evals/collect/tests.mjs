// SUITE ci · TESTS: test counts per module (a deleted test is a lower number, and a lower number
// breaches), and the mutation score cargo-mutants left behind (§B.2 Correctness).
//
// Counts are `#[test]` attributes by grep, not `cargo test -- --list`: listing compiles every
// crate, and on this box a compile goes through the one slot. The grep counts what the source
// declares; a `#[cfg]`-disabled test is counted too, and the note says so.
import fs from 'node:fs';
import path from 'node:path';
import { walk } from './graph.mjs';
import { ind, unverified } from '../rules.mjs';

export const CRATES = {
  kernel: 'kernel',
  dowiz_core: 'crates/dowiz-core',
  dowiz_hub: 'crates/dowiz-hub',
  bebop_store: 'crates/bebop-store',
  worker: 'workers/api/src',
};
export const MUTANTS = 'kernel/mutants.out/outcomes.json';
export const SKIP = /^(node_modules|target|mutants\.out.*|\..+)$/; // and every dot-directory (.git, .lanes, .claude)

export function files(dir, re) {
  if (!fs.existsSync(dir)) return [];
  return walk(dir, SKIP).map(f => f.path).filter(p => re.test(p));
}

export function countMatches(paths, re) {
  let n = 0;
  for (const p of paths) n += (fs.readFileSync(p, 'utf8').match(re) || []).length;
  return n;
}

/** caught / (caught + missed), per mille; timeouts and unviable mutants are neither. */
export function mutationScore(o) {
  const done = (o.caught || 0) + (o.missed || 0);
  return done ? Math.round((1000 * o.caught) / done) : null;
}

export async function collect(ctx) {
  const out = [];
  for (const [name, rel] of Object.entries(CRATES)) {
    const n = countMatches(files(path.join(ctx.root, rel), /\.rs$/), /#\[test\]/g);
    out.push(ind(`tests.rust.${name}`, n, 'tests', 'floor', `grep -c '#[test]' ${rel}/**/*.rs`));
  }
  const js = files(ctx.root, /\.test\.mjs$/).filter(p => !p.includes('/spikes/'));
  out.push(ind('tests.js.files', js.length, 'files', 'floor', 'find -name *.test.mjs'));
  out.push(ind('tests.js.cases', countMatches(js, /^\s*(?:test|it)\(/gm), 'tests', 'floor', 'test( / it( calls in *.test.mjs'));
  const e2e = files(path.join(ctx.root, 'e2e/tests'), /\.(mjs|js|ts)$/);
  out.push(ind('tests.e2e.files', e2e.length, 'files', 'floor', 'e2e/tests'));
  const mp = path.join(ctx.root, MUTANTS);
  if (!fs.existsSync(mp)) {
    out.push(unverified('mutants.kernel_permille', 'permille', 'floor', MUTANTS,
      'no cargo-mutants output on this checkout: mutations.yml runs weekly and keeps no artifact'));
    return out;
  }
  const o = JSON.parse(fs.readFileSync(mp, 'utf8'));
  const files_ = [...new Set((o.outcomes || []).map(x => x.scenario?.Mutant?.file).filter(Boolean))];
  out.push(ind('mutants.kernel_permille', mutationScore(o), 'permille', 'floor', MUTANTS, {
    note: `${o.caught} caught, ${o.missed} missed of ${o.total_mutants}; run ${String(o.start_time).slice(0, 10)} over ${files_.join(', ')}`,
  }));
  out.push(ind('mutants.kernel_missed', o.missed || 0, 'mutants', 'ratchet', MUTANTS));
  return out;
}
