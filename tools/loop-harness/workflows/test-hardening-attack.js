export const meta = {
  name: 'test-hardening-attack',
  description: 'Adversarially attack test specs with 3 decorrelated agents → ranked blind-spot ledger',
  whenToUse: 'Find false-greens / weak assertions / coverage gaps in existing test specs before hardening them',
  phases: [
    { title: 'Attack', detail: '3 agents (critique·security·QA) per spec, in parallel' },
    { title: 'Synthesize', detail: 'dedupe + rank per spec into a fix-now / defer ledger' },
  ],
};

// Default high-value targets: integration / e2e / lifecycle / auth-isolation specs where false-greens
// hide. Pure-unit tests (order-pricing, order-canonical, …) are deterministic + tightly asserted —
// not worth a 3-agent attack. Override via args: { specs: ["e2e/tests/foo.spec.ts", ...] }.
const DEFAULT_SPECS = [
  'e2e/tests/cross-tenant-realtime-qa.spec.ts',
  'e2e/tests/flow-order-lifecycle-trace.spec.ts',
  'e2e/tests/flow-order-creation.spec.ts',
  'e2e/tests/ws-courier-assignment.spec.ts',
  'e2e/tests/client/order-stepper.spec.ts',
  'e2e/tests/api-real.spec.ts',
];

const specs = (args && Array.isArray(args.specs) && args.specs.length) ? args.specs : DEFAULT_SPECS;

const FINDINGS = {
  type: 'object',
  additionalProperties: false,
  required: ['findings'],
  properties: {
    findings: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['severity', 'blindspot', 'location', 'slips_through', 'stronger_assertion'],
        properties: {
          severity: { type: 'string', enum: ['CRITICAL', 'HIGH', 'MED', 'LOW'] },
          blindspot: { type: 'string', description: 'one-line: what the spec misses / asserts weakly' },
          location: { type: 'string', description: 'file:line or dimension' },
          slips_through: { type: 'string', description: 'the real bug that would stay green' },
          stronger_assertion: { type: 'string', description: 'the concrete stronger assertion/scenario to add' },
        },
      },
    },
  },
};

const LEDGER = {
  type: 'object',
  additionalProperties: false,
  required: ['spec', 'fix_now', 'defer', 'top3'],
  properties: {
    spec: { type: 'string' },
    fix_now: { type: 'array', items: { type: 'string' }, description: 'dedup ranked fixable findings (severity · what · fix)' },
    defer: { type: 'array', items: { type: 'string' }, description: 'infeasible / covered-elsewhere / product-gap-to-escalate, each with the reason' },
    top3: { type: 'array', items: { type: 'string' }, description: 'highest-ROI hardenings to implement first' },
  },
};

const lens = (spec) => ({
  critique: `Adversarially critique the test spec at \`${spec}\` (read it fully). PROVE it gives FALSE CONFIDENCE: find assertions that pass even when the feature is broken — too-loose matchers, snapshot-not-delta, wrong-thing-measured, swallowed errors, happy-path-only, serial/timing flakiness. For each: severity, the blind spot, file:line, the exact bug that stays green, and the concrete stronger assertion.`,
  security: `Security-review the test spec at \`${spec}\` (read it fully) AND the boundary it claims to test. Find: weak authz/isolation assertions that pass while the boundary is broken (empty fixtures, permissive status arrays masking leaks, missing positive controls), untested IDOR/cross-tenant/privilege-escalation vectors, secrets/PII in the spec, and risky side effects on the real environment. Cross-ref the real route auth in apps/api/src/routes where useful. For each: severity, blind spot, file:line, the risk that ships, the fix.`,
  qa: `QA coverage-gap review of the test spec at \`${spec}\` (read it fully). Find: error-matrix gaps (401/403/404/409/422/429/5xx/network), unexercised loading/empty/error/terminal states, real-time rigor (does it assert the USER-FACING live update, not just a buffer?), determinism (fixed sleeps, self-pollution), assertion strength, and OVERLAP with existing specs in e2e/tests (don't propose duplicates — note what's already covered elsewhere). For each: priority, the gap, why it matters, the concrete test/assertion to add.`,
});

const results = await pipeline(
  specs,
  (spec) => parallel([
    () => agent(lens(spec).critique, { agentType: 'system-breaker', label: `critique:${spec}`, phase: 'Attack', schema: FINDINGS }),
    () => agent(lens(spec).security, { agentType: 'security-sentinel', label: `security:${spec}`, phase: 'Attack', schema: FINDINGS }),
    () => agent(lens(spec).qa, { agentType: 'test-scout', label: `qa:${spec}`, phase: 'Attack', schema: FINDINGS }),
  ]).then((rs) => ({ spec, findings: rs.filter(Boolean).flatMap((r) => r.findings ?? []) })),
  (collected, spec) => agent(
    `Synthesize this adversarial sweep of \`${spec}\` into a triaged ledger. Dedupe overlapping findings across the 3 agents, rank by severity, and split into FIX-NOW (false-greens / weak assertions / missing scenarios that strengthen the test) vs DEFER (infeasible in the staging env, OR already covered by another e2e spec, OR a real PRODUCT gap to escalate — give the reason for each). Then the top 3 highest-ROI hardenings. Raw findings:\n\n${JSON.stringify(collected.findings, null, 2)}`,
    { label: `synth:${spec}`, phase: 'Synthesize', schema: LEDGER },
  ),
);

return results.filter(Boolean);
