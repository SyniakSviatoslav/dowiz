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
### iteration 1 — (in progress)
- Grounding before run: admin Orders (m) buries the first order card to ~73% down the viewport
  (welcome banner + 5-tile KPI grid); `0 Në pritje` KPI is alert-amber; order-status filter chips
  still ad-hoc (not SegmentedControl).
