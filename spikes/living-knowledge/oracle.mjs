// oracle.mjs — the hand-verified ground-truth query set, shared by eval.mjs and diagnostics.
// Queries are natural-language operator questions (paraphrases, NOT copied distinctive tokens) so
// lexical hashing gets no free ride. Single canonical answer per query, plus expected-MISS queries
// (falsifiability floor).
//
// RE-ALIGNED 2026-07-14: the corpus shrank from the 174-file era to the current harness tree.
// Entries whose target files no longer exist on disk were PRUNED (a stale oracle referencing dead
// files is a lie, not a proof). Every `want` below is verified present by `eval.mjs` at load time —
// if any disappears again, the ORACLE ERROR guard fails loudly. Ground-truth over stale plan.
export const ORACLE = [
  // ── core-rules ──
  { q: 'before writing any code enumerate a checkable exit list then verify each item passed afterward', want: ['docs/operating-model/task-exit-rule.md'] },
  // ── infra: hooks ──
  { q: 'which bash hook blocks mutations of protected paths like migrations and env', want: ['.claude/hooks/guard-bash.sh'] },
  { q: 'the gate that refuses edits to contract schema and governance zones', want: ['.claude/hooks/protect-paths.sh'] },
  { q: 'hook that forces a task to be classified before any work begins', want: ['.claude/hooks/require-classification.sh'] },
  { q: 'hook escalating edits that touch auth money or row level security to a human', want: ['.claude/hooks/red-line-doubt-gate.sh'] },
  // ── self-evolution: loops ──
  { q: 'loop that recovers from a production incident with reversing actions first', want: ['loops/incident-recovery.yaml'] },
  { q: 'loop that bisects git history down to the single culprit commit', want: ['loops/regression-hunt.yaml'] },
  { q: 'loop that measures a baseline against a budget and adds a performance gate', want: ['loops/performance.yaml'] },
  { q: 'loop that implements exactly one roadmap stage up to its gate checkpoint', want: ['loops/build-stage.yaml'] },
  // ── expected MISS (falsifiability floor — nothing in the harness corpus answers these) ──
  { q: 'kubernetes helm chart ingress controller autoscaling', want: [], miss: true },
  { q: 'react usestate css keyframes flexbox animation component', want: [], miss: true },
  { q: 'graphql federation apollo gateway schema stitching resolvers', want: [], miss: true },
];
