# CHANGE-MANIFEST

CLASSIFICATION: build   # one of: spike | build | audit | challenge  (§1 — routes the governance mode)

FINDING-id: fe-polish-qa-loop-2026-06-23
Intent: fix — apply the FE-polish + QA loop findings (cosmetic tap targets inline; one contract bug).

Touched files:
- packages/ui/src/components/atoms/{Button,Input,SunlightToggle,CurrencySwitcher}.tsx — tap targets
  to ≥44px (WCAG 2.5.5): md/lg Button + Input + SunlightToggle (36→44) + CurrencySwitcher (30→44).
- apps/api/src/routes/orders.ts — PATCH /orders/:id/status used a bare StatusUpdateInput.parse()
  outside the try → a bad `status` enum threw a ZodError the global handler didn't normalize → raw
  500. Switched to .safeParse() → typed 400 (matches the create route; client input ≠ 5xx).
- e2e/tests/polish-qa-loop.spec.ts — proof (login controls ≥44px; PATCH bad-enum → 400).

Proof: ui rebuilt, web+api typecheck green; Playwright polish-qa-loop on staging.

FLAG-only (not patched — per the QA/FE loops): map null-coord console warnings (3rd-party tile
style, not our coords); logo.webp 404 (stale demo location logo_url — FE fallback already correct);
notifications/status returns duplicated channel rows (LOW contract noise, separate fix); PATCH-500
guardrail/regression carried by the spec. Filter-pill h-9 + KPI-zero mute left as deliberate LOW.

# Reminder (§5): a well-proven FAIL / MISSING / BLOCKED is a SUCCESSFUL run, equal to PASS.
