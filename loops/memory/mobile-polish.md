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

### iteration 2 — DONE (commit 3f6f2204, deployed + verified on staging)
- FIXED (verify-real): CRM toolbar search/sort/export → full-width stack on mobile (`flex-col sm:flex-row`);
  tap targets — menu edit/delete + couriers modal-close → `w-11 h-11 sm:w-8 sm:h-8` (44px touch / 32px visual).
- VERIFIED `m-admin-crm`: toolbar stacks full-width, search shows its placeholder (no longer an icon square).
- **Lessons:** row chevrons (CRM/Couriers) are NOT separate small targets — the whole ~74px row is the
  button (agent misread). Lang pills → dropdown REJECTED (1-tap quick-switch > 2-tap dropdown for 3 langs).
  Courier empty-state "void" = sparse page (content above the empty card), not a defect.
- Report: loops/reports/mobile-polish-0.1.md. Rubric PASS/improved on all 8 items; remaining = documented
  low-value/UX deferrals. Loop effective; M1–M11 certification offered as a separate step.

### CERTIFIED 2026-06-25 (loop-architect) — M1–M11 all PASS → v0.1 DRAFT promoted to v1.0 CERTIFIED (registry + card updated; report stamped).
