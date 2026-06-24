# UI Build-Verification Loop — Run 2026-06-24 (Layer-4 vision, parallel agents)

First live run of the [UI Build-Verification Loop](../../docs/operating-model/ui-build-verification-loop.md).
Layer 4 (agent-as-eye) executed by **4 parallel Claude subagents**, each reviewing one rendered screen
from staging (`https://dowiz-staging.fly.dev`) against the A–F rubric. Screenshots captured at 390/1280,
`reducedMotion`, locale SQ. Triage, not a phase verdict.

## Findings (routed per the matrix)

| # | Screen | Finding | Dim | Severity | Route | Status |
|---|--------|---------|-----|----------|-------|--------|
| 1 | all (StateChip) | `state.open/closed/busy/sold_out/available` missing from catalog → venue badge + sold-out/availability chips render **English to SQ/UK users** | E | high | **inline** | ✅ FIXED — added via `i18n-add` (5 keys, parity green) |
| 2 | storefront 390/1280 | `Test-Cat-1782148864348` test category leaks into the public menu tabs | E | high | **data-cleanup** | routed (demo seed data, not UI) |
| 3 | storefront 1280 | desktop menu is a single narrow column stretched into 1280px — one card/row, large empty right | D/F | high | **frontend-gate** | routed (responsive grid; phase-level layout) |
| 4 | storefront 390 | add `+` button overlaps price/name and clips at card bottom-right | F | high | **frontend-gate** | routed (card layout; verify in current code on deploy) |
| 5 | storefront 390 | filter/sort chip row clipped at right edge ("SOJE" sliced); weak horizontal-scroll cue | D | med | frontend-gate | routed |
| 6 | product modal 390 | add-to-cart CTA "Shto në …" truncated by the inline price chip | E | med | **inline** | routed (FE; confirm key/layout in current code) |
| 7 | not-found 390 | invalid slug rendered as transient "retry" state, no escape to home/search; copy implies temporary failure | spec | med | frontend-gate | routed |
| 8 | `/admin/login` 390 | renders **blank** — 0 inputs, 0 buttons, empty root (48 chars), no console errors after networkidle+3s | — | high | **investigate** | routed (candidate regression; DOM probe + dark screenshot as proof) |

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
