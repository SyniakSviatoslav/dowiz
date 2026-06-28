import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

// deliver v2 — the council's C1/Q5 DURABLE artifact (counsel binding condition, materialized NOW as a failing
// pending-guardrail, NOT prose). The ethical justification of the cash-as-proof bond leans entirely on Stage-21
// reconciliation NEVER auto-deducting a no-fault shortfall from a (often minimum-wage) courier and NEVER
// deriving a courier score/penalty from a crumb. This test is RED until the Stage-21 author records that
// invariant in docs/adr/ADR-stage21-reconciliation.md with BOTH markers — so it cannot be silently forgotten.
// R-8 (no-auto-deduct) + R-9 (embedded-staff) collapse into this ONE invariant + the anti-scoring-creep guard.
const here = dirname(fileURLToPath(import.meta.url));
const adrPath = resolve(here, '../../../docs/adr/ADR-stage21-reconciliation.md');

// `todo` = the assertion runs and is REPORTED in every test run (visible — cannot be forgotten), but a todo
// failure does NOT fail CI. RED today (the ADR is unwritten); when Stage-21 authors ADR-stage21-reconciliation.md
// with both markers this passes — at which point the author removes the `todo` flag to make it a hard gate.
test('Stage-21 reconciliation ADR records NO-AUTO-DEDUCT + NO-COURIER-SCORING (deliver v2 carried invariant)', { todo: 'RED until Stage-21 authors ADR-stage21-reconciliation.md' }, () => {
  assert.ok(
    existsSync(adrPath),
    `MISSING ${adrPath} — Stage-21 reconciliation must record the no-auto-deduct invariant BEFORE any deduction/` +
      `scoring logic ships. This guardrail is intentionally RED until that ADR exists (deliver v2 C1/Q5).`,
  );
  const body = readFileSync(adrPath, 'utf8');
  assert.match(body, /NO-AUTO-DEDUCT/, 'ADR must carry the NO-AUTO-DEDUCT marker (no auto shortfall deduction from a courier without human friction-review)');
  assert.match(body, /NO-COURIER-SCORING/, 'ADR must carry the NO-COURIER-SCORING marker (no crumb-derived courier score/penalty layer without its own Triadic Council)');
});
