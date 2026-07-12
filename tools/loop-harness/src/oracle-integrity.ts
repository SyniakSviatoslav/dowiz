// §3 Oracle-Integrity meta-check — protects the ground truth. The whole stack
// trusts green tests + the fresh-context reviewer + the benchmark. BE-polish /
// autoupgrade loops can EDIT test files — a loop that "fixes" a test by weakening
// it CORRUPTS the oracle (fake-green at the infrastructure level). This runs on any
// change touching tests/reviewer/benchmark, BEFORE it's accepted. A trip → block +
// route to the review queue (ground-truth changes are inherently Class B).
//
// Independent by design: it asks "did this change corrupt the thing we use to DECIDE
// correctness?" — a different question from "is this change correct?" (the per-loop
// reviewer). It must not be checkable by the loops it polices.

// Strip comments + string/template literals so counts reflect REAL code only.
// Without this, `// test('x',…)` or `const s = "test('x',…)"` inflate the count —
// letting a loop wrap real tests in a string to keep the regex-count steady while
// the live test count silently drops (the exact evasion the oracle must catch).
function stripNonCode(src: string): string {
  let out = '';
  for (let i = 0, n = src.length; i < n; ) {
    const c = src[i], d = src[i + 1];
    if (c === '/' && d === '/') { while (i < n && src[i] !== '\n') i++; continue; }
    if (c === '/' && d === '*') { i += 2; while (i < n && !(src[i] === '*' && src[i + 1] === '/')) i++; i += 2; continue; }
    if (c === '"' || c === "'" || c === '`') {
      const q = c; i++;
      while (i < n && src[i] !== q) { if (src[i] === '\\') i++; i++; }
      i++; continue;
    }
    out += c; i++;
  }
  return out;
}
export function countTests(src: string): number {
  return (stripNonCode(src).match(/(?:^|\b)(?:test|it|Deno\.test)\s*\(/g) ?? []).length;
}
export function countAssertions(src: string): number {
  // matches expect(...), assert(...), and method forms like assert.equal(...)
  return (stripNonCode(src).match(/\bexpect\s*\(|\bassert(?:\.\w+)?\s*\(/g) ?? []).length;
}
// no-fake-green markers (enforced at the meta-level against the loops' OWN test edits).
const WEAKENERS: { re: RegExp; label: string }[] = [
  { re: /\.(?:skip|only|fixme|todo)\s*\(/g, label: 'skip/only/fixme/todo' },
  { re: /expect\s*\(\s*true\s*\)/g, label: 'expect(true)' },
  { re: /assert\s*\(\s*true\s*\)/g, label: 'assert(true)' },
  { re: /(?:timeout|timeoutMillis)\s*[:=]\s*\d{5,}/g, label: 'inflated timeout' },
  { re: /^\s*\/\/\s*(?:assert|expect)\b/gim, label: 'commented-out assertion' },
];
export function countWeakeners(src: string): number {
  return WEAKENERS.reduce((n, w) => n + (src.match(w.re)?.length ?? 0), 0);
}
export function weakenersAdded(before: string, after: string): string[] {
  return WEAKENERS.filter((w) => (after.match(w.re)?.length ?? 0) > (before.match(w.re)?.length ?? 0)).map((w) => w.label);
}

export interface OracleFile { path: string; before: string; after: string }
export interface IntegrityInput {
  files: OracleFile[];
  /** Path fragments of the immutable benchmark scenario(s). */
  benchmarkPaths?: string[];
  /** Did the fresh-context reviewer provably run in a clean, separate context? */
  reviewerFresh?: boolean;
}
export interface IntegrityResult { ok: boolean; trips: string[] }

const isTestFile = (p: string) => /\.(?:test|spec)\.[tj]sx?$/.test(p) || /(?:^|\/)tests?\//.test(p);
const isReviewerFile = (p: string) => /reviewer/i.test(p);

/** Block (→ review queue) if a change silently weakens the oracle. */
export function checkOracleIntegrity(input: IntegrityInput): IntegrityResult {
  const trips: string[] = [];
  for (const f of input.files) {
    const benchHit = input.benchmarkPaths?.some((b) => f.path.includes(b));
    if (benchHit && f.before !== f.after) {
      trips.push(`benchmark scenario MUTATED (immutable to loops): ${f.path} — editing it would let a loop fake "proven speedup"`);
    }
    if (isTestFile(f.path)) {
      const tb = countTests(f.before), ta = countTests(f.after);
      if (ta < tb) trips.push(`test count dropped ${tb}→${ta} in ${f.path} (removed test must be justified, never silently)`);
      const ab = countAssertions(f.before), aa = countAssertions(f.after);
      if (aa < ab) trips.push(`assertion count dropped ${ab}→${aa} in ${f.path}`);
      const added = weakenersAdded(f.before, f.after);
      if (added.length) trips.push(`assertion WEAKENED in ${f.path}: ${added.join(', ')} (no-fake-green)`);
    }
  }
  if (input.reviewerFresh === false && input.files.some((f) => isReviewerFile(f.path))) {
    trips.push('reviewer changed but fresh-context not verifiable (it must run in clean, separate context — not the author grading itself)');
  }
  return { ok: trips.length === 0, trips };
}
