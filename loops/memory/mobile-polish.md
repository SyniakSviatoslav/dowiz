# Loop memory — mobile-polish

Per-run learnings for the mobile (390px) polish loop. Append one block per run.

## Conventions
- SENSE uses `e2e/tests/capture-states.spec.ts` → `audit/mobile-polish-iN/` (the `-m` shots).
- DIAGNOSE grades each `-m` shot vs the Mobile Rubric (see `loops/mobile-polish.yaml`). **verify-before-fix**:
  tag each finding real / artifact / flag-only with evidence before any edit.
- ACT: FE-only, token-conformant; reuse shared atoms; flag logic/contract/security.

## Standing gotchas (carry forward)
- The Playwright sandbox **cannot fetch jsdelivr** — but icons are now self-hosted (bundled), so `-m`
  captures DO render icons. If a capture shows blank icons, suspect a NEW CDN ref, not the components.
- Visual contrast/size estimates are unreliable — confirm AA by computing, sizes by reading the class.
- Demo seed data (E2E/QA names, blank thumbs) is a known data issue, NOT a mobile bug → flag-only.

## Runs
### iteration 1 — DONE (commit fe37616f, deployed + verified on staging)
- Grounding: admin Orders (m) buried the first order ~73% down (welcome banner + 5-tile KPI grid);
  `0 Në pritje` KPI alert-amber; chip rows clip with no cue.
- DIAGNOSE (3 vision agents) → docs/design-review/MOBILE-POLISH.md. ~50% of raw findings were
  ARTIFACT/FLAG-ONLY (seed data, no food photos, fullPage-capture "sticky overlap").
- FIXED (verified-real): (1) iOS zoom — shared form controls → `text-base md:text-sm`;
  (2) KPI grid → single horizontal-scroll strip on mobile (1 row, not 2) → first order moved up;
  (3) zero-value KPI → neutral `--brand-text-muted`; (4) `.scroll-fade-x` mask utility (+ `no-scrollbar`
  alias that was missing) on SegmentedControl + storefront tabs/sort + Dashboard KPI/filter rows.
- VERIFIED on m-admin-orders: KPI now one fading scroll row, `0` neutral, order card higher.
- **Key lesson:** `StickyActionBar` already pads `var(--safe-bottom)` + checkout has pb-32 → the
  "sticky CTA overlaps content" finding is a fullPage-screenshot ARTIFACT, not a real overlap.
  Also: `no-scrollbar` was used in components but NOT defined in CSS — now aliased.
- DEFERRED to iter 2: CRM toolbar stack (search→full-width row), couriers "+ Shto Postier" → bottom
  FAB, tap-target ≥44px sweep (row icons ~36px), courier empty-state vertical-centering, lang→dropdown.
