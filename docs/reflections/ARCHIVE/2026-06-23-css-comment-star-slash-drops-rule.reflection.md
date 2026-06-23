---
CONTEXT:   Building the Paper/Moebius internal skin — a [data-skin="paper"] token block appended
           to packages/ui/src/theme/tokens.css, with a descriptive block comment above it.
DECISIONS: Wrote the comment prose with a token list "ADDS --ink-*/--paper-*/--font-display".
WHERE:     The skin appeared NOT to apply: /admin stayed dark, --brand-bg resolved to the
           #061b1a default. The whole [data-skin="paper"] token block was silently absent from
           the browser CSSOM — present in the file + the vite-served CSS + the minified dist,
           but dropped by the browser on parse. typecheck/build/lint/grep ALL passed.
WHY:       The comment prose contained the literal `*/` (inside "--ink-*/"), which CLOSES a CSS
           block comment. Everything after became stray CSS; the parser's error-recovery
           consumed the next rule (the entire token block) until it resynced at the rule after.
           Root cause: a CSS comment is terminated by the first `*/`, and authoring tools do not
           flag `*/` appearing in comment PROSE — the failure is invisible to every text/build
           gate and only manifests as a dropped rule at browser parse time.
CONFIDENCE: high
NEXT-TIME: Never put a literal `*/` (or `/*`) in CSS comment prose — rephrase token lists
           ("ink/paper/display tokens"). For any token/theme block, verify it APPLIES with a
           live getComputedStyle read, not a file grep — a rule can be in the file yet dropped
           by the browser. Guardrail added (ledger #13): e2e/tests/paper-skin-tokens.spec.ts
           asserts the paper scope resolves on a real browser (red on the bug, green now).
LINK:      packages/ui/src/theme/tokens.css ; e2e/tests/paper-skin-tokens.spec.ts ; commit a0a28c05
---
