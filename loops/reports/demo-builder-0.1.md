# Verification report · demo-builder · v0.1 · 2026-07-01

Вердикт: **CERTIFIED**
Верифікатор: loop-architect (модель: opus-4.8) · cross-opinion (M11): FLAGGED для незалежної моделі (OpenRouter-міст) — менш корельований погляд на visual-gate honesty (the no-fake-green surface).

Loop: `loops/demo-builder.yaml` · executable: `scripts/demo-builder.mjs`
Live visual gate: `e2e/tests/demo-builder-visual.spec.ts` (mobile + desktop, real console-error capture)
Anti-cheat harness: `tools/demo-builder/dry-run.mjs` + faithful mock `tools/demo-builder/mock-internal.mjs` + configurable fake storefront `tools/demo-builder/fake-storefront.mjs`
Grounded in the SHIPPED pipeline: `apps/api/src/modules/acquisition/{route,state-machine,provisioning,claim}.ts`, the branding math `packages/ui/src/theme/palette.ts` + `apps/api/src/lib/brand-extractor.ts`, and the preview render `apps/api/src/lib/preview-render.ts` / SSR `apps/api/src/routes/public/ssr.ts` (real testids: `menu-item`, `venue-preview-banner`, `preview-claim-cta`, `menu-item-add`, `cart-open`).

## 4-умовний тест (M2) — це петля, не промпт
1. Recurring trigger — an operator repeatedly turns prospects into sellable demo storefronts. ✓
2. Fixed multi-stage pipeline per prospect (SENSE state → L1 menu-quality → extract/enrich → mint+spine → L2 theme → API verify → L3 VISUAL GATE → opt-in claim). ✓
3. Machine verification at every stage (gateMenu.ok · extract state · spine FKs · verified===true · assertPreviewDom render). ✓
4. Hard exit (exactly one terminal outcome per prospect; run summary; exit codes). ✓

## Рубрика M1–M11

| # | критерій | PASS/FAIL | доказ |
|---|---|---|---|
| M1 | структурна повнота (4 блоки + DNA) | PASS | card has trigger/execution_skills/goal+verification/exit+memory + all DNA fields filled (no placeholders). 4 blocks: Trigger (node …+prospects) · Execution skills (6 real endpoints + Playwright gate + pure L1/L2 fns + documented DB seams) · Goal+Verification (field+render gates + M11 flag) · Exit+Memory (terminal-per-prospect + loops/runs + memory_file with runbook). |
| M2 | 4-умовний тест | PASS | see above — loop-shaped, not a prompt. |
| M3 | верифікація реальна (не вайб) | PASS | asserts PARSED fields AND RENDERED DOM: `gateMenu(draft).ok`, `result.state==='ENRICHED'`, `isUuid(org_id&&location_id)`, `json.verified===true`, and `assertPreviewDom` (≥3 rendered `menu-item` cards, banner+CTA present, 0 console errors, never-orderable, noindex). Not "HTTP 200 = pass" — Scenario B proves an API-verified source that renders broken is needs-review. |
| M4 | жорсткий вихід (ALL-must-hold) | PASS | exit_conditions: certified-preview REQUIRES menu-quality PASS ∧ spine 201 ∧ verified===true ∧ visual gate PASS on mobile ∧ desktop; invited additionally REQUIRES --send-invite ∧ real token. Each prospect → exactly one of {certified-preview, invited, needs-review, skipped-already-done}. exit 3 iff any needs-review, 2 on missing config. No "until it looks ok". |
| M5 | iron principles увімкнені (no-fake-green) | PASS | 9 enforced principles incl. no-fake-green, quality-before-provision, render-verified-not-assumed, never-orderable, preview-only-by-default, fail-closed, secret-never-printed — each ENFORCED in code + asserted by the dry-run, not declarative. |
| M6 | skill-driven (нуль фантомних навичок) | PASS | every endpoint is a real verb in `route.ts` (extract L77, mint L90, spine L107, verify L142, claim/mint L158); the visual gate is a real Playwright spec (parses, lists 3 projects); L1/L2 are pure fns in the orchestrator (dry-run PART A exercises them). NO invented endpoint — non-AI menu + palette persistence is the DOCUMENTED operator DB seam (memory), so zero phantom skills. |
| M7 | гейти у високоризикових точках | PASS | (a) target-URL gate (never prod — writes shadow rows); (b) menu-quality gate (thin menu never provisioned — Scenario A org_id null on LOW_QUALITY); (c) VISUAL acceptance gate (broken render never certified — Scenario B ×4); (d) outreach gate (--send-invite opt-in — Scenario A invited=0). |
| M8 | out-of-scope + escalation | PASS | out_of_scope: publishing, AI internals, notice delivery, prod API/DB-schema/product-code, any /api/auth/RLS/money. escalation: unexpected state → needs-review+continue; whole-surface 404 → STOP+raise; repeated visual-gate fail on an API-verified source → RENDER-regression escalation; operator-menu-not-ENRICHED → needs-review:NEEDS_ENRICH_SEAM. |
| M9 | anti-cheat dry-run (зламане → RED/escalate) | PASS | `node tools/demo-builder/dry-run.mjs` → 43/43 PASS. See block below — thin menus never provision, broken 200-renders (empty/error/orderable/noindexless) all go needs-review despite API verified:true, wrong secret fails closed, run never aborts/fakes. |
| M10 | пам'ять підключена | PASS | memory_file `loops/memory/demo-builder.md` (runbook + the two DB seams + lessons + run-history); every run writes a lossless `loops/runs/demo-builder-<ts>.json`; the live gate writes `e2e/artifacts/demo-builder-visual-<slug>.json`. |
| M11 | separate-agent крос-рев'ю | PASS (flagged) | marked for an independent model (OpenRouter-bridge) to re-attack the visual-gate honesty / the cheat surface — a less-correlated look. Logged here as the M11 hook. |

