TASK: i18n single-source-of-truth + tiers 0/1/2 (parity gate, add-helper, dev warning)

ENRICHED-DONE (beyond "add tooling"):
- Translations have ONE authoritative home: a key-major catalog `{ 'key': { en, sq, uk } }` where a
  key and all its locales live together. `messages` (locale-major) is DERIVED from it — not a second
  source. `t()` / `translate()` signatures and behavior unchanged → zero call-site edits.
- The migration is provably lossless: derived `messages` deep-equals the pre-migration object (same
  1079 keys, same exact values per locale). This is the red→green proof.
- A missing translation can no longer ship silently: a parity gate fails CI when any catalog entry
  lacks en/sq/uk or carries a TODO draft.
- Adding a string is one action, not three manual edits in a 3300-line monolith.

EXIT CHECKLIST (written before code):
[PASS] Catalog generated from CURRENT messages, values exact — equivalence "MATCH: 3237 entries across 3 locales, 0 diffs" (also re-verified post-prettier).
[PASS] messages derived from catalog (fromCatalog pivot); t()/translate() unchanged — pnpm typecheck exit 0 across all 9 workspaces.
[PASS] Runtime parity: every entry has non-empty en+sq+uk, no TODO — scripts/i18n-parity.ts "OK: 1079 keys × 3 locales", exit 0.
[PASS] Parity gate red→green — GREEN (exit 0) → blanked one uk → FAIL (exit 1) → restored → GREEN (exit 0).
[PASS] i18n-add helper — added EN-only key → sq/uk became TODO drafts → gate FAILED (exit 1); full 3-locale add → gate PASSED.
[PASS] Dev-only missing-key warning — i18n.ts:31-36 (guarded by `(import.meta as any)?.env?.DEV`).
[PASS] No call-site churn — git diff: only i18n.ts + i18n-catalog.ts + scripts + docs + .husky; "✓ no call sites touched".
[PASS] Regression-ledger row 9 added (gate guardrail, red→green documented).
[PASS] Enforcement wired — .husky/pre-commit runs the parity gate when i18n files are staged (real "never miss").
[N/A]  UI Playwright — no visible UI change (byte-equivalent strings); covered by the equivalence proof + typecheck.
[FLAG: escalate] package.json `i18n:add`/`i18n:parity` aliases — package.json protect-paths-blocked → docs/operating-model/proposed-package-json/APPLY.md for operator.
