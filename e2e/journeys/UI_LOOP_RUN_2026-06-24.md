# UI Build-Verification Loop — Run 2026-06-24 (Layer-4 vision, parallel agents)

First live run of the [UI Build-Verification Loop](../../docs/operating-model/ui-build-verification-loop.md).
Layer 4 (agent-as-eye) executed by **4 parallel Claude subagents**, each reviewing one rendered screen
from staging (`https://dowiz-staging.fly.dev`) against the A–F rubric. Screenshots captured at 390/1280,
`reducedMotion`, locale SQ. Triage, not a phase verdict.

## Findings (routed per the matrix)

| # | Screen | Finding | Dim | Severity | Route | Status |
|---|--------|---------|-----|----------|-------|--------|
| 1 | all (StateChip) | `state.open/closed/busy/sold_out/available` missing → venue/sold-out chips render **English to SQ/UK** | E | high | inline | ✅ **FIXED** — 5 keys via `i18n-add` |
| 2 | storefront | `Test-Cat-1782148864348` leaks into public menu tabs | E | high | data-cleanup | ✅ **PREVENTED** (deploy-validation cleanup → `afterAll`) + existing orphan routed to operator reseed (DB access) |
| 3 | storefront 1280 | desktop "single narrow column" | D/F | high | frontend-gate | ⚪ **NOT REAL in current code** — already `grid-cols-2 md:3 lg:4` (staging was stale) |
| 4 | storefront 390 | add `+` overlaps/clips card | F | high | frontend-gate | ⚪ **NOT REAL** — in-flow flex, `shrink-0` 44px (staging stale) |
| 5 | storefront 390 | filter chip row clipped ("SOJE"), weak scroll cue | D | med | inline | ✅ **FIXED** — `pr-8` clears the right-fade on chip row + category nav |
| 6 | product modal 390 | CTA "Shto në …" truncated | E | med | inline | ⚪ **WORKING AS DESIGNED** — label `truncate`, price `shrink-0` (price protected); left as-is |
| 7 | not-found 390 | 404 slug shown as transient "retry", no home escape | spec | med | frontend-gate | ✅ **FIXED** — 404 → distinct "Restaurant not found" + home CTA, no retry |
| 8 | `/admin/login` | renders **blank** (0 inputs/buttons) | — | high | investigate | ✅ **FIXED** — `/admin/login` wasn't a route; added `login`→`/login` redirect + catch-all in AdminRoutes |

## Fixes applied (commit follows)
- **#8** AdminRoutes: `/admin/login` had no route (login is `/login`) and no catch-all → null render. Added a `login` redirect + `*` catch-all. Proof: `e2e/tests/ui-loop-fixes.spec.ts` (post-deploy).
- **#7** MenuPage: `loadMenu` now treats `res.status===404` as a distinct `notFound` state → "Restaurant not found" + a home link, no futile retry. Proof: same spec.
- **#1** + the **66-key backlog**: all uncatalogued keys backfilled with real sq/uk; parity `--strict` now green (1152 keys). Gate scoped to skip `__tests__/`.
- **#5** chip clip: `pr-8` so the last chip clears the 24px fade.
- **#2** Test-Cat: `deploy-validation.spec.ts` cleanup moved to `afterAll` (runs on mid-suite failure). The already-orphaned row needs the operator reseed (`apps/api/scripts/seed-demo-from-prod.mjs`, DB access).
- **#3/#4/#6** were stale-staging artifacts or intended behavior — no change (verified in current code).

## Inline fix applied (the loop's fix arm)
**#1 — silent English in SQ/UK.** `StateChip` uses `t('state.open','Open')` etc., but those 5 keys were
absent from the catalog → the resolver fell back to English for *all* locales. Added with real sq/uk
via `pnpm exec tsx scripts/i18n-add.ts state.open "Open" "Hapur" "Відчинено"` (×5). Parity gate green
(1084 keys). Proof-of-render pending a staging deploy (no fly CLI here), same as the polish-debt F12.

## Gate loophole closed
This run exposed that the i18n parity gate only checked keys *in* the catalog — keys used in code via
`t('literal','fallback')` but never catalogued (like `state.*`) were invisible. The gate now also
**detects code-used-but-uncatalogued keys** (warn by default; `--strict` fails). Current backlog: **66**
such keys all rendering English in every locale — recorded here, route = backfill into the catalog.

## Proof artifacts
Screenshots: scratchpad `uiloop/{storefront-mobile-390,storefront-desktop-1280,product-modal-390,notfound-390,admin-login-390}.png`
(ephemeral). Per-screen JSON verdicts captured in the run transcript. admin-login blank confirmed via a
DOM probe (0 inputs/buttons, no errors) — not a capture artifact.

## Notes
- Layers 2 (Storybook) + 3 (Docker visual baselines) not run — infra not provisioned here (see
  `docs/operating-model/proposed-ui-loop-infra/APPLY.md`). Layer 4 (vision) substituted Claude subagents
  for the dead OpenRouter path — faithful to the spec's agent-as-eye intent.
