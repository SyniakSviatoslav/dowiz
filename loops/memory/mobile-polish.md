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

### iteration 3 — DONE (commit f5896b5e, deployed + verified)
- Extended SENSE to the 2 surfaces the main harness can't reach (need a live order):
  new `e2e/tests/capture-delivery.spec.ts` seeds an order (`/dev/seed-visual-state`) → assigns it to
  a fresh mock courier (`/dev/create-assignment`; the delivery route param is the ASSIGNMENT id, not
  the order id) → captures `/courier/delivery/:asgnId` + `/s/:slug/order/:id` at 390px.
- REVERSED an earlier mistake: the courier "stranded card + dead void" was deferred as a "sparse-page
  artifact" in iter 1–2. A 3rd confirmation (delivery not-found + tracking session-expired, both
  full-page) proves it's a REAL cross-cutting defect. Fix: `<EmptyState fullPage>` (min-h-[68dvh] flex
  center) + meaningful icons, on courier DeliveryPage/TasksPage + client OrderStatusPage full-page states.
  VERIFIED `m-client-tracking`: card now vertically centred.
- **Capture limitation (not a bug):** the LIVE active-delivery view couldn't be seeded into a
  renderable state (needs specific order/shift status) — the not-found state is what rendered.
- **Lesson:** a deferral labelled "artifact" must be re-tested when the same pattern recurs on a new
  surface — repetition flips "sparse page" into "real cross-cutting defect."

### iteration 3b — live active-delivery capture: HARD INFRA LIMIT (traced to root, not built)
- Goal: render `/courier/delivery/:asgnId` LIVE (not the not-found state). Traced the full chain:
  - seed returns ids NESTED under `open`/`closed`/`busy` (use `seed.open.locationId` + `seed.open.slug`,
    not top-level — fixed in `capture-delivery.spec.ts`).
  - `/dev/create-assignment` 500s: `courier_assignments.courier_id → couriers(id)`, and the mock courier
    (random uuid) / `VIS_COURIER_ID` have NO row in `couriers`. The seed only creates an OWNER user.
  - `couriers` requires encrypted PII (`email_encrypted`/`full_name_encrypted` bytea, `email_hash`,
    `password_hash`) → seeding a courier needs the app's crypto/argon2 pipeline. Too heavy for a 390px
    re-capture of a view already audited this session (the recapture pass fixed its {{minutes}} token bug,
    state/CTA mismatch, missing map). **Documented as a capture limitation; not built.**
  - WHAT DID get captured/verified: the delivery NOT-FOUND state — now centered + iconed (iter-3 fix holds
    on this surface too). To enable the live view later: enhance the seed to UPSERT an encrypted courier +
    courier_shift + courier_assignment for the seeded order, and let `/dev/mock-auth` impersonate it.

### iteration 3b → RESOLVED 2026-06-25 (via Triadic Council "fee-courier-seed", Item 3, commit d619ea5f)
- The "build the encrypted courier seed" follow-up was taken through the council (it touched money/contract/
  state-machine alongside two other changes) and built HARDENED: synthetic-only RE-DERIVED mock-auth mint
  (NOT arbitrary courierId — that was the dev-login-backdoor shape), idempotent seed, namespaced sentinel
  email-hash, `.test`-TLD reject, synthetic excluded from owner counts. See docs/design/fee-courier-seed/.
- The LIVE active-delivery view now renders at 390px (audit/item3-verify/m-courier-delivery.png): destination
  + "~15 min" ETA + Telefono + "Shënoni si të Marrë" CTA. **The iter-3b capture limitation is closed.**
- **Lesson:** a "hard infra limit" can become tractable when it's worth routing through the proper gate —
  the council turned a risky `body.courierId` impersonation into a safe synthetic-only fixture.

### 2026-06-30 · REUSE run — post-redesign QA gate (storefront+admin, characteristics flags ON)
- Dispatched by loop-orchestrator after the big storefront redesign + vendor-zone + allergen-freeze.
- SENSE: mobile(390)+desktop(1280) over 10 surfaces → `audit/qa-loop/`. DIAGNOSE vs Mobile Rubric + checklist
  (allergens-frozen, i18n sq/en/uk, console, persistence). **8/10 PASS**, no logic/contract/security escalations.
- FIXED (verified-real, FE-only): **F1** compare-toggle (`absolute top-1.5 left-1.5`, ~36px) CLIPPED photoless
  card titles ("Cola"→"ola") on both bp → ProductCard gained `compareGutter` prop reserving a left gutter on
  the photoless title/badge row; MenuPage passes `compareGutter={COMPARISON_ENABLED}`. **F2** product modal
  (role=dialog) had NO Escape handler → added a keydown Esc→closeDetail effect.
- FLAG-ONLY: F3 add-to-cart label truncates beside price (by-design); F4 one product image 404 (Margherita
  stale pre-R2 image_key) = data.
- **Lessons:** (1) an absolute-positioned card overlay WILL clip text on photoless cards — reserve a gutter.
  (2) every role=dialog needs an Esc handler (grabber/X/backdrop aren't enough for a11y). (3) the big-redesign
  batch held up (allergens frozen, i18n parity, persistence, compare-no-verdict all PASS first try). (4) ui/dist
  is gitignored → a new ProductCard prop needs `pnpm --filter @deliveryos/ui build` before apps/web tsc sees it.
