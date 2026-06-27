# DeliveryOS / dowiz — Test-Hardening Loop (v1)

**Date:** 2026-06-27 · A reusable loop that takes an existing test/spec and **adversarially
hardens it** — three independent agents attack its blind spots, the lead triages + strengthens the
assertions, then re-verifies. Born from the cross-tenant realtime QA loop sweep, where v1 ran 6/6
green yet its *headline* assertions (real-time, cross-tenant) proved nothing.

## 0. Why (the problem it solves)

A green test is not a correct test. A test can pass while the feature is broken — the assertion is
too loose (`refs() > 0`), measures the wrong thing (a snapshot, not a delta), tests an empty fixture
(a fake tenant can't leak), or swallows errors (a dead socket reads as "no leak"). This loop finds
those **false-greens** and closes them — the inverse of a normal test run.

## 1. The decision the loop drives

Metric = **count of open HIGH/CRITICAL blind-spots** for the target spec, driven to 0.
- Deterministic: each round's findings come from agents with a fixed rubric + a re-run that must stay
  green; "closed" means the stronger assertion exists AND exercises the path.
- Terminal: two consecutive rounds surface no new HIGH/CRITICAL blind-spot (loop-until-dry), or budget.

## 2. One iteration (5 hooks)

1. **ATTACK** — fan out 3 decorrelated agents on the spec, each with a distinct lens:
   - `system-breaker` (critique): prove FALSE CONFIDENCE — what passes while the feature is broken.
   - `security-sentinel` (security): authz/isolation rigor, secrets, IDOR, side effects, PII.
   - `test-scout` (QA): error-matrix/state/coverage gaps, flakiness, assertion strength, overlap with
     existing specs (don't duplicate).
2. **TRIAGE** — dedupe + rank. Classify each finding: **fix-now** (false-green / weak assertion /
   missing scenario) vs **defer** (infeasible in the env / covered by another spec / product gap to
   escalate). Honest deferral is allowed; silent skipping is not.
3. **HARDEN** — implement the fix-now findings: strengthen assertions, add scenarios, add positive
   controls, replace fixed sleeps with deterministic polls.
4. **VERIFY** — re-run the spec; it must stay green AND the new assertions must actually exercise the
   path (a new assertion that can't fail is itself a finding).
5. **GATE (no-weakening)** — every edit passes `checkOracleIntegrity` (tools/loop-harness): no
   test/assertion-count drop, no `skip`/`only`/`expect(true)`/inflated-timeout/commented assertion
   introduced. **A test-refinement loop must STRENGTHEN, never weaken** — this is the load-bearing
   guardrail (a loop that "fixes" a test by gutting it corrupts the oracle).

## 3. Scope + cost discipline (anti-gold-plating)

- **High value on INTEGRATION / e2e / lifecycle / realtime / auth-isolation specs** — where false-greens
  hide. **Low value on tight pure-unit tests** (e.g. `order-pricing`, `order-canonical`) that are already
  deterministic with exact assertions — running 3 agents on those burns tokens for little. The loop
  targets the former by default.
- The ATTACK phase is the parallelizable, token-heavy part → run it as a Workflow fan-out. HARDEN +
  VERIFY is lead-driven + sequential (file edits + a staging re-run that is itself rate-limited).
- Always **log what was NOT covered** (deferred findings) — silent truncation reads as "hardened" when
  it isn't.

## 4. Boundaries (inherited)

- Never weakens a gate (§2.5). Never edits product code to make a test pass — a test that fails because
  the PRODUCT is wrong is a **finding to escalate**, not a thing to paper over.
- Routes only to hardening; security-class product changes it surfaces are propose-only (review queue).

## 5. Run

- ATTACK fan-out (Workflow): `tools/loop-harness/workflows/test-hardening-attack.js` → per-spec ranked
  blind-spot ledger.
- Then the lead triages → hardens → re-verifies each spec, gated by `checkOracleIntegrity`.
- Findings ledger: `docs/design-review/test-hardening-findings.md`.

## Implementation status

- **2026-06-27** — loop defined; the cross-tenant realtime QA spec was the v0 proof (v1→v2 hardening:
  real-time vacuous→per-transition-delta on owner+customer rooms; fake-tenant→real cross-order WS
  denial; orphan-order/idempotency fix; positive control; random courier password; error-swallow guard;
  see `docs/design-review/qa-loop-agent-sweep.md`). Workflow fan-out + registry entry next.