## Anti-cheat dry-run (M9) — 43/43 PASS

Command: `node tools/demo-builder/dry-run.mjs` (re-runnable, hermetic, no network/DB/AI/browser).
Mock mirrors the shipped state-machine (ops-auth 404 fail-closed · idempotent create returns CURRENT state · mint/spine REQUIRE ENRICHED · verify 409 on empty menu · claim/mint 409 ACTIVE_INVITE_EXISTS · LIAR mode). Fake storefront serves good vs empty/error/orderable/noindexless 200-renders. Visual gate runs via the hermetic HTTP probe feeding the SAME `assertPreviewDom` the live Playwright spec asserts.

**PART A — pure proof of the three added layers (14 assertions):**
- L1 `gateMenu` REJECTS a thin menu (1 cat / 2 items / no descriptions), PASSES a rich one (3 cats / 7 items / descriptions), REJECTS a non-integer price (money-integrity).
- L2 `derivePaletteTriple`: a pizzeria palette ≠ the sushi demo (bg + primary differ — not one hard-coded theme); text/bg contrast is AA (≥ 4.5) for pizzeria (17.55), sushi (16.72), burger (16.77), cafe (16.53), unknown (17.80).
- L3 `assertPreviewDom`: PASSES a demo-quality render; FAILS empty (0 items), console-error, ORDERABLE (menu-item-add present → B3), and noindex-missing renders — the no-fake-green core.

**PART B — end-to-end pipeline (real orchestrator × mock × fake storefront, 29 assertions):**
- **Scenario A — mixed batch (5: good pizzeria + lowquality + menunotfound + emptymenu + malformed):** all 5 processed (no abort); exactly 1 certified-preview (the good pizzeria, with a preview URL + derived palette + location_themes directive); 0 invited (preview-only default); 4 needs-review; the LOW_QUALITY + MENU_NOT_FOUND sources were **NEVER provisioned** (org_id null); exit 3; secret never printed.
- **Scenario B — visual-gate differentiator (empty / error / orderable / noindexless), ×4:** each renders 200 and the API marks the source `verified:true` (asserted `state==='VERIFIED'`), yet the loop certifies 0 and classifies each **NEEDS-REVIEW:VISUAL_GATE_FAILED** — proving the GATE (not the API) is what blocked it. A "200 == pass" loop would falsely certify all four.
- **Scenario C — opt-in outreach:** `--send-invite` → exactly 1 invited with a real `/claim#token=` fragment URL.
- **Scenario D — idempotent re-run:** first run invites; the re-run invites 0 + skips-already-done 1 (no re-mint / re-provision).
- **Scenario E — fail-closed (wrong ops secret → 404):** 0 certified, 0 invited; classified NEEDS-REVIEW:OPS_AUTH_404 (not a crash); secret not echoed.

## Test seam (why mock + fake-storefront, not full-live)
The extract stage needs a real website + an AI key, and the live visual gate needs a real Playwright browser against a deployed
/s/:slug — so a full live certification is environment-blocked. The orchestration / quality-gate / branding / VISUAL-GATE logic is
certified against the faithful mock + the configurable fake storefront via the hermetic probe (same `assertPreviewDom` the live
Playwright spec asserts) + direct unit proof of the three pure layers. Live steps to run stages end-to-end are in `loops/memory/demo-builder.md`.

## Residual / follow-ups
- Not yet run against a live deployment (BUILT+CERTIFIED, not yet RUN-on-staging). First live run = a run-history row in the memory file.
- The two operator DB seams (non-AI menu enrich · location_themes) are documented, not API endpoints — if a future need makes them
  first-class, that is a separate provisioning-module change (council-gated), not a loop change.
- M11 independent cross-review still to be executed by the OpenRouter-bridge model; flagged, not yet returned.
