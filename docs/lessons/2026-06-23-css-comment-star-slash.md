---
TRIGGER: packages/ui/src/theme/**.css
CAUSE: >
  A CSS block comment is terminated by the FIRST `*/`. If `*/` (or `/*`) appears
  in comment PROSE — e.g. a token list like "--ink-*/--paper-*" — the comment
  closes early; the trailing prose becomes stray CSS, and the browser's
  error-recovery consumes the NEXT rule (silently dropping it) until it resyncs.
  The dropped rule is still present in the file, the vite-served CSS, and the
  minified dist — so grep / typecheck / build / lint ALL pass. The defect only
  manifests at browser parse time as a missing rule.
ACTION: >
  When editing CSS comments in packages/ui/src/theme/**.css (esp. tokens.css)
  → cause: a literal `*/` in comment prose closes the comment early and drops the
  next rule → do: never write `*/` or `/*` inside comment prose (rephrase token
  lists, e.g. "ink/paper/display tokens"); and verify any token/theme block
  APPLIES via a live getComputedStyle read, not a file grep — a rule can be in
  the file yet dropped by the browser. Guardrail: e2e/tests/paper-skin-tokens.spec.ts.
LINK: e2e/tests/paper-skin-tokens.spec.ts ; packages/ui/src/theme/tokens.css ; ledger #13
SCOPE: CSS comment authoring (the `*/`-in-prose drop class). Not JS/TS comments.
STATUS: active
---

# CSS comments: a literal `*/` in prose silently drops the next rule

Source: reflection `2026-06-23-css-comment-star-slash-drops-rule.reflection.md`.

While adding the `[data-skin="paper"]` token block to `tokens.css`, the comment
above it read `ADDS --ink-*/--paper-*/--font-display`. The `*/` inside `--ink-*/`
**closed the comment early**. The browser then tried to parse the leftover prose
as CSS, failed, and its error-recovery **swallowed the entire `[data-skin="paper"]`
token block** before resyncing at the next rule (`:is(h1,h2,h3)`). Result: the
paper skin didn't apply (`--brand-bg` stayed the dark default), even though the
rule was visibly present in the file and the served/built CSS.

Every text-level gate missed it (grep found the rule; typecheck/build/lint were
green). The bug only showed up under a real browser via `getComputedStyle`.

Rules:
1. Never put `*/` or `/*` in CSS comment prose. Rephrase (avoid `-*/`, `/*`).
2. Prove a token/theme block by its EFFECT (getComputedStyle in a real browser),
   not by its presence in the file. `e2e/tests/paper-skin-tokens.spec.ts` does
   this red→green for the paper scope.
