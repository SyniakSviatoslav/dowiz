# Verification — fee-courier-seed (council batch, all 3 items)

Branch `fix/design-system-consistency`. Deployed + verified on `dowiz-staging`. 2026-06-25.

## Item 1 — delivery fee = server mirror (ADR-0005, Approach M)
Commits `82c05314` (+ ledger `4dbbba2a`).
- **Parity guardrail (PRIMARY ship-blocker) — GREEN 5/5.** `apps/api/tests/fee-parity.test.ts` asserts the
  client mirror (`packages/ui/src/lib/money.ts`) equals the real server `applyTax` + fee ladder across a
  subtotal×tax×threshold matrix, incl. the free-over-2000 boundary the hardcode broke and the pickup/min-order
  gates. Run: `(cd apps/api && node --test --import tsx tests/fee-parity.test.ts)`.
- **`/info` contract — VERIFIED on staging.** `GET /public/locations/demo/info` →
  `{deliveryFeeFlat:200, freeDeliveryThreshold:2000, minOrderValue:500, taxRate:0, priceIncludesTax:true, hasDistanceTiers:true}`.
- **Checkout UI — VERIFIED (`audit/item12-verify/m-client-checkout.png`).** Demo is distance-tiered → the
  CTA shows the subtotal ("Porosit • 3000 ALL"), NOT the old hardcoded subtotal+200, and the fee row degrades
  to "calculated at checkout" with a "+ delivery fee" note (the council-approved honest path for tiered venues).
  The `CASH_AMOUNT_TOO_LOW` 422 handler is the door-handover-parity backstop.
- Server money math UNCHANGED (the charged amount never moved). Approach R deferred (YAGNI).

## Item 2 — courier status honesty (ADR-0006)
Commits `d009553e`, `8927c092`.
- **VERIFIED (`audit/item12-verify/m-admin-couriers.png`):** header reads "**60 aktivë**" (active accounts),
  NOT "60 online"; each phone-less seed courier shows the labelled "**Aktiv**" pill, NOT a faked green "Online".
- Avatar presence dot removed (a green dot reads as live presence the owner endpoint can't prove). Map markers
  neutral. FE-only; no contract/state-machine change. Option B (real presence from `courier_shifts`) deferred.

## Item 3 — hardened encrypted dev-seed (ADR-0006 §Item3, DEV-ONLY)
Commit `d619ea5f`.
- **VERIFIED (`audit/item3-verify/m-courier-delivery.png`):** the LIVE courier active-delivery view now renders
  (was the not-found state) — destination address, "~15 min" ETA, Telefono, messages, "Shënoni si të Marrë" CTA.
  The synthetic encrypted courier + shift + assignment seed works; closes the mobile-polish iter-3b capture gap.
- All 5 council constraints in code: synthetic-only re-derived mint · idempotent seed · sentinel email-hash ·
  `.test`-TLD reject at every `email()` site · synthetic excluded from owner counts. Prod-safe (ADR-0003 dev gate;
  zero new prod surface; verified `/info` synthetic courier is NOT in the "60 aktivë" count).

## Residual / accepted (council-owned)
- NEW-H1 (reviewed vs committed total, 30s `/info` cache + MVCC) — accept-risk, backstopped by the cash-422.
- Parity test → CI needs a `package.json` script (protected zone → manual approval). The test passes standalone.
- Approach R (single shared money source) + Option B (courier presence) deferred per the resolution.
