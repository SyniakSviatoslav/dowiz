// oracle.mjs — the hand-verified ground-truth query set, shared by eval.mjs and diagnostics.
// Queries are natural-language operator questions (paraphrases, NOT copied distinctive tokens) so
// lexical hashing gets no free ride. Single canonical answer per query (success@K over 77 candidates;
// random ≈ K/77 ≈ 6.5%), one 2-answer case, plus expected-MISS queries (falsifiability floor).
export const ORACLE = [
  // ── core-rules ──
  { q: 'a task counts as validated only when the proof is falsifiable and would flip red on a bad input', want: ['docs/operating-model/verified-by-math.md'] },
  { q: 'before writing any code enumerate a checkable exit list then verify each item passed afterward', want: ['docs/operating-model/task-exit-rule.md'] },
  { q: 'how to keep the harness behaving the same no matter which language model is driving it', want: ['docs/operating-model/model-agnostic-playbook.md'] },
  { q: 'the long-form reference document the trimmed core operating instructions defer to', want: ['docs/operating-model/claude-md-reference.md'] },
  // ── infra: hooks ──
  { q: 'which bash hook blocks mutations of protected paths like migrations and env', want: ['.claude/hooks/guard-bash.sh'] },
  { q: 'the gate that refuses edits to contract schema and governance zones', want: ['.claude/hooks/protect-paths.sh'] },
  { q: 'hook that denies handing a subtask off to the fable model', want: ['.claude/hooks/agent-dispatch-gate.sh'] },
  { q: 'detector for a subagent that returns zero tool calls and just parrots the injected reminder', want: ['.claude/hooks/subagent-return-guard.sh'] },
  { q: 'hook that warns when the working context window is growing too large', want: ['.claude/hooks/context-budget-guard.sh'] },
  { q: 'hook that forces a task to be classified before any work begins', want: ['.claude/hooks/require-classification.sh'] },
  { q: 'hook escalating edits that touch auth money or row level security to a human', want: ['.claude/hooks/red-line-doubt-gate.sh'] },
  // ── infra: guardrails / runners ──
  { q: 'the suite that runs every governance guardrail before a commit', want: ['scripts/run-armaments.sh'] },
  { q: 'knowledge as circuits red-line pattern registry runner', want: ['scripts/run-circuits.mjs', 'docs/operating-model/circuits/registry.json'] },
  { q: 'gate that fails when a guardrail file exists but no runner ever invokes it', want: ['scripts/guardrail-no-orphan-guardrails.mjs'] },
  { q: 'checker that a loop marked certified actually has its report artifact', want: ['scripts/guardrail-loop-registry-parity.mjs'] },
  { q: 'guardrail that checks source files carry the open-source license header', want: ['scripts/guardrail-license.mjs'] },
  { q: 'guardrail enforcing the token budget routing gates', want: ['scripts/guardrail-token-gates.mjs'] },
  { q: 'guardrail that proves every enforced proof has a red case that can fail', want: ['scripts/guardrail-falsifiable-proof.mjs'] },
  { q: 'script that verifies the structural integrity of the codebase modules', want: ['scripts/module-integrity.mjs'] },
  // ── self-evolution ──
  { q: 'spreading activation bands over a helixdb-backed living knowledge store', want: ['docs/operating-model/living-knowledge-helixdb-arc.md'] },
  { q: 'the design of the self-evolving loop system version three', want: ['docs/operating-model/living-loop-system-v3.md'] },
  { q: 'the one-shot fable audit backlog with the root cause and fifteen findings', want: ['docs/operating-model/fable-audit-findings-2026-07-07.md'] },
  { q: 'loop that recovers from a production incident with reversing actions first', want: ['loops/incident-recovery.yaml'] },
  { q: 'loop that bisects git history down to the single culprit commit', want: ['loops/regression-hunt.yaml'] },
  { q: 'loop that measures a baseline against a budget and adds a performance gate', want: ['loops/performance.yaml'] },
  { q: 'loop that implements exactly one roadmap stage up to its gate checkpoint', want: ['loops/build-stage.yaml'] },
  // ── living-memory ──
  { q: 'reflection on removing proxy reasoning agents while keeping deterministic gates', want: ['docs/reflections/INBOX/2026-07-07-ground-truth-over-proxy.reflection.md'] },
  { q: 'reflection on compiling expensive model reasoning into cheap doer checklists', want: ['docs/reflections/INBOX/2026-07-06-metacognition-transfer-0b2.reflection.md'] },
  { q: 'reflection on the decide function composing the state machine actor gate and pricing', want: ['docs/reflections/INBOX/2026-07-07-0b3-decide-composition.reflection.md'] },
  // ── expected MISS (falsifiability floor — nothing in the harness corpus answers these) ──
  { q: 'kubernetes helm chart ingress controller autoscaling', want: [], miss: true },
  { q: 'react usestate css keyframes flexbox animation component', want: [], miss: true },
  { q: 'graphql federation apollo gateway schema stitching resolvers', want: [], miss: true },
];
