# Reflection — test-hardening #2 (2026-06-27T13:47:09Z)

**WHAT:** Scanned 245 files via the attack workflow; 2023 findings, 18 systemic patterns, 10 class-level fixes
**OUTCOME:** natural_stop

**WHY (causal root):** The test suite was a paper gate — it existed as files on disk but was
never wired into any actual runner (whole suites were `.js` imports of unbuilt `.ts`, so they
never executed at all), and where suites DID run, a large class of assertions were
inspection-stubs (tautological / body.length-as-render-proof / permissive-status-array) that
pass by construction regardless of the code under test. Green was structurally guaranteed, not
earned — the suite measured "the file parses" and "a promise resolved", not behavior. This is
not a one-off typo; it is a systemic authoring pattern repeated 217× across the scan, which is
why it surfaces as class-level fixes rather than point fixes.
CONFIDENCE: medium (back-filled during curation from `docs/regressions/REGRESSION-LEDGER.md`
row #46 evidence + this reflection's own ISSUES section; the original author did not record a
live WHY at authoring time, so this is a librarian reconstruction, not a first-hand account).

**ISSUES:**
- 217 CRITICAL false-greens across the suite
- whole suites never execute (.js import of unbuilt .ts)

**PROPAGATE TO:**
- [guardrail] promote → guardrail (red→green): tautological assertions — recurring across this run — a lesson must become a gate so it stops recurring
- [guardrail] promote → guardrail (red→green): body.length-as-render-proof — recurring across this run — a lesson must become a gate so it stops recurring
- [guardrail] promote → guardrail (red→green): permissive status arrays — recurring across this run — a lesson must become a gate so it stops recurring
- [memory] write/update a memory note for "test-hardening" with this run's issues + learnings — so the next session does not re-discover them
- [guardrail] activate permissive-status ESLint rule — carry-forward guard from this run
- [guardrail] tautology lint rule — carry-forward guard from this run
- [doc] document/track watch-item: red-line money/RLS/PII vacuous proofs — carry-forward watch from this run

_Advisory: the worker/librarian enacts; do not auto-edit sibling surfaces._
