# Council retro — 2026-06-23 (paper-skin + RSI driver session)

Big-change retro (≫3 files, many iterations, stage closes, touched a red-line auth file).
Inputs: the two archived reflections (CSS-comment-`*/`-drop; agent-driver grounding+settle).
Critics: cause-critic · pattern-critic · ratchet-critic. Executor: worker (librarian toolset is
read-only here). Each line below → artifact OR explicit no-op.

## Per-reflection verdicts (cause-critic)
- **CSS-comment-`*/`-drops-rule → CONFIRMED (very high).** Deterministic CSS-parser behaviour;
  guardrail #13 is sharp (red arm proves the bite). No-op beyond what shipped.
- **agent-driver grounding+settle → DOWNGRADED to MEDIUM.** Causes correct (hallucinated
  selectors from thin observation; observe-before-hydrate), both fixed in code, but the fix was
  advisory (system-prompt CRITICAL line + settle), not locked by a gate.

## Enacted ratchet artifacts
1. **CSS-comment drop → cheap static arm (ENACTED).** ESLint is JS-AST-only here (won't parse
   `.css`), so the static gate is a unit test: `packages/ui/src/theme/css-comment-integrity.test.ts`
   — strip canonical comments, flag any leftover `/*`/`*/` marker (early-closed comment). Red/green
   arms; green on the current repo. Complements the live-DOM E2E #13 with fast, browser-free
   feedback. Ledger #13 updated.
2. **agent-driver grounding+settle → lesson + CLAUDE.md-pointer PROPOSAL (no product gate).**
   ratchet-critic: it's test-harness design, not a product regression, and "the LLM doesn't
   hallucinate" isn't mechanizable as a static rule. The fixes live in `e2e/driver/agent-driver.ts`
   (grounded observe + settle) and `reasoners.ts` (selector-from-list-only). **Proposed CLAUDE.md
   addition** (not enacted — `.claude/` is protect-paths-blocked; needs human): a "Test & Harness
   Discipline" pointer — *agentic browser drivers must (a) feed grounded real selectors and forbid
   invented ones; (b) settle the SPA (networkidle + first actionable) before observing.*

## Systemic roots (pattern-critic) — candidate ratchet items (NOT yet enacted; need design)
- **R1 · Defects invisible to text-level gates** (grep/typecheck/build/lint green, behaviour
  wrong): CSS-comment drop #13, contrast #6, read_public_menu stale-copy, localStorage #12, the
  driver findings. Point-guardrails exist per symptom; the **gap** is a generalized behavioural/
  live-DOM verification before merge/deploy. Candidate: a small pre-merge behavioural smoke
  asserting computed outcomes (theme applies + WCAG, status monotonic, token fields present).
- **R2 · Stale/fragmented single-source-of-truth**: read_public_menu (multi-`CREATE OR REPLACE`),
  inline-vs-plugin local-login, dev-login shared path #1. Lessons are advisory; the **gap** is a
  deterministic barrier (e.g. CI check that a redefined plpgsql function matches the live
  signature; a duplicate-route-handler detector). Candidate ratchet, larger scope.

These two systemic candidates are recorded for a future ratchet pass — not built this session
(they are design-level, not point fixes). Monotonic: nothing weakened; #13 strengthened.
